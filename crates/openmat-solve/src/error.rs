//! Error type shared by every solver backend.

use std::fmt;

/// Everything that can go wrong while solving an ODE.
#[derive(Debug, Clone)]
pub enum SolveError {
    /// The problem itself is not solvable as stated (bad t_span, empty state, etc).
    InvalidProblem(String),
    /// The adaptive step size dropped below the minimum allowed step without
    /// the local error estimate ever satisfying the tolerance. Usually means
    /// the problem is stiffer than the pure-Rust backend can handle, or the
    /// right hand side has a singularity near `t`.
    StepSizeUnderflow { t: f64, h: f64 },
    /// The solver used its full step budget before reaching the end of t_span.
    TooManySteps { steps: usize },
    /// A backend-specific failure (SUNDIALS return code, allocation failure, etc).
    Backend(String),
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolveError::InvalidProblem(msg) => write!(f, "invalid ODE problem: {msg}"),
            SolveError::StepSizeUnderflow { t, h } => write!(
                f,
                "step size underflow near t = {t}: step shrank to {h}, which is below the minimum allowed step; the problem may be stiff or have a singularity there"
            ),
            SolveError::TooManySteps { steps } => write!(
                f,
                "exceeded the maximum step budget ({steps} steps) before reaching the end of t_span"
            ),
            SolveError::Backend(msg) => write!(f, "solver backend error: {msg}"),
        }
    }
}

impl std::error::Error for SolveError {}
