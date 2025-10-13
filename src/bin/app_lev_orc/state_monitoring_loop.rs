/***************************************/
/*        STATE MONITORING LOOP        */
/***************************************/

use paho_mqtt::{self as mqtt, MQTT_VERSION_5};
use futures::{executor::block_on, stream::StreamExt};

use crate::state::{ApplicationState, NodeState};
use crate::linux_utils;

/// Data and functions associated with the
/// state_monitoring_loop.
pub struct ControlSystem
{
    client   : mqtt::AsyncClient,
    topic    : String,
    priority : i32,
    affinity : usize,
}

impl ControlSystem
{
    pub fn new (application_index: usize,
                node_index       : usize,
                priority         : i32,
                affinity         : usize,
                broker_address   : String) -> Self
    {

        #[cfg(feature = "print_log")]
        println! ("state_monitoring_loop - new START");

        // Initialization. 
        let host = format! ("mqtt://{}:1883", broker_address).to_string ();

        let client_id = format! ("node_{}_app_{}_sml", node_index, application_index);

        // Create the client. Use an ID for a persistent session. 
        let create_opts = mqtt::CreateOptionsBuilder::new ()
            .server_uri (host)
            .client_id( client_id)
            .finalize ();

        // Create the subscriber connection. 
        let client = mqtt::AsyncClient::new (create_opts).unwrap_or_else (|e|
            {
                panic! ("state_monitoring_loop - error creating the client: {:?}", e);
            }
        );

        client.set_disconnected_callback(|_, _props, reason|
            {
                panic! ("state_monitoring_loop - server disconnected with reason: {}", reason);
            }
        );

        #[cfg(feature = "print_log")]
        println! ("state_monitoring_loop - new END");

        Self
        {
            client,
            topic : format! ("node_state_{}", node_index).to_string (),
            priority,
            affinity,
        }
    
    }

    /// Start the state monitoring loop.
    pub fn start (&mut self, application_state: std::sync::Arc<std::sync::Mutex<ApplicationState>>)
    {

        #[cfg(feature = "print_log")]
        println! ("state_monitoring_loop - INIT");

        // Initialization.
        linux_utils::set_priority (self.priority, self.affinity);

        if let Err (err) = block_on (async
            {
                // Get message stream before connecting.
                let mut strm = self.client.get_stream (15);

                // Define the set of options for the connection.
                let lwt = mqtt::Message::new (
                    self.topic.clone (),
                    "[LWT] Async subscriber 'state_monitoring_loop' lost connection",
                    mqtt::QOS_1,
                );

                // Connect with MQTT v5 and a persistent server session (no clean start).
                let conn_opts = mqtt::ConnectOptionsBuilder::with_mqtt_version (MQTT_VERSION_5)
                    .clean_start (true)
                    .properties (mqtt::properties![mqtt::PropertyCode::SessionExpiryInterval => 3600])
                    .will_message (lwt)
                    .finalize ();

                // Make the connection to the broker.
                self.client.connect (conn_opts).await?;

                let sub_opts = vec![mqtt::SubscribeOptions::with_retain_as_published (); 2];
                self.client.subscribe_many_with_options (
                    &[self.topic.clone ()],
                    &[mqtt::QOS_1],
                    &sub_opts,
                    None).await?;

                #[cfg(feature = "print_log")]
                println! ("state_monitoring_loop - LOOP");

                // Just loop on incoming messages.
                while let Some (msg_opt) = strm.next ().await
                {
                    if let Some (msg) = msg_opt
                    {
                        // Check if a "federate" command has been issued.
                        if msg.topic () == self.topic
                        {

                            #[cfg(feature = "print_log")]
                            println! ("state_monitoring_loop - message ARRIVED");

                            // Parse the received message.
                            let node_state = msg.payload_str ().parse::<NodeState> ()
                                .expect ("msg");

                            // The update the NodeState object within ApplicationState.
                            {
                                application_state.lock ().unwrap ().set_node_state (node_state.get_coord ());
                            }

                            #[cfg(feature = "print_log")]
                            println! ("state_monitoring_loop - message ELABORATED");

                        }
                    }
                }

                // Explicit return type for the async block.
                Ok::<(), mqtt::Error> (())
        }) 
        {
           println! ("state_monitoring_loop - error creating the client: {:?}", err);
        }
    }
}