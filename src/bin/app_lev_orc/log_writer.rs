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
    writeln!(convergence, "{}", convergence_micros)
        .expect("Unable to write in convergence file. ");
    writeln!(iterations, "{}", iterations_num)
        .expect("Unable to write in iterations file. ");
}