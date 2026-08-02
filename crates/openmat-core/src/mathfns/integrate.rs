//! `Integrate[expr, x]`: table-driven indefinite integration for the
//! calculus-course core: constants, the power rule (`1/x` giving `Log[x]`),
//! linearity, `Sin`/`Cos`/`Exp` (including the linear-argument form
//! `f[a x + b]` and, for `Exp`-shaped bases, `a^(c x + d)`), and polynomial
//! products/powers handled by expanding first and integrating termwise.
//!
//! This is deliberately not a general integrator: anything outside this
//! table returns `None` so the caller stays symbolic rather than risking a
//! wrong closed form. No constant of integration is added, matching
//! `Integrate`'s own convention (the constant is the caller's job if wanted).

use crate::eval::Evaluator;
use crate::expr::Expr;
use crate::mathfns::expand;
use crate::mathfns::support::{depends_on, linear_form};
use crate::pattern::replace_all;

pub fn dispatch_integrate(args: &[Expr], ev: &Evaluator) -> Option<Expr> {
    if args.len() != 2 {
        return None;
    }
    // Definite form: Integrate[f, {x, a, b}].
    if let Some((var, a, b)) = iterator_spec(&args[1]) {
        return definite(&args[0], &var, &a, &b, ev);
    }
    let var = args[1].as_symbol()?;
    let result = integrate(&args[0], var, ev)?;
    Some(ev.eval(&result))
}

/// Reads `{x, a, b}` into (variable name, lower, upper).
fn iterator_spec(e: &Expr) -> Option<(String, Expr, Expr)> {
    let (head, items) = e.as_normal()?;
    if head.as_symbol() != Some("List") || items.len() != 3 {
        return None;
    }
    Some((items[0].as_symbol()?.to_string(), items[1].clone(), items[2].clone()))
}

/// A definite integral: the antiderivative difference F(b) - F(a) when the
/// symbolic table finds F, folding to a plain number when nothing symbolic
/// remains (so `1 - Sin[4 Pi]/(4 Pi)` becomes 1 rather than staying
/// unevaluated); otherwise numeric quadrature over finite numeric bounds.
fn definite(f: &Expr, var: &str, a: &Expr, b: &Expr, ev: &Evaluator) -> Option<Expr> {
    if let Some(anti) = integrate(f, var, ev) {
        let at = |bound: &Expr| replace_all(&anti, &Expr::list(vec![Expr::rule(Expr::symbol(var), bound.clone())]));
        let difference = Expr::plus(vec![at(b), Expr::times(vec![Expr::integer(-1), at(a)])]);
        let folded = ev.eval(&difference);
        // Trig-of-Pi leftovers (Sin[4 Pi] etc.) don't reduce symbolically in
        // this kernel yet; if the result is constant AND still carries a
        // function call, N-fold it. A purely arithmetic constant like 1/3
        // stays exact.
        if !folded.is_numeric() && is_constant(&folded) && has_function_call(&folded) {
            let numeric = ev.eval(&Expr::call("N", vec![folded.clone()]));
            if numeric.is_numeric() {
                return Some(numeric);
            }
        }
        return Some(folded);
    }
    numeric_definite(f, var, a, b, ev)
}

/// True when `e` contains no symbols other than the constants N[...] can
/// resolve numerically.
fn is_constant(e: &Expr) -> bool {
    match e {
        Expr::Symbol(s) => matches!(s.as_str(), "Pi" | "E" | "Degree"),
        Expr::Normal { head, args } => head.as_symbol().is_some() && args.iter().all(is_constant),
        _ => true,
    }
}

/// True when `e` contains a call to anything beyond bare arithmetic, i.e. a
/// piece the arithmetic folder cannot reduce on its own.
fn has_function_call(e: &Expr) -> bool {
    match e {
        Expr::Normal { head, args } => {
            !matches!(head.as_symbol(), Some("Plus") | Some("Times") | Some("Power")) || args.iter().any(has_function_call)
        }
        _ => false,
    }
}

fn to_f64(e: &Expr) -> Option<f64> {
    match e {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Real(x) => Some(*x),
        _ => None,
    }
}

/// Composite Simpson quadrature for a definite integral the symbolic table
/// cannot handle (Exp[-x^2], for example). Requires finite numeric bounds
/// and an integrand that evaluates to a finite number at every sample;
/// anything else stays symbolic rather than returning a wrong answer.
fn numeric_definite(f: &Expr, var: &str, a: &Expr, b: &Expr, ev: &Evaluator) -> Option<Expr> {
    let lo = to_f64(&ev.eval(&Expr::call("N", vec![a.clone()])))?;
    let hi = to_f64(&ev.eval(&Expr::call("N", vec![b.clone()])))?;
    if !lo.is_finite() || !hi.is_finite() {
        return None;
    }
    if lo == hi {
        return Some(Expr::integer(0));
    }

    let sample = |x: f64| -> Option<f64> {
        let bound = replace_all(f, &Expr::list(vec![Expr::rule(Expr::symbol(var), Expr::real(x))]));
        let value = to_f64(&ev.eval(&Expr::call("N", vec![bound])))?;
        value.is_finite().then_some(value)
    };

    const INTERVALS: usize = 800; // even, ample for smooth course-level integrands
    let h = (hi - lo) / INTERVALS as f64;
    let mut sum = sample(lo)? + sample(hi)?;
    for i in 1..INTERVALS {
        let weight = if i % 2 == 1 { 4.0 } else { 2.0 };
        sum += weight * sample(lo + i as f64 * h)?;
    }
    Some(Expr::real(sum * h / 3.0))
}

/// The antiderivative of `expr` with respect to `var`, or `None` when no
/// rule in this table applies. Builds a raw `Expr` tree; the caller re-runs
/// it through [`Evaluator::eval`] to fold and canonicalize.
fn integrate(expr: &Expr, var: &str, ev: &Evaluator) -> Option<Expr> {
    if !depends_on(expr, var) {
        // Constant rule: Integrate[c, x] -> c*x
        return Some(Expr::times(vec![expr.clone(), Expr::symbol(var)]));
    }
    match expr {
        Expr::Symbol(s) if s == var => {
            // Integrate[x, x] -> x^2/2
            let half = Expr::power(Expr::integer(2), Expr::integer(-1));
            Some(Expr::times(vec![half, Expr::power(Expr::symbol(var), Expr::integer(2))]))
        }
        Expr::Normal { head, args } => {
            let name = head.as_symbol()?;
            match name {
                "Plus" => {
                    let mut terms = Vec::with_capacity(args.len());
                    for a in args {
                        terms.push(integrate(a, var, ev)?);
                    }
                    Some(Expr::plus(terms))
                }
                "Times" => integrate_times(expr, args, var, ev),
                "Power" if args.len() == 2 => integrate_power(expr, &args[0], &args[1], var, ev),
                "Sin" | "Cos" | "Exp" if args.len() == 1 => integrate_table_fn(name, &args[0], var),
                _ => None,
            }
        }
        _ => None,
    }
}

fn integrate_times(whole: &Expr, factors: &[Expr], var: &str, ev: &Evaluator) -> Option<Expr> {
    let mut const_factors = Vec::new();
    let mut var_factors = Vec::new();
    for f in factors {
        if depends_on(f, var) {
            var_factors.push(f.clone());
        } else {
            const_factors.push(f.clone());
        }
    }
    if var_factors.len() == 1 {
        let inner = integrate(&var_factors[0], var, ev)?;
        let mut result = const_factors;
        result.push(inner);
        Some(Expr::times(result))
    } else {
        // A product of two trig factors (Sin[2 Pi x] Sin[Pi x], the
        // orthogonality integrals): rewrite product-to-sum, then integrate
        // the sum of single trig terms.
        if var_factors.len() == 2 {
            if let Some(rewritten) = trig_product(&var_factors[0], &var_factors[1]) {
                let mut product = const_factors;
                product.push(rewritten);
                let inner = integrate(&ev.eval(&Expr::times(product.clone())), var, ev);
                if inner.is_some() {
                    return inner;
                }
            }
        }
        // More than one factor depends on x (a product of polynomial
        // pieces, e.g. x*(x+1)): expand into a sum and integrate termwise.
        expand_then_integrate(whole, var, ev)
    }
}

/// Product-to-sum for a pair of trig factors:
///   Sin[u] Sin[v] -> (Cos[u-v] - Cos[u+v]) / 2
///   Cos[u] Cos[v] -> (Cos[u-v] + Cos[u+v]) / 2
///   Sin[u] Cos[v] -> (Sin[u+v] + Sin[u-v]) / 2
fn trig_product(f1: &Expr, f2: &Expr) -> Option<Expr> {
    let (h1, a1) = f1.as_normal()?;
    let (h2, a2) = f2.as_normal()?;
    let (n1, n2) = (h1.as_symbol()?, h2.as_symbol()?);
    if a1.len() != 1 || a2.len() != 1 {
        return None;
    }
    let (u, v) = (a1[0].clone(), a2[0].clone());
    let sum = Expr::plus(vec![u.clone(), v.clone()]);
    let diff = Expr::plus(vec![u, Expr::times(vec![Expr::integer(-1), v])]);
    let neg = |e: Expr| Expr::times(vec![Expr::integer(-1), e]);
    let combined = match (n1, n2) {
        ("Sin", "Sin") => Expr::plus(vec![Expr::call("Cos", vec![diff]), neg(Expr::call("Cos", vec![sum]))]),
        ("Cos", "Cos") => Expr::plus(vec![Expr::call("Cos", vec![diff]), Expr::call("Cos", vec![sum])]),
        ("Sin", "Cos") => Expr::plus(vec![Expr::call("Sin", vec![sum]), Expr::call("Sin", vec![diff])]),
        ("Cos", "Sin") => Expr::plus(vec![Expr::call("Sin", vec![sum]), neg(Expr::call("Sin", vec![diff]))]),
        _ => return None,
    };
    Some(Expr::times(vec![Expr::power(Expr::integer(2), Expr::integer(-1)), combined]))
}

fn integrate_power(whole: &Expr, base: &Expr, exp: &Expr, var: &str, ev: &Evaluator) -> Option<Expr> {
    let base_dep = depends_on(base, var);
    let exp_dep = depends_on(exp, var);

    // Sin[u]^2 / Cos[u]^2: power reduction, the workhorse of normalization
    // integrals (Integrate[2 Sin[2 Pi x]^2, {x, 0, 1}] and friends).
    if base_dep && !exp_dep && matches!(exp, Expr::Integer(2)) {
        if let Some(rewritten) = trig_square(base) {
            if let Some(result) = integrate(&ev.eval(&rewritten), var, ev) {
                return Some(result);
            }
        }
    }

    if base_dep && !exp_dep {
        // (a x + b)^n, n constant: power rule (with the a x + b -> x
        // special case falling out when a = 1, b = 0).
        if let Some((a, _b)) = linear_form(base, var) {
            if !a.is_zero() {
                if let Some(result) = power_rule_linear_base(&a, base, exp, ev) {
                    return Some(result);
                }
            }
        }
    } else if !base_dep && exp_dep {
        // a^(c x + d), a constant: exponential rule via a^u / (c Log[a]).
        if let Some((c, _d)) = linear_form(exp, var) {
            if !c.is_zero() {
                let denom = Expr::times(vec![c, Expr::call("Log", vec![base.clone()])]);
                return Some(Expr::times(vec![Expr::power(base.clone(), exp.clone()), Expr::power(denom, Expr::integer(-1))]));
            }
        }
    }
    expand_then_integrate(whole, var, ev)
}

fn power_rule_linear_base(a: &Expr, base: &Expr, exp: &Expr, ev: &Evaluator) -> Option<Expr> {
    let is_neg_one = matches!(exp, Expr::Integer(-1)) || matches!(exp, Expr::Real(v) if *v == -1.0);
    if is_neg_one {
        // Integrate[1/(a x + b), x] -> Log[a x + b] / a
        let lg = Expr::call("Log", vec![base.clone()]);
        return Some(Expr::times(vec![lg, Expr::power(a.clone(), Expr::integer(-1))]));
    }
    if !exp.is_numeric() {
        return None;
    }
    let n_plus_1 = ev.eval(&Expr::plus(vec![exp.clone(), Expr::integer(1)]));
    if n_plus_1.is_zero() {
        return None;
    }
    let denom = Expr::times(vec![a.clone(), n_plus_1.clone()]);
    Some(Expr::times(vec![Expr::power(base.clone(), n_plus_1), Expr::power(denom, Expr::integer(-1))]))
}

/// Sin[u]^2 -> (1 - Cos[2u])/2, Cos[u]^2 -> (1 + Cos[2u])/2.
fn trig_square(base: &Expr) -> Option<Expr> {
    let (head, args) = base.as_normal()?;
    let name = head.as_symbol()?;
    if args.len() != 1 {
        return None;
    }
    let cos2u = Expr::call("Cos", vec![Expr::times(vec![Expr::integer(2), args[0].clone()])]);
    let half = Expr::power(Expr::integer(2), Expr::integer(-1));
    match name {
        "Sin" => Some(Expr::times(vec![half, Expr::plus(vec![Expr::integer(1), Expr::times(vec![Expr::integer(-1), cos2u])])])),
        "Cos" => Some(Expr::times(vec![half, Expr::plus(vec![Expr::integer(1), cos2u])])),
        _ => None,
    }
}

fn integrate_table_fn(name: &str, u: &Expr, var: &str) -> Option<Expr> {
    let (a, _b) = linear_form(u, var)?;
    if a.is_zero() {
        return None;
    }
    let antiderivative = match name {
        "Sin" => Expr::times(vec![Expr::integer(-1), Expr::call("Cos", vec![u.clone()])]),
        "Cos" => Expr::call("Sin", vec![u.clone()]),
        "Exp" => Expr::call("Exp", vec![u.clone()]),
        _ => return None,
    };
    Some(Expr::times(vec![antiderivative, Expr::power(a, Expr::integer(-1))]))
}

/// Fallback for products/powers the direct rules above don't cover: expand
/// the polynomial structure and integrate the resulting sum termwise. Bails
/// out (`None`) if expansion doesn't actually change anything, so a
/// genuinely unsupported integrand (e.g. `x*Sin[x]`, needing integration by
/// parts) fails once rather than looping.
fn expand_then_integrate(expr: &Expr, var: &str, ev: &Evaluator) -> Option<Expr> {
    let expanded = ev.eval(&expand::expand(expr));
    if expanded.to_string() == expr.to_string() {
        return None;
    }
    integrate(&expanded, var, ev)
}
