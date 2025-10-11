use sporadic_server::{SporadicServer, SporadicServerController, Workload};

fn main ()
{
    println!("START TEST SUITE");

    // The number of pending migration requests.
    let reqs_enqueued =
        std::sync::Arc::new ((std::sync::Mutex::new (1u8), std::sync::Condvar::new ()));

    // The state of the server task (is it running?).
    let is_server_running =
        std::sync::Arc::new ((std::sync::Mutex::new (false), std::sync::Condvar::new ()));

    let controller =
        std::sync::Arc::new (
            std::sync::Mutex::new (
                SporadicServerController::new(reqs_enqueued.clone (),
                                              is_server_running.clone ())
            )
        );

    let mut server = SporadicServer::new (
        std::time::Duration::from_millis (2),
        std::time::Duration::from_millis (100),
        80u32);

    let first_controller = controller.clone ();
    let first_is_server_running = is_server_running.clone ();
    let mut handles = vec![];
    let controller_handle = std::thread::spawn (move ||
        {
            // Initialization.
            unsafe
                {

                    // Scheduling properties.
                    let tid = libc::gettid ();
                    println!("tid of controller is {:?}", tid);
                    let sched_param = libc::sched_param
                    {
                        sched_priority: 89 as libc::c_int,
                    };
                    libc::sched_setscheduler (tid, libc::SCHED_FIFO, &sched_param);

                    // Affinity.
                    let mut cpuset : libc::cpu_set_t = std::mem::zeroed ();
                    libc::CPU_ZERO (&mut cpuset);
                    libc::CPU_SET (2, &mut cpuset);
                    libc::sched_setaffinity (tid, size_of::<libc::cpu_set_t> (), &mut cpuset);
                }

            SporadicServerController::start (first_controller, first_is_server_running)
        });
    handles.push (controller_handle);

    struct MyWorkload {}
    impl Workload for MyWorkload
    {
        #[allow(unused_assignments)]
        fn exec_workload(&mut self)
        {
            let start = std::time::Instant::now();
            println!(" time is {:?}", start);
            for _i in 0..5_000_000
            {
                assert!(true);
            }
            println!(" time is {:?}", std::time::Instant::now ());
            println!(" elapsed time is {:?}", std::time::Instant::elapsed (&start));
        }
    }

    let mut workload : MyWorkload = MyWorkload {};

    let server_handle = std::thread::spawn (move ||
        {
            // Initialization.
            unsafe
                {

                    // Scheduling properties.
                    let tid = libc::gettid ();
                    server.set_id (tid as u32);
                    println!("tid of server is {:?}", tid);
                    let sched_param = libc::sched_param
                    {
                        sched_priority: server.priority as libc::c_int,
                    };
                    libc::sched_setscheduler (tid, libc::SCHED_FIFO, &sched_param);

                    // Affinity.
                    let mut cpuset : libc::cpu_set_t = std::mem::zeroed ();
                    libc::CPU_ZERO (&mut cpuset);
                    libc::CPU_SET (2, &mut cpuset);
                    libc::sched_setaffinity (tid, size_of::<libc::cpu_set_t> (), &mut cpuset);
                }
            server.start(controller.clone (), &mut workload);
        });
    handles.push (server_handle);

    for handle in handles
    {
        handle.join ().unwrap ();
    }

    println!("END TEST SUITE");

}