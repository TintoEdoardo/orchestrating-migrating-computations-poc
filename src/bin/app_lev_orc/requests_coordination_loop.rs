///                                   ///
///     REQUESTS COORDINATION LOOP    ///
///                                   ///

use paho_mqtt::{self as mqtt, MQTT_VERSION_5};
use futures::{executor::block_on, stream::StreamExt};
use std::{str::FromStr, string::ParseError, sync::{Arc, Mutex}};

use crate::{admm_solver::{GlobalSolver, LocalSolver}, state::{ApplicationState, Coord, NodeState, Request}};

struct MessageRequest {
    src     : usize,
    request : Request,
}

impl FromStr for MessageRequest {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let strs : Vec<&str> = s.split_terminator('#').collect();
        match (strs.first(), strs.last()) {
            (Some(&str1), Some(&str2)) => {
                Ok(MessageRequest { 
                    src: usize::from_str(str1).expect("Unable to convert string to usize"),
                    request: Request::from_str(str2).expect("Unable to convert string to Request") })
            }
            _ => {
                panic!("Error while parsing a MessageRequest");
            }
        }
    }
}

impl ToString for MessageRequest {
    fn to_string(&self) -> String {
        format!("{}#{}", self.src, self.request.to_string())
    }
}

struct MessageLocal {
    src       : usize, 
    local_sum : f32,
}

impl FromStr for MessageLocal {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let strs : Vec<&str> = s.split_terminator('#').collect();
        match (strs.first(), strs.last()) {
            (Some(&str1), Some(&str2)) => {
                Ok(MessageLocal { 
                    src: usize::from_str(str1).expect("Unable to convert string to usize"),
                    local_sum: f32::from_str(str2).expect("Unable to convert string to f32") })
            }
            _ => {
                panic!("Error while parsing a MessageLocal");
            }
        }
    }
}

impl ToString for MessageLocal {
    fn to_string(&self) -> String {
        format!("{}#{}", self.src, self.local_sum)
    }
}
pub struct OperationControlSystems {
    client          : mqtt::AsyncClient,
    topics          : [String; 4],
    node_number     : usize,
    node_index      : usize,
    penalty         : f32,
    iteration_limit : usize,
}

impl OperationControlSystems {

    pub fn new(application_index : usize, node_index : usize) -> Self {
        // Initialization. 
        // TODO! Decide on IPs
        let host = "mqtt://localhost:1883".to_string();

        let client_id = format!("node_{}_app_{}", node_index, application_index);

        // Create the client. Use an ID for a persistent session. 
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(host)
            .client_id(client_id)
            .finalize();

        // Create the subscriber connection. 
        let client = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
            println!("Error creating the client: {:?}", e);
            std::process::exit(1);
        });

        client.set_disconnected_callback(|_, _props, reason| {
            println!("Server disconnected with reason: {}", reason);
        });

        Self {
            client,
            topics : 
                ["federation/migration".to_string(),
                 "federation/local_update".to_string(),
                 format!("federation/src/{}", node_index).to_string(),
                 format!("federation/dst/{}", node_index).to_string()],
            node_number     : 4,
            node_index,
            penalty         : 20.0,
            iteration_limit : 20,
        }
    
    }

    /// The function contains the implementation of the ADMM solver. 
    /// It runs 
    pub fn start_monitoring_state_loop(&mut self, application_state : Arc<Mutex<ApplicationState>>) {
        if let Err(err) = block_on(async {

            // Get message stream before connecting. 
            let mut strm = self.client.get_stream(5);

            // Define the set of options for the connection
            /* let lwt = mqtt::Message::new(
                self.topics,
                "[LWT] Async subscriber lost connection",
                mqtt::QOS_1,
            ); */

            // Connect with MQTT v5 and a persistent server session (no clean start) . 
            let conn_opts = mqtt::ConnectOptionsBuilder::with_mqtt_version(MQTT_VERSION_5)
                .clean_start(false)
                .properties(mqtt::properties![mqtt::PropertyCode::SessionExpiryInterval => 30])
                //.will_message(lwt)
                .finalize();

            println!("Connect...");
            // Make the connection to the broker. 
            self.client.connect(conn_opts).await?;

            println!("Subscribing to topics...");
            let sub_opts = vec![mqtt::SubscribeOptions::with_retain_as_published(); 2];
            self.client.subscribe_many_with_options(
                &self.topics,
                &[mqtt::QOS_1],
                &sub_opts,
                None).await?;


            // Variables used in the ADMM consensus algorithm. 
            let mut incoming_request : Option<Request> = None;
            let mut src_node : Option<usize> = None;
            let mut local_solver = 
                LocalSolver::new(self.node_number, 0.0, Coord::new());
            let mut global_solver =
                GlobalSolver::new(self.node_number, self.iteration_limit);
            let mut node_state : NodeState;
            let mut dest_node : Option<usize> = None;

            println!(" - START RCL - ");

            // Loop on incoming messages.
            println!("Waiting for messages...");
            while let Some(msg_opt) = strm.next().await {
                if let Some(msg) = msg_opt {
                    // federation/migration -> request. 
                    if msg.topic() == self.topics[0]
                    {
                        // Parse the received message. 
                        let message_request = 
                            msg.payload_str().parse::<MessageRequest>().expect("Unable to parse message into Request");
                        let request = message_request.request;
                        src_node = Some(message_request.src);

                        // If nodes are already deciding where to migrate a request, 
                        // hence incoming_request is Some(...), simply ignore any 
                        // further requests. 
                        match incoming_request {
                            None => {
                                // Acquire a copy of the node state. 
                                // It will be preseved during the all execution of 
                                // the consensus algorithm. 
                                {
                                    let state = application_state.lock().unwrap();
                                    node_state = state.node_state.clone();
                                    // Drop the value to force relinquishing the lock. 
                                    drop(state);
                                }

                                // Initialize the variables for a new execution. 
                                incoming_request = Some(request);
                                dest_node = None;
                                local_solver.clear(self.node_number, self.penalty, node_state.get_coord());
                                global_solver.clear();

                                // Perform the local update. 
                                local_solver.local_x_update(request.read_desired_coord());

                                // Send x + u, note that the client will receive 
                                // its own message. 
                                let local_sum = local_solver.read_local() + local_solver.read_dual();
                                let message_local = MessageLocal {
                                    src: self.node_index,
                                    local_sum,
                                };
                                let msg = mqtt::Message::new(
                                    &self.topics[1],
                                    message_local.to_string(),
                                    paho_mqtt::QOS_1);
                                self.client.publish(msg).await?;
                            }
                            _ => { 
                                    // Ignore the request. 
                            }
                        }
                    }
                    // federation/local_update -> f32. 
                    else if msg.topic() == self.topics[1]
                    {

                        // Parse the received message
                        let message_local = 
                            msg.payload_str().parse::<MessageLocal>().expect("Unable to parse message into f32 (in federation/local_update)");
                        let x = message_local.local_sum;
                        let src = message_local.src;

                        // Add x to X, the set of local variables. 
                        global_solver.add_local_sum(x, src);

                        // Wait until all nodes have completed their local update. 
                        if global_solver.locals_len() == self.node_number as usize {
                            // Global update on Z. 
                            global_solver.global_z_updater();
                            if global_solver.terminated() {
                                dest_node = Some(global_solver.get_max_global_index());
                                match (src_node, dest_node) {
                                    (Some(src), Some(dst)) => {
                                        // If the current node is the elected host, signal
                                        // to src_node that when it i sready to host a receive
                                        // the requesr. 
                                        if dst == self.node_index {
                                            let msg = mqtt::Message::new(
                                            format!("federation/src/{}", src),
                                            format!("SEND"),
                                            paho_mqtt::QOS_1);
                                            self.client.publish(msg).await?;
                                        } else {
                                            incoming_request = None;
                                        }
                                    }
                                    _ => {
                                        panic!("Src or dst node unkown");
                                    }
                                }
                            } else {
                                // Update the local_solver with the new global value, 
                                // then perform the dua lupdate. 
                                local_solver.set_global(global_solver.get_global_from_index(self.node_index));
                                local_solver.local_dual_update();
                            }
                        }
                    }
                    // federation/src/i -> SEND
                    else if msg.topic() == self.topics[2] {
                        match incoming_request {
                            Some(_) => {
                                // TODO! Send the request to dst_node. 
                                // TODO! Remove the request from the node. 
                                incoming_request = None;
                            }
                            None => {
                                // Do nothing. 
                            }
                        }
                    }
                    else if msg.topic() == self.topics[3] {
                        match incoming_request {
                            Some(_) => {
                                // TODO! Start the sporadic server. 
                                incoming_request = None;
                            }
                            None => {
                                // Do nothing. 
                            }
                        }
                    }
                }
            }
            
            // Explicit return type for the async block. 
            Ok::<(), mqtt::Error>(())
        }) 
        {
           eprintln!("Error creating the client: {:?}", err);
        }
    }
}