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

/// Return the difference between now and start_time, in microseconds.
pub fn get_completion_time (start_time: libc::timespec) -> u64
{
    let mut end_time   = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe
        {
            libc::clock_gettime (libc::CLOCK_MONOTONIC, &mut end_time);
            let time_to_completion : u64;
            let mut diff_sec = end_time.tv_sec - start_time.tv_sec;
            let mut diff_nsec = end_time.tv_nsec - start_time.tv_nsec;
            if diff_nsec < 0
            {
                diff_nsec += 1_000_000_000;
                diff_sec -= 1;
            }
            time_to_completion = (diff_sec * 1_000_000) as u64 + (diff_nsec / 1_000) as u64;
            time_to_completion
        }
}