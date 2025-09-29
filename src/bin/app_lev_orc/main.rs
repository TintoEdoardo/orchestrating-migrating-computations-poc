/*****************************************/
/*     APPLICATION LEVEL ORCHESTRATOR    */
/*****************************************/

mod state;
mod state_monitoring_loop;
mod requests_monitoring_loop;
mod requests_coordination_loop_d;
mod admm_solver;
mod sporadic_server;
mod configuration_loader;
mod requests_coordination_loop_c;
mod mqtt_utils;
mod linux_utils;
mod log_writer;

/// Example of invocation: ./app_lev_orc config_file.conf
fn main ()
{

    #[cfg(feature = "timing_log")]
    println!("timing_log : ACTIVE");
    #[cfg(feature = "centralized")]
    println!("centralized : ACTIVE");
    #[cfg(feature = "distributed")]
    println!("distributed : ACTIVE");
    #[cfg(feature = "print_log")]
    println!("print_log : ACTIVE");

    // Parse input arguments.
    let args: Vec<String> = std::env::args ().collect ();

    let node_index       : usize;
    let application_index: usize;
    let node_address     : String;
    let node_state       : state::NodeState;
    let affinity         : usize;
    let node_number      : usize;
    let broker_address   : String;
    let is_controller    : bool;

    let lines = configuration_loader::load_config (args[1].clone ());

    node_index        = lines.get (0).expect ("Failed to read node_index. ")
        .parse ().expect ("Failed to parse node_index. ");
    application_index = lines.get (1).expect ("Failed to read application_index. ")
        .parse ().expect ("Failed to parse application_index. ");
    node_address      = lines.get (2).expect ("Failed to read node_address.").clone ();
    node_state        = lines.get (3).expect ("Failed to read node_state. ")
        .parse ().expect ("Failed to parse node_state. ");
    affinity          = lines.get (4).expect ("Failed to read affinity. ")
        .parse ().expect ("Failed to parse affinity. ");
    node_number       = lines.get (5).expect ("Failed to read node_number. ")
        .parse ().expect ("Failed to parse node_number. ");
    broker_address    = lines.get (6).expect ("Failed to read broker_address").clone ();
    match lines.get (7)
    {
        None =>
            {
                is_controller = false;
            }
        Some(flag) =>
            {
                is_controller = flag.parse ().expect ("Failed to parse is_controller. ");
            }
    }

    // Node data.
    let node_coords : state::Coord = node_state.get_coord ();
    let node_speedup_factor : f32  = node_state.get_speedup_factor ();

    // Initialize the application state. 
    let application_state: std::sync::Arc<std::sync::Mutex<state::ApplicationState>> =
        std::sync::Arc::new (
            std::sync::Mutex::new (
                state::ApplicationState::new (
                    node_coords, 100, 20, node_speedup_factor, 1_000_000)));
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
                                                   50,
                                                   affinity,
                                                   broker_address.clone ());
    let mut requests_monitoring_loop   =
        requests_monitoring_loop::ControlSystem::new (node_index,
                                                      application_index,
                                                      1_000,
                                                      first_activation,
                                                      50,
                                                      affinity,
                                                      broker_address.clone ());
    #[cfg(feature = "distributed")]
    #[allow(unused_variables, unused_mut)]
    let mut requests_coordination_loop =
        requests_coordination_loop_d::ControlSystem::new (node_number,
                                                          application_index,
                                                          node_index,
                                                          45,
                                                          affinity,
                                                          node_address.to_string (),
                                                          broker_address.clone ());

    #[cfg(feature = "centralized")]
    let mut requests_coordination_loop =
        requests_coordination_loop_c::ControlSystem::new (node_number,
                                                          is_controller,
                                                          application_index,
                                                          node_index,
                                                          45,
                                                          affinity,
                                                          node_address.to_string (),
                                                          broker_address.clone ());

    let mut sporadic_server                         =
        sporadic_server::ControlSystem::new (application_index,
                                             20,
                                             100,
                                             20,
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