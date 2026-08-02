//! The output of a solve: a sampled trajectory over evenly spaced time points.

/// The result of solving an `OdeProblem`.
///
/// `t` holds `n_output_points` evenly spaced sample times across `t_span`
/// (inclusive of both endpoints), and `y[i]` is the state vector at `t[i]`.
/// Every sample is produced by dense output, evaluated after the fact from
/// the accepted integration steps, not by forcing the integrator to step
/// exactly on these times.
#[derive(Debug, Clone, PartialEq)]
pub struct OdeSolution {
    pub t: Vec<f64>,
    pub y: Vec<Vec<f64>>,
}

impl OdeSolution {
    /// Number of sample points in the solution.
    pub fn len(&self) -> usize {
        self.t.len()
    }

    /// True if the solution has no sample points.
    pub fn is_empty(&self) -> bool {
        self.t.is_empty()
    }

    /// Extract a single state component across every sample time, e.g. the
    /// first coordinate of a trajectory for plotting.
    pub fn component(&self, index: usize) -> Vec<f64> {
        self.y.iter().map(|row| row[index]).collect()
    }
}
