//! y' = -k y, analytic solution y(t) = y0 exp(-k t).

use openmat_solve::{DormandPrince, OdeProblem, OdeSolver};

const K: f64 = 0.7;
const Y0: f64 = 3.0;

fn decay_problem() -> OdeProblem {
    OdeProblem::new(
        |_t, y, dy| {
            dy[0] = -K * y[0];
        },
        vec![Y0],
        (0.0, 10.0),
    )
    .with_rtol(1e-8)
    .with_atol(1e-12)
}

fn max_error_vs_analytic(t: &[f64], y: &[Vec<f64>]) -> f64 {
    t.iter()
        .zip(y)
        .map(|(&ti, yi)| (yi[0] - Y0 * (-K * ti).exp()).abs())
        .fold(0.0, f64::max)
}

#[test]
fn dormand_prince_matches_exponential_decay() {
    let problem = decay_problem();
    let sol = DormandPrince::default()
        .solve(&problem, 200)
        .expect("solve should succeed");
    let err = max_error_vs_analytic(&sol.t, &sol.y);
    assert!(err < 1e-6, "max error {err} exceeded 1e-6");
}

#[cfg(feature = "sundials")]
#[test]
fn cvode_matches_exponential_decay() {
    use openmat_solve::CvodeSolver;
    let problem = decay_problem();
    let sol = CvodeSolver::default()
        .solve(&problem, 200)
        .expect("solve should succeed");
    let err = max_error_vs_analytic(&sol.t, &sol.y);
    assert!(err < 1e-6, "max error {err} exceeded 1e-6");
}
