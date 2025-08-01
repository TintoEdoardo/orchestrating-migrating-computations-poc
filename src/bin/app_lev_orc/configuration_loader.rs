/***************************************/
/*         CONFIGURATION LOADER        */
/***************************************/

// This component is responsible for loading the initial
// set of requests for a node.
// To do so, a configuration file is used, located in the
// 'requests' folder.

use crate::state::{ApplicationState, Request};

pub fn load_requests (application_state: std::sync::Arc<std::sync::Mutex<ApplicationState>>)
{
    let config_file_name = "requests/requests.txt".to_string ();

    let mut application_state = application_state.lock ().unwrap ();

    // Read the file line by line, each line encodes for a request.
    for line in std::fs::read_to_string (config_file_name).unwrap ().lines ()
    {
        let request : Request = line.parse ()
            .expect ("Failed to parse line in requests.txt");

        application_state.add_request (request);
    }

}