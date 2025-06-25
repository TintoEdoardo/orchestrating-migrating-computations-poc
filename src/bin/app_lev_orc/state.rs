use std::{convert::Infallible, str::FromStr};

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