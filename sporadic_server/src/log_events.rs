/***********************************************/
/*  L O G G E R   F O R   T I M E   E V E N T  */
/***********************************************/
use std::fmt::Display;
use std::io::Write;

#[allow(dead_code)]
pub enum EventType
{
    BudgetExhausted,
    ReleaseEvent,
    RequestCompleted,
}

impl Display for EventType
{
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let event_string = match self
        {
            EventType::BudgetExhausted  => "BE",
            EventType::ReleaseEvent     => "RE",
            EventType::RequestCompleted => "RC",
        };
        write! (f, "{}", event_string)?;
        Ok(())
    }
}

/// We use this structure to track the time event
/// associated with the sporadic server task.
pub struct EventLogger
{
    events         : Vec<(EventType, std::time::Instant)>,

    /// The number of recorded events.
    recorded_events: u32,
}

#[allow(dead_code)]
impl EventLogger
{
    pub fn new() -> Self
    {
        Self
        {
            events         : Vec::with_capacity (1_000),
            recorded_events: 0,
        }
    }

    pub fn add_event (&mut self, event_type: EventType, start_time: std::time::Instant)
    {
        self.events.push ((event_type, start_time));
        self.recorded_events += 1;
    }

    pub fn print_events (&mut self) -> std::io::Result<()>
    {
        let mut log_file = std::fs::File::options ()
            .create (true)
            .append (true)
            .open ("sporadic_server_log.txt")?;

        for (event_type, time) in &self.events
        {
            writeln! (log_file, "{}--{:?}", event_type, time)?;
        }

        // The clear the logger.
        self.events.clear ();
        self.recorded_events = 0;

        Ok(())
    }

    pub fn print_event (&mut self, event_type: EventType, time: std::time::Instant)
        -> std::io::Result<()>
    {
        let mut log_file = std::fs::File::options ()
            .create (true)
            .append (true)
            .open ("sporadic_server_log.txt")?;

        writeln! (log_file, "{}--{:?}", event_type, time)?;

        Ok(())
    }
}