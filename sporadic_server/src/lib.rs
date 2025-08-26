/***************************************/
/*   S P O R A D I C    S E R V E R    */
/***************************************/
use std::ops::Add;

mod utilities;

#[derive(Clone, Copy)]
pub struct SporadicServer
{
    // Platform identifier of the current server task.
    id          : u32,

    /// Task budget in milliseconds.
    budget      : std::time::Duration,

    /// Period of the server task.
    period      : std::time::Duration,

    /// Priority of the server when running.
    priority    : u32,
}

impl SporadicServer
{
    pub fn new (budget      : std::time::Duration,
                period      : std::time::Duration,
                priority    : u32) -> SporadicServer
    {
        Self
        {
            id: utilities::get_platform_tid (),
            budget,
            period,
            priority,
        }
    }

    pub fn start (&mut self,
                  controller       : std::sync::Arc<std::sync::Mutex<SporadicServerController>>)
    {

        // Register the current task to the controller,
        // then relinquish the lock on the controller.
        {
            controller.lock ().unwrap ().register (self);
        }

        // Sporadic server body.
        loop
        {
            {
                // Wait for the next activation.
                controller.lock ().unwrap ().wait_next_activation ();
            }

            // Workload.
            {
                println!("Start workload");
                let mut result = 0;
                for _i in 0..10_000
                {
                    for _j in 0..100_000
                    {
                        result = result + 1;
                    }
                }
                println!("End workload");
            }
        }
    }

    fn lower_priority (&self)
    {
        utilities::set_priority (self.id, 1);
    }

    fn rise_priority (&self)
    {
        utilities::set_priority (self.id, self.priority);
    }

}

#[derive(Clone, Copy, Debug)]
enum EventType
{
    BudgetExhausted,
    ReleaseEvent,
}

impl std::cmp::PartialEq for EventType
{
    fn eq (&self, other: &Self) -> bool
    {
        match (*self, *other)
        {
            (EventType::BudgetExhausted, EventType::BudgetExhausted) => true,
            (EventType::ReleaseEvent, EventType::ReleaseEvent)       => true,
            _                                                        => false
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Event
{
    /// Type of event.
    event_type: EventType,

    /// Time of the event.
    event_time: std::time::Instant,

    /// The budget consumed so far, used for replenishing the
    /// server budget.
    budget    : std::time::Duration,
}

pub struct SporadicServerController
{
    /// The controller task respond to events of two kinds:
    /// replenish events, when the budget is replenished, and
    /// budget exceeded events.
    r_event_queue   : std::collections::VecDeque<Event>,
    be_event_queue  : std::collections::VecDeque<Event>,

    /// Registered servers, in this implementation we assume
    /// a single server task (hence, a single variable).
    server          : Option<SporadicServer>,

    /// Starting budget for the current job of the server task.
    start_budget    : std::time::Duration,

    /// Release time of the server task.
    release_time    : std::time::Instant,

    /// Barrier used to trigger the execution of the server task.
    barrier         : std::sync::Arc<(std::sync::Mutex<u8>, std::sync::Condvar)>,

    /// Barrier used to suspend the controller task when
    /// the server task is not active.
    is_server_running: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,

    /// Whether the server is running or not.
    is_executing    : bool,

    /// Whether the budget has expired.
    has_expired     : bool,
}

impl SporadicServerController
{
    pub fn new (barrier       : std::sync::Arc<(std::sync::Mutex<u8>, std::sync::Condvar)>,
                is_ser_running: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>)
        -> SporadicServerController
    {
        Self
        {
            r_event_queue            : std::collections::VecDeque::new (),
            be_event_queue           : std::collections::VecDeque::new (),
            server                   : None,
            start_budget             : Default::default(),
            release_time             : std::time::Instant::now(),
            barrier                  : barrier,
            is_server_running        : is_ser_running,
            is_executing             : false,
            has_expired              : false,
        }
    }

    // Register a sporadic server task to this controller.
    pub fn register (&mut self, server : &SporadicServer)
    {
        self.server = Some(server.clone());
        self.start_budget = server.budget;
    }

    // Extract the next event from the two queues.
    fn get_next_event (&mut self) -> Option<Event>
    {
        match (self.r_event_queue.front (), self.be_event_queue.front ())
        {
            (None, None) =>
                {
                    None
                }
            (Some(_), None) =>
                {
                    Some(self.r_event_queue.pop_front ().unwrap ())
                }
            (None, Some(_)) =>
                {
                    Some(self.be_event_queue.pop_front ().unwrap ())
                }
            (Some(r_event), Some(be_event)) =>
                {
                    if r_event.event_time >= be_event.event_time
                    {
                        Some(self.be_event_queue.pop_front ().unwrap ())
                    }
                    else
                    {
                        Some(self.r_event_queue.pop_front ().unwrap ())
                    }
                }
        }
    }

    fn budget_consumed (&self) -> std::time::Duration
    {
        let clock = std::time::Instant::now ();
        // This is correct only when the server task is
        // not interfered. Otherwise, the real budget is
        // higher than this. This is a safe higher bound.
        clock - self.release_time
    }

    fn budget_remaining (&self) -> std::time::Duration
    {
        // As for budget_consumed, this over-approximate
        // the correct value in case of interference.
        let clock = std::time::Instant::now ();
        self.start_budget.checked_sub (clock - self.release_time)
            .unwrap_or (std::time::Duration::from_millis (0))
    }

    // Invoked by the server task, before starting the execution
    // of a migrating request.
    fn wait_next_activation (&mut self)
    {
        // Calculate the new remaining budget and consumed budget.
        let remaining_budget : std::time::Duration = self.budget_remaining ();
        let consumed_budget  : std::time::Duration = self.budget_consumed ();

        println!("Inside wait next activation: ");
        println!(" -> release time {:?}",     self.release_time);
        println!(" -> consumed budget {:?}",  consumed_budget);
        println!(" -> remaining budget {:?}", remaining_budget);

        // Extract the server from its envelop.
        let server = self.server.unwrap ();

        // Add an event for the next budget exhaustion of the server task.
        let next_be_event = Event
        {
            event_type: EventType::BudgetExhausted,
            event_time: self.release_time.add (remaining_budget),
            budget    : std::time::Duration::from_millis (0),
        };
        self.be_event_queue.push_back (next_be_event);

        // Add an event for the next release event of the server task.
        let next_release_event = Event
        {
            event_type: EventType::ReleaseEvent,
            event_time: self.release_time.add (server.period),
            budget    : std::cmp::min (consumed_budget, server.budget),
        };
        self.r_event_queue.push_back (next_release_event);

        // Here, we are not sure if any migrating request is
        // enqueued: hence, suspend the controller until further
        // notice.
        {
            let (is_running, cvar) = &*self.is_server_running;
            *is_running.lock ().unwrap () = false;
            cvar.notify_one ();
        }

        // Track the state of the server task.
        self.is_executing = false;

        // Wait for some migrating request.
        {
            let (barrier, cvar) = &*self.barrier;
            let _r = cvar.wait_while (barrier.lock ().unwrap (),
                                      |&mut num_reqs| { num_reqs < 1 }).unwrap ();
        }

        // Check if the budget has expired.
        if !self.has_expired
        {
            // Get the release time.
            self.release_time = std::time::Instant::now ();

            // Update the budget.
            self.start_budget = remaining_budget;

            // Rise the priority of the server task.
            server.rise_priority ();
        }

        // Activate the controller task signalling that the
        // server task is active.
        {
            let (is_running, cvar) = &*self.is_server_running;
            *is_running.lock ().unwrap () = true;
            cvar.notify_one ();
        }

        self.is_executing = true;
    }

    pub fn release_sporadic (&mut self)
    {
        let (_barrier, cvar) = &*self.barrier;
        cvar.notify_all ();
    }

    fn timing_event_handler (&mut self, event: Event)
    {
        // Extract the server from its envelop.
        let server = self.server.unwrap ();

        if self.has_expired && self.is_executing
        {
            // Then rise the priority.
            server.rise_priority ();

            self.release_time = std::time::Instant::now ();
            self.start_budget = event.budget;
            self.has_expired  = false;

            // Remove previous budget exhausted events and create
            // a new one for the current budget.
            self.be_event_queue.clear ();
            let next_budget_expired_event = Event
            {
                event_type: EventType::BudgetExhausted,
                event_time: self.release_time.add (self.start_budget),
                budget    : std::time::Duration::ZERO,
            };
            self.be_event_queue.push_back (next_budget_expired_event);
        }
        else if !self.has_expired && self.is_executing
        {
            self.start_budget = self.start_budget.add (event.budget);

            // Update the existing budget exhausted event.
            let mut updated_be_event: Event = self.be_event_queue.pop_front (). unwrap ();
            updated_be_event.event_time     = updated_be_event.event_time.add (event.budget);
            updated_be_event.budget         = updated_be_event.budget.add (event.budget);
            self.be_event_queue.push_back (updated_be_event);
        }
    }

    fn budget_expired_handler (&mut self)
    {
        // Notice that the budget expired.
        self.has_expired = true;

        // Extract the server from its envelop.
        let server = self.server.unwrap ();

        // Add the next release event.
        let next_release_event = Event
        {
            event_type: EventType::ReleaseEvent,
            event_time: self.release_time.add (server.period),
            budget    : std::cmp::min (self.start_budget, server.budget),
        };
        self.r_event_queue.push_back (next_release_event);

        // Then lower the server task priority and update start_budget.
        server.lower_priority ();
        self.start_budget = std::time::Duration::ZERO;
    }

    pub fn start (controller       : std::sync::Arc<std::sync::Mutex<SporadicServerController>>,
                  is_server_running: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>)
    {
        loop
        {

            // Run only when the server task is active.
            // Doing so prevent the event loop from running
            // when there are no requests.
            {
                let (barrier, cvar) = &*is_server_running;
                let _r = cvar.wait_while (barrier.lock ().unwrap (),
                                          |&mut is_running| { !is_running }).unwrap ();
            }

            // If we are here, the server task is active.
            // Then gain access to the controller to run
            // the body of the event loop.
            let mut controller = controller.lock ().unwrap ();

            let event = controller.get_next_event ();

            if let Some(event) = event
            {
                // Sleep until the first event expiration.
                let clock = std::time::Instant::now ();
                std::thread::sleep (event.event_time.duration_since (clock));

                // Then process it.
                if event.event_type == EventType::ReleaseEvent
                {
                    controller.timing_event_handler (event);
                }
                else if event.event_type == EventType::BudgetExhausted
                {
                    controller.budget_expired_handler ();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn main ()
    {
        println!("START TEST SUITE");

        // The number of pending migration requests.
        let reqs_enqueued =
            std::sync::Arc::new ((std::sync::Mutex::new (10u8), std::sync::Condvar::new ()));

        // The state of the server task (is it running?).
        let is_server_running =
            std::sync::Arc::new ((std::sync::Mutex::new (false), std::sync::Condvar::new ()));

        let controller =
            std::sync::Arc::new (
                std::sync::Mutex::new (
                    SporadicServerController::new(reqs_enqueued.clone (),
                                                  is_server_running.clone ())
                )
            );

        let mut server = SporadicServer::new (
            std::time::Duration::from_millis (20),
            std::time::Duration::from_millis (100),
            80u32);

        let first_controller = controller.clone ();
        let first_is_server_running = is_server_running.clone ();
        let mut handles = vec![];
        let controller_handle = std::thread::spawn( move ||
            {
                // Initialization.
                unsafe
                    {

                        // Scheduling properties.
                        let tid = libc::gettid ();
                        let sched_param = libc::sched_param
                        {
                            sched_priority: 89 as libc::c_int,
                        };
                        libc::sched_setscheduler (tid, libc::SCHED_FIFO, &sched_param);

                        // Affinity.
                        let mut cpuset : libc::cpu_set_t = std::mem::zeroed ();
                        libc::CPU_ZERO (&mut cpuset);
                        libc::CPU_SET (8, &mut cpuset);
                        libc::sched_setaffinity (tid, size_of::<libc::cpu_set_t> (), &mut cpuset);
                    }

                SporadicServerController::start (first_controller, first_is_server_running)
            });
        handles.push (controller_handle);

        let server_handle = std::thread::spawn (move ||
            {
                // Initialization.
                unsafe
                    {

                        // Scheduling properties.
                        let tid = libc::gettid ();
                        let sched_param = libc::sched_param
                        {
                            sched_priority: server.priority as libc::c_int,
                        };
                        libc::sched_setscheduler (tid, libc::SCHED_FIFO, &sched_param);

                        // Affinity.
                        let mut cpuset : libc::cpu_set_t = std::mem::zeroed ();
                        libc::CPU_ZERO (&mut cpuset);
                        libc::CPU_SET (8, &mut cpuset);
                        libc::sched_setaffinity (tid, size_of::<libc::cpu_set_t> (), &mut cpuset);
                    }
                server.start (controller.clone ())
            });
        handles.push (server_handle);

        for handle in handles
        {
            handle.join ().unwrap ();
        }

    }
}