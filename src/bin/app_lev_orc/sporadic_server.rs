/***************************************/
/*    S P O R A D I C   S E R V E R    */
/*         ( I N S T A N C E )         */
/***************************************/

use crate::state::{ApplicationState, Request};
use sporadic_server;
use sporadic_server::{SporadicServer, SporadicServerController};
use crate::main;

/// Once a node accepts a request, the request
/// is executed by a thread.
pub struct ControlSystem
{
    /// Index of the application.
    application_index : usize,

    /// Budget of the sporadic server task.
    budget            : u64,

    /// Period of the sporadic server task.
    period            : u64,

    /// Priority of the sporadic server task.
    priority          : usize,

    /// Affinity of the sporadic server task.
    affinity          : usize,

    /// Directory with the requests files.
    request_directory : String,
}

impl ControlSystem
{
    pub fn new (application_index: usize,
                budget           : u64,
                period           : u64,
                priority         : usize,
                affinity         : usize,
                request_directory: String) -> Self
    {
        Self
        {
            application_index,
            budget,
            period,
            priority,
            affinity,
            request_directory,
        }
    }

    pub fn start (&mut self,
                  application_state : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
                  barrier           : std::sync::Arc<(std::sync::Mutex<u8>, std::sync::Condvar)>,
                  checkpoint_barrier: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>)
    {

        #[cfg(feature = "print_log")]
        println! ("sporadic_server - STARTED");

        // First, configure a sporadic server controller.
        // Its purpose is to respond to timing event related to
        // the sporadic server task (namely replenishment and
        // budget exceeded).

        // State of the server task.
        let is_server_running =
            std::sync::Arc::new ((std::sync::Mutex::new (false), std::sync::Condvar::new ()));

        // The control object.
        let controller : std::sync::Arc<std::sync::Mutex<SporadicServerController>>  =
            std::sync::Arc::new (
                std::sync::Mutex::new (
                    SporadicServerController::new(barrier.clone (),
                                                  is_server_running.clone ())
                )
            );

        // Then configure the sporadic server task.
        let mut server =
            SporadicServer::new(std::time::Duration::from_millis (self.budget),
                                std::time::Duration::from_millis (self.period),
                                self.priority as u32);

        // Finally start the controller and server threads.
        let mut handles = vec![];

        // Controller thread.
        let priority : usize = self.priority;
        let affinity : usize = self.affinity;
        let crl_controller = controller.clone ();
        let controller_handle = std::thread::spawn (move ||
            {
                set_linux_sched (std::cmp::max (priority + 15, 89), affinity);
                SporadicServerController::start (crl_controller, is_server_running.clone ());
            });
        handles.push (controller_handle);

        // Server thread.
        let mut workload = WasmWorkload::new (self.application_index,
                                                            self.request_directory.clone (),
                                                            application_state.clone (),
                                                            checkpoint_barrier.clone ());
        let srv_controller = controller.clone ();
        let server_handle = std::thread::spawn (move ||
            {
                set_linux_sched (priority, affinity);
                server.start (srv_controller, &mut workload);
            });
        handles.push (server_handle);

        for handle in handles
        {
            handle.join ().unwrap ();
        }

        /* 'main_loop: loop
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





            #[cfg(feature = "print_log")]
            println! ("sporadic_server - JOB COMPLETE");

            // Update the guard.
            {
                let (number_of_requests, _barrier) = &*barrier;
                *number_of_requests.lock().unwrap () -= 1;
            }
        }  */
    }
}

// To use the sporadic_server crate, we should first
// provide an implementation of a Workload.
struct WasmWorkload
{
    /// Index of the application.
    application_index : usize,

    /// Path to the request directory.
    request_directory : String,

    /// The state of the application.
    application_state: std::sync::Arc<std::sync::Mutex<ApplicationState>>,

    /// Whether or not a checkpoint is ready.
    checkpoint_barrier: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,

    /// The current request being served.
    current_request   : std::option::Option<Request>
}

impl WasmWorkload
{
    fn new(application_index : usize,
           request_directory : String,
           application_state : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
           checkpoint_barrier: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>) -> Self
    {
        Self
        {
            application_index,
            request_directory,
            application_state,
            checkpoint_barrier,
            current_request: None,
        }
    }
}

impl sporadic_server::Workload for WasmWorkload
{
    fn exec_workload(&mut self) {

        match self.application_state.lock ().unwrap ().requests.first ()
        {
            None =>
                {

                    // We reach this point if the number of requests was greater than 0
                    // during `wait_for_activation ()' but then got empty before checking
                    // `requests'.
                    // TODO: possible extension.
                    // in would be possible to move the current served request
                    // out of the pending queue, to avoid this issue.
                    // For now, we can simply terminate the function, and re-run
                    // `wait_for_activation ()'.
                    #[cfg(feature = "print_log")]
                    println! ("sporadic_server - requests.is_empty (). ");
                    return;
                }
            Some (&request) =>
                {
                    self.current_request = Some(request);
                }
        }

        let &current_request = self.current_request.as_ref ().unwrap ();

        struct MyState
        {
            wasi              : wasmtime_wasi::preview1::WasiP1Ctx,
            application_state : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
            request_index     : usize,
            main_memory_file        : Option<String>,
            checkpoint_memory_file  : Option<String>,
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

                #[cfg(feature = "periodic_activation")]
                println! ("request {} - should_migrate START", request_index);

                {
                    let mut app_state =
                        caller.data_mut ().application_state.lock ().unwrap ();

                    // Update request.current_region.
                    app_state.advance_cur_region_of_request (request_index);

                    // Check if a migration is needed. 
                    if app_state.get_should_migrate_of_request (request_index)
                    {
                        result = 1;
                    }

                    // Drop the mutex variable, forcing unlocking.
                    drop (app_state);
                }

                #[cfg(feature = "periodic_activation")]
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
            .open (&main_memory);
        let main_memory_file = match main_memory_file_r
        {
            Ok (_file) => Some (main_memory),
            Err (_) => None,
        };

        let checkpoint_memory_file_r = std::fs::OpenOptions::new ()
            .create (true)
            .write (true)
            .open (&checkpoint_memory);
        let checkpoint_memory_file = match checkpoint_memory_file_r
        {
            Ok (_file) => Some (checkpoint_memory),
            Err (_) => None,
        };

        let main_mem_export = module.get_export_index ("memory")
            .expect ("Unable to find main_mem_export. ");
        let checkpoint_mem_export = module.get_export_index ("checkpoint_memory")
            .expect("Unable to find checkpoint_mem_export. ");

        #[cfg(feature = "periodic_activation")]
        let request_index = current_request.get_index ();

        // Add the restore_memory, which do nothing right now.
        linker.func_wrap ("host", "restore_memory", move |mut caller: wasmtime::Caller<'_, MyState>|
            {

                #[cfg(feature = "periodic_activation")]
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
                    Some (path_to_file) =>
                        unsafe {
                            // Copy the main linear memory from its checkpoint.
                            let main_memory_data : Vec<u8> = std::fs::read (path_to_file)
                                .expect ("Unable to read main_memory.b");
                            for i in 0..main_memory_data.len() {
                                *main_mem_ptr.wrapping_add (i) = main_memory_data[i];
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
                    Some (path_to_file) =>
                        unsafe {
                            // Copy the main linear memory from its checkpoint.
                            let checkpoint_memory_data : Vec<u8> = std::fs::read (path_to_file)
                                .expect ("Unable to read checkpoint_memory.b");
                            for i in 0..checkpoint_memory_data.len() {
                                *checkpoint_memory_ptr.wrapping_add (i) = checkpoint_memory_data[i];
                            }
                        }
                }

                #[cfg(feature = "periodic_activation")]
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
            application_state : self.application_state.clone (),
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
                    std::fs::remove_dir_all (path_to_req_folder).unwrap ();
                    {
                        // Then remove the request from the list.
                        self.application_state
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
                        #[cfg(feature = "print_log")]
                        println! ("sporadic_server - CHECKPOINT occurred");

                        // Notify that the computation is ready to migrate.
                        let (barrier, cvar) = &*self.checkpoint_barrier;
                        *barrier.lock ().unwrap () = true;
                        cvar.notify_all ();
                    }
                    else
                    {
                        // Remove the directory.
                        std::fs::remove_dir_all (path_to_req_folder).unwrap ();
                        {
                            // Then remove the request from the list.
                            self.application_state
                                .lock ()
                                .unwrap ()
                                .remove_request (current_request.get_index ())
                        }
                    }
                }
        }
    }
}

/// Utility function for configuring priority
/// and affinity over a Linux system.
fn set_linux_sched (priority: usize, affinity: usize)
{
    unsafe
        {

            // Scheduling properties.
            let tid = libc::gettid ();
            let sched_param = libc::sched_param
            {
                sched_priority: priority as libc::c_int,
            };
            libc::sched_setscheduler (tid, libc::SCHED_FIFO, &sched_param);

            // Affinity.
            let mut cpuset : libc::cpu_set_t = std::mem::zeroed ();
            libc::CPU_ZERO (&mut cpuset);
            libc::CPU_SET (affinity, &mut cpuset);
            libc::sched_setaffinity (tid, size_of::<libc::cpu_set_t> (), &mut cpuset);
        }
}