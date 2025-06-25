///                                     ///
///     APPLICATION LEVEL ORCHESTRATOR  ///
///                                     ///

mod state;
mod state_monitoring_loop;
mod requests_monitoring_loop;
mod requests_coordination_loop;

use std::sync::{Arc, Mutex};

fn main() {

    // Node data
    let node_coords : state::Coord = state::Coord::new_from(0.0, 0.0);

    // Initialize the application state
    let application_state: Arc<Mutex<state::ApplicationState>> = 
        std::sync::Arc::new(
            std::sync::Mutex::new(
                state::ApplicationState::new(node_coords)));

    // Initialize the taskset

    // Start each task
    for _ in 0..10 {
        let counter = Arc::clone(&application_state);
        let handle = std::thread::spawn(move || {
            let mut num = counter.lock().unwrap();

            num.read_node_state();
        });
    }

    // End of main
}