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

pub fn dispatch_integrate(args: &[Expr], ev: &Evaluator) -> Option<Expr> {
    if args.len() != 2 {
        return None;
    }
    let var = args[1].as_symbol()?;
    let result = integrate(&args[0], var, ev)?;
    Some(ev.eval(&result))
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
        // More than one factor depends on x (a product of polynomial
        // pieces, e.g. x*(x+1)): expand into a sum and integrate termwise.
        expand_then_integrate(whole, var, ev)
    }
}

fn integrate_power(whole: &Expr, base: &Expr, exp: &Expr, var: &str, ev: &Evaluator) -> Option<Expr> {
    let base_dep = depends_on(base, var);
    let exp_dep = depends_on(exp, var);

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
