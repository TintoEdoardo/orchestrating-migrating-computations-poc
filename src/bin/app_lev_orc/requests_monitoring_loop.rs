/***************************************/
/*       REQUESTS MONITORING LOOP      */
/***************************************/

use paho_mqtt::{self as mqtt};

use crate::state::{should_migrate, ApplicationState, MessageRequest};

/// Data and functions associated with the
/// requests_monitoring_loop.
pub struct ControlSystem
{
    /// Period in us. Note that the activation step
    /// will not work properly with a period >= 1 sec.
    period           : i32,

    /// The time of the first activation.
    first_activation : libc::timespec,

    /// The priority of this thread.
    priority         : i32,

    /// Affinity of this thread.
    affinity         : usize,

    /// The MQTT client.
    client           : mqtt::AsyncClient,

    /// The index of the current node.
    node_index       : usize,

    /// The index of the application.
    #[allow(dead_code)]
    application_index : usize,
}

impl ControlSystem
{
    pub fn new (node_index        : usize,
                application_index : usize,
                period            : i32,
                first_activation  : libc::timespec,
                priority          : i32,
                affinity          : usize) -> Self
    {

        #[cfg(feature = "print_log")]
        println! ("request_monitoring_loop - new START");

        // Initialization of the MQTT link.
        let host = "mqtt://192.168.1.12:1883".to_string ();

        let client_id = format! ("node_{}_app_{}_rml", node_index, application_index);

        // Create the client. Use an ID for a persistent session.
        let create_opts = mqtt::CreateOptionsBuilder::new ()
            .server_uri (host)
            .client_id (client_id)
            .finalize ();

        // Create the subscriber connection.
        let client = mqtt::AsyncClient::new (create_opts).unwrap_or_else (|e| 
            {
                panic! ("requests_monitoring_loop - error creating the client: {:?}", e);
            }
        );

        client.set_disconnected_callback (|_, _props, reason|
            {
                panic! ("requests_monitoring_loop - server disconnected with reason: {}", reason);
            }
        );

        #[cfg(feature = "print_log")]
        println! ("request_monitoring_loop - new END");

        Self { period, first_activation, priority, affinity, client, node_index, application_index }
    }

    /// Start the requests monitoring loop.
    pub fn start (&mut self, application_state: std::sync::Arc<std::sync::Mutex<ApplicationState>>) 
    {

        #[cfg(feature = "print_log")]
        println! ("request_monitoring_loop - INIT");

        // Initialization. 
        unsafe
            {

                // Scheduling properties.
                let tid = libc::gettid ();
                let sched_param = libc::sched_param
                {
                    sched_priority: self.priority,
                };
                libc::sched_setscheduler (tid, libc::SCHED_FIFO, &sched_param);

                // Affinity.
                let mut cpuset : libc::cpu_set_t = std::mem::zeroed ();
                libc::CPU_ZERO (&mut cpuset);
                libc::CPU_SET (self.affinity, &mut cpuset);
                libc::sched_setaffinity (tid, size_of::<libc::cpu_set_t> (), &mut cpuset);
            }

        // Activation. 
        let mut next_activation = self.first_activation;
        unsafe
            {
                libc::clock_nanosleep (libc::CLOCK_MONOTONIC,
                                       libc::TIMER_ABSTIME,
                                       &mut next_activation,
                                       core::ptr::null_mut ());
            }

        #[cfg(feature = "print_log")]
        println! ("request_monitoring_loop - LOOP");

        loop
        {

            #[cfg(all(feature = "print_log", feature = "periodic_activation"))]
            {
                let activation =
                    next_activation.tv_sec * 1_000_000 + next_activation.tv_nsec / 1_000;
                println! ("request_monitoring_loop - job START at {:.2} us", activation);
            }


            // Compute next activation. 
            // This step misbehaves if the period is >= 1_000_000_000. 
            next_activation.tv_nsec = next_activation.tv_nsec + (self.period * 1000) as i64;
            next_activation.tv_sec  = next_activation.tv_sec;
            if next_activation.tv_nsec > 1_000_000_000
            {
                next_activation.tv_sec   = next_activation.tv_sec + 1;
                next_activation.tv_nsec -= 1_000_000_000;
            }

            // Check if some requests need to migrate. 
            {
                // Here we access the application state as mutable.
                let mut app_state = application_state.lock ().unwrap ();
                let node_state = app_state.node_state;
                let requests = &mut app_state.requests;

                for &mut mut request in requests
                {
                    if should_migrate (&request, &node_state)
                    {
                        // Update the application state.
                        request.set_should_migrate (true);

                        // Then trigger a migration.
                        let message_request =
                            MessageRequest::new (self.node_index, request);
                        let msg = mqtt::Message::new (
                            "federation/migration".to_string (),
                            message_request.to_string (),
                            paho_mqtt::QOS_1);
                        self.client.publish (msg);
                    }
                }

                // Drop the mutex variable, forcing unlocking.
                drop (app_state);
            }

            #[cfg(all(feature = "print_log", feature = "periodic_activation"))]
            {
                let activation =
                    next_activation.tv_sec * 1_000_000 + next_activation.tv_nsec / 1_000;
                println! ("request_monitoring_loop - job END at {:.2} us", activation);
            }

            // Sleep until next activation. 
            unsafe
                {
                    libc::clock_nanosleep (libc::CLOCK_MONOTONIC,
                                           libc::TIMER_ABSTIME,
                                           &mut next_activation,
                                           core::ptr::null_mut ());
                }
        }
    }
}