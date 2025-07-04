///                                     ///
///     APPLICATION LEVEL ORCHESTRATOR  ///
///                                     ///

mod state;
mod state_monitoring_loop;
mod requests_monitoring_loop;
mod requests_coordination_loop;
mod admm_solver;

use std::{mem::zeroed, sync::{Arc, Mutex}};

fn main() {

    // Node data. 
    let node_coords : state::Coord = state::Coord::new_from(0.0, 0.0);

    // Initialize the application state. 
    let application_state: Arc<Mutex<state::ApplicationState>> = 
        std::sync::Arc::new(
            std::sync::Mutex::new(
                state::ApplicationState::new(node_coords)));

    // First activation (10ms in the future). 
    let mut first_activation : libc::timespec = unsafe { zeroed() };
    unsafe { 
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut first_activation)
    };
    first_activation.tv_nsec += 10_000_000;
    if first_activation.tv_nsec >= 1_000_000_000 {
        first_activation.tv_nsec -= 1_000_000_000;
        first_activation.tv_sec  += 1;
    }

    // Initialize the taskset. 
    let mut state_monitoring_loop = state_monitoring_loop::OperationControlSystems::new(0, 0);
    let mut requests_monitoring_loop = requests_monitoring_loop::OperationControlSystems::new(100_000, first_activation, 20, 8);
    let mut requests_coordination_loop = requests_coordination_loop::OperationControlSystems::new(0, 0);

    // Start each task. 
    let mut handles = vec![];
    let sml_app_state = Arc::clone(&application_state);
    let sml_handle = std::thread::spawn(move || {
        state_monitoring_loop.start_monitoring_state_loop(sml_app_state);
    });
    handles.push(sml_handle);

    let rml_app_state = Arc::clone(&application_state);
    let rml_handle = std::thread::spawn(move || {
        requests_monitoring_loop.requests_monitoring_loop(rml_app_state);
    });
    handles.push(rml_handle);

    let rcl_app_state = Arc::clone(&application_state);
    let rcl_handle = std::thread::spawn(move || {
        requests_coordination_loop.start_monitoring_state_loop(rcl_app_state);
    });
    handles.push(rcl_handle);

    for handle in handles {
        handle.join().unwrap();
    }

    // End of main
}