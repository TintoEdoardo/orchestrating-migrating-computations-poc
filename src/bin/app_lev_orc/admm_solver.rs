/*****************************/
/*        ADMM SOLVER        */
/*****************************/

use crate::state::Coord;

/// Tolerance value used to determine when to stop
/// the ADMM executions.
pub static TOLERANCE : f32 = 0.05;


/// The variables use in the ADMM execution.
pub struct Variables
{
    /// x[i], with 'i' a node index.
    pub x : Vec<f32>
}

impl Variables
{
    pub fn new (vec: Vec<f32>) -> Self
    {
        Self { x: vec }
    } 
}

/// The global variables use in the ADMM execution.
pub struct Globals
{
    /// z[i], with 'i' a node index.
    z : Vec<f32>
}

impl Globals
{
    pub fn new (vec: Vec<f32>) -> Self
    {
        Self { z: vec }
    } 
}


/* pub struct DualsVariables {
    // u[i], with 'i' a node index.
    pub u: Vec<f32>
}

impl DualsVariables {
    pub fn new(vec : Vec<f32>) -> Self {
        Self { u: vec }
    } 
} */


/// Solver of the local ADMM problem for
/// this experimentation.
pub struct LocalSolver
{
    /// x_i in the model.
    pub local      : f32,

    /// u_i in the model.
    pub dual       : f32,

    /// z_i in the model.
    pub global     : f32,

    /// rho in the model.
    pub penalty    : f32,

    /// Coordinates of the node, used to determine
    /// the distance from the desired position of
    /// a request.
    pub coordinate : Coord,
}

impl LocalSolver
{
    pub fn new (number_of_nodes : usize, penalty : f32, coordinate : Coord) -> Self
    {
        Self
        {
            local  : 0.0,
            dual   : 0.0,
            global : 1.0 / number_of_nodes as f32,
            penalty,
            coordinate }
    }

    pub fn clear (&mut self, number_of_nodes : usize, penalty : f32, coordinate : Coord)
    {
        self.local      = 0.0;
        self.dual       = 0.0;
        self.global     = 1.0 / number_of_nodes as f32;
        self.penalty    = penalty;
        self.coordinate = coordinate;
    }

    /// Local x-update, performs the local update of the
    /// x variable on the current node. 
    pub fn local_x_update (&mut self, desired_coord: Coord)
    {
        // First check if this node might host the incoming request.
        // Meaning, check whether the constraints are met with local x = 1.
        // TODO: future plan.

        // The object function of the minimization problem.
        fn to_minimize (local    :   f32,
                        dual     :   f32,
                        global   :   f32,
                        distance :   f32,
                        penalty  :   f32) -> f32
        {
            distance * local + (penalty / 2f32) * (local - global + dual).powf (2f32)
        }

        let distance: f32 =
            ((desired_coord.get_x () - self.coordinate.get_x ()).powi (2) + (desired_coord.get_y () - self.coordinate.get_y ()).powi (2)).sqrt ();

        let local_at_0 =
            to_minimize (0f32, self.dual, self.global, distance, self.penalty);
        let local_at_1 =
            to_minimize (1f32, self.dual, self.global, distance, self.penalty);

        self.local =
            if f32::min (local_at_0, local_at_1) == local_at_0
            {
                0f32
            }
            else
            {
                1f32
            };
    }

    pub fn get_local (&self) -> f32
    {
        self.local
    }

    pub fn get_dual (&self) -> f32
    {
        self.dual
    }

    pub fn set_global (&mut self, global: f32)
    {
        self.global = global
    }

    /// Dual variables update (u-updates).
    pub fn local_dual_update (&mut self)
    {
        self.dual = self.dual + (self.local - self.global);
    }
}


/// Solver of the global problem in the ADMM execution,
/// it performs the global update step.
pub struct GlobalSolver
{
    /// x = { x_i } in the model.
    // pub variables : Variables,

    /// z = { z_i } in the model.
    globals   : Globals,

    // u = { u_i } in the model.
    // duals     : DualsVariables,

    /// Cumulative x_+_u = {x_i + u_i}.
    locals    : Variables,

    /// Received local data.
    received_locals : std::collections::HashSet<usize>,

    /// Number of nodes. 
    number_of_nodes : usize,

    /// Maximum number of iterations. 
    iteration_limit : usize,

    /// Current iteration.
    iteration       : usize,
}

impl GlobalSolver
{
    pub fn new (number_of_nodes : usize, iteration_limit : usize) -> GlobalSolver
    {
        Self
        {
            globals         : Globals::new (vec![1.0 / number_of_nodes as f32; number_of_nodes]),
            locals          : Variables::new (vec![0.0; number_of_nodes]),
            received_locals : std::collections::HashSet::new (),
            number_of_nodes,
            iteration_limit,
            iteration       : 0
        }
    }

    pub fn clear (&mut self)
    {
        self.globals.z = vec![1.0 / self.number_of_nodes as f32; self.number_of_nodes];
        self.locals.x  = vec![0.0; self.number_of_nodes];
        self.iteration = 0;
    }

    pub fn clear_locals (&mut self)
    {
        self.locals.x = vec![0.0; self.number_of_nodes];
    }

    pub fn add_local_sum (&mut self, sum: f32, src: usize)
    {
        self.locals.x.insert (src,sum);
        self.received_locals.insert (src);
    }

    pub fn locals_len (&self) -> usize
    {
        self.received_locals.len ()
    }

    fn clear_received_locals (&mut self)
    {
        self.received_locals.clear ();
    }

    pub fn get_global_from_index (&self, node_index: usize) -> f32
    {
        self.globals.z[node_index]
    }

    /// Update the global variable z.
    pub fn global_z_updater (&mut self)
    {
        // Compute the vector v.
        let mut v : Vec<f32> = Vec::new ();
        for i in 0..self.locals.x.len ()
        {
            v.push (self.locals.x[i]);
        }

        // Produce the subtrahend in the z-update.
        let subt = (1f32 / v.len () as f32) * (v.iter ().sum::<f32> () - 1f32);

        // Update the global variables.
        for i in 0..self.globals.z.len ()
        {
            self.globals.z[i] = v[i] - subt;
        }

        #[cfg(feature = "print_log")]
        println!("requests_coordination_loop - global_z_updater = [{:?}]", self.globals.z);

        // Update the iteration index.
        self.iteration += 1;

        // Then clean self.
        self.clear_received_locals ();
        self.clear_locals ();
    }

    /// Perform the termination checks to determine if the
    /// ADMM execution has reached an end.
    pub fn terminated (&self) -> bool
    {
        let result: bool;
        if self.iteration > self.iteration_limit
        {
            result = true;
        }
        else
        {
            let mut has_converged = true;
            let sum : f32 = self.globals.z.iter ().sum ();
            if (sum - 1.0).abs () > TOLERANCE
            {
                has_converged = false;
            }
            result = has_converged;
        }
        result
    }

    /// This function returns the index of the node with
    /// the higher z value, namely, the most suitable candidate
    /// to host the migrating request.
    pub fn get_max_global_index (&self) -> usize
    {
        let mut max_z = 0.0;
        let mut result = 0;
        for i in 0..self.globals.z.len ()
        {
            let &z = &self.globals.z[i];
            if z > max_z
            {
                max_z  = z;
                result = i;
            }
        }
        result
    }
}