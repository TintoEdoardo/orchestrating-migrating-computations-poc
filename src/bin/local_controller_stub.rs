use std::os::fd::AsRawFd;

#[path = "../config.rs"]
mod config;

fn main() {

    /****************
       SET AFFINITY
     ****************/
    println!("SET AFFINITY");
    // First, fix this computation to a specific CPU (now 2).
    let cores: Vec<usize> = vec![4];
    affinity::set_thread_affinity(&cores).unwrap();

    /*******************************
       INTER-PROCESS COMMUNICATION
     *******************************/
    println!("CONFIGURE IPC");
    // IPC happens through shared memory.
    let fd =
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(config::SEMAPHORE)
            .expect("Unable to open file. ");

    unsafe { libc::ftruncate(fd.as_raw_fd(), config::SHARED_MEMORY_SIZE as libc::off_t); }

    let shared_data_ptr : *mut config::IPC_SHARED_MEMORY = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            core::mem::size_of::<config::IPC_SHARED_MEMORY>(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_FILE | libc::MAP_SHARED,
            fd.as_raw_fd(),
            0) as *mut config::IPC_SHARED_MEMORY
    };

    /* unsafe {
        if libc::sem_init(&mut (*shared_data_ptr).semaphore, 1, 0) == -1
        { panic!("sem_init failed"); }
    }; */

    let shared_data    : &mut config::IPC_SHARED_MEMORY = unsafe {&mut *shared_data_ptr};
    let semaphore      : &mut libc::sem_t = unsafe {&mut (*shared_data).semaphore};
    let should_migrate : &mut i32 = unsafe {&mut (*shared_data).should_migrate};

    let mut next_activation = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut next_activation); }
    next_activation.tv_sec += 10;
    println!("START LOOP");
    // Body of the task.
    loop {

        /****************
             WORKLOAD
         ****************/

        unsafe { libc::clock_nanosleep(libc::CLOCK_MONOTONIC, libc::TIMER_ABSTIME, &mut next_activation, core::ptr::null_mut()); }
        next_activation.tv_sec += 10;
        unsafe {
            libc::sem_post(semaphore);
        }
        unsafe {
            libc::sem_wait(semaphore);
        }
        *should_migrate = 1;
        unsafe {
            libc::sem_post(semaphore);
        }

    }

}