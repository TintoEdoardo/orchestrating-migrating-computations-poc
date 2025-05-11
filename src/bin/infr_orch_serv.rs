#[path = "../config.rs"]
mod config;

use paho_mqtt::{self as mqtt, MQTT_VERSION_5};
use futures::{executor::block_on, stream::StreamExt};
use std::{env, process, time::Duration};

///     INFRASTRUCTURE ORCHESTRATOR
/// This is the broker server of the 1st-level orchestrator. 

fn main() {

    // Initialization.
    let host = env::args()
        .nth(1)
        .unwrap_or_else(|| "mqtt://localhost:1883".to_string());

    // Create the client. Use an ID for a persistent session.
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(host)
        .client_id("infr_orch_serv")
        .finalize();

    // Create the subscriber connection.
    let mut subscriber = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
        println!("Error creating the client: {:?}", e);
        process::exit(1);
    });

    subscriber.set_disconnected_callback(|_, _props, reason| {
        println!("Server disconnected with reason: {}", reason);
    });

    println!("Start async...");
    if let Err(err) = block_on(async {
        // Get message stream before connecting.
        let mut strm = subscriber.get_stream(25);

        // Define the set of options for the connection
        let lwt = mqtt::Message::new(
            "test/lwt",
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
        // Make the connection to the broker
        subscriber.connect(conn_opts).await?;

        println!("Subscribing to topics...");

        let sub_opts = vec![mqtt::SubscribeOptions::with_retain_as_published(); 2];
        subscriber.subscribe_many_with_options(
            &[config::TOPIC_FEDERATION_GATHER, config::TOPIC_COMMAND],
            &[config::QUALITY_OF_SERVICE, config::QUALITY_OF_SERVICE],
            &sub_opts,
            None)
            .await?;

        // Has a "federate" command been issued?
        let mut fed_cmd_req = false;

        // The list of data gathered in the last ADMM iteration.
        let mut local_data : Vec<f32> = Vec::new();

        // Just loop on incoming messages.
        println!("Waiting for messages...");

        while let Some(msg_opt) = strm.next().await {
            if let Some(msg) = msg_opt {
                // Check is a "federate" command has been issued.
                if msg.topic() == config::TOPIC_COMMAND &&
                    msg.payload() == "FEDERATE".as_bytes()
                {
                    println!("msg.payload() == FEDERATE");
                    fed_cmd_req = true;
                    // Notify to all the nodes that the federation process starts.
                    let msg = mqtt::Message::new(
                        config::TOPIC_FEDERATION_MAIN,
                        "FEDERATE",
                        config::QUALITY_OF_SERVICE);
                    subscriber.publish(msg).await?;
                }
                else if msg.topic() == config::TOPIC_FEDERATION_GATHER && fed_cmd_req {
                    if local_data.len() < config::NUMBER_OF_NODES {
                        // TODO: Push the message content.
                        local_data.push(0.0);
                        // Check if we have enough data to proceed with the updates.
                        if local_data.len() == config::NUMBER_OF_NODES {
                            println!("local_data.len() == config::NUMBER_OF_NODES");
                            let mut result_found = false;
                            // TODO: Global update.
                            // Notify to all the nodes what have been found.
                            if result_found {
                                let msg = mqtt::Message::new(
                                    config::TOPIC_FEDERATION_MAIN,
                                    "RESULT FOUND",
                                    config::QUALITY_OF_SERVICE);
                                subscriber.publish(msg).await?;
                                fed_cmd_req = false;
                            }
                            else {
                                println!("result_found == FALSE");
                                // Notify to all the nodes that the federation process starts.
                                let msg = mqtt::Message::new(
                                    config::TOPIC_FEDERATION_MAIN,
                                    "CONTINUE (z)",
                                    config::QUALITY_OF_SERVICE);
                                subscriber.publish(msg).await?;
                            }
                            local_data.clear();
                        }
                    }
                }
            }
        }

        // Explicit return type for the async block
        Ok::<(), mqtt::Error>(())

    }) {
        eprintln!("Error creating the client: {:?}", err);
    }

}
