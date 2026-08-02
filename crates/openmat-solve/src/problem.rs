//! The public description of an initial value problem: right hand side,
//! initial state, time span, and tolerances.

/// Default relative tolerance, matches common ODE solver defaults.
pub const DEFAULT_RTOL: f64 = 1e-6;
/// Default absolute tolerance.
pub const DEFAULT_ATOL: f64 = 1e-9;

/// An initial value problem `y' = f(t, y)`, `y(t0) = y0`, to be solved over
/// `t_span`.
///
/// `rhs` writes the derivative of `y` at `(t, y)` into its third argument.
/// It is boxed and `Send` so a problem can be built on one thread and handed
/// to a solver running on another, but it is not required to be `Sync`: a
/// solve only ever calls it from a single thread at a time.
pub struct OdeProblem {
    pub rhs: Box<dyn Fn(f64, &[f64], &mut [f64]) + Send>,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    pub rtol: f64,
    pub atol: f64,
}

impl OdeProblem {
    /// Build a problem with the default tolerances (rtol = 1e-6, atol = 1e-9).
    pub fn new(
        rhs: impl Fn(f64, &[f64], &mut [f64]) + Send + 'static,
        y0: Vec<f64>,
        t_span: (f64, f64),
    ) -> Self {
        Self {
            rhs: Box::new(rhs),
            y0,
            t_span,
            rtol: DEFAULT_RTOL,
            atol: DEFAULT_ATOL,
        }
    }

    /// Builder style setter for the relative tolerance.
    pub fn with_rtol(mut self, rtol: f64) -> Self {
        self.rtol = rtol;
        self
    }

    /// Builder style setter for the absolute tolerance.
    pub fn with_atol(mut self, atol: f64) -> Self {
        self.atol = atol;
        self
    }
}
