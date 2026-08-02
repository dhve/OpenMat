//! Damped pendulum sanity check: x'' + 0.5 x' + sin(x) = 0. No analytic
//! solution, so this checks the physical invariants a correct integrator
//! must respect: mechanical energy only decreases (damping dissipates it),
//! and the trajectory stays bounded rather than blowing up.

use openmat_solve::{DormandPrince, OdeProblem, OdeSolver};

fn pendulum_problem() -> OdeProblem {
    OdeProblem::new(
        |_t, y, dy| {
            dy[0] = y[1];
            dy[1] = -0.5 * y[1] - y[0].sin();
        },
        vec![2.0, 0.0],
        (0.0, 30.0),
    )
}

/// Mechanical energy for x'' + c x' + sin(x) = 0 (unit mass, unit length,
/// unit gravity): kinetic + potential, with potential zeroed at the bottom.
fn energy(x: f64, xdot: f64) -> f64 {
    0.5 * xdot * xdot + (1.0 - x.cos())
}

#[test]
fn damped_pendulum_energy_decreases_and_stays_bounded() {
    let problem = pendulum_problem();
    let sol = DormandPrince::default()
        .solve(&problem, 300)
        .expect("solve should succeed");

    let energies: Vec<f64> = sol.y.iter().map(|y| energy(y[0], y[1])).collect();

    // Energy must not increase beyond a small numerical tolerance (dense
    // output interpolation and step error both add a little noise around
    // the true, strictly non-increasing curve).
    for w in energies.windows(2) {
        assert!(
            w[1] <= w[0] + 1e-4,
            "energy increased beyond tolerance: {} -> {}",
            w[0],
            w[1]
        );
    }

    // Damping should visibly remove energy over the run, not just hold it
    // constant within tolerance.
    let start = *energies.first().unwrap();
    let end = *energies.last().unwrap();
    assert!(
        start - end > 0.1,
        "expected meaningful energy loss, got {start} -> {end}"
    );

    // Bounded motion: no blow up.
    for y in &sol.y {
        assert!(y[0].abs() < 10.0, "position blew up: {}", y[0]);
        assert!(y[1].abs() < 10.0, "velocity blew up: {}", y[1]);
    }
}
