/***************************************/
/*     W O R K L O A D   T R A I T     */
/***************************************/

/// This trait model the workload of a sporadic server.
/// The parameter T refers to the application state.
pub trait Workload
{
    /// The workload that the server should execute.
    fn exec_workload (&mut self);
}