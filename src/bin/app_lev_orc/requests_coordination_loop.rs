/****************************************/
/*      REQUESTS COORDINATION LOOP      */
/****************************************/

use paho_mqtt::{self as mqtt, MQTT_VERSION_5};
use futures::{executor::block_on, stream::StreamExt};
use std::io::{Read, Write};
use crate::{admm_solver::{GlobalSolver, LocalSolver},
            state::{ApplicationState, Coord, NodeState, Request}};

/// This is the message sent through MQTT containing
/// the local update in the ADMM algorithm.
struct MessageLocal
{
    src       : usize, 
    local_sum : f32,
}

impl std::str::FromStr for MessageLocal {
    type Err = std::string::ParseError;

    fn from_str (s: &str) -> Result<Self, Self::Err>
    {
        let strs : Vec<&str> = s.split_terminator ('#').collect ();
        match (strs.first (), strs.last ())
        {
            (Some (&str1), Some (&str2)) =>
                {
                    Ok (MessageLocal
                    {
                        src: usize::from_str (str1)
                            .expect ("Unable to convert string to usize"),
                        local_sum: f32::from_str (str2)
                            .expect ("Unable to convert string to f32")
                    })
                }
            _ =>
                {
                    panic! ("Error while parsing a MessageLocal");
                }
        }
    }
}

impl std::fmt::Display for MessageLocal
{
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write! (f, "{}", format! ("{}#{}", self.src, self.local_sum))
    }
}


/// Data and functions associated with the
/// requests_coordination_loop.
pub struct ControlSystem
{
    /// The MQTT client.
    client            : mqtt::AsyncClient,

    /// Address (ip and port) of the current node.
    ip_and_port       : String,

    /// MQTT topics this application has to interact with.
    topics            : [String; 5],

    /// The number of nodes in the federation assigned
    /// to this application.
    node_number       : usize,

    /// The index of the node.
    node_index        : usize,

    /// Priority of this task.
    priority         : i32,

    /// Affinity of the task.
    affinity         : usize,

    /// The index of the application.
    application_index : usize,

    /// A penalty factor used in the ADMM algorithm, which is arbitrary
    /// and its tuning has to be performed offline.
    penalty           : f32,

    /// The maximum number of iterations in the ADMM algorithm.
    iteration_limit   : usize,
}

impl ControlSystem
{

    pub fn new (node_number      : usize,
                application_index: usize,
                node_index       : usize,
                priority         : i32,
                affinity         : usize,
                ip_and_port      : String,
                broker_address   : String) -> Self
    {

        #[cfg(feature = "print_log")]
        println! ("requests_coordination_loop - new START");

        // Initialization.
        let host = format! ("mqtt://{}:1883", broker_address).to_string ();

        let client_id = format! ("node_{}_app_{}_rcl", node_index, application_index);

        // Create the client. Use an ID for a persistent session. 
        let create_opts = mqtt::CreateOptionsBuilder::new ()
            .server_uri (host)
            .client_id (client_id)
            .finalize ();

        // Create the subscriber connection. 
        let client = mqtt::AsyncClient::new (create_opts)
            .unwrap_or_else (|e|
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
        println! ("requests_coordination_loop - new END");

        Self 
        {
            client,
            ip_and_port,
            topics : 
                ["federation/migration".to_string (),
                 "federation/local_update".to_string (),
                 format! ("federation/src/{}", node_index).to_string (),
                 format! ("federation/dst/{}", node_index).to_string (),
                 "disconnect".to_string ()],
            node_number,
            node_index,
            // ss_affinity     : affinity,
            priority,
            affinity,
            application_index,
            penalty         : 20.0,
            iteration_limit : 20,
        }
    }

    /// Start the request coordination loop implementing the
    /// ADMM consensus algorithm. 
    pub fn start (&mut self,
                  application_state : std::sync::Arc<std::sync::Mutex<ApplicationState>>,
                  barrier           : std::sync::Arc<(std::sync::Mutex<u8>, std::sync::Condvar)>)
    {

        #[cfg(feature = "print_log")]
        println! ("requests_coordination_loop - INIT");

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

        if let Err (err) = block_on (async {

            // Get message stream before connecting. 
            let mut strm = self.client.get_stream (15);

            // Define the set of options for the connection
            let lwt = mqtt::Message::new (
                self.topics[4].clone (),
                "[LWT] Async subscriber 'requests_coordination_loop' lost connection",
                mqtt::QOS_1,
            );

            // Connect with MQTT v5 and a persistent server session (no clean start).
            let conn_opts =
                mqtt::ConnectOptionsBuilder::with_mqtt_version (MQTT_VERSION_5)
                    .clean_start (true)
                    .properties (mqtt::properties![mqtt::PropertyCode::SessionExpiryInterval => 3600])
                    .will_message (lwt)
                    .finalize ();

            // Make the connection to the broker. 
            self.client.connect (conn_opts).await?;

            let topics_len = self.topics.len ();
            let sub_opts =
                vec![mqtt::SubscribeOptions::with_retain_as_published (); topics_len];
            self.client.subscribe_many_with_options (
                &self.topics,
                &vec![mqtt::QOS_1; topics_len],
                &sub_opts,
                None).await?;

            // Variables used in the ADMM consensus algorithm. 
            let mut incoming_request : Option<Request> = None;
            let mut src_node         : Option<usize>   = None;
            let mut local_solver = 
                LocalSolver::new (self.node_number, 20.0, Coord::new ());
            let mut global_solver =
                GlobalSolver::new (self.node_number, self.iteration_limit);
            let mut node_state       : NodeState;
            let mut dest_node        : Option<usize>;

            #[cfg(feature = "timing_log")]
            let mut start_time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
            let mut end_time   = libc::timespec { tv_sec: 0, tv_nsec: 0 };

            #[cfg(feature = "print_log")]
            println! ("requests_coordination_loop - LOOP");

            // Loop on incoming messages.
            while let Some (msg_opt) = strm.next ().await 
            {
                if let Some (msg) = msg_opt 
                {
                    // federation/migration -> request. 
                    if msg.topic () == self.topics[0]
                    {

                        #[cfg(feature = "print_log")]
                        println! ("requests_coordination_loop - federation/migration MIGRATION");

                        #[cfg(feature = "timing_log")]
                        unsafe
                            {
                                libc::clock_gettime (libc::CLOCK_MONOTONIC, &mut start_time);
                            }

                        // Parse the received message. 
                        let message_request = 
                            msg.payload_str ().parse::<crate::state::MessageRequest> ()
                                .expect ( "Unable to parse message into Request");
                        let &request = message_request.get_request ();
                        src_node = Some (message_request.get_src ());

                        // If nodes are already deciding where to migrate a request, 
                        // hence incoming_request is Some (...), simply ignore any 
                        // further requests. 
                        match incoming_request
                        {
                            None =>
                                {
                                    // Acquire a copy of the node state. 
                                    // It will be preserved during the execution of
                                    // the consensus algorithm. 
                                    {
                                        let state = 
                                            application_state.lock().unwrap ();
                                        node_state = state.node_state.clone ();
                                        // Drop the value to force relinquishing the lock. 
                                        drop (state);
                                    }
    
                                    // Initialize the variables for a new execution. 
                                    incoming_request = Some (request);
                                    local_solver.clear (self.node_number,
                                                        self.penalty,
                                                        node_state.get_coord ());
                                    global_solver.clear ();
    
                                    // Perform the local update. 
                                    local_solver.local_x_update (request.get_desired_coord ());
    
                                    // Send x + u, note that the client will receive 
                                    // its own message. 
                                    let local_sum =
                                        local_solver.get_local() + local_solver.get_dual();
                                    let message_local = MessageLocal
                                    {
                                        src: self.node_index,
                                        local_sum,
                                    };
                                    let msg = mqtt::Message::new (
                                        &self.topics[1],
                                        message_local.to_string (),
                                        paho_mqtt::QOS_1);
                                    self.client.publish (msg).await?;
                                }
                            _ =>
                                {
                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - federation/migration IGNORE");
                                    // Ignore the request.
                                }
                        }
                    }
                    // federation/local_update -> f32. 
                    else if msg.topic () == self.topics[1]
                    {

                        #[cfg(feature = "print_log")]
                        println! ("requests_coordination_loop - federation/local_update LOCAL {:?}", msg.payload_str ());

                        // Parse the received message. 
                        let message_local = 
                            msg.payload_str ().parse::<MessageLocal> ()
                                .expect ("Unable to parse message into f32 (in federation/local_update)");
                        let x   : f32   = message_local.local_sum;
                        let src : usize = message_local.src;

                        // Add x to X, the set of local variables. 
                        global_solver.add_local_sum (x, src);

                        #[cfg(feature = "print_log")]
                        println! ("requests_coordination_loop - global_solver.locals_len () = {}", global_solver.locals_len ());

                        // Wait until all nodes have completed their local update. 
                        if global_solver.locals_len () == self.node_number
                        {
                            // Global update on Z. 
                            global_solver.global_z_updater ();
                            if global_solver.terminated ()
                            {

                                // Load the value of the dest_node.
                                dest_node = Some (global_solver.get_max_global_index ());

                                #[cfg(feature = "print_log")]
                                println! ("requests_coordination_loop - global_solver.terminated() = {}", global_solver.get_max_global_index());

                                // Check if this node is the src node, hence the one
                                // that has to send the request.
                                match src_node
                                {
                                    Some (src) =>
                                        {
                                            if src == self.node_index
                                            {

                                                #[cfg(feature = "print_log")]
                                                println! ("requests_coordination_loop - src == self.node_index");

                                                if dest_node.expect("Destination node missing. ") == self.node_index
                                                {

                                                    #[cfg(feature = "print_log")]
                                                    println! ("requests_coordination_loop - src == dest");

                                                    // The migration is not convenient after all.
                                                    // TODO: what to do here?

                                                }
                                                else
                                                {
                                                    let dest_topic = format! ("{}/{}",
                                                                              "federation/dst",
                                                                              dest_node.expect ("Missing dst node. "));

                                                    #[cfg(feature = "print_log")]
                                                    println! ("requests_coordination_loop - dest_topic = {dest_topic}");

                                                    // Send your address to the destination node.
                                                    let msg = mqtt::Message::new (
                                                        dest_topic,
                                                        self.ip_and_port.to_string (),
                                                        paho_mqtt::QOS_1);
                                                    self.client.publish (msg).await?;
                                                }
                                            }
                                        }
                                    _ => {
                                        panic! ("Src node unknown");
                                        }
                                }

                                #[cfg(feature = "timing_log")]
                                unsafe
                                    {
                                        libc::clock_gettime (libc::CLOCK_MONOTONIC, &mut end_time);
                                        let time_to_completion : f64;
                                        let mut diff_sec = end_time.tv_sec - start_time.tv_sec;
                                        let mut diff_nsec = end_time.tv_nsec - start_time.tv_nsec;
                                        if diff_nsec < 0
                                        {
                                            diff_nsec += 1_000_000_000;
                                            diff_sec -= 1;
                                        }
                                        time_to_completion = (diff_sec * 1_000) as f64 + (diff_nsec / 1_000_000) as f64;
                                        println! ("requests_coordination_loop - time_to_completion = {} ms",
                                                  time_to_completion);
                                    }
                            }
                            else
                            {
                                // Update the local_solver with the new global value, 
                                // then perform the dual update.
                                local_solver.set_global (
                                    global_solver.get_global_from_index (self.node_index));
                                local_solver.local_dual_update ();

                                // Finally, do the local update and send it.
                                local_solver.local_x_update (
                                    incoming_request.unwrap ().get_desired_coord ());

                                // Send x + u, note that the client will receive
                                // its own message.
                                let local_sum =
                                    local_solver.get_local () + local_solver.get_dual ();
                                let message_local = MessageLocal
                                {
                                    src: self.node_index,
                                    local_sum,
                                };
                                let msg = mqtt::Message::new (
                                    &self.topics[1],
                                    message_local.to_string (),
                                    paho_mqtt::QOS_1);
                                self.client.publish (msg).await?;
                            }
                        }
                    }
                    // federation/src/i -> ip:port
                    else if msg.topic () == self.topics[2]
                    {

                        #[cfg(feature = "print_log")]
                        println! ("requests_coordination_loop - federation/src SEND");

                        match incoming_request
                        {
                            Some (request) =>
                                {

                                    // First, we need to remove the request from the
                                    // pool of requests served in this node for this
                                    // application.
                                    {
                                        let mut state =
                                            application_state.lock ().unwrap ();
                                        state.remove_request (request.get_index ());
                                        drop (state);
                                    }

                                    // Then, update the barrier for the sporadic server.
                                    {
                                        let (number_of_requests, _barrier) = &*barrier;
                                        *number_of_requests.lock ().unwrap () -= 1;
                                    }

                                    incoming_request = None;

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - incoming_request = None");

                                    // Compress the folder of the request.

                                    let zip_archive_name =
                                        format! ("{}_{}_req.zip", self.application_index, request.get_index ());
                                    let zip_archive_path_str = format! ("requests/{}", zip_archive_name);
                                    let zip_archive_path =
                                        std::path::Path::new (&zip_archive_path_str);
                                    let zip_archive = std::fs::File::create (&zip_archive_path).unwrap ();

                                    let mut zip = zip::ZipWriter::new (zip_archive);

                                    // The files that might be compressed (memories are optional).
                                    let path_to_req_dir = format! ("requests/{}_{}_req/",
                                                                   self.application_index,
                                                                   request.get_index ());
                                    let module_path = format! ("{}/module.wasm", path_to_req_dir);
                                    let main_mem_path = format! ("{}/main_memory.b", path_to_req_dir);
                                    let checkpoint_mem_path = format! ("{}/checkpoint_memory.b", path_to_req_dir);
                                    let files_to_compress: Vec<std::path::PathBuf> = vec![
                                        std::path::PathBuf::from (module_path),
                                        std::path::PathBuf::from (main_mem_path),
                                        std::path::PathBuf::from (checkpoint_mem_path)
                                    ];

                                    let options: zip::write::FileOptions<()> = zip::write::FileOptions::default ()
                                        .compression_method (zip::CompressionMethod::DEFLATE);

                                    for file_path in &files_to_compress
                                    {
                                        let file = std::fs::File::open (file_path);
                                        match file
                                        {
                                            Ok(mut file) =>
                                                {
                                                    // Rather ugly, but.
                                                    let file_name =
                                                        file_path.file_name ().unwrap ()
                                                            .to_str ().unwrap ();

                                                    #[cfg(feature = "print_log")]
                                                    println! ("requests_coordination_loop - COMPRESSING = {}", file_name);

                                                    zip.start_file (file_name, options).unwrap ();

                                                    let mut buffer = Vec::new ();
                                                    file.read_to_end (&mut buffer).unwrap ();

                                                    zip.write_all (&buffer).unwrap ();
                                                }
                                            Err(_) =>
                                                {
                                                    // The file is missing, proceed.
                                                    continue;
                                                }
                                        }
                                    }
                                    zip.finish ().unwrap ();

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - FILE COMPRESSED");

                                    // Connect to the listener.
                                    let dst = msg.payload_str ()
                                        .parse::<String> ()
                                        .expect ("Unable to parse message into String");

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - dst is {}", dst);

                                    let mut writer = std::net::TcpStream::connect (dst)
                                        .expect ("Unable to bind to address");
                                    let mut buffer = [0; 512];
                                    let mut compressed_file =
                                        std::fs::OpenOptions::new ()
                                            .read (true)
                                            .open (&zip_archive_path)
                                            .expect ("Unable to open compressed_file. ");
                                    loop
                                    {
                                        let n = compressed_file.read (&mut buffer)?;
                                        if n == 0
                                        {
                                            writer.shutdown (std::net::Shutdown::Both)?;
                                            break;
                                        }
                                        writer.write_all (&buffer[..n])?;
                                    }

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - END TRANSMISSION");

                                    // Remove the directory corresponding to the request.
                                    let request_dir =
                                        format! ("/requests/{}_{}_req", self.application_index, request.get_index ());
                                    std::fs::remove_dir_all (request_dir).unwrap ();
                                }
                            None =>
                                {
                                    // Do nothing.
                                }
                        }
                    }
                    // federation/dst/i -> ip:port
                    else if msg.topic () == self.topics[3]
                    {

                        #[cfg(feature = "print_log")]
                        println! ("requests_coordination_loop - federation/dst RECEIVE");

                        match incoming_request
                        {
                            Some (request) =>
                                {

                                    // First, we need to accept the request, adding it to
                                    // the pool of requests served in this node for this
                                    // application.
                                    // To do so, we need to modify the application state.
                                    {
                                        let mut state =
                                            application_state.lock ().unwrap ();
                                        state.add_request (request);
                                        drop (state);
                                    }

                                    // Then receive the bytecode (and checkpoint).

                                    // Open a TCP stream for receiving the data.
                                    let listener =
                                        std::net::TcpListener::bind (self.ip_and_port.to_string ())
                                        .expect ("Unable to bind to address");

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - prepare message for Node {}", src_node.unwrap_or(999));

                                    // Prepare the message to signal the sender that you are ready.
                                    let msg = mqtt::Message::new (
                                        format! ("federation/src/{}", src_node.expect ("Missing src node. ")),
                                        self.ip_and_port.to_string (),
                                        paho_mqtt::QOS_1);

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - START RECEIVING");

                                    // Connect to the src and receive the data (or
                                    // wait for it).
                                    let compressed_file_name =
                                        format! ("{}_{}_req.zip", self.application_index, request.get_index ());
                                    let mut compressed_file = std::fs::OpenOptions::new ()
                                        .append (true)
                                        .write (true)
                                        .read (true)
                                        .create (true)
                                        .open (&compressed_file_name)
                                        .expect ("Unable to create compressed_file. ");

                                    self.client.publish (msg).await?;
                                    'handle_connection: for stream in listener.incoming ()
                                    {
                                        match stream 
                                        {
                                            Ok (mut stream) => 
                                                {
                                                    // Then loop on the incoming data from the stream.
                                                    let mut buffer = [0; 512];
                                                    loop
                                                    {

                                                        let n = stream.read (&mut buffer)?;
                                                        compressed_file.write_all (&buffer[0..n])?;

                                                        #[cfg(feature = "print_log")]
                                                        println! ("requests_coordination_loop - NEW CHUNK of size {}", n);

                                                        if n == 0
                                                        {
                                                            break 'handle_connection;
                                                        }
                                                    }
                                                }
                                            Err (e) =>
                                                {
                                                    eprintln! ("Connection failed: {e}")
                                                }
                                        }
                                    }

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - compressed FILE RECEIVED");

                                    // Then decompress the file as a folder.
                                    let fname : &std::path::Path =
                                        std::path::Path::new (&compressed_file_name);
                                    let file  : std::fs::File    =
                                        std::fs::File::open (fname).unwrap ();

                                    let mut archive =
                                        zip::ZipArchive::new (file).unwrap ();

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - archive len = {}", archive.len ());

                                    for i in 0..archive.len ()
                                    {
                                        let mut file = archive.by_index (i).unwrap ();
                                        let outpath = match file.enclosed_name ()
                                        {
                                            Some (path) => path,
                                            None => continue,
                                        };

                                        if file.is_dir ()
                                        {

                                            #[cfg(feature = "print_log")]
                                            println! ("requests_coordination_loop - File {} extracted to \"{}\"", i, outpath.display ());

                                            std::fs::create_dir_all (&outpath).unwrap ();
                                        }
                                        else
                                        {

                                            #[cfg(feature = "print_log")]
                                            println! ("requests_coordination_loop - File {} extracted to \"{}\" ({} bytes)", i, outpath.display (), file.size ());

                                            if let Some (p) = outpath.parent ()
                                            {
                                                if !p.exists ()
                                                {
                                                    std::fs::create_dir_all (p).unwrap ();
                                                }
                                            }
                                            let mut outfile = std::fs::File::create (&outpath)
                                                .unwrap ();
                                            std::io::copy (&mut file, &mut outfile).unwrap ();
                                        }
                                    }

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - FILE DECOMPRESSED");

                                    // Finally, update the barrier of the sporadic server.
                                    {
                                        let (number_of_requests, barrier) = &*barrier;
                                        *number_of_requests.lock ().unwrap () += 1;
                                        barrier.notify_one ();
                                    }

                                    // This final instruction allows the node to start a new
                                    // ADMM execution.
                                    incoming_request = None;

                                    #[cfg(feature = "print_log")]
                                    println! ("requests_coordination_loop - incoming_request = None");

                                }
                            None =>
                                {
                                    // Do nothing.
                                }
                        }
                    }
                }
            }
            
            // Explicit return type for the async block. 
            Ok::<(), mqtt::Error> (())
        }) 
        {
           eprintln! ("requests_monitoring_loop - error creating the client: {:?}", err);
        }
    }
}