///                              ///
///     STATE MONITORING LOOP    ///
///                              ///

use paho_mqtt::{self as mqtt, MQTT_VERSION_5};
use futures::{executor::block_on, stream::StreamExt};

use crate::state::{self, NodeState};

pub struct OperationControlSystems {
    client : mqtt::AsyncClient,
    topic  : String,
}

impl OperationControlSystems {

    pub fn new(application_index : usize, node_index : usize) -> Self {
        // Initialization 
        let host = "mqtt://localhost:1883".to_string();

        let client_id = format!("node_{}_app_{}", node_index, application_index);

        // Create the client. Use an ID for a persistent session 
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(host)
            .client_id(client_id)
            .finalize();

        // Create the subscriber connection 
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
    /// with a fixed period
    pub fn start_monitoring_state_loop(&mut self) {
        if let Err(err) = block_on(async {

            // Get message stream before connecting
            let mut strm = self.client.get_stream(5);

            // Define the set of options for the connection
            let lwt = mqtt::Message::new(
                self.topic,
                "[LWT] Async subscriber lost connection",
                mqtt::QOS_1,
            );

            // Connect with MQTT v5 and a persistent server session (no clean start) 
            let conn_opts = mqtt::ConnectOptionsBuilder::with_mqtt_version(MQTT_VERSION_5)
                .clean_start(false)
                .properties(mqtt::properties![mqtt::PropertyCode::SessionExpiryInterval => 30])
                .will_message(lwt)
                .finalize();

            println!("Connect...");
            // Make the connection to the broker
            self.client.connect(conn_opts).await?;

            println!("Subscribing to topics...");
            let sub_opts = vec![mqtt::SubscribeOptions::with_retain_as_published(); 2];
            self.client.subscribe_many_with_options(
                &[self.topic],
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
                        let node_coords = msg.payload_str().parse::<NodeState>().expect("msg");

                        // Notify to all the nodes that the federation process starts.
                        let msg = mqtt::Message::new(
                            config::TOPIC_FEDERATION_MAIN,
                            "FEDERATE",
                            config::QUALITY_OF_SERVICE);
                        client.publish(msg).await?;

                        // TODO: Perform the local update.
                        let local_update: f32 = 0.0;

                        let msg = mqtt::Message::new(
                            config::TOPIC_FEDERATION_GATHER,
                            local_update.to_string(),
                            config::QUALITY_OF_SERVICE);
                        client.publish(msg).await?;
                    }
                    else if msg.topic() == config::TOPIC_FEDERATION_GATHER && fed_cmd_req {
                        let local_updates =
                            msg
                                .payload_str()
                                .parse::<f32>()
                                .expect("Couldn't parse payload");
                        local_data.push(local_updates);
                        println!("locals received : {}", local_updates);

                        if local_data.len() < config::NUMBER_OF_NODES {
                            // Check if we have enough data to proceed with the updates.
                            if local_data.len() == config::NUMBER_OF_NODES {
                                println!("local_data.len() == config::NUMBER_OF_NODES");
                                let mut result_found = false;

                                // Global update.
                                global_solver.global_z_updater();

                                // Termination check.
                                let sum_of_globals = f32::sum(global_solver.globals.x.iter());
                                if (sum_of_globals - 1f32).abs() < admm_solver::TOLERANCE {
                                    let mut has_converged = true;
                                    for i in 0..config::NUMBER_OF_NODES {
                                        if (global_solver.globals.x[i] - global_solver.variables.x[i]).abs()
                                            > (admm_solver::TOLERANCE / config::NUMBER_OF_NODES as f32) {
                                            has_converged = false;
                                        }
                                    }
                                    result_found  = has_converged;MQTT_VERSION_5
                                let msg = mqtt::Message::new(
                                    config::TOPIC_FEDERATION_MAIN,
                                    serde_json::to_string(&global_solver.globals)
                                        .expect("Couldn't serialize globals"),
                                    config::QUALITY_OF_SERVICE);
                                client.publish(msg).await?;

                                // Notify to all the nodes what have been found.
                                if result_found {
                                    let msg = mqtt::Message::new(
                                        config::TOPIC_FEDERATION_MAIN,
                                        "END",
                                        config::QUALITY_OF_SERVICE);
                                    client.publish(msg).await?;
                                    fed_cmd_req = false;
                                }
                                else {
                                    // Notify to all the nodes that the federation process starts.
                                    let msg = mqtt::Message::new(
                                        config::TOPIC_FEDERATION_MAIN,
                                        "CONTINUE",
                                        config::QUALITY_OF_SERVICE);
                                    client.publish(msg).await?;
                                }
                                local_data.clear();
                            }
                        }
                    }
                }
            }
        }
            // Explicit return type for the async block
            Ok::<(), mqtt::Error>(())

        }) 
        {
           eprintln!("Error creating the client: {:?}", err);
        }
    }
}