//! Pure-Rust Dormand-Prince RK5(4) backend with adaptive step size control.
//!
//! No external dependencies, no unsafe code. This backend is always
//! compiled, including with `--no-default-features`, so it is the one that
//! ships in WASM builds where SUNDIALS (a C library) cannot go.
//!
//! Dense output is provided by a cubic Hermite interpolant built from the
//! state and derivative at both ends of each accepted step. Dormand-Prince
//! is a "first same as last" (FSAL) method: the derivative at the end of a
//! step is already computed as part of the step (it becomes the first stage
//! of the next step), so both endpoint derivatives are available for free.
//! A cubic Hermite fit through two points and two derivatives is exact for
//! cubics and has pointwise interpolation error O(h^4), matching the local
//! accuracy of the propagated solution, so it costs nothing extra and does
//! not degrade the method's accuracy. This is the same tradeoff the DP54
//! "free" interpolant makes; a full degree-4 continuous extension needs
//! extra coefficients (as in Shampine's dense DP formulas) for a further
//! constant-factor accuracy gain that is not needed here.

use crate::error::SolveError;
use crate::problem::OdeProblem;
use crate::solution::OdeSolution;
use crate::solver::{output_grid, OdeSolver};

/// Dormand-Prince RK5(4) with adaptive step size control.
#[derive(Debug, Clone, Copy)]
pub struct DormandPrince {
    /// Shrinks or grows the next step relative to what the error estimate
    /// alone would suggest, to keep steps accepted more often than rejected.
    pub safety: f64,
    /// Smallest factor by which a step size may shrink in one adjustment.
    pub min_factor: f64,
    /// Largest factor by which a step size may grow in one adjustment.
    pub max_factor: f64,
    /// Step budget before giving up with `SolveError::TooManySteps`.
    pub max_steps: usize,
    /// Steps are not allowed to shrink below this absolute size.
    pub min_step: f64,
}

impl Default for DormandPrince {
    fn default() -> Self {
        Self {
            safety: 0.9,
            min_factor: 0.2,
            max_factor: 5.0,
            max_steps: 200_000,
            min_step: 1e-13,
        }
    }
}

// Dormand-Prince RK5(4) Butcher tableau (Dormand & Prince, 1980). This is
// the same tableau used by MATLAB's ode45 and scipy's RK45.
const C2: f64 = 1.0 / 5.0;
const C3: f64 = 3.0 / 10.0;
const C4: f64 = 4.0 / 5.0;
const C5: f64 = 8.0 / 9.0;
// C6 = 1.0, C7 = 1.0 (not needed as named constants, both stages land at t + h)

const A21: f64 = 1.0 / 5.0;

const A31: f64 = 3.0 / 40.0;
const A32: f64 = 9.0 / 40.0;

const A41: f64 = 44.0 / 45.0;
const A42: f64 = -56.0 / 15.0;
const A43: f64 = 32.0 / 9.0;

const A51: f64 = 19372.0 / 6561.0;
const A52: f64 = -25360.0 / 2187.0;
const A53: f64 = 64448.0 / 6561.0;
const A54: f64 = -212.0 / 729.0;

const A61: f64 = 9017.0 / 3168.0;
const A62: f64 = -355.0 / 33.0;
const A63: f64 = 46732.0 / 5247.0;
const A64: f64 = 49.0 / 176.0;
const A65: f64 = -5103.0 / 18656.0;

// Stage 7 uses the 5th order propagation weights (the FSAL property):
// A71..A76 are exactly B1..B6 below, so the value used to compute stage 7
// is exactly the accepted 5th order solution.
const B1: f64 = 35.0 / 384.0;
const B2: f64 = 0.0;
const B3: f64 = 500.0 / 1113.0;
const B4: f64 = 125.0 / 192.0;
const B5: f64 = -2187.0 / 6784.0;
const B6: f64 = 11.0 / 84.0;
const B7: f64 = 0.0;

// 4th order weights, used only to form the error estimate B - BSTAR.
const BSTAR1: f64 = 5179.0 / 57600.0;
const BSTAR2: f64 = 0.0;
const BSTAR3: f64 = 7571.0 / 16695.0;
const BSTAR4: f64 = 393.0 / 640.0;
const BSTAR5: f64 = -92097.0 / 339200.0;
const BSTAR6: f64 = 187.0 / 2100.0;
const BSTAR7: f64 = 1.0 / 40.0;

const E1: f64 = B1 - BSTAR1;
const E2: f64 = B2 - BSTAR2;
const E3: f64 = B3 - BSTAR3;
const E4: f64 = B4 - BSTAR4;
const E5: f64 = B5 - BSTAR5;
const E6: f64 = B6 - BSTAR6;
const E7: f64 = B7 - BSTAR7;

// Error estimator order: the embedded solution is 4th order, so the local
// error scales like h^5 and the standard step controller exponent is
// 1 / (order + 1) = 1/5.
const ERROR_ORDER: f64 = 4.0;

fn rms_norm(values: impl Iterator<Item = f64>, count: usize) -> f64 {
    if count == 0 {
        return 0.0;
    }
    let sum_sq: f64 = values.map(|v| v * v).sum();
    (sum_sq / count as f64).sqrt()
}

/// Hairer/Norsett/Wanner style initial step size guess (the same algorithm
/// scipy's RK45 uses): take one Euler-ish trial step, compare how fast the
/// derivative changes to how fast the state changes, and pick a step that
/// keeps the first real step's error near the tolerance.
fn initial_step_size(
    rhs: &(dyn Fn(f64, &[f64], &mut [f64]) + Send),
    t0: f64,
    y0: &[f64],
    f0: &[f64],
    rtol: f64,
    atol: f64,
) -> f64 {
    let n = y0.len();
    let scale: Vec<f64> = y0.iter().map(|y| atol + rtol * y.abs()).collect();

    let d0 = rms_norm(y0.iter().zip(&scale).map(|(y, s)| y / s), n);
    let d1 = rms_norm(f0.iter().zip(&scale).map(|(f, s)| f / s), n);

    let h0 = if d0 < 1e-5 || d1 < 1e-5 {
        1e-6
    } else {
        0.01 * d0 / d1
    };

    let y1: Vec<f64> = y0.iter().zip(f0).map(|(y, f)| y + h0 * f).collect();
    let mut f1 = vec![0.0; n];
    rhs(t0 + h0, &y1, &mut f1);

    let d2 = rms_norm(
        f1.iter().zip(f0).zip(&scale).map(|((a, b), s)| (a - b) / s),
        n,
    ) / h0;

    let h1 = if d1.max(d2) <= 1e-15 {
        (h0 * 1e-3).max(1e-6)
    } else {
        (0.01 / d1.max(d2)).powf(1.0 / (ERROR_ORDER + 1.0))
    };

    (100.0 * h0).min(h1)
}

/// Cubic Hermite interpolation using the state and derivative at both ends
/// of the step. See the module doc comment for why this is a sound choice
/// of dense output formula for DP54.
fn hermite_interpolate(
    t0: f64,
    y0: &[f64],
    f0: &[f64],
    t1: f64,
    y1: &[f64],
    f1: &[f64],
    tau: f64,
    out: &mut [f64],
) {
    let h = t1 - t0;
    let s = (tau - t0) / h;
    let s2 = s * s;
    let s3 = s2 * s;
    let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
    let h10 = s3 - 2.0 * s2 + s;
    let h01 = -2.0 * s3 + 3.0 * s2;
    let h11 = s3 - s2;
    for i in 0..out.len() {
        out[i] = h00 * y0[i] + h10 * h * f0[i] + h01 * y1[i] + h11 * h * f1[i];
    }
}

impl OdeSolver for DormandPrince {
    fn solve(
        &self,
        problem: &OdeProblem,
        n_output_points: usize,
    ) -> Result<OdeSolution, SolveError> {
        let (t0, t1) = problem.t_span;
        if !(t1 > t0) {
            return Err(SolveError::InvalidProblem(
                "t_span end must be greater than t_span start".to_string(),
            ));
        }
        let n = problem.y0.len();
        if n == 0 {
            return Err(SolveError::InvalidProblem(
                "initial state must have at least one component".to_string(),
            ));
        }

        let rhs = problem.rhs.as_ref();
        let output_times = output_grid(t0, t1, n_output_points);

        let mut out_t = Vec::with_capacity(output_times.len());
        let mut out_y = Vec::with_capacity(output_times.len());
        let mut next_out = 0usize;

        // Emit anything at or before t0 up front (only relevant if n == 1
        // or the first requested point coincides with t0).
        while next_out < output_times.len() && output_times[next_out] <= t0 {
            out_t.push(t0);
            out_y.push(problem.y0.clone());
            next_out += 1;
        }

        let mut t_cur = t0;
        let mut y_cur = problem.y0.clone();
        let mut f_cur = vec![0.0; n];
        rhs(t_cur, &y_cur, &mut f_cur);

        let mut h = initial_step_size(rhs, t0, &y_cur, &f_cur, problem.rtol, problem.atol);
        if h <= 0.0 || !h.is_finite() {
            h = (t1 - t0) * 1e-3;
        }

        let mut k2 = vec![0.0; n];
        let mut k3 = vec![0.0; n];
        let mut k4 = vec![0.0; n];
        let mut k5 = vec![0.0; n];
        let mut k6 = vec![0.0; n];
        let mut k7 = vec![0.0; n];
        let mut stage_y = vec![0.0; n];
        let mut y_new = vec![0.0; n];

        let mut steps = 0usize;

        while t_cur < t1 && steps < self.max_steps {
            if t_cur + h > t1 {
                h = t1 - t_cur;
            }

            // Stage 1 is f_cur (FSAL: it is the stage-7 derivative from the
            // previous accepted step, or the initial derivative).
            for i in 0..n {
                stage_y[i] = y_cur[i] + h * A21 * f_cur[i];
            }
            rhs(t_cur + C2 * h, &stage_y, &mut k2);

            for i in 0..n {
                stage_y[i] = y_cur[i] + h * (A31 * f_cur[i] + A32 * k2[i]);
            }
            rhs(t_cur + C3 * h, &stage_y, &mut k3);

            for i in 0..n {
                stage_y[i] = y_cur[i] + h * (A41 * f_cur[i] + A42 * k2[i] + A43 * k3[i]);
            }
            rhs(t_cur + C4 * h, &stage_y, &mut k4);

            for i in 0..n {
                stage_y[i] = y_cur[i]
                    + h * (A51 * f_cur[i] + A52 * k2[i] + A53 * k3[i] + A54 * k4[i]);
            }
            rhs(t_cur + C5 * h, &stage_y, &mut k5);

            for i in 0..n {
                stage_y[i] = y_cur[i]
                    + h * (A61 * f_cur[i] + A62 * k2[i] + A63 * k3[i] + A64 * k4[i]
                        + A65 * k5[i]);
            }
            rhs(t_cur + h, &stage_y, &mut k6);

            for i in 0..n {
                y_new[i] = y_cur[i]
                    + h * (B1 * f_cur[i] + B3 * k3[i] + B4 * k4[i] + B5 * k5[i] + B6 * k6[i]);
            }
            rhs(t_cur + h, &y_new, &mut k7);

            let scale = |i: usize| {
                problem.atol + problem.rtol * y_cur[i].abs().max(y_new[i].abs())
            };
            let err_norm = rms_norm(
                (0..n).map(|i| {
                    let err_i = h
                        * (E1 * f_cur[i]
                            + E2 * k2[i]
                            + E3 * k3[i]
                            + E4 * k4[i]
                            + E5 * k5[i]
                            + E6 * k6[i]
                            + E7 * k7[i]);
                    err_i / scale(i)
                }),
                n,
            );

            if err_norm <= 1.0 {
                let t_next = t_cur + h;

                while next_out < output_times.len() && output_times[next_out] <= t_next {
                    let tau = output_times[next_out];
                    let mut y_tau = vec![0.0; n];
                    hermite_interpolate(t_cur, &y_cur, &f_cur, t_next, &y_new, &k7, tau, &mut y_tau);
                    out_t.push(tau);
                    out_y.push(y_tau);
                    next_out += 1;
                }

                t_cur = t_next;
                y_cur.copy_from_slice(&y_new);
                f_cur.copy_from_slice(&k7);
                steps += 1;

                let factor = if err_norm == 0.0 {
                    self.max_factor
                } else {
                    (self.safety * err_norm.powf(-1.0 / (ERROR_ORDER + 1.0)))
                        .clamp(self.min_factor, self.max_factor)
                };
                h *= factor;
            } else {
                let factor = (self.safety * err_norm.powf(-1.0 / (ERROR_ORDER + 1.0)))
                    .clamp(self.min_factor, 1.0);
                h *= factor;

                if h < self.min_step {
                    return Err(SolveError::StepSizeUnderflow { t: t_cur, h });
                }
            }
        }

        if t_cur < t1 {
            return Err(SolveError::TooManySteps { steps });
        }

        // Floating point safety net: make sure every requested point got
        // emitted even if t_cur landed a few ULPs short of t1.
        while next_out < output_times.len() {
            out_t.push(t_cur);
            out_y.push(y_cur.clone());
            next_out += 1;
        }

        Ok(OdeSolution { t: out_t, y: out_y })
    }
}
