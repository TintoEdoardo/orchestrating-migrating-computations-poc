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

    println!("Convergence micros: {}", convergence_micros);
    println!("Iterations num: {}", iterations_num);

    // Then add the metrics acquired.
    convergence.write (convergence_micros.to_string ().as_bytes ())
        .expect("Unable to write in convergence file. ");
    convergence.write (b"\n").expect ("Filed to add newline. ");
    iterations.write (iterations_num.to_string ().as_bytes ())
        .expect("Unable to write in iterations file. ");
    convergence.write (b"\n").expect ("Filed to add newline. ");
    /* writeln!(convergence, "{}", convergence_micros)
        .expect("Unable to write in convergence file. ");
    writeln!(iterations, "{}", iterations_num)
        .expect("Unable to write in iterations file. "); */
}