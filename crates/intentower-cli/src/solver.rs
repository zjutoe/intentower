//! Solver adapters. Everything here reports what the solver itself said;
//! deciding proof status from a verdict is `verify`'s job (T20).

pub mod cvc5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Sat,
    Unsat,
    Unknown,
    /// The solver printed `unsupported` (an option or feature it lacks).
    Unsupported,
    /// The external timeout fired; the solver was killed.
    Timeout,
    /// The solver could not be run, exited non-zero, or said nothing usable.
    ToolError(String),
}
