//! The `OdeSolver` trait every backend implements.

use crate::error::SolveError;
use crate::problem::OdeProblem;
use crate::solution::OdeSolution;

/// A backend that can integrate an `OdeProblem` and report the trajectory at
/// evenly spaced output times.
pub trait OdeSolver {
    /// Solve `problem` and sample the result at `n_output_points` points
    /// evenly spaced across `problem.t_span` (both endpoints included).
    fn solve(
        &self,
        problem: &OdeProblem,
        n_output_points: usize,
    ) -> Result<OdeSolution, SolveError>;
}

/// Build the evenly spaced output grid shared by every backend: `n` points
/// covering `[t0, t1]` inclusive. `n == 0` yields no points, `n == 1` yields
/// just `t0`. The last point is set to exactly `t1` rather than computed by
/// repeated addition, so callers never miss it to floating point drift.
pub(crate) fn output_grid(t0: f64, t1: f64, n: usize) -> Vec<f64> {
    match n {
        0 => Vec::new(),
        1 => vec![t0],
        _ => {
            let step = (t1 - t0) / (n as f64 - 1.0);
            (0..n)
                .map(|i| if i == n - 1 { t1 } else { t0 + step * i as f64 })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_endpoints() {
        let g = output_grid(0.0, 20.0, 5);
        assert_eq!(g.len(), 5);
        assert_eq!(g[0], 0.0);
        assert_eq!(g[4], 20.0);
    }

    #[test]
    fn grid_single_point() {
        assert_eq!(output_grid(0.0, 20.0, 1), vec![0.0]);
    }

    #[test]
    fn grid_empty() {
        assert!(output_grid(0.0, 20.0, 0).is_empty());
    }
}
