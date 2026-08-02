//! Tests that only make sense when the SUNDIALS backend is compiled in.
//! The whole file compiles away under `--no-default-features`.

#![cfg(feature = "sundials")]

use openmat_solve::{CvodeSolver, DormandPrince, OdeProblem, OdeSolver};
use std::time::Instant;

#[test]
fn cvode_solves_stiff_van_der_pol_fast() {
    let mu = 1000.0;
    let problem = OdeProblem::new(
        move |_t, y, dy| {
            dy[0] = y[1];
            dy[1] = mu * ((1.0 - y[0] * y[0]) * y[1] - y[0]);
        },
        vec![2.0, 0.0],
        (0.0, 3000.0),
    );

    let start = Instant::now();
    let sol = CvodeSolver::default()
        .solve(&problem, 100)
        .expect("stiff solve should succeed");
    let elapsed = start.elapsed();

    assert_eq!(sol.t.len(), 100);
    assert!(
        elapsed.as_secs_f64() < 10.0,
        "CVODE took too long on stiff Van der Pol: {elapsed:?}"
    );

    // Van der Pol's relaxation oscillation has a well known bounded limit
    // cycle; a correct stiff solve should never blow up.
    for y in &sol.y {
        assert!(y[0].abs() < 3.0, "position left the limit cycle: {}", y[0]);
    }
}

#[test]
fn pure_rust_and_cvode_agree_on_harmonic_oscillator() {
    let problem = OdeProblem::new(
        |_t, y, dy| {
            dy[0] = y[1];
            dy[1] = -y[0];
        },
        vec![1.0, 0.0],
        (0.0, 20.0),
    );

    let rk = DormandPrince::default()
        .solve(&problem, 200)
        .expect("rk45 solve failed");
    let cv = CvodeSolver::default()
        .solve(&problem, 200)
        .expect("cvode solve failed");

    let max_diff = rk
        .y
        .iter()
        .zip(&cv.y)
        .map(|(a, b)| (a[0] - b[0]).abs().max((a[1] - b[1]).abs()))
        .fold(0.0, f64::max);

    assert!(max_diff < 1e-4, "backends disagree by {max_diff}");
}
