/**********************************/
/*      UTILITIES FOR LINUX       */
/**********************************/

pub fn set_priority (priority : i32, affinity: usize)
{
    unsafe
        {

            // Scheduling properties.
            let tid = libc::gettid ();
            let sched_param = libc::sched_param
            {
                sched_priority: priority,
            };
            libc::sched_setscheduler (tid, libc::SCHED_FIFO, &sched_param);

            // Affinity.
            let mut cpuset : libc::cpu_set_t = std::mem::zeroed ();
            libc::CPU_ZERO (&mut cpuset);
            libc::CPU_SET (affinity, &mut cpuset);
            libc::sched_setaffinity (tid, size_of::<libc::cpu_set_t> (), &mut cpuset);
        }
}