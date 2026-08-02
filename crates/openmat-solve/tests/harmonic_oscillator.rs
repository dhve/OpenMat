//! x'' = -x, analytic solution x(t) = cos(t) for x(0) = 1, x'(0) = 0.
//! Requirement: max error < 1e-5 over t in [0, 20].

use openmat_solve::{DormandPrince, OdeProblem, OdeSolver};

fn harmonic_problem() -> OdeProblem {
    OdeProblem::new(
        |_t, y, dy| {
            dy[0] = y[1];
            dy[1] = -y[0];
        },
        vec![1.0, 0.0],
        (0.0, 20.0),
    )
}

fn max_error_vs_cos(t: &[f64], y: &[Vec<f64>]) -> f64 {
    t.iter()
        .zip(y)
        .map(|(&ti, yi)| (yi[0] - ti.cos()).abs())
        .fold(0.0, f64::max)
}

#[test]
fn dormand_prince_matches_cosine() {
    let problem = harmonic_problem();
    let sol = DormandPrince::default()
        .solve(&problem, 400)
        .expect("solve should succeed");
    let err = max_error_vs_cos(&sol.t, &sol.y);
    assert!(err < 1e-5, "max error {err} exceeded 1e-5");
}

#[cfg(feature = "sundials")]
#[test]
fn cvode_matches_cosine() {
    use openmat_solve::CvodeSolver;
    // BDF is a multistep method built for stiff problems; on a smooth
    // non-stiff oscillator it needs a tighter tolerance than the default to
    // reach the same accuracy DP54 gets "for free". Tighten explicitly
    // rather than loosen the error bound.
    let problem = harmonic_problem().with_rtol(1e-8).with_atol(1e-10);
    let sol = CvodeSolver::default()
        .solve(&problem, 400)
        .expect("solve should succeed");
    let err = max_error_vs_cos(&sol.t, &sol.y);
    assert!(err < 1e-5, "max error {err} exceeded 1e-5");
}
