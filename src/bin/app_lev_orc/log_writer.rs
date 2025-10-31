/*********************/
/*     LOG WRITER    */
/*********************/
use std::io::Write;

pub fn save_admm_data (is_centralized    : bool,
                       convergence_micros: u64,
                       iterations_num    : usize)
{
    // Open the log files.
    let mut convergence : std::fs::File = if is_centralized
    {
        std::fs::OpenOptions::new ()
            .append (true)
            .create (true)
            .open ("../experiment_data/convergence_c.txt")
            .expect ("Failed to open ../experiment_data/convergence_c.txt")
    }
    else
    {
        std::fs::OpenOptions::new ()
            .append (true)
            .create (true)
            .open ("../experiment_data/convergence_d.txt")
            .expect ("Failed to open ../experiment_data/convergence_d.txt")
    };
    let mut iterations : std::fs::File = if is_centralized
    {
        std::fs::OpenOptions::new ()
            .append (true)
            .create (true)
            .open ("../experiment_data/iterations_c.txt")
            .expect ("Failed to open ../experiment_data/iterations_c.txt")
    }
    else
    {
        std::fs::OpenOptions::new ()
            .append (true)
            .create (true)
            .open ("../experiment_data/iterations_d.txt")
            .expect ("Failed to open ../experiment_data/iterations_d.txt")
    };

    // Then add the metrics acquired.
    convergence.write (convergence_micros.to_string ().as_bytes ())
        .expect("Unable to write to convergence file. ");
    convergence.write (b"\n").expect ("Failed to add newline. ");
    iterations.write (iterations_num.to_string ().as_bytes ())
        .expect("Unable to write to iterations file. ");
    iterations.write (b"\n").expect ("Failed to add newline. ");
}

pub fn save_send_time (send_micros: u64)
{
    let mut send : std::fs::File = std::fs::OpenOptions::new ()
        .append (true)
        .create (true)
        .open ("../experiment_data/send.txt")
        .expect ("Failed to open ../experiment_data/send.txt");

    send.write (send_micros.to_string ().as_bytes ())
        .expect ("Failed to write to send.txt");
    send.write (b"\n").expect ("Failed to add newline. ");
}

pub fn save_receive_time (receive_micros: u64)
{
    let mut receive : std::fs::File = std::fs::OpenOptions::new ()
        .append (true)
        .create (true)
        .open ("../experiment_data/receive.txt")
        .expect ("Failed to open ../experiment_data/receive.txt");

    receive.write (receive_micros.to_string ().as_bytes ())
        .expect ("Failed to write to receive.txt");
    receive.write (b"\n").expect ("Failed to add newline. ");
}

pub fn save_ss_time (receive_micros: u64)
{
    let mut server : std::fs::File = std::fs::OpenOptions::new ()
        .append (true)
        .create (true)
        .open ("../experiment_data/migration.txt")
        .expect ("Failed to open ../experiment_data/migration.txt");

    server.write (receive_micros.to_string ().as_bytes ())
        .expect ("Failed to write to receive.txt");
    server.write (b"\n").expect ("Failed to add newline. ");
}