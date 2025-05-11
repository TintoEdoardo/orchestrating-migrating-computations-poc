///     COMMUNICATION
/// Structs, types, and constants associated with communication.

// Number of nodes. 
pub static NUMBER_OF_NODES : usize = 4;

// MQTT topics. 
pub static TOPIC_FEDERATION_MAIN   : &str = "federate/main";
pub static TOPIC_FEDERATION_GATHER : &str = "federate/gather";
pub static TOPIC_COMMAND : &str = "federate";

// Send exactly once. 
pub static QUALITY_OF_SERVICE : u32 = 1;