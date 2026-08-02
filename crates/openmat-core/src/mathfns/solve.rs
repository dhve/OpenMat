//! `Solve[eq, x]`: linear and quadratic equations, plus factored-polynomial
//! input (`(x-2)(x-3) == 0`) via `Expand` first. Degree 0/1/2 only; anything
//! higher, or an equation that isn't polynomial in `x` at all, returns
//! `None` so the caller stays symbolic rather than guessing.
//!
//! Output shape matches Mathematica: a `List` of solutions, each itself a
//! `List` of one `Rule[x, value]` (single-variable case here, so always
//! exactly one rule per solution).

use crate::eval::Evaluator;
use crate::expr::Expr;
use crate::mathfns::expand;
use crate::mathfns::support::{poly_coeffs, simplify_sqrt_of_integer};

pub fn dispatch_solve(args: &[Expr], ev: &Evaluator) -> Option<Expr> {
    if args.len() != 2 {
        return None;
    }
    let var = args[1].as_symbol()?;
    let (head, eq_args) = args[0].as_normal()?;
    if head.as_symbol() != Some("Equal") || eq_args.len() != 2 {
        return None;
    }
    let diff_expr = ev.eval(&Expr::plus(vec![eq_args[0].clone(), Expr::times(vec![Expr::integer(-1), eq_args[1].clone()])]));
    // Factored input like (x-2)(x-3) == 0 needs expanding before the
    // coefficients of x, x^2, ... can be read off.
    let expanded = ev.eval(&expand::expand(&diff_expr));
    let coeffs = poly_coeffs(&expanded, var)?;

    match coeffs.len() {
        // Degree 0: no x term at all, not solvable as a single-variable
        // equation in x either way (identically true or a contradiction).
        1 => None,
        2 => solve_linear(&coeffs[0], &coeffs[1], var, ev),
        3 => solve_quadratic(&coeffs[0], &coeffs[1], &coeffs[2], var, ev),
        _ => None, // degree >= 3: out of scope for this pass
    }
}

fn one_solution(var: &str, root: Expr) -> Expr {
    Expr::list(vec![Expr::list(vec![Expr::rule(Expr::symbol(var), root)])])
}

fn solve_linear(c0: &Expr, c1: &Expr, var: &str, ev: &Evaluator) -> Option<Expr> {
    if c1.is_zero() {
        return None; // degenerate: identically true or no solution, neither expressible as a root list
    }
    // c1 x + c0 = 0 -> x = -c0 / c1
    let root = ev.eval(&Expr::times(vec![Expr::integer(-1), c0.clone(), Expr::power(c1.clone(), Expr::integer(-1))]));
    Some(one_solution(var, root))
}

fn solve_quadratic(c0: &Expr, c1: &Expr, c2: &Expr, var: &str, ev: &Evaluator) -> Option<Expr> {
    if c2.is_zero() {
        return solve_linear(c0, c1, var, ev);
    }
    // Discriminant b^2 - 4ac, kept exact: Sqrt folds to an exact integer for
    // perfect squares and otherwise stays a symbolic Sqrt[...], never a float.
    let disc = ev.eval(&Expr::plus(vec![
        Expr::power(c1.clone(), Expr::integer(2)),
        Expr::times(vec![Expr::integer(-4), c2.clone(), c0.clone()]),
    ]));
    // Plain non-negative integer discriminant: pull out perfect-square
    // factors (x^2 - 2 == 0 must give Sqrt[2], not Sqrt[8]/2). Anything else
    // (symbolic coefficients, a negative discriminant) just folds through
    // the evaluator's own Sqrt rule, which only handles exact perfect squares.
    let sqrt_disc = match &disc {
        Expr::Integer(n) if *n >= 0 => simplify_sqrt_of_integer(*n),
        _ => ev.eval(&Expr::call("Sqrt", vec![disc])),
    };
    let denom = Expr::times(vec![Expr::integer(2), c2.clone()]);
    let neg_c1 = Expr::times(vec![Expr::integer(-1), c1.clone()]);

    let root_minus = ev.eval(&Expr::times(vec![
        Expr::plus(vec![neg_c1.clone(), Expr::times(vec![Expr::integer(-1), sqrt_disc.clone()])]),
        Expr::power(denom.clone(), Expr::integer(-1)),
    ]));
    let root_plus = ev.eval(&Expr::times(vec![
        Expr::plus(vec![neg_c1, sqrt_disc]),
        Expr::power(denom, Expr::integer(-1)),
    ]));

    Some(Expr::list(vec![
        Expr::list(vec![Expr::rule(Expr::symbol(var), root_minus)]),
        Expr::list(vec![Expr::rule(Expr::symbol(var), root_plus)]),
    ]))
}
