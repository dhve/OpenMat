//! `Factor[expr]`: quadratics with integer roots, factored over the single
//! variable the expression is written in. Anything else (higher degree,
//! irrational or non-integer roots, more than one variable) returns `None`
//! rather than a guess: this is meant to be cheap and honest, not a general
//! factoring engine (that belongs to a real polynomial-factorization pass,
//! out of scope here).

use crate::eval::Evaluator;
use crate::expr::Expr;
use crate::mathfns::expand;
use crate::mathfns::support::{int_val, only_symbol, poly_coeffs};

pub fn dispatch_factor(args: &[Expr], ev: &Evaluator) -> Option<Expr> {
    if args.len() != 1 {
        return None;
    }
    let var = only_symbol(&args[0])?;
    let expanded = ev.eval(&expand::expand(&args[0]));
    let coeffs = poly_coeffs(&expanded, &var)?;
    if coeffs.len() != 3 {
        return None; // only quadratics are in scope
    }
    let c0 = int_val(&coeffs[0])?;
    let c1 = int_val(&coeffs[1])?;
    let c2 = int_val(&coeffs[2])?;
    if c2 == 0 {
        return None;
    }

    let disc = c1 * c1 - 4 * c2 * c0;
    if disc < 0 {
        return None; // complex roots: not "integer roots"
    }
    let sqrt_disc = integer_sqrt(disc)?;
    let denom = 2 * c2;
    if (-c1 - sqrt_disc) % denom != 0 || (-c1 + sqrt_disc) % denom != 0 {
        return None; // roots aren't integers
    }
    let r1 = (-c1 - sqrt_disc) / denom;
    let r2 = (-c1 + sqrt_disc) / denom;

    // expr = c2 * (x - r1) * (x - r2)
    let factor1 = Expr::plus(vec![Expr::symbol(&var), Expr::integer(-r1)]);
    let factor2 = Expr::plus(vec![Expr::symbol(&var), Expr::integer(-r2)]);
    let result = if c2 == 1 {
        Expr::times(vec![factor1, factor2])
    } else {
        Expr::times(vec![Expr::integer(c2), factor1, factor2])
    };
    // `eval` here only folds the numeric leading coefficient and sorts
    // factors; canonicalize_times never distributes across a `Plus` factor,
    // so the product stays factored rather than expanding back out.
    Some(ev.eval(&result))
}

fn integer_sqrt(n: i64) -> Option<i64> {
    if n < 0 {
        return None;
    }
    let approx = (n as f64).sqrt().round() as i64;
    for candidate in [approx - 1, approx, approx + 1] {
        if candidate >= 0 && candidate * candidate == n {
            return Some(candidate);
        }
    }
    None
}
