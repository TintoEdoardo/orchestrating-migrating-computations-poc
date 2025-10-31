use rand::{rng};
use rand::seq::SliceRandom;
use std::io::{BufWriter, Write};

fn main ()
{
    // Coordinates of nodes in 'F' and, at '|F| + 1' the optimal
    // coordinates of the migrating computation.
    fn distances(max_distance: u32, number_of_nodes: u32) -> Vec<(f32, f32)> {
        let mut result : Vec<(f32, f32)> = Vec::new();

        // We need to prevent overlapping coordinates, which would be
        // unrealistic. To do so, we generate two vector for X and Y,
        // with at least number_of_nodes + 1 elements each, from 0 to
        // max_distance.
        let mut X : Vec<f32> = Vec::new();
        let mut Y : Vec<f32> = Vec::new();

        let step : f32 = max_distance as f32 / (number_of_nodes + 1) as f32;
        let base : f32 = 0.0;
        for i in 0..(number_of_nodes + 1) {
            X.push(base + step * i as f32);
            Y.push(base + step * i as f32);
        }

        // Then we shuffle them.
        let mut rng = rng();
        X.shuffle(&mut rng);
        Y.shuffle(&mut rng);

        for i in 0..(number_of_nodes + 1) as usize {
            result.push((X[i], Y[i]));
        }
        result
    }

    let file = std::fs::OpenOptions::new ()
        .write (true)
        .create (true)
        .open ("distances.txt")
        .expect ("Unable to open distances.txt");
    let mut buf_writer = std::io::BufWriter::new (file);

    let number_of_samples = 1000;

    for _i in 0..number_of_samples
    {
        let node_1_state : String;
        let node_2_state : String;
        let node_3_state : String;
        let request      : String;

        // Compute the distances as in the simulation.
        let coords : Vec<(f32, f32)> = distances (100, 100);

        node_1_state = format! ("[({:.1},{:.1});1.0]", coords[0].0, coords[0].1);
        node_2_state = format! ("[({:.1},{:.1});0.5]", coords[1].0, coords[1].1);
        node_3_state = format! ("[({:.1},{:.1});0.5]", coords[2].0, coords[2].1);
        request = format! ("2#[0;25;128;({:.1},{:.1});5.0]", coords[3].0, coords[3].1);

        writeln! (buf_writer, "{} {} {} {}", node_1_state, node_2_state, node_3_state, request).unwrap ();
    }

    buf_writer.flush ().unwrap ();
}