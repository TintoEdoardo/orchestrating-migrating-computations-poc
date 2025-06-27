use std::{convert::Infallible, str::FromStr, string::ParseError};

///              ///
///     STATE    ///
///              ///

extern crate alloc;

#[derive(PartialEq, Clone, Copy)]
pub struct Coord {
    x : f32,
    y : f32,
}

impl Coord {
    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn new_from(x : f32, y : f32) -> Self {
        Self { x, y }
    }

    pub fn get_x(&self) -> f32 {
        self.x
    }

    pub fn get_y(&self) -> f32 {
        self.y
    }

}

/// The state of the hosting node
#[derive(PartialEq, Clone, Copy)]
pub struct NodeState {
    node_coords  : Coord,
}

impl NodeState {
    pub fn update(&mut self, coord : Coord) {
        self.node_coords = coord;
    }

    pub fn get_coord(&self) -> Coord {
        self.node_coords
    }

}

impl FromStr for NodeState {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed_s = s.replace('(', "").replace(')', "");
        let coords : Vec<&str> = trimmed_s.split_terminator(',').collect();
        match (coords.first(), coords.last()) {
            (Some(&x), Some(&y)) => Ok(
                NodeState { 
                    node_coords: Coord { 
                        x : f32::from_str(x).expect("Failed to parse x coord. "),
                        y : f32::from_str(y).expect("Failed to parse y coord. ")}
                    }),
            _ => Ok(
                NodeState { 
                    node_coords: Coord { 
                        x : -1.0,
                        y : -1.0}
                    }),
        }
    }
}

impl ToString for NodeState {
    fn to_string(&self) -> String {
        format!("({},{})", self.node_coords.x, self.node_coords.y)
    }
}

/// A Request is characterized by three elements: 
/// (1) its worst-case execution time (WCET) 
/// (2) its reuirements, e.g., memory 
/// (3) its desired configuration, e.g., its position
#[derive(Clone, Copy)]
pub struct Request {
    // Estimated WCET in millisec (ms)
    execution_time  : u32,

    // Required memory in kB
    required_memory : u32,

    // Desired geographical position
    desired_coord   : Coord,

    // Threshold for migration
    threshold       : f32,

    // Migration flag
    should_migrate  : bool
}

impl Request {
    pub fn new_from(
        execution_time  : u32,
        required_memory : u32,
        desired_coord   : Coord,
        threshold       : f32) -> Self {
        Self { execution_time, required_memory, desired_coord, threshold, should_migrate : false }
    }

    pub fn set_should_migrate(&mut self, migrate : bool) {
        self.should_migrate = migrate;
    }

    pub fn read_desired_coord(&self) -> Coord {
        self.desired_coord
    }
}

impl FromStr for Request {
    type Err = ParseError;

    /// Expected string: 
    ///     \[xx;xx;(xx,xx);xx\]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut execution_time  : u32   = 0;
        let mut required_memory : u32   = 0;
        let mut desired_coord   : Coord = Coord { x: -1.0, y: -1.0 };
        let mut threshold       : f32   = 0.0;

        let trimmed_s = s.replace('[', "").replace(']', "");
        let fields : Vec<&str> = trimmed_s.split_terminator(';').collect();

        match (fields.get(0), fields.get(1), fields.get(2), fields.get(3)) {
            (Some(&exec_time), Some(&req_memory), Some(&des_coord), Some(&thresh)) => {
                execution_time = u32::from_str(exec_time).expect("Unable to convert exec_time to u32");
                required_memory = u32::from_str(req_memory).expect("Unable to convert req_memory to u32");
                desired_coord = NodeState::from_str(des_coord).expect("Unable to convert des_coord to Coord").node_coords; // This is atrocious, correct: TODO!
                threshold = f32::from_str(thresh).expect("Unable to convert thresh to f32");
            }
            _ => {}
        }

        Ok(Request {
            execution_time, required_memory, desired_coord, threshold, should_migrate : false,
        })
    }
}

impl ToString for Request {
    fn to_string(&self) -> String {
        format!("[{};{};({},{});{}]", 
            self.execution_time, 
            self.required_memory, 
            self.desired_coord.x,
            self.desired_coord.y,
            self.threshold)
    }
}

/// Decide whether to trigger a migraiton, depending
/// on the distance between a request desired state 
/// and the node state
pub fn should_migrate(request : &Request, node_state : &NodeState) -> bool {
    let mut result = false;
    let distance = f32::sqrt(
        (request.desired_coord.x - node_state.get_coord().x).powi(2) 
        + (request.desired_coord.y - node_state.get_coord().y).powi(2));
    if request.threshold > distance {
        result = true
    }
    result
}

/// The state of the application is composed of: 
/// (1) node-related information
/// (2) application-specific information
#[derive(Clone)]
pub struct ApplicationState {
    // Node-related fields
    pub node_state : NodeState,

    // Appliation-related fields
    pub requests           : Vec<Request>,
    pub number_of_requests : u32,
}

impl ApplicationState {
    pub fn new(node_coords : Coord) -> Self {
        Self { 
            node_state         : NodeState { node_coords }, 
            requests           : Vec::with_capacity(5),
            number_of_requests : 0, 
        }
    }

    pub fn read_node_state(self) -> NodeState {
        self.node_state.clone()
    }

    pub fn update_node_state(&mut self, coord : Coord) {
        self.node_state.update(coord);
    }

    /// Function invoked by the destination node, after
    /// a migration occured
    pub fn add_request(&mut self, request : Request) {
        self.requests.push(request);
    }

    /// Function invoked in the source node, after
    /// a migration occured
    pub fn remove_request(&mut self, index : usize) {
        self.requests.remove(index);
    }

    pub fn get_request_number(&self) -> u32 {
        self.number_of_requests
    }

    pub fn read_request(&self, index : usize) -> &Request {
        &self.requests[index]
    }
}