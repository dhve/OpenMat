//! Math builtins beyond the core evaluator: calculus (D, Integrate), algebra
//! (Expand, Solve), and structural functions (Table, Map, Range). Each area
//! lives in its own submodule; `dispatch` tries them in order and returns
//! `None` when no rule applies, which the evaluator treats as "leave the
//! expression symbolic".

use crate::eval::Evaluator;
use crate::expr::Expr;

pub fn dispatch(_name: &str, _args: &[Expr], _ev: &Evaluator) -> Option<Expr> {
    None
}
