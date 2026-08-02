//! OpenMat ODE solving: `OdeSolver` trait, pure-Rust Dormand-Prince RK5(4)
//! backend (always available, WASM-safe), and a SUNDIALS CVODE backend
//! behind the `sundials` cargo feature (the default desktop engine).
//!
//! Build a problem with `OdeProblem::new`, then either pick a backend
//! directly (`DormandPrince::default()`, or `CvodeSolver::default()` when
//! the `sundials` feature is enabled) or call `solve_default`, which picks
//! whichever backend is compiled in and best suited to the current build.

#[cfg(feature = "sundials")]
mod cvode;
mod error;
mod problem;
mod rk45;
mod solution;
mod solver;

pub use error::SolveError;
pub use problem::{OdeProblem, DEFAULT_ATOL, DEFAULT_RTOL};
pub use rk45::DormandPrince;
pub use solution::OdeSolution;
pub use solver::OdeSolver;

#[cfg(feature = "sundials")]
pub use cvode::CvodeSolver;

/// Solve `problem`, sampled at `n_output_points` points evenly spaced across
/// `problem.t_span`, using the best backend available in this build: CVODE
/// (SUNDIALS, BDF) when the `sundials` feature is enabled, otherwise the
/// pure-Rust Dormand-Prince backend.
pub fn solve_default(
    problem: &OdeProblem,
    n_output_points: usize,
) -> Result<OdeSolution, SolveError> {
    #[cfg(feature = "sundials")]
    {
        CvodeSolver::default().solve(problem, n_output_points)
    }
    #[cfg(not(feature = "sundials"))]
    {
        DormandPrince::default().solve(problem, n_output_points)
    }
}
