//! `Simplify[expr]`: a cheap, honest subset. Evaluate, try `Expand`, and
//! keep whichever printed form is shorter. This is nowhere near real
//! `Simplify`'s search over rewrite rules (see `specs/01-core-language.md`
//! section 3.1), but it never claims to be: it always returns a form that is
//! genuinely equal to the input (both candidates are equivalent rewrites of
//! the same evaluated expression), just possibly not the simplest one.

use crate::eval::Evaluator;
use crate::expr::Expr;
use crate::mathfns::expand;

pub fn dispatch_simplify(args: &[Expr], ev: &Evaluator) -> Option<Expr> {
    if args.len() != 1 {
        return None;
    }
    let evaluated = ev.eval(&args[0]);
    let expanded = ev.eval(&expand::expand(&evaluated));
    let best = if expanded.to_string().len() < evaluated.to_string().len() { expanded } else { evaluated };
    Some(best)
}
