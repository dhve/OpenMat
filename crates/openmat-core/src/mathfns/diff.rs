//! `D[expr, x]`: symbolic differentiation.
//!
//! `diff` is total over the elementary functions this crate knows about
//! (`Plus`, `Times`, `Power`, `Sin`, `Cos`, `Tan`, `Exp`, `Log`, `Sqrt`) and
//! honest everywhere else: an expression built from anything outside that
//! set (an unknown function call that actually depends on the variable)
//! makes `diff` return `None`, which propagates all the way out so `D[...]`
//! is left symbolic rather than silently wrong.
//!
//! The quotient rule falls out for free: this crate represents `a/b` as
//! `Times[a, Power[b, -1]]`, so the product rule plus the `Power` chain rule
//! (with a constant, possibly negative, exponent) already differentiates it
//! correctly without any special-casing.

use crate::eval::Evaluator;
use crate::expr::Expr;
use crate::mathfns::support::depends_on;

/// `D[expr, x]` entry point: `args` is `[expr, x]`, already evaluated by the
/// caller. Returns `None` for anything not shaped like a two-argument call
/// with a plain symbol variable, or when [`diff`] can't handle some part of
/// `expr`.
pub fn dispatch_d(args: &[Expr], ev: &Evaluator) -> Option<Expr> {
    if args.len() != 2 {
        return None;
    }
    let var = args[1].as_symbol()?;
    let result = diff(&args[0], var, ev)?;
    Some(ev.eval(&result))
}

/// The derivative of `expr` with respect to `var`, or `None` if some
/// subexpression depends on `var` in a shape this module doesn't have a rule
/// for. Builds a (not necessarily canonical) `Expr` tree; the caller is
/// expected to run it back through [`Evaluator::eval`] to fold and sort it.
pub fn diff(expr: &Expr, var: &str, ev: &Evaluator) -> Option<Expr> {
    if !depends_on(expr, var) {
        return Some(Expr::integer(0));
    }
    match expr {
        Expr::Symbol(s) if s == var => Some(Expr::integer(1)),
        Expr::Symbol(_) | Expr::Integer(_) | Expr::Real(_) | Expr::Str(_) => Some(Expr::integer(0)),
        Expr::Normal { head, args } => {
            let name = head.as_symbol()?;
            match name {
                "Plus" => {
                    let mut terms = Vec::with_capacity(args.len());
                    for a in args {
                        terms.push(diff(a, var, ev)?);
                    }
                    Some(Expr::plus(terms))
                }
                "Times" => diff_times(args, var, ev),
                "Power" if args.len() == 2 => diff_power(&args[0], &args[1], var, ev),
                "Sin" if args.len() == 1 => {
                    let du = diff(&args[0], var, ev)?;
                    Some(Expr::times(vec![Expr::call("Cos", vec![args[0].clone()]), du]))
                }
                "Cos" if args.len() == 1 => {
                    let du = diff(&args[0], var, ev)?;
                    Some(Expr::times(vec![Expr::integer(-1), Expr::call("Sin", vec![args[0].clone()]), du]))
                }
                "Tan" if args.len() == 1 => {
                    // d/dx Tan[u] = u' / Cos[u]^2, expressed with only the
                    // builtins the evaluator already knows how to fold.
                    let du = diff(&args[0], var, ev)?;
                    let cos_sq = Expr::power(Expr::call("Cos", vec![args[0].clone()]), Expr::integer(-2));
                    Some(Expr::times(vec![cos_sq, du]))
                }
                "Exp" if args.len() == 1 => {
                    let du = diff(&args[0], var, ev)?;
                    Some(Expr::times(vec![Expr::call("Exp", vec![args[0].clone()]), du]))
                }
                "Log" if args.len() == 1 => {
                    let du = diff(&args[0], var, ev)?;
                    Some(Expr::times(vec![du, Expr::power(args[0].clone(), Expr::integer(-1))]))
                }
                "Sqrt" if args.len() == 1 => {
                    let du = diff(&args[0], var, ev)?;
                    let half = Expr::power(Expr::integer(2), Expr::integer(-1));
                    let recip_sqrt = Expr::power(Expr::call("Sqrt", vec![args[0].clone()]), Expr::integer(-1));
                    Some(Expr::times(vec![half, du, recip_sqrt]))
                }
                _ => None,
            }
        }
    }
}

fn diff_times(factors: &[Expr], var: &str, ev: &Evaluator) -> Option<Expr> {
    let mut terms = Vec::with_capacity(factors.len());
    for i in 0..factors.len() {
        let di = diff(&factors[i], var, ev)?;
        let mut product = vec![di];
        for (j, f) in factors.iter().enumerate() {
            if j != i {
                product.push(f.clone());
            }
        }
        terms.push(Expr::times(product));
    }
    Some(Expr::plus(terms))
}

fn diff_power(base: &Expr, exp: &Expr, var: &str, ev: &Evaluator) -> Option<Expr> {
    let base_dep = depends_on(base, var);
    let exp_dep = depends_on(exp, var);
    if !exp_dep {
        // f(x)^n, n constant: n * f^(n-1) * f'
        let db = diff(base, var, ev)?;
        let n_minus_1 = Expr::plus(vec![exp.clone(), Expr::integer(-1)]);
        Some(Expr::times(vec![exp.clone(), Expr::power(base.clone(), n_minus_1), db]))
    } else if !base_dep {
        // a^g(x), a constant: a^g * Log[a] * g'
        let dg = diff(exp, var, ev)?;
        Some(Expr::times(vec![Expr::power(base.clone(), exp.clone()), Expr::call("Log", vec![base.clone()]), dg]))
    } else {
        // f(x)^g(x), general case via logarithmic differentiation:
        // f^g * (g' Log[f] + g f'/f)
        let db = diff(base, var, ev)?;
        let dg = diff(exp, var, ev)?;
        let term1 = Expr::times(vec![dg, Expr::call("Log", vec![base.clone()])]);
        let term2 = Expr::times(vec![exp.clone(), db, Expr::power(base.clone(), Expr::integer(-1))]);
        Some(Expr::times(vec![Expr::power(base.clone(), exp.clone()), Expr::plus(vec![term1, term2])]))
    }
}
