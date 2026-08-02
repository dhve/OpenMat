// Fixed-step RK4 integrator for the damped pendulum: x'' + c x' + sin(x) = 0.
// This is the mock engine's stand-in for the real openmat-kernel's NDSolve,
// which will dispatch to the Rust Solver trait (SUNDIALS or pure-Rust RK45).

export interface PendulumSample {
  t: number;
  x: number;
}

export interface PendulumParams {
  c: number;
  x0: number;
  v0: number;
  t0: number;
  t1: number;
  steps: number;
}

// State is [x, v] where v = dx/dt. The ODE as a first-order system:
//   dx/dt = v
//   dv/dt = -c v - sin(x)
function derivative(c: number, state: [number, number]): [number, number] {
  const [x, v] = state;
  return [v, -c * v - Math.sin(x)];
}

export function integratePendulum(params: PendulumParams): PendulumSample[] {
  const { c, x0, v0, t0, t1, steps } = params;
  if (steps < 1) throw new Error("integratePendulum needs at least one step");

  const h = (t1 - t0) / steps;
  const samples: PendulumSample[] = [{ t: t0, x: x0 }];

  let t = t0;
  let state: [number, number] = [x0, v0];

  for (let i = 0; i < steps; i++) {
    const k1 = derivative(c, state);
    const s2: [number, number] = [state[0] + (h / 2) * k1[0], state[1] + (h / 2) * k1[1]];
    const k2 = derivative(c, s2);
    const s3: [number, number] = [state[0] + (h / 2) * k2[0], state[1] + (h / 2) * k2[1]];
    const k3 = derivative(c, s3);
    const s4: [number, number] = [state[0] + h * k3[0], state[1] + h * k3[1]];
    const k4 = derivative(c, s4);

    state = [
      state[0] + (h / 6) * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]),
      state[1] + (h / 6) * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1]),
    ];
    t += h;
    samples.push({ t, x: state[0] });
  }

  return samples;
}
