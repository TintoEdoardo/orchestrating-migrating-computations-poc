///                                 ///
///     REQUESTS MONITORING LOOP    ///
///                                 ///

use std::{mem::zeroed, sync::{Arc, Mutex}};
use libc::timespec;

use crate::state::{should_migrate, ApplicationState};

pub struct OperationControlSystems {
    // Period in us. Note that the activation step
    // will not work properly with a period >= 1 sec. 
    period           : i32,
    first_activation : timespec,
    priority         : i32,
    affinity         : usize,
}

impl OperationControlSystems {

    pub fn new(period : i32, first_activation : timespec, priority : i32, affinity : usize) -> Self {
        Self { period, first_activation, priority, affinity }
    }

    /// Workload of a cyclic component. It runs as a 
    /// periodic task. 
    pub fn requests_monitoring_loop(&mut self, application_state : Arc<Mutex<ApplicationState>>) {

        // Initialization. 
        unsafe {

            // Scheduling properties. 
            let tid = libc::gettid();
            let sched_param = libc::sched_param {
                sched_priority: self.priority,
            };
            libc::sched_setscheduler(tid, libc::SCHED_FIFO, &sched_param);

            // Affinity. 
            let mut cpuset : libc::cpu_set_t = zeroed();
            libc::CPU_ZERO(&mut cpuset);
            libc::CPU_SET(self.affinity, &mut cpuset);
        }

        // Activation. 
        let mut next_activation = self.first_activation;
        unsafe {
            libc::clock_nanosleep(libc::CLOCK_MONOTONIC, libc::TIMER_ABSTIME, &mut next_activation, core::ptr::null_mut());
        }

        // Workload. 
        loop {
            // Compute next activation. 
            // This step misbehaves if the period is >= 1_000_000_000. 
            next_activation.tv_nsec = next_activation.tv_nsec + (self.period * 1000) as i64;
            next_activation.tv_sec  = next_activation.tv_sec;
            if next_activation.tv_nsec > 1_000_000_000 {
                next_activation.tv_sec = next_activation.tv_sec + 1;
                next_activation.tv_nsec -= 1_000_000_000;
            }

            // Check if some requests need to migrate. 
            {
                // Here we access the applicaiton state as mutable. 
                let mut app_state = application_state.lock().unwrap();
                let node_state = app_state.node_state;
                let requests = &mut app_state.requests;

                for &mut mut request in requests {
                    let mut migrate = false;
                    if should_migrate(&request, &node_state) {
                        migrate = true;
                    }
                    if migrate {
                        request.set_should_migrate(true);
                    }
                }

                // Drop the mutex variable, forcing unlock. 
                drop(app_state);
            }

            // Sleep until next activation. 
            unsafe {
                libc::clock_nanosleep(libc::CLOCK_MONOTONIC, libc::TIMER_ABSTIME, &mut next_activation, core::ptr::null_mut());
            }

        }

    }
}