use wasmtime_wasi::{DirPerms, FilePerms};

fn main()
{
    let mut start      : std::time::Instant;
    let mut elapsed_vec: Vec<u64> = Vec::new ();
    for _i in 0..10
    {
        // Get the start time.
        start = std::time::Instant::now ();
        struct MyState
        {
            wasi              : wasmtime_wasi::preview1::WasiP1Ctx,
        }

        // Create the engine.
        let engine = wasmtime::Engine::default ();

        // Load the module.
        let path_to_module = "module.wasm";
        let module =
            wasmtime::Module::from_file (&engine, path_to_module)
                .expect ("Failed to load wasm file. ");

        // Create the Linker.
        let mut linker: wasmtime::Linker<MyState>  = wasmtime::Linker::new (&engine);
        wasmtime_wasi::preview1::add_to_linker_sync (&mut linker, |cx| &mut cx.wasi)
            .expect ("add_to_linker_sync failed. ");

        // Add the should_migrate function mock.
        linker.func_wrap ("host", "should_migrate", |_caller: wasmtime::Caller<'_, MyState>|
            {
                0
            }
        ).expect ("func_wrap failed. ");

        // Add the restore_memory, which do nothing right now.
        linker.func_wrap ("host", "restore_memory", move |_caller: wasmtime::Caller<'_, MyState>|
            {
                // Do nothing.
            }
        )
            .expect ("func_wrap failed. ");

        let pre = linker.instantiate_pre (&module)
            .expect ("Instantiate failed. ");

        // Create the Store.
        let host_path = format! ("./{}", ".");
        let wasi_ctx = wasmtime_wasi::WasiCtxBuilder::new ()
            .inherit_stdio ()
            .inherit_env ()
            .preopened_dir(host_path, ".", DirPerms::all(), FilePerms::all())
            .expect("Unable to config directory. ")
            .build_p1 ();

        let state = MyState
        {
            wasi              : wasi_ctx,
        };
        let mut store = wasmtime::Store::new (&engine, state);

        // Instantiate the module.
        let instance = pre.instantiate (&mut store)
            .expect ("instantiate failed. ");

        // Invoke the start function of the module.
        let func = instance.get_func (&mut store, "_start")
            .expect ("Unable to find function _start. ");
        let mut result = [];

        let _function_result = func.call (&mut store, &[], &mut result);

        // Record time.
        elapsed_vec.push (start.elapsed().as_millis () as u64);
    }

    // Print the avg.
    let avg_exec_time = elapsed_vec.iter().sum::<u64>() as f64 / elapsed_vec.len() as f64;
    println!("Avg exec time is {} ms", avg_exec_time);
}
