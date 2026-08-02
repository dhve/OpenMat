//! Math builtins beyond the core evaluator: calculus (`D`, `Integrate`),
//! algebra (`Expand`, `Solve`, `Factor`, `Simplify`). Each area lives in its
//! own submodule; `dispatch` tries the one matching the call's head and
//! returns `None` when no rule applies or the input is outside that area's
//! supported subset, which the evaluator treats as "leave the expression
//! symbolic" (see `eval.rs::apply_rules`).
//!
//! Every submodule here works purely through the crate's public surface
//! (`Expr`, `Evaluator::eval`) rather than reaching into evaluator or
//! canonicalization internals, so it stays correct across changes to those
//! modules. Numeric leaves are `Integer`/`Real`; exact fractions are
//! `Times[a, Power[b, -1]]` (see `canon.rs`); every rule here is written to
//! preserve exactness; on purely exact input none of these builtins ever
//! introduce a `Real`.

mod diff;
mod expand;
mod factor;
mod integrate;
mod simplify;
mod solve;
mod support;

use crate::eval::Evaluator;
use crate::expr::Expr;

pub fn dispatch(name: &str, args: &[Expr], ev: &Evaluator) -> Option<Expr> {
    match name {
        "D" => diff::dispatch_d(args, ev),
        "Integrate" => integrate::dispatch_integrate(args, ev),
        "Expand" => expand::dispatch_expand(args, ev),
        "Solve" => solve::dispatch_solve(args, ev),
        "Factor" => factor::dispatch_factor(args, ev),
        "Simplify" => simplify::dispatch_simplify(args, ev),
        _ => None,
    }
}
