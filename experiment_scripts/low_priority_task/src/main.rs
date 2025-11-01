fn main ()
{
    // Fix the affinity of this task.
    let pid : libc::pid_t = unsafe { libc::getpid () };
    let mut cpu_set : libc::cpu_set_t = unsafe { core::mem::zeroed () };
    unsafe { libc::CPU_SET(2, &mut cpu_set); }
    unsafe { libc::sched_setaffinity(pid, core::mem::size_of::<libc::cpu_set_t> (),&mut cpu_set); }

    // Fix the priority.

    // Schedule as SCHED_OTHER.
    let sched_param = libc::sched_param
    {
        sched_priority: 0 as libc::c_int,
    };
    unsafe
        {
            libc::sched_setscheduler(pid, libc::SCHED_OTHER, &sched_param);
            libc::nice (10 as libc::c_int);
        }
    /*
    let sched_param = libc::sched_param
    {
        sched_priority: 1 as libc::c_int,
    };
    unsafe { libc::sched_setscheduler (pid, libc::SCHED_FIFO, &sched_param); }*/

    // Then loop forever.
    loop
    {
        assert! (true);
    }
}
