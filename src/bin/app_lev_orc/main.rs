/*****************************************/
/*     APPLICATION LEVEL ORCHESTRATOR    */
/*****************************************/

mod state;
mod state_monitoring_loop;
mod requests_monitoring_loop;
mod requests_coordination_loop;
mod admm_solver;
mod sporadic_server;
mod configuration_loader;

/// Example of invocation: ./app_lev_orc 4 192.168.1.2:8080 192.168.1.3 0 0 (2.15,9.8) 2
fn main ()
{
    // Parse input arguments.
    let args: Vec<String> = std::env::args ().collect ();
    let node_number       : usize = args[1].parse::<usize > ()
        .expect ( "Unable to parse node number. " );
    let node_address      : String = args[2].to_string ();
    let broker_address    : String = args[3].to_string ();
    let node_index        : usize = args[4].parse::<usize > ()
        .expect ( "Unable to parse node index. " );
    let application_index : usize = args[5].parse::<usize > ()
        .expect( "Unable to parse application index. " );
    let node_state        : state::NodeState = args[6].parse::<state::NodeState> ()
        .expect("Unable to parse into NodeState. ");
    let affinity          : usize = args[7].parse::<usize> ()
        .expect ( "Unable to parse affinity. " );

    // Node data. 
    let node_coords : state::Coord = node_state.get_coord ();

    // Initialize the application state. 
    let application_state: std::sync::Arc<std::sync::Mutex<state::ApplicationState>> =
        std::sync::Arc::new (
            std::sync::Mutex::new (
                state::ApplicationState::new (node_coords)));
    configuration_loader::load_requests (application_state.clone ());

    // Initialize the sporadic server barrier.
    // The first element refers to the number of requests
    // waiting to be served.
    let number_of_requests = application_state.lock ().unwrap ().number_of_requests;
    let barrier : std::sync::Arc<(std::sync::Mutex<u8>, std::sync::Condvar)> =
        std::sync::Arc::new (
            (std::sync::Mutex::new (number_of_requests as u8), std::sync::Condvar::new ()));

    // First activation (10ms in the future).
    let mut first_activation : libc::timespec = unsafe { std::mem::zeroed () };
    unsafe
        {
            libc::clock_gettime (libc::CLOCK_MONOTONIC, &mut first_activation)
        };
    first_activation.tv_nsec += 10_000_000;
    if first_activation.tv_nsec >= 1_000_000_000
    {
        first_activation.tv_nsec -= 1_000_000_000;
        first_activation.tv_sec  += 1;
    }

    // Initialize the taskset. 
    let mut state_monitoring_loop      =
        state_monitoring_loop::ControlSystem::new (application_index,
                                                   node_index,
                                                   20,
                                                   affinity,
                                                   broker_address.clone ());
    let mut requests_monitoring_loop   =
        requests_monitoring_loop::ControlSystem::new (node_index,
                                                      application_index,
                                                      1_000,
                                                      first_activation,
                                                      20,
                                                      affinity,
                                                      broker_address.clone ());
    let mut requests_coordination_loop =
        requests_coordination_loop::ControlSystem::new (node_number,
                                                        application_index,
                                                        node_index,
                                                        20,
                                                        affinity,
                                                        node_address.to_string (),
                                                        broker_address.clone ());
    let mut sporadic_server                         =
        sporadic_server::ControlSystem::new (application_index,
                                             10,
                                             affinity,
                                             "requests".to_string ());

    // Start each task. 
    let mut handles = vec![];
    let sml_app_state = std::sync::Arc::clone (&application_state);
    let sml_handle = std::thread::spawn(move ||
        {
            state_monitoring_loop.start (sml_app_state);
        }
    );
    handles.push (sml_handle);

    let rml_app_state = std::sync::Arc::clone (&application_state);
    let rml_handle = std::thread::spawn(move ||
        {
            requests_monitoring_loop.start(rml_app_state);
        }
    );
    handles.push (rml_handle);

    let rcl_app_state = std::sync::Arc::clone (&application_state);
    let rcl_barrier = std::sync::Arc::clone (&barrier);
    let rcl_handle = std::thread::spawn(move ||
        {
            requests_coordination_loop.start (rcl_app_state, rcl_barrier);
        }
    );
    handles.push (rcl_handle);

    let ss_app_state = std::sync::Arc::clone (&application_state);
    let ss_barrier = std::sync::Arc::clone (&barrier);
    let ss_handle = std::thread::spawn (move ||
        {
            sporadic_server.start (ss_app_state, ss_barrier);
        }
    );
    handles.push (ss_handle);

    for handle in handles
    {
        handle.join ().unwrap ();
    }
    // End of main. 
}