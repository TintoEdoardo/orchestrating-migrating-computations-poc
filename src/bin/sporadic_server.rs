use std::os::fd::AsRawFd;
use affinity;

#[path = "../config.rs"]
mod config;

/// The criticality level of a task. 
enum Criticality {
    HIGH, 
    LOW
}

/// A sporadic server. 
/// The behavior of the server is the following: 
/// (1) Upon arriving of a migrating computation, the local controller starts the SS. 
/// (2) As soon as activated, the SS enroll in the corresponding cgroup, to enforce the 
///     bandwidth limitation. 
/// (3) Load the Wasm module (plus memory) and run it. 
fn main() {

    /****************
       SET AFFINITY
     ****************/
    println!("SET AFFINITY");
    // First, fix this computation to a specific CPU (now 2).
    let cores: Vec<usize> = vec![8];
    affinity::set_thread_affinity(&cores).unwrap();

    /*******************************
       INTER-PROCESS COMMUNICATION
     *******************************/
    println!("CONFIGURE IPC");
    // IPC happens through shared memory. 
    let fd = 
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(config::SEMAPHORE)
            .expect("Unable to open file. ");

    unsafe { libc::ftruncate(fd.as_raw_fd(), config::SHARED_MEMORY_SIZE as libc::off_t); }

    let shared_data_ptr : *mut config::IPC_SHARED_MEMORY = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            core::mem::size_of::<config::IPC_SHARED_MEMORY>(), 
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_FILE | libc::MAP_SHARED,
            fd.as_raw_fd(), 
            0) as *mut config::IPC_SHARED_MEMORY
    };
    println!("ptr {:?}", shared_data_ptr);

    unsafe {
        if libc::sem_init(&mut (*shared_data_ptr).semaphore, 1, 0) == -1
            { panic!("sem_init failed"); }
    };
    
    unsafe {
        libc::futex
    };

    let shared_data    : &mut config::IPC_SHARED_MEMORY = unsafe {&mut *shared_data_ptr};
    let semaphore      : &mut libc::sem_t = unsafe {&mut (*shared_data).semaphore};
    let should_migrate : &i32 = unsafe {&mut (*shared_data).should_migrate};

    println!("START LOOP");
    // Body of the task.
    loop {

        /****************
             WORKLOAD
         ****************/

        // Start executing a migrating request.
        // In this experimentation, there is just one request to run.
        struct MyState<'a> {
            wasi          : wasmtime_wasi::preview1::WasiP1Ctx,
            checkpoint    : i32,
            semaphore_ptr : &'a mut libc::sem_t,
        }

        // Create the engine.
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let engine = wasmtime::Engine::default();

        // Load the module.
        let path_to_module = "loop.wasm";
        let module =
            wasmtime::Module::from_file(&engine, path_to_module).expect("Failed to load wasm file. ");

        // Create the Linker, with a simple callback to signal a migration request.
        let mut linker: wasmtime::Linker<MyState>  = wasmtime::Linker::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| &mut cx.wasi)
            .expect("add_to_linker_sync failed. ");

        // Add the should_migrate function.
        linker.func_wrap("host", "should_migrate", |mut caller: wasmtime::Caller<'_, MyState>| {
            // The repetition of unsafe blocks is on purpose, it highlights the actions
            // that are not safe in the function body.
            let mut result = 1;
            println!("Acquiring lock");
            unsafe {
                libc::sem_wait(caller.data_mut().semaphore_ptr);
            }
            println!("should_migrate: {:}", *should_migrate);
            if *should_migrate == 1 {
                result = 0;
            }
            unsafe {
                libc::sem_post(caller.data_mut().semaphore_ptr);
            }
            println!("Relinquishing lock");
            result
        } ).expect("func_wrap failed. ");

        // Add the restore_memory, which do nothing right now.
        linker.func_wrap("host", "restore_memory", || { } )
            .expect("func_wrap failed. ");

        let pre = linker.instantiate_pre(&module).expect("instantiate failed. ");

        // Create the Store.
        let wasi_ctx = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_env()
            .args(&args)
            .build_p1();
        let state = MyState {
            wasi: wasi_ctx,
            checkpoint: 0,
            semaphore_ptr: semaphore,
        };
        let mut store = wasmtime::Store::new(&engine, state);

        // Instantiate the module.
        let instance = pre.instantiate(&mut store).expect("instantiate failed. ");

        // Invoke the start function of the module.
        let func = instance.get_func(&mut store, "endless_loop")
            .expect("Unable to find function _start. ");
        let mut result = [];

        let _r = func.call(&mut store, &[], &mut result);

        break;

        println!("Checkpoint! ");

        // Save the memory.
        let checkpoint_memory = instance.get_memory(&mut store, "checkpoint_memory").expect("Unable to find memory");

        let checkpoint_data = checkpoint_memory.data_mut(&mut store).to_vec();

        let main_linear_memory = instance.get_memory(&mut store, "memory").expect("Unable to find memory");

        let main_lin_mem_data = main_linear_memory.data(&mut store).to_vec();

    }
    
}