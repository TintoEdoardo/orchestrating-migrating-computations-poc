/***************************/
/*     SPORADIC SERVER     */
/***************************/

use std::io::Read;
use crate::state::{ApplicationState, Request};

// Todo: cgroup.

/// Once a node accepts a request, the request
/// is executed by a thread. In this experimentation,
/// cgroup is used to restrict the CPU time consumption
/// for the sporadic server.
pub struct ControlSystem
{
    application_index : usize,
    request_directory : String,
    current_request   : std::option::Option<Request>,
}

impl ControlSystem
{
    pub fn new (application_index    : usize,
                priority          : usize,
                affinity          : usize,
                request_directory    : String,
                ) -> Self
    {

        // Configure a priority and scheduler policy.
        unsafe
            {
                let tid = libc::gettid ();
                let sched_param = libc::sched_param
                {
                    sched_priority: priority as libc::c_int,
                };
                libc::sched_setscheduler (tid, libc::SCHED_FIFO, &sched_param);
            }

        // Fix this computation to a specific CPU.
        let tid : i32;
        unsafe
            {
                let mut cpu_set : libc::cpu_set_t = std::mem::zeroed ();
                libc::CPU_ZERO (&mut cpu_set);
                libc::CPU_SET (affinity, &mut cpu_set);
                tid = libc::gettid ();
                libc::sched_setaffinity (tid, size_of::<libc::cpu_set_t> (), &mut cpu_set);
            }

        Self
        {
            application_index,
            request_directory,
            current_request: None,
        }
    }

    pub fn start (&mut self,
                  application_state    : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
                  barrier              : std::sync::Arc<(std::sync::Mutex<u8>, std::sync::Condvar)>)
    {

        #[cfg(feature = "print_log")]
        println! ("sporadic_server - STARTED");

        'main_loop: loop
        {

            // Extract the next request. If no request is
            // pending, wait until notified.
            {
                let (number_of_requests, barrier) = &*barrier;

                #[cfg(feature = "print_log")]
                println! ("sporadic_server - number of requests = {}", number_of_requests.lock ().unwrap ());

                let _guard =
                    barrier.wait_while (number_of_requests.lock ().unwrap (),
                                        |n_requests|
                                            {
                                                *n_requests == 0
                                            }
                    ).expect ("sporadic_server - panic while starting a new job. ");

                match application_state.lock ().unwrap ().requests.first ()
                {
                    None =>
                        {

                            // It is not obvious that we might end up here:
                            // it means that number_of_requests is != 0, but the vector
                            // of requests is empty.
                            // In this case, skip this job.
                            #[cfg(feature = "print_log")]
                            println! ("sporadic_server - requests.is_empty (). ");

                            continue 'main_loop;
                        }
                    Some (&request) =>
                        {
                            self.current_request = Some(request);
                        }
                }
            }

            #[cfg(feature = "print_log")]
            println! ("sporadic_server - NEW JOB");

            let &current_request = self.current_request.as_ref ().unwrap ();

            struct MyState
            {
                wasi              : wasmtime_wasi::preview1::WasiP1Ctx,
                application_state : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
                request_index     : usize,
                main_memory_file        : Option<std::fs::File>,
                checkpoint_memory_file  : Option<std::fs::File>,
            }

            // Create the engine.
            let engine = wasmtime::Engine::default ();

            // Produce the path to the request folder.
            let path_to_req_folder = format! ("{}/{}_{}_req",
                                          self.request_directory.to_string (),
                                          self.application_index,
                                          current_request.get_index ());
            // Load the module.
            let path_to_module = format! ("{}/{}", path_to_req_folder.to_string (), "module.wasm");
            let module =
                wasmtime::Module::from_file (&engine, path_to_module)
                    .expect ("Failed to load wasm file. ");

            // Create the Linker.
            let mut linker: wasmtime::Linker<MyState>  = wasmtime::Linker::new (&engine);
            wasmtime_wasi::preview1::add_to_linker_sync (&mut linker, |cx| &mut cx.wasi)
                .expect ("add_to_linker_sync failed. ");

            // Add the should_migrate function.
            linker.func_wrap ("host", "should_migrate", |mut caller: wasmtime::Caller<'_, MyState>|
                {
                    let mut result    : i32   = 0;
                    let request_index : usize = caller.data ().request_index;

                    #[cfg(feature = "print_log")]
                    println! ("request {} - should_migrate START", request_index);

                    {
                        let mut app_state =
                            caller.data_mut ().application_state.lock ().unwrap ();
                        let requests = &mut app_state.requests;

                        if requests[request_index].get_should_migrate ()
                        {
                            result = 1;
                        }

                        // Drop the mutex variable, forcing unlocking.
                        drop (app_state);
                    }

                    #[cfg(feature = "print_log")]
                    println! ("request {} - should_migrate END with {}", request_index, result);

                    result
                }
            ).expect ("func_wrap failed. ");

            // Prepare the file for a possible checkpoint.
            let main_memory =
                format! ("{}/{}", path_to_req_folder.to_string (), "main_memory.b");
            let checkpoint_memory =
                format! ("{}/{}", path_to_req_folder.to_string (), "checkpoint_memory.b");

            let main_memory_file_r = std::fs::OpenOptions::new ()
                .create (true)
                .write (true)
                .open (main_memory);
            let main_memory_file = match main_memory_file_r
            {
                Ok (file) => Some (file),
                Err (_) => None,
            };

            let checkpoint_memory_file_r = std::fs::OpenOptions::new ()
                .create (true)
                .write (true)
                .open (checkpoint_memory);
            let checkpoint_memory_file = match checkpoint_memory_file_r
            {
                Ok (file) => Some (file),
                Err (_) => None,
            };

            let main_mem_export = module.get_export_index ("memory")
                .expect ("Unable to find main_mem_export. ");
            let checkpoint_mem_export = module.get_export_index ("checkpoint_memory")
                .expect("Unable to find checkpoint_mem_export. ");

            #[cfg(feature = "print_log")]
            let request_index = current_request.get_index ();

            // Add the restore_memory, which do nothing right now.
            linker.func_wrap ("host", "restore_memory", move |mut caller: wasmtime::Caller<'_, MyState>|
                {

                    #[cfg(feature = "print_log")]
                    println! ("request {} - restore_memory START", request_index);

                    let main_memory = match caller.get_module_export (&main_mem_export)
                    {
                        Some (wasmtime::Extern::Memory (mem)) => mem,
                        _ => panic! ("Failed to find host memory. "),
                    };
                    let main_mem_ptr = main_memory.data_ptr (&caller);

                    let checkpoint_mem = match caller.get_module_export (&checkpoint_mem_export)
                    {
                        Some (wasmtime::Extern::Memory (mem)) => mem,
                        _ => panic! ("Failed to find host checkpoint memory. "),
                    };
                    let checkpoint_memory_ptr = checkpoint_mem.data_ptr (&caller);

                    // Restore the main memory, only if a checkpoint is provided
                    // in the first place.
                    match &caller.data().main_memory_file
                    {
                        None =>
                            {
                                // Do nothing.
                            }
                        Some(file) =>
                            {
                                let index : usize = 0;
                                for byte in file.bytes()
                                {
                                    match byte {
                                        Ok (byte) =>
                                            {
                                                unsafe
                                                    {
                                                        *main_mem_ptr.wrapping_add (index) = byte;
                                                    }
                                            }
                                        _ =>
                                            {
                                                break;
                                            }
                                    }
                                }
                            }
                    }

                    // Same for the checkpoint memory containing the stored variables.
                    match &caller.data().checkpoint_memory_file
                    {
                        None =>
                            {
                                // Do nothing.
                            }
                        Some(file) =>
                            {
                                let index : usize = 0;
                                for byte in file.bytes()
                                {
                                    match byte {
                                        Ok (byte) =>
                                            {
                                                unsafe
                                                    {
                                                        *checkpoint_memory_ptr.wrapping_add (index) = byte;
                                                    }
                                            }
                                        _ =>
                                            {
                                                break;
                                            }
                                    }
                                }
                            }
                    }

                    #[cfg(feature = "print_log")]
                    println! ("request {} - restore_memory END", request_index);

                }
            )
                .expect ("func_wrap failed. ");

            let pre = linker.instantiate_pre (&module)
                .expect ("Instantiate failed. ");

            // Create the Store.
            let wasi_ctx = wasmtime_wasi::WasiCtxBuilder::new ()
                .inherit_stdio ()
                .inherit_env ()
                .build_p1 ();

            let state = MyState
            {
                wasi              : wasi_ctx,
                application_state : application_state.clone (),
                request_index     : current_request.get_index (),
                main_memory_file,
                checkpoint_memory_file,
            };
            let mut store = wasmtime::Store::new (&engine, state);

            // Instantiate the module.
            let instance = pre.instantiate (&mut store)
                .expect ("instantiate failed. ");

            // Invoke the start function of the module.
            let func = instance.get_func (&mut store, "_start")
                .expect ("Unable to find function _start. ");
            let mut result = [];

            #[cfg(feature = "print_log")]
            println! ("sporadic_server - RUN request");

            let function_result = func.call (&mut store, &[], &mut result);

            // Finalize.
            match function_result
            {
                Ok (_) =>
                    {
                        // Remove the directory.
                        // std::fs::remove_dir_all(path_to_req_folder).unwrap();
                        {
                            // Then remove the request from the list.
                            application_state
                                .lock ()
                                .unwrap ()
                                .remove_request (current_request.get_index ())
                        }
                    }
                Err (error) =>
                    {
                        let trap = *error.downcast_ref::<wasmtime::Trap> ().unwrap ();
                        if trap == wasmtime::Trap::UnreachableCodeReached
                        {
                            // Do nothing and pass to the next request.
                        }
                        else
                        {
                            // Remove the directory.
                            // std::fs::remove_dir_all(path_to_req_folder).unwrap();
                            {
                                // Then remove the request from the list.
                                application_state
                                    .lock ()
                                    .unwrap ()
                                    .remove_request (current_request.get_index ())
                            }
                        }
                    }
            }

            // Update the guard.
            {
                let (number_of_requests, _barrier) = &*barrier;
                *number_of_requests.lock().unwrap () -= 1;
            }

            #[cfg(feature = "print_log")]
            println! ("sporadic_server - JOB COMPLETE");

        }
    }
}
