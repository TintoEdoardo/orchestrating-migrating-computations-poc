/***************************************/
/*          U T I L I T I E S          */
/***************************************/

#[allow(dead_code)]
pub(crate) fn get_platform_tid () -> u32
{
    unsafe
        {
            libc::gettid () as u32
        }
}

pub(crate) fn set_priority (thread: u32, priority: u32)
{
    // Set the priority for the thread TID.
    unsafe
        {
            let tid : libc::pid_t = thread as libc::pid_t;
            let sched_param = libc::sched_param
            {
                sched_priority: priority as libc::c_int,
            };
            libc::sched_setscheduler (tid, libc::SCHED_FIFO, &sched_param);
        }
}

pub(crate) fn set_niceness (thread: u32, niceness: u32)
{
    unsafe
        {
            // Schedule as SCHED_OTHER.
            let sched_param = libc::sched_param
            {
                sched_priority: 0 as libc::c_int,
            };
            let tid : libc::pid_t = thread as libc::pid_t;
            libc::sched_setscheduler (tid, libc::SCHED_OTHER, &sched_param);
            libc::nice (niceness as libc::c_int);
        }
}