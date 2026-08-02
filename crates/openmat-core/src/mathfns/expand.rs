//! `Expand[expr]`: distribute products over sums, and integer powers of sums.
//!
//! `expand` is total (it always returns *some* `Expr`, the original one
//! unchanged when there's nothing to distribute); the caller re-runs the
//! result through [`Evaluator::eval`] to fold numeric coefficients and
//! collect like terms, so this module only needs to worry about the
//! distribution shape, not arithmetic cleanup.

use crate::eval::Evaluator;
use crate::expr::Expr;

/// Binomial/multinomial expansion is capped at this exponent: past this, the
/// term count explodes and it stops being "reasonable" for a course-level
/// CAS pass. `Power[base, n]` with `n` above the cap is left unexpanded.
const MAX_EXPAND_POWER: i64 = 8;

pub fn dispatch_expand(args: &[Expr], ev: &Evaluator) -> Option<Expr> {
    if args.len() != 1 {
        return None;
    }
    Some(ev.eval(&expand(&args[0])))
}

/// Distribute `Times` over `Plus` and expand non-negative integer powers of
/// `Plus`, recursively. Function-call arguments (`Sin[(x+1)^2]`, etc.) are
/// left alone: `Expand` only distributes the polynomial structure it is
/// actually built from, not everything nested inside opaque calls.
pub fn expand(e: &Expr) -> Expr {
    match e {
        Expr::Normal { head, args } => match head.as_symbol() {
            Some("Plus") => Expr::plus(args.iter().map(expand).collect()),
            Some("Times") => {
                let expanded_args: Vec<Expr> = args.iter().map(expand).collect();
                distribute_times(&expanded_args)
            }
            Some("Power") if args.len() == 2 => {
                let base = expand(&args[0]);
                if let Expr::Integer(n) = &args[1] {
                    if *n >= 0 && *n <= MAX_EXPAND_POWER && base.has_head("Plus") {
                        return expand_integer_power(&base, *n);
                    }
                }
                Expr::power(base, args[1].clone())
            }
            _ => e.clone(),
        },
        _ => e.clone(),
    }
}

/// Multiply out a list of already-expanded factors: every `Plus` factor
/// forces a cartesian-product distribution over the accumulated sum-of-products.
fn distribute_times(factors: &[Expr]) -> Expr {
    let mut acc: Vec<Expr> = vec![Expr::integer(1)];
    for f in factors {
        if let Some((head, fargs)) = f.as_normal() {
            if head.as_symbol() == Some("Plus") {
                let mut next = Vec::with_capacity(acc.len() * fargs.len());
                for term in fargs {
                    for a in &acc {
                        next.push(Expr::times(vec![a.clone(), term.clone()]));
                    }
                }
                acc = next;
                continue;
            }
        }
        for a in acc.iter_mut() {
            *a = Expr::times(vec![a.clone(), f.clone()]);
        }
    }
    if acc.len() == 1 {
        acc.into_iter().next().unwrap()
    } else {
        Expr::plus(acc)
    }
}

/// `Power[Plus[terms...], n]` for a small non-negative integer `n`: repeated
/// distribution rather than a closed-form binomial coefficient table, so it
/// generalizes to trinomials and beyond for free.
fn expand_integer_power(base: &Expr, n: i64) -> Expr {
    if n == 0 {
        return Expr::integer(1);
    }
    let terms: Vec<Expr> = match base.as_normal() {
        Some((head, args)) if head.as_symbol() == Some("Plus") => args.to_vec(),
        _ => vec![base.clone()],
    };
    let mut acc: Vec<Expr> = vec![Expr::integer(1)];
    for _ in 0..n {
        let mut next = Vec::with_capacity(acc.len() * terms.len());
        for term in &terms {
            for a in &acc {
                next.push(Expr::times(vec![a.clone(), term.clone()]));
            }
        }
        acc = next;
    }
    if acc.len() == 1 {
        acc.into_iter().next().unwrap()
    } else {
        Expr::plus(acc)
    }
}
