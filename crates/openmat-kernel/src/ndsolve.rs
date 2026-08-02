//! NDSolve dispatch: turns `NDSolve[{eqs...}, x, {t, t0, t1}]` into an
//! `openmat_solve::OdeProblem` and back into a plottable [`NdsolveOutcome`].
//!
//! Supported shapes: scalar first order (`x'[t] == f(t, x[t])`, one initial
//! condition `x[t0] == a`) and scalar second order (`x''[t] + ... == 0`, two
//! initial conditions `x[t0] == a` and `x'[t0] == b`), the flagship damped
//! pendulum being the motivating second-order case.
//!
//! ## Residual extraction
//!
//! Rather than solving the equation symbolically for the highest derivative,
//! we exploit that every equation this crate accepts is linear in that
//! derivative (the physics here never squares an acceleration). Build the
//! residual `R = lhs - rhs`, substitute placeholder symbols for the unknown
//! function, its derivatives, and the independent variable, then evaluate `R`
//! twice with the highest derivative placeholder set to 0 and 1. Since `R` is
//! affine in that placeholder, `R = a * d + b` for the two samples, so
//! `a = R(1) - R(0)`, `b = R(0)`, and the derivative solving `R = 0` is
//! `-b / a`. This needs no symbolic algebra, only two numeric evaluations.

use openmat_core::{replace_all, to_latex, Evaluator, Expr};
use openmat_solve::{solve_default, OdeProblem};

use crate::Curve;

const N_OUTPUT_POINTS: usize = 400;

/// What a successful NDSolve produces: the typeset equation system and the
/// solved trajectory, ready for the kernel to wrap into `Display`s.
#[derive(Debug)]
pub(crate) struct NdsolveOutcome {
    pub latex: String,
    pub curves: Vec<Curve>,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
}

/// Symbols used only inside this module to stand in for the unknown
/// function's value, its derivatives, and the independent variable while a
/// residual is evaluated numerically. Chosen unlikely to collide with any
/// user-typed symbol; never round-tripped through the parser.
fn placeholder(order: i64) -> Expr {
    Expr::symbol(format!("__D{order}"))
}
fn placeholder_t() -> Expr {
    Expr::symbol("__T")
}

const BUILTIN_NAMES: &[&str] =
    &["Plus", "Times", "Power", "Sin", "Cos", "Tan", "Exp", "Log", "Sqrt", "Abs", "N", "List", "Equal", "Rule", "Derivative"];

fn is_builtin_name(name: &str) -> bool {
    BUILTIN_NAMES.contains(&name)
}

/// True for a name that is expected to appear unbound in the residual: a
/// builtin function name, or one of this module's own placeholder symbols.
fn is_known_symbol(name: &str) -> bool {
    is_builtin_name(name) || (name.starts_with("__D") && name["__D".len()..].chars().all(|c| c.is_ascii_digit())) || name == "__T"
}

/// Handle `expr` (already confirmed to have head `NDSolve`), returning either
/// a ready [`NdsolveOutcome`], or a human readable error message.
pub(crate) fn solve(evaluator: &Evaluator, expr: &Expr) -> Result<NdsolveOutcome, String> {
    let (_head, args) = expr.as_normal().expect("caller checked head is NDSolve");
    if args.len() != 3 {
        return Err(format!("NDSolve expects 3 arguments (equations, function, {{t, t0, t1}}), got {}", args.len()));
    }

    let equations = list_items(&args[0]).ok_or_else(|| "NDSolve: first argument must be a list of equations".to_string())?;
    let var = args[1].as_symbol().ok_or_else(|| "NDSolve: second argument must be the function symbol, e.g. x".to_string())?;
    let (indep, t0, t1) = parse_range(&args[2])?;
    let indep = indep.as_str();

    if equations.is_empty() {
        return Err("NDSolve: no equations given".to_string());
    }

    // Split the equation list into the one dynamic (differential) equation
    // and the initial conditions, by trying to read each as an IC first.
    let mut dynamic: Option<(&Expr, &Expr)> = None;
    let mut ic0: Option<f64> = None; // x(t0)
    let mut ic1: Option<f64> = None; // x'(t0)

    for eq in &equations {
        let (lhs, rhs) = equal_parts(eq).ok_or_else(|| "NDSolve: every equation must use ==".to_string())?;
        match parse_ic(lhs, rhs, var) {
            Some((0, value)) => ic0 = Some(value),
            Some((1, value)) => ic1 = Some(value),
            Some((n, _)) => return Err(format!("NDSolve: initial conditions above order {n} are not supported")),
            None => {
                if dynamic.is_some() {
                    return Err("NDSolve: found more than one differential equation; only a single scalar ODE is supported".to_string());
                }
                dynamic = Some((lhs, rhs));
            }
        }
    }

    let (lhs, rhs) = dynamic.ok_or_else(|| "NDSolve: no differential equation found among the given equations".to_string())?;

    let order = if contains_subexpr(lhs, &deriv_expr(2, var, indep)) || contains_subexpr(rhs, &deriv_expr(2, var, indep)) {
        2
    } else if contains_subexpr(lhs, &deriv_expr(1, var, indep)) || contains_subexpr(rhs, &deriv_expr(1, var, indep)) {
        1
    } else {
        return Err(format!("NDSolve: could not find a derivative of {var} with respect to {indep} in the equations"));
    };

    // Build the residual R = lhs - rhs, unevaluated, then substitute
    // placeholder symbols for every occurrence of the derivative structure,
    // from the highest order down to the bare function value.
    let residual = Expr::plus(vec![lhs.clone(), Expr::times(vec![Expr::integer(-1), rhs.clone()])]);
    let mut rules = Vec::new();
    for k in (0..=order).rev() {
        rules.push(Expr::rule(deriv_expr(k, var, indep), placeholder(k)));
    }
    rules.push(Expr::rule(Expr::symbol(indep), placeholder_t()));
    let residual = replace_all(&residual, &Expr::list(rules));

    // Reject free parameters (like `c` in the pendulum equation) by scanning
    // the substituted residual directly, before binding any numbers. Probing
    // numerically instead (evaluate at a couple of sample derivative values)
    // is unsound here: a coefficient that legitimately multiplies a zero
    // initial condition (`x'[0] == 0` is common) would zero out the free
    // symbol along with it and hide the error, so a symbolic scan is the
    // only check that cannot be fooled by the specific numbers involved.
    if let Some(stray) = find_free_symbol_undefined(evaluator, &residual) {
        return Err(format!(
            "NDSolve: '{stray}' is not bound to a number; substitute a numeric value for '{stray}' before calling NDSolve"
        ));
    }

    let y0 = if order == 2 {
        let x0 = ic0.ok_or_else(|| format!("NDSolve: second-order equation needs an initial condition {var}({t0}) == ..."))?;
        let xp0 = ic1.ok_or_else(|| format!("NDSolve: second-order equation needs an initial condition {var}'({t0}) == ..."))?;
        vec![x0, xp0]
    } else {
        let x0 = ic0.ok_or_else(|| format!("NDSolve: first-order equation needs an initial condition {var}({t0}) == ..."))?;
        if ic1.is_some() {
            return Err(format!("NDSolve: first-order equation should not have a {var}' initial condition"));
        }
        vec![x0]
    };

    // Performance note: `residual` above is substituted once, outside the
    // solver's hot loop. Each RHS call below only binds four scalar
    // placeholders and re-evaluates the small residual tree; a future fast
    // path could compile `residual` into a native closure instead of walking
    // the Expr tree on every step, which is where the time actually goes
    // over a long integration.
    let residual_for_rhs = residual;
    // Owned snapshot: OdeProblem's RHS closure must be Send + 'static, so it
    // cannot borrow the session evaluator, but a fork carries the session's
    // definitions into the integration loop.
    let evaluator_for_rhs = evaluator.fork();
    let t_span = (t0, t1);

    let problem = if order == 2 {
        OdeProblem::new(
            move |t, y, dy| {
                let (x, xp) = (y[0], y[1]);
                let r_at = |d2: f64| bind_and_eval(&evaluator_for_rhs, &residual_for_rhs, 2, x, xp, d2, t);
                let r0 = r_at(0.0);
                let r1 = r_at(1.0);
                let a = r1 - r0;
                let b = r0;
                let xpp = if a == 0.0 { 0.0 } else { -b / a };
                dy[0] = xp;
                dy[1] = xpp;
            },
            y0,
            t_span,
        )
    } else {
        OdeProblem::new(
            move |t, y, dy| {
                let x = y[0];
                let r_at = |d1: f64| bind_and_eval(&evaluator_for_rhs, &residual_for_rhs, 1, x, 0.0, d1, t);
                let r0 = r_at(0.0);
                let r1 = r_at(1.0);
                let a = r1 - r0;
                let b = r0;
                let xp = if a == 0.0 { 0.0 } else { -b / a };
                dy[0] = xp;
            },
            y0,
            t_span,
        )
    };

    let solution = solve_default(&problem, N_OUTPUT_POINTS).map_err(|e| format!("NDSolve: {e}"))?;

    let xs = solution.component(0);
    let points: Vec<(f64, f64)> = solution.t.iter().copied().zip(xs.iter().copied()).collect();
    let y_range = padded_range(&xs);

    let latex = to_latex(&args[0]);

    Ok(NdsolveOutcome { latex, curves: vec![Curve { points, label: Some(format!("{var}(t)")) }], x_range: (t0, t1), y_range })
}

/// Bind the function value, its first derivative, and the independent
/// variable to concrete numbers, and substitute `highest_value` for the
/// order-th (highest) derivative placeholder. Used both by the preflight
/// check and, per solver step, by [`bind_and_eval`].
fn bind_placeholders(residual: &Expr, order: i64, d0: f64, d1: f64, highest_value: f64, t: f64) -> Expr {
    let mut rules = vec![Expr::rule(placeholder(0), Expr::real(d0)), Expr::rule(placeholder_t(), Expr::real(t))];
    if order == 1 {
        rules.push(Expr::rule(placeholder(1), Expr::real(highest_value)));
    } else {
        rules.push(Expr::rule(placeholder(1), Expr::real(d1)));
        rules.push(Expr::rule(placeholder(2), Expr::real(highest_value)));
    }
    replace_all(residual, &Expr::list(rules))
}

fn bind_and_eval(evaluator: &Evaluator, residual: &Expr, order: i64, d0: f64, d1: f64, highest_value: f64, t: f64) -> f64 {
    let bound = bind_placeholders(residual, order, d0, d1, highest_value, t);
    eval_numeric(evaluator, &bound).unwrap_or(f64::NAN)
}

/// Force full numeric evaluation via `N[...]`. Returns the offending free
/// symbol's name if the expression does not fold down to a plain number.
fn eval_numeric(evaluator: &Evaluator, expr: &Expr) -> Result<f64, String> {
    let wrapped = Expr::call("N", vec![expr.clone()]);
    match evaluator.eval(&wrapped) {
        Expr::Real(v) => Ok(v),
        Expr::Integer(v) => Ok(v as f64),
        other => Err(find_free_symbol(&other).unwrap_or_else(|| other.to_string())),
    }
}

fn find_free_symbol(e: &Expr) -> Option<String> {
    match e {
        Expr::Symbol(s) if !is_known_symbol(s) => Some(s.clone()),
        Expr::Normal { head, args } => find_free_symbol(head).or_else(|| args.iter().find_map(find_free_symbol)),
        _ => None,
    }
}

/// Like [`find_free_symbol`], but session-aware: a symbol carrying a user
/// definition (`g = 9.8`) is bound, not stray, since the evaluator resolves
/// it during residual evaluation.
fn find_free_symbol_undefined(evaluator: &Evaluator, e: &Expr) -> Option<String> {
    match e {
        Expr::Symbol(s) if !is_known_symbol(s) && !evaluator.has_definition(s) => Some(s.clone()),
        Expr::Normal { head, args } => {
            find_free_symbol_undefined(evaluator, head).or_else(|| args.iter().find_map(|a| find_free_symbol_undefined(evaluator, a)))
        }
        _ => None,
    }
}

/// Build the exact tree shape the parser produces for the `order`-th
/// derivative of `var` applied to `indep`: `var[indep]` for order 0,
/// `Derivative[order][var][indep]` otherwise.
fn deriv_expr(order: i64, var: &str, indep: &str) -> Expr {
    if order == 0 {
        Expr::call(var, vec![Expr::symbol(indep)])
    } else {
        let deriv_op = Expr::normal(Expr::symbol("Derivative"), vec![Expr::integer(order)]);
        let applied = Expr::normal(deriv_op, vec![Expr::symbol(var)]);
        Expr::normal(applied, vec![Expr::symbol(indep)])
    }
}

fn contains_subexpr(haystack: &Expr, needle: &Expr) -> bool {
    if haystack == needle {
        return true;
    }
    if let Expr::Normal { head, args } = haystack {
        contains_subexpr(head, needle) || args.iter().any(|a| contains_subexpr(a, needle))
    } else {
        false
    }
}

fn list_items(e: &Expr) -> Option<Vec<Expr>> {
    let (head, args) = e.as_normal()?;
    if head.as_symbol() == Some("List") {
        Some(args.to_vec())
    } else {
        None
    }
}

fn equal_parts(e: &Expr) -> Option<(&Expr, &Expr)> {
    let (head, args) = e.as_normal()?;
    if head.as_symbol() == Some("Equal") && args.len() == 2 {
        Some((&args[0], &args[1]))
    } else {
        None
    }
}

/// Read `{t, t0, t1}` into the independent variable's name and its numeric
/// bounds. Returns an owned name rather than a borrow, since the list is
/// read from a freshly cloned `Vec<Expr>`.
fn parse_range(e: &Expr) -> Result<(String, f64, f64), String> {
    let items = list_items(e).ok_or_else(|| "NDSolve: third argument must be {t, t0, t1}".to_string())?;
    if items.len() != 3 {
        return Err("NDSolve: third argument must be {t, t0, t1}".to_string());
    }
    let indep = items[0].as_symbol().ok_or_else(|| "NDSolve: {t, t0, t1}: t must be a symbol".to_string())?.to_string();
    let t0 = to_f64(&items[1]).ok_or_else(|| "NDSolve: {t, t0, t1}: t0 must be a number".to_string())?;
    let t1 = to_f64(&items[2]).ok_or_else(|| "NDSolve: {t, t0, t1}: t1 must be a number".to_string())?;
    Ok((indep, t0, t1))
}

fn to_f64(e: &Expr) -> Option<f64> {
    match e {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Real(x) => Some(*x),
        _ => None,
    }
}

/// Try to read `lhs == rhs` as an initial condition on `var` at a numeric
/// time: `var[t0] == a` (order 0) or `var'[t0] == a` (order 1, i.e.
/// `Derivative[1][var][t0]`). Returns `None` if the shape does not match,
/// meaning this equation is the dynamic one instead.
fn parse_ic(lhs: &Expr, rhs: &Expr, var: &str) -> Option<(i64, f64)> {
    let value = to_f64(rhs)?;
    let (head, args) = lhs.as_normal()?;
    if args.len() != 1 {
        return None;
    }
    to_f64(&args[0])?; // the time argument must be a concrete number
    if head.as_symbol() == Some(var) {
        return Some((0, value));
    }
    let (mid_head, f_args) = head.as_normal()?;
    if f_args.len() != 1 || f_args[0].as_symbol() != Some(var) {
        return None;
    }
    let (deriv_head, n_args) = mid_head.as_normal()?;
    if deriv_head.as_symbol() != Some("Derivative") || n_args.len() != 1 {
        return None;
    }
    match &n_args[0] {
        Expr::Integer(n) => Some((*n, value)),
        _ => None,
    }
}

fn padded_range(values: &[f64]) -> (f64, f64) {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !min.is_finite() || !max.is_finite() {
        return (-1.0, 1.0);
    }
    let span = max - min;
    let pad = if span.abs() < 1e-9 { 1.0 } else { span * 0.05 };
    (min - pad, max + pad)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openmat_core::parse;

    fn ndsolve_expr(src: &str) -> Expr {
        parse(src).unwrap()
    }

    #[test]
    fn damped_pendulum_with_c_bound_produces_plot() {
        let e = ndsolve_expr("NDSolve[{x''[t] + 0.5 x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]");
        let result = solve(&Evaluator::new(), &e).expect("should solve");
        assert_eq!(result.curves.len(), 1);
        assert_eq!(result.curves[0].points.len(), 400);
        let first = result.curves[0].points[0];
        assert!((first.0 - 0.0).abs() < 1e-9);
        assert!((first.1 - 2.0).abs() < 1e-9);

        // Decaying oscillation: a local max well after the start should sit
        // below the initial amplitude.
        let pts = &result.curves[0].points;
        let mut found_late_max_below_start = false;
        for i in 1..pts.len() - 1 {
            let (t, x) = pts[i];
            if t > 5.0 && x > pts[i - 1].1 && x > pts[i + 1].1 && x < 2.0 {
                found_late_max_below_start = true;
                break;
            }
        }
        assert!(found_late_max_below_start, "expected a decaying local max after t = 5");
    }

    #[test]
    fn harmonic_oscillator_matches_cosine() {
        let e = ndsolve_expr("NDSolve[{x''[t] + x[t] == 0, x[0] == 1, x'[0] == 0}, x, {t, 0, 6.28}]");
        let result = solve(&Evaluator::new(), &e).expect("should solve");
        let (t_at_pi, x_at_pi) = result.curves[0]
            .points
            .iter()
            .copied()
            .min_by(|a, b| (a.0 - std::f64::consts::PI).abs().total_cmp(&(b.0 - std::f64::consts::PI).abs()))
            .unwrap();
        assert!((t_at_pi - std::f64::consts::PI).abs() < 0.05);
        assert!((x_at_pi - (-1.0)).abs() < 1e-3, "x(pi) = {x_at_pi}");
    }

    #[test]
    fn first_order_decay_matches_exp() {
        let e = ndsolve_expr("NDSolve[{x'[t] == -x[t], x[0] == 1}, x, {t, 0, 1}]");
        let result = solve(&Evaluator::new(), &e).expect("should solve");
        let last = *result.curves[0].points.last().unwrap();
        assert!((last.0 - 1.0).abs() < 1e-9);
        assert!((last.1 - std::f64::consts::E.recip()).abs() < 1e-3, "x(1) = {}", last.1);
    }

    #[test]
    fn unbound_parameter_is_a_clear_error() {
        let e = ndsolve_expr("NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]");
        let err = solve(&Evaluator::new(), &e).unwrap_err();
        assert!(err.contains('c'), "error should mention the unbound symbol: {err}");
        assert!(err.to_lowercase().contains("substitute"), "error should tell the user to substitute a value: {err}");
    }
}
