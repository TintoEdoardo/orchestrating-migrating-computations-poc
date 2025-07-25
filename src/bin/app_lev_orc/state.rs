/*************************/
/*         STATE         */
/*************************/

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


/// The state of the hosting node, as a set of
/// coordinates corresponding to the position
/// of a node (device) in a plant.
#[derive(PartialEq, Clone, Copy)]
pub struct NodeState
{
    node_coords  : Coord,
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
}

impl std::str::FromStr for NodeState
{
    type Err = std::convert::Infallible;

    fn from_str (s: &str) -> Result<Self, Self::Err>
    {
        let trimmed_s = s.replace ('(', "").replace (')', "");
        let coords : Vec<&str> = trimmed_s.split_terminator (',').collect ();

        match (coords.first(), coords.last())
        {
            (Some (&x), Some (&y)) => Ok (
                NodeState {
                    node_coords: Coord { 
                        x : f32::from_str (x).expect ("Failed to parse x coord. "),
                        y : f32::from_str (y).expect ("Failed to parse y coord. ")}
                    }
            ),
            _ => Ok (
                NodeState
                {
                    node_coords: Coord
                    {
                        x : -1.0,
                        y : -1.0
                    }
                }
            ),
        }
    }
}

impl std::fmt::Display for NodeState
{
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write! (f, "{}", format! ("({},{})", self.node_coords.x, self.node_coords.y))
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
    should_migrate  : bool
}

impl Request
{
    #[allow(dead_code)]
    pub fn new_from (
        index           : usize,
        execution_time  : u32,
        required_memory : u32,
        desired_coord   : Coord,
        threshold       : f32) -> Self
    {
        Self { index, execution_time, required_memory, desired_coord, threshold, should_migrate : false }
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
}

impl std::str::FromStr for Request
{
    type Err = std::string::ParseError;

    /// Expected string: '\[us; u32; u32; (f32, f32); f32\]'
    fn from_str(s: &str) -> Result<Self, Self::Err>
    {
        let mut index           : usize = 0;
        let mut execution_time  : u32   = 0;
        let mut required_memory : u32   = 0;
        let mut desired_coord   : Coord = Coord { x: -1.0, y: -1.0 };
        let mut threshold       : f32   = 0.0;

        let trimmed_s = s.replace ('[', "").replace (']', "");
        let fields : Vec<&str> = trimmed_s.split_terminator (';').collect ();

        match
        (
            fields.get (0),
            fields.get (1),
            fields.get (2),
            fields.get (3),
            fields.get (4)
        )
        {
            (
                Some (&req_index),
                Some (&exec_time),
                Some (&req_memory),
                Some (&des_coord),
                Some (&thresh)
            ) => {
                index           = usize::from_str (req_index)
                    .expect ("Failed to parse index. ");

                execution_time  = u32::from_str(exec_time)
                    .expect ("Unable to convert exec_time to u32");

                required_memory = u32::from_str(req_memory)
                    .expect ("Unable to convert req_memory to u32");

                desired_coord   = NodeState::from_str(des_coord)
                        .expect ("Unable to convert des_coord to Coord")
                        .node_coords; // This is atrocious, correct: TODO!

                threshold       = f32::from_str(thresh)
                    .expect ("Unable to convert thresh to f32");
            }
            _ => {}
        }

        Ok (Request
        {
            index,
            execution_time,
            required_memory,
            desired_coord,
            threshold,
            should_migrate : false,
        })
    }
}

impl std::fmt::Display for Request
{
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let str = format! ("[{};{};{};({},{});{}]",
                          self.index,
                          self.execution_time,
                          self.required_memory,
                          self.desired_coord.x,
                          self.desired_coord.y,
                          self.threshold);
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
    // Node-related fields.
    pub node_state : NodeState,

    // Application-related fields.
    pub requests           : Vec<Request>,
    pub number_of_requests : u32,
}

impl ApplicationState
{
    pub fn new (node_coords : Coord) -> Self
    {
        Self
        {
            node_state         : NodeState { node_coords }, 
            requests           : Vec::with_capacity (5),
            number_of_requests : 0, 
        }
    }

    #[allow(dead_code)]
    pub fn read_node_state (self) -> NodeState
    {
        self.node_state.clone ()
    }

    pub fn set_node_state (&mut self, coord : Coord)
    {
        self.node_state.set_coords (coord);
    }

    #[allow(dead_code)]
    pub fn add_request (&mut self, request : Request)
    {
        self.requests.push (request);
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
        self.requests.remove (local_index);
    }

    #[allow(dead_code)]
    pub fn get_number_of_requests (&self) -> u32
    {
        self.number_of_requests
    }

    #[allow(dead_code)]
    pub fn get_requests (&self, index : usize) -> &Request
    {
        &self.requests[index]
    }
}