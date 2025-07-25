/***************************/
/*     SPORADIC SERVER     */
/***************************/

use std::io::Read;
use crate::state::{ApplicationState};

// Todo: cgroup and sequential execution of requests.

/// Once a node accepts a request, the request
/// is executed by a thread. In this experimentation,
/// cgroup is used to restrict the CPU time consumption
/// for the sporadic server.
pub struct ControlSystem
{
    request_directory : String,
    application_state : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
    request_index     : usize,
}

impl ControlSystem
{
    pub fn new (request_directory: String,
               application_state : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
               request_index     : usize) -> Self
    {
        Self
        {
            request_directory,
            application_state,
            request_index,
        }
    }

    /// Initialize the priority and affinity of the server. Skipping this step
    /// causes the thread to run as a non-RT task, possibly migrating across
    /// cores at run time.
    pub fn initialize (&self, priority: usize, affinity: usize)
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

        #[cfg(feature = "print_log")]
        println! ("sporadic_server - INITIALIZED");

    }

    pub fn start (&self)
    {

        #[cfg(feature = "print_log")]
        println! ("sporadic_server - STARTED");

        struct MyState
        {
            wasi              : wasmtime_wasi::preview1::WasiP1Ctx,
            application_state : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
            request_index     : usize,
            main_memory_file        : Option<std::fs::File>,
            checkpoint_memory_file  : Option<std::fs::File>,
        }

        // Create the engine.
        let args = std::env::args ().skip (1).collect::<Vec<_>> ();
        let engine = wasmtime::Engine::default ();

        // Load the module.
        let path_to_module = format! ("{}/{}", self.request_directory.to_string (), "module.wasm");
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
            format! ("{}/{}", self.request_directory, "main_memory.b");
        let checkpoint_memory =
            format! ("{}/{}", self.request_directory, "checkpoint_memory.b");

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
        let request_index = self.request_index;

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
            .args (&args)
            .build_p1 ();

        let state = MyState
        {
            wasi              : wasi_ctx,
            application_state : self.application_state.clone (),
            request_index     : self.request_index,
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
        println! ("sporadic_server - RUN");

        let _r = func.call (&mut store, &[], &mut result);
    }
}
