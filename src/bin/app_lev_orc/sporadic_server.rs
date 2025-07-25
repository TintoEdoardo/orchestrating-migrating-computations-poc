/***************************/
/*     SPORADIC SERVER     */
/***************************/

use crate::state::{ApplicationState};

// Todo: cgroup and sequential execution of requests.

/// Once a node accepts a request, the request
/// is executed by a thread. In this experimentation,
/// cgroup is used to restrict the CPU time consumption
/// for the sporadic server.
pub struct ControlSystem
{
    wasm_file         : String,
    application_state : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
    request_index     : usize,
}

impl ControlSystem
{
    pub fn new(wasm_file: String,
               application_state: std::sync::Arc<std::sync::Mutex<ApplicationState>>,
               request_index : usize) -> Self
    {
        Self
        {
            wasm_file,
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
        }

        // Create the engine.
        let args = std::env::args ().skip (1).collect::<Vec<_>> ();
        let engine = wasmtime::Engine::default ();

        // Load the module.
        let path_to_module = &self.wasm_file;
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

        // Add the restore_memory, which do nothing right now.
        linker.func_wrap ("host", "restore_memory", |caller: wasmtime::Caller<'_, MyState>|
            {
                let request_index : usize = caller.data ().request_index;

                #[cfg(feature = "print_log")]
                println! ("request {} - restore_memory START", request_index);



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
