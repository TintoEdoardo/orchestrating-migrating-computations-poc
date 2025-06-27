///                              ///
///     STATE MONITORING LOOP    ///
///                              ///

use paho_mqtt::{self as mqtt, MQTT_VERSION_5};
use futures::{executor::block_on, stream::StreamExt};
use std::sync::{Arc, Mutex};

use crate::state::{ApplicationState, NodeState};

pub struct OperationControlSystems {
    client : mqtt::AsyncClient,
    topic  : String,
}

impl OperationControlSystems {

    pub fn new(application_index : usize, node_index : usize) -> Self {
        // Initialization. 
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
            topic : "node_state".to_string(),
        }
    
    }

    /// The function is invoked asynchronously by the 
    /// infrastructure-level orchestrator, sporadically, 
    /// with a fixed period. 
    pub fn start_monitoring_state_loop(&mut self, application_state : Arc<Mutex<ApplicationState>>) {
        if let Err(err) = block_on(async {

            // Get message stream before connecting. 
            let mut strm = self.client.get_stream(5);

            // Define the set of options for the connection. 
            let lwt = mqtt::Message::new(
                self.topic.clone(),
                "[LWT] Async subscriber lost connection",
                mqtt::QOS_1,
            );

            // Connect with MQTT v5 and a persistent server session (no clean start). 
            let conn_opts = mqtt::ConnectOptionsBuilder::with_mqtt_version(MQTT_VERSION_5)
                .clean_start(false)
                .properties(mqtt::properties![mqtt::PropertyCode::SessionExpiryInterval => 30])
                .will_message(lwt)
                .finalize();

            println!("Connect...");
            // Make the connection to the broker. 
            self.client.connect(conn_opts).await?;

            println!("Subscribing to topics...");
            let sub_opts = vec![mqtt::SubscribeOptions::with_retain_as_published(); 2];
            self.client.subscribe_many_with_options(
                &[self.topic.clone()],
                &[mqtt::QOS_1],
                &sub_opts,
                None).await?;


            // Just loop on incoming messages.
            println!("Waiting for messages...");
            while let Some(msg_opt) = strm.next().await {
                if let Some(msg) = msg_opt {
                    // Check is a "federate" command has been issued.
                    if msg.topic() == self.topic
                    {
                        // Parse the received message. 
                        let node_state = msg.payload_str().parse::<NodeState>().expect("msg");

                        // The update the NodeState object within ApplicationState. 
                        {
                            application_state.lock().unwrap().update_node_state(node_state.get_coord());
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