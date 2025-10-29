/*************************/
/*         STATE         */
/*************************/
use std::fmt::{Display, Formatter};

/// The main information used to determine the state of
/// the system in this experimentation is the physical
/// location of the nodes, expressed as coordinates.
#[derive(PartialEq, Clone, Copy)]
pub struct Coord
{
    x : f32,
    y : f32,
}

impl Coord
{
    pub fn new () -> Self
    {
        Self { x: 0.0, y: 0.0 }
    }

    #[allow(dead_code)]
    pub fn new_from (x : f32, y : f32) -> Self
    {
        Self { x, y }
    }

    pub fn get_x (&self) -> f32
    {
        self.x
    }

    pub fn get_y (&self) -> f32
    {
        self.y
    }
}

impl std::str::FromStr for Coord
{
    type Err = std::convert::Infallible;

    /// The expected string: (f32,f32).
    fn from_str(s: &str) -> Result<Self, Self::Err>
    {
        let trimmed_s = s.replace ('(', "").replace (')', "");
        let coords : Vec<&str> = trimmed_s.split_terminator (',').collect ();

        match (coords.first(), coords.last())
        {
            (Some (&x), Some (&y)) => Ok (
                Coord
                {
                    x : f32::from_str (x).expect ("Failed to parse x coord. "),
                    y : f32::from_str (y).expect ("Failed to parse y coord. ")
                }
            ),
            _ => Ok (
                Coord
                {
                    x : -1.0,
                    y : -1.0
                }
            ),
        }
    }
}

impl Display for Coord
{
    fn fmt (&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write! (f, "{}", format! ("({},{})", self.x, self.y))
    }
}

/// The state of the hosting node, as a set of
/// coordinates corresponding to the position
/// of a node (device) in a plant.
#[derive(PartialEq, Clone, Copy)]
pub struct NodeState
{
    /// Coordinates of the current node.
    node_coords    : Coord,

    /// Speedup factor with respect to a reference node.
    speedup_factor : f32,
}

impl NodeState
{
    pub fn set_coords (&mut self, coord : Coord)
    {
        self.node_coords = coord;
    }

    pub fn get_coord (&self) -> Coord
    {
        self.node_coords
    }

    pub fn get_speedup_factor (&self) -> f32
    {
        self.speedup_factor
    }
}

impl std::str::FromStr for NodeState
{
    type Err = std::convert::Infallible;

    /// The expected string: [(f32,f32);f32]
    fn from_str (s: &str) -> Result<Self, Self::Err>
    {
        let trimmed_s = s.replace ('[', "").replace (']', "");
        let fields : Vec<&str> = trimmed_s.split_terminator (';').collect ();

        match (fields.first (), fields.last ())
        {
            (Some (&coord), Some (&speedup_factor)) => Ok (
                NodeState
                {
                    node_coords   : coord.parse ().unwrap (),
                    speedup_factor: speedup_factor.parse::<f32> ().unwrap (),
                }
            ),
            _ => Ok (
                NodeState
                {
                    node_coords: Coord
                    {
                        x : -1.0,
                        y : -1.0
                    },
                    speedup_factor: f32::MAX,
                }
            ),
        }
    }
}

impl std::fmt::Display for NodeState
{
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write! (f, "{}", format! ("[({},{});{}]", self.node_coords.x, self.node_coords.y, self.speedup_factor))
    }
}


/// A migratable request, in this experimentation
/// in the form of a Wasm function within a module.
#[derive(Clone, Copy)]
pub struct Request
{
    /// Request index (application-wise).
    index : usize,

    /// Estimated WCET in millisec (ms).
    execution_time  : u32,

    /// Desired completion time.
    desired_completion_time : u32,

    /// Migratable up to this checkpoint. From there on
    /// it is not convenient to migrate.
    migratable_up_to : usize,

    /// Required memory in kB.
    required_memory : u32,

    /// Desired geographical position.
    desired_coord   : Coord,

    /// Threshold for triggering a migration as
    /// maximum allowed difference from the desired
    /// coordinates (of a request) and the actual
    /// coordinates (of the hosting node).
    threshold       : f32,

    /// Migration flag: 'true' that this request has
    /// to migrate.
    should_migrate  : bool,

    /// Current region.
    current_region  : usize,

    /// Arrival time.
    arrival_time    : std::time::Instant
}

impl Request
{
    #[allow(dead_code)]
    pub fn new_from (
        index                   : usize,
        execution_time          : u32,
        desired_completion_time : u32,
        migratable_up_to        : usize,
        required_memory         : u32,
        desired_coord           : Coord,
        threshold               : f32,
        current_region          : usize) -> Self
    {
        Self
        {
            index,
            execution_time,
            desired_completion_time,
            migratable_up_to,
            required_memory,
            desired_coord,
            threshold,
            should_migrate : false,
            current_region,
            arrival_time   : std::time::Instant::now ()
        }
    }

    pub fn set_should_migrate (&mut self, migrate : bool)
    {
        self.should_migrate = migrate;
    }

    pub fn get_should_migrate (&self) -> bool
    {
        self.should_migrate
    }

    pub fn get_desired_coord(&self) -> Coord
    {
        self.desired_coord
    }

    pub fn get_index(&self) -> usize
    {
        self.index
    }

    pub fn get_execution_time(&self) -> u32
    {
        self.execution_time
    }
}

impl std::str::FromStr for Request
{
    type Err = std::string::ParseError;

    /// Expected string:
    /// [index; execution_time; desired_completion_time; migratable_up_to; required_memory; (desired_coord); threshold; current_region]
    /// '\[usize; u32; u32; usize; u32; (f32, f32); f32; f32\]'
    fn from_str(s: &str) -> Result<Self, Self::Err>
    {
        let mut index                   : usize = 0;
        let mut execution_time          : u32   = 0;
        let mut desired_completion_time : u32   = 0;
        let mut migratable_up_to        : usize = 0;
        let mut required_memory         : u32   = 0;
        let mut desired_coord           : Coord = Coord { x: -1.0, y: -1.0 };
        let mut threshold               : f32   = 0.0;
        let mut current_region          : usize = 0;

        let trimmed_s = s.replace ('[', "").replace (']', "");
        let fields : Vec<&str> = trimmed_s.split_terminator (';').collect ();

        match
        (
            fields.get (0),
            fields.get (1),
            fields.get (2),
            fields.get (3),
            fields.get (4),
            fields.get (5),
            fields.get (6),
            fields.get (7)
        )
        {
            (
                Some (&req_index),
                Some (&exec_time),
                Some (&des_com_time),
                Some (&mig_up_to),
                Some (&req_memory),
                Some (&des_coord),
                Some (&thresh),
                Some (&cur_region)
            ) => {
                index           = usize::from_str (req_index)
                    .expect ("Failed to parse index. ");

                execution_time  = u32::from_str (exec_time)
                    .expect ("Unable to convert exec_time to u32");

                desired_completion_time = u32::from_str (des_com_time)
                    .expect ("Unable to convert des_com_time to u32");

                migratable_up_to = usize::from_str (mig_up_to)
                    .expect ("Unable to convert mig_up_to to usize");

                required_memory = u32::from_str (req_memory)
                    .expect ("Unable to convert req_memory to u32");

                desired_coord   = Coord::from_str (des_coord)
                        .expect ("Unable to convert des_coord to Coord");

                threshold       = f32::from_str(thresh)
                    .expect ("Unable to convert thresh to f32");

                current_region  = usize::from_str (cur_region)
                    .expect ("Unable to convert curr_region to usize");
            }
            _ => {}
        }

        Ok (Request
        {
            index,
            execution_time,
            desired_completion_time,
            migratable_up_to,
            required_memory,
            desired_coord,
            threshold,
            should_migrate : false,
            current_region,
            arrival_time   : std::time::Instant::now ()
        })
    }
}

impl std::fmt::Display for Request
{
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let str = format! ("[{};{};{};{};{};({},{});{};{}]",
                          self.index,
                          self.execution_time,
                          self.desired_completion_time,
                          self.migratable_up_to,
                          self.required_memory,
                          self.desired_coord.x,
                          self.desired_coord.y,
                          self.threshold,
                          self.current_region);
        write! (f, "{}", str)
    }
}


pub struct MessageRequest
{
    src     : usize,
    request : Request,
}

impl MessageRequest
{
    pub fn new (src: usize, request : Request) -> Self
    {
        Self { src, request }
    }
    pub fn get_src (&self) -> usize
    {
        self.src
    }

    pub fn get_request (&self) -> &Request
    {
        &self.request
    }
}

impl std::str::FromStr for MessageRequest
{
    type Err = std::string::ParseError;

    fn from_str (s: &str) -> Result<Self, Self::Err>
    {
        let strs : Vec<&str> = s.split_terminator ('#').collect ();
        match (strs.first (), strs.last ()) {
            (Some (&str1), Some (&str2)) =>
                {
                    Ok (MessageRequest
                    {
                        src: usize::from_str(str1)
                            .expect ("Unable to convert string to usize"),
                        request: Request::from_str(str2)
                            .expect ("Unable to convert string to Request")
                    })
                }
            _ =>
                {
                    panic! ("Error while parsing a MessageRequest");
                }
        }
    }
}

impl std::fmt::Display for MessageRequest
{
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write! (f, "{}", format! ("{}#{}", self.src, self.request.to_string ()))
    }
}


/// Decide whether to trigger a migraiton, depending
/// on the distance between a request desired state 
/// and the node state
pub fn should_migrate (request : &Request, node_state : &NodeState) -> bool
{
    let mut result = false;
    let distance = f32::sqrt (
        (request.desired_coord.x - node_state.get_coord ().x).powi (2)
        + (request.desired_coord.y - node_state.get_coord ().y).powi (2));
    if request.threshold > distance
    {
        result = true
    }
    result
}


/// The state of the application is composed of: 
/// (1) node-related information,
/// (2) application-specific information.
#[derive(Clone)]
pub struct ApplicationState
{
    /// Node-related fields.
    pub node_state : NodeState,

    /// Period of the sporadic server associated to
    /// this application, in milliseconds.
    pub sporadic_server_t  : u32,

    /// Execution time of the sporadic server associated
    /// to this application, in milliseconds.
    pub sporadic_server_c  : u32,

    #[allow(dead_code)]
    /// Memory assigned to the application in kB.
    pub assigned_memory    : u32,

    /// Available memory in kB.
    pub available_memory   : u32,

    /// Sum of the computation time for all the requests in the
    /// backlog, in milliseconds.
    pub backlog_sum_of_c   : u32,

    /// Application-related fields.
    pub requests           : Vec<Request>,
    pub number_of_requests : u32,

    /// A vector of request indexes sorted by
    /// earlier desired completion time.
    pub requests_by_dct    : Vec<usize>,

    /// Checkpoint ready.
    pub checkpoint_is_ready: bool,
}

impl ApplicationState
{
    pub fn new (node_coords       : Coord,
                sporadic_server_t : u32,
                sporadic_server_c : u32,
                speedup_factor    : f32,
                assigned_memory: u32) -> Self
    {
        Self
        {
            node_state         : NodeState { node_coords, speedup_factor },
            sporadic_server_t,
            sporadic_server_c,
            assigned_memory,
            available_memory   : assigned_memory,
            backlog_sum_of_c   : 0,
            requests           : Vec::with_capacity (5),
            number_of_requests : 0,
            requests_by_dct    : Vec::with_capacity (5),
            checkpoint_is_ready: false,
        }
    }

    #[allow(dead_code)]
    pub fn get_node_state (self) -> NodeState
    {
        self.node_state.clone ()
    }

    pub fn set_node_state (&mut self, coord : Coord)
    {
        self.node_state.set_coords (coord);
    }

    pub fn get_request (&self, request_index : usize) -> Option<&Request>
    {
        for request in &self.requests
        {
            if request.index == request_index
            {
                return Some (request);
            }
        }
        None
    }

    pub fn add_request (&mut self, request : Request)
    {
        // Enqueue the incoming request.
        self.requests.push (request);
        self.number_of_requests += 1;

        // Then update backlog_sum_of_c.
        self.backlog_sum_of_c += request.execution_time;

        // Update the resource consumption variables.
        self.available_memory -= request.required_memory;

        // Insert the request index into requests_by_dct.
        if self.requests_by_dct.is_empty ()
        {
            self.requests_by_dct.push (request.index);
        }
        else
        {
            // Compute how much ime we have to complete
            // `request' according to its desired specs.
            let remaining_time =
                request.desired_completion_time as i64
                    - request.arrival_time.elapsed ().as_millis () as i64;

            for i in 0..self.requests_by_dct.len () {
                let index = self.requests_by_dct[i];
                let request_i = self.get_request (index)
                    .expect ("Unable to get request from index");

                // We do the same also for the request with index 'index'.
                let remaining_time_i =
                    request_i.desired_completion_time as i64
                        - request_i.arrival_time.elapsed ().as_millis () as i64;
                if remaining_time_i > remaining_time
                {
                    self.requests_by_dct.insert (i, request.index);
                    break;
                }
            }
        }
    }

    pub fn remove_request (&mut self, request_index : usize)
    {
        let mut local_index : usize = 0;
        for i in 0..self.requests.len ()
        {
            if self.requests[i].index == request_index
            {
                local_index = i;
            }
        }

        // Update backlog_sum_of_c before removing the request.
        let request    = self.requests[local_index];
        self.backlog_sum_of_c -= request.execution_time;

        // Update the resource consumption variables.
        self.available_memory += request.required_memory;

        // Remove the request index from requests_by_dct.
        if !self.requests_by_dct.is_empty ()
        {
            for i in 0..self.requests_by_dct.len () {
                if request.index == self.requests_by_dct[i]
                {
                    self.requests_by_dct.remove (i);
                    break;
                }
            }
        }

        // Then, remove the request.
        self.requests.remove (local_index);
        self.number_of_requests -= 1;
    }

    #[allow(dead_code)]
    pub fn get_number_of_requests (&self) -> u32
    {
        self.number_of_requests
    }

    pub fn advance_cur_region_of_request (&mut self, request_index : usize)
    {
        for i in 0..self.requests.len ()
        {
            if self.requests[i].index == request_index
            {
                self.requests[i].current_region += 1;
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_cur_region_of_request (&mut self, request_index : usize) -> usize
    {
        let mut cur_region : usize = 0;
        for i in 0..self.requests.len ()
        {
            if self.requests[i].index == request_index
            {
                cur_region = self.requests[i].current_region
            }
        }
        cur_region
    }

    pub fn is_request_migratable (&self, request_index : usize) -> bool
    {
        match self.get_request (request_index)
        {
            Some(request) =>
                {
                    #[cfg(feature = "print_log")]
                    println! ("request.migratable_up_to = {}; request.current_region = {}", request.migratable_up_to, request.current_region);
                    request.migratable_up_to > request.current_region
                }
            None =>
                {
                    false
                }
        }
    }
    pub fn get_should_migrate_of_request (&mut self, request_index : usize) -> bool
    {
        self.get_request (request_index).unwrap ().should_migrate
    }

    pub fn set_should_migrate_of_request (&mut self, request_index : usize, should_migrate : bool)
    {
        for i in 0..self.requests.len ()
        {
            if self.requests[i].index == request_index
            {
                self.requests[i].should_migrate = should_migrate;
            }
        }
    }

    pub fn get_expected_completion_time (&self, request_c: u32) -> u32
    {
        (((self.backlog_sum_of_c + request_c ) / self.sporadic_server_c) as f32 * self.node_state.speedup_factor).ceil () as u32 * self.sporadic_server_t
    }
    pub fn could_host_computation (&self, request: &Request) -> bool
    {
        let mut result = false;
        if request.required_memory < self.available_memory
        {
            result = true;
        }
        result
    }
}