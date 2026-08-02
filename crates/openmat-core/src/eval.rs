//! The evaluator: a fixed-point rewriter over [`Expr`], attribute-aware.
//!
//! For a `Normal` expression, one evaluation step is: evaluate the head,
//! evaluate each argument unless the head's attributes hold it (`HoldAll`
//! holds every argument, `HoldFirst` holds only the first), rebuild the
//! expression, then apply whatever builtin rule matches the (possibly new)
//! head. That step repeats until the expression stops changing or
//! `MAX_ITERATIONS` is hit, which bounds runaway rewriting instead of hanging
//! forever.
//!
//! This is a downvalue-free evaluator: there is no `Set`/`SetDelayed` yet, so
//! the only rules that ever fire are the builtins wired up in [`apply_rules`].
//! User-defined rewrite rules go through [`crate::pattern::replace_all`]
//! instead, applied on demand rather than during evaluation.

use crate::canon::{canonicalize_plus, canonicalize_times, eval_numeric_builtin, eval_power};
use crate::expr::Expr;
use crate::symtab::{Attribute, SymbolTable};

/// Cap on evaluation-loop iterations per subexpression. Generous for
/// anything this crate's builtins can produce (none of them oscillate), but
/// present so a future rule set with a non-terminating rewrite can't hang.
const MAX_ITERATIONS: usize = 4096;

pub struct Evaluator {
    pub symtab: SymbolTable,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator { symtab: SymbolTable::new() }
    }

    pub fn with_symtab(symtab: SymbolTable) -> Self {
        Evaluator { symtab }
    }

    /// Evaluate `expr` to a fixed point.
    pub fn eval(&self, expr: &Expr) -> Expr {
        match expr {
            Expr::Integer(_) | Expr::Real(_) | Expr::Str(_) | Expr::Symbol(_) => expr.clone(),
            Expr::Normal { .. } => {
                let mut current = expr.clone();
                for _ in 0..MAX_ITERATIONS {
                    let next = self.eval_step(&current);
                    if next == current {
                        return next;
                    }
                    current = next;
                }
                current
            }
        }
    }

    fn eval_step(&self, expr: &Expr) -> Expr {
        let (head, args) = match expr.as_normal() {
            Some(v) => v,
            None => return expr.clone(),
        };
        let eval_head = self.eval(head);
        let head_name = eval_head.as_symbol();
        let hold_all = head_name.map_or(false, |n| self.symtab.has_attribute(n, Attribute::HoldAll));
        let hold_first = head_name.map_or(false, |n| self.symtab.has_attribute(n, Attribute::HoldFirst));

        let new_args: Vec<Expr> = args
            .iter()
            .enumerate()
            .map(|(i, a)| if hold_all || (hold_first && i == 0) { a.clone() } else { self.eval(a) })
            .collect();

        let rebuilt = Expr::normal(eval_head, new_args);
        self.apply_rules(&rebuilt)
    }

    /// One rewrite pass over an already head/arg-evaluated `Normal` expression.
    fn apply_rules(&self, expr: &Expr) -> Expr {
        let (head, args) = match expr.as_normal() {
            Some(v) => v,
            None => return expr.clone(),
        };
        let name = match head.as_symbol() {
            Some(n) => n,
            None => return expr.clone(),
        };
        match name {
            "Plus" => canonicalize_plus(args),
            "Times" => canonicalize_times(args),
            "Power" => eval_power(args),
            "N" if args.len() == 1 => self.eval(&to_real_tree(&args[0])),
            "Sin" | "Cos" | "Tan" | "Exp" | "Log" | "Sqrt" | "Abs" if args.len() == 1 => {
                eval_numeric_builtin(name, &args[0])
            }
            _ => {
                // Math builtins (calculus, algebra, solving) live in their own
                // module so the core loop stays small; None means no rule fired.
                match crate::mathfns::dispatch(name, args, self) {
                    Some(result) => result,
                    None => expr.clone(),
                }
            }
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Evaluator::new()
    }
}

/// Recursively rewrite every `Integer` leaf to the equivalent `Real`, the
/// first step of `N[expr]`: the caller re-evaluates the result so any
/// builtin that only folds numerically on `Real` input (see `canon.rs`) gets
/// a chance to run.
fn to_real_tree(e: &Expr) -> Expr {
    match e {
        Expr::Integer(n) => Expr::Real(*n as f64),
        Expr::Real(_) | Expr::Symbol(_) | Expr::Str(_) => e.clone(),
        Expr::Normal { head, args } => Expr::normal(to_real_tree(head), args.iter().map(to_real_tree).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn eval_src(src: &str) -> Expr {
        let e = parse(src).unwrap();
        Evaluator::new().eval(&e)
    }

    #[test]
    fn atoms_are_self_evaluating() {
        let ev = Evaluator::new();
        assert_eq!(ev.eval(&Expr::integer(5)), Expr::integer(5));
        assert_eq!(ev.eval(&Expr::symbol("x")), Expr::symbol("x"));
    }

    #[test]
    fn plus_and_times_canonicalize_through_full_eval() {
        assert_eq!(eval_src("2 + x + 3"), Expr::plus(vec![Expr::integer(5), Expr::symbol("x")]));
        assert_eq!(eval_src("x + 2 x"), Expr::times(vec![Expr::integer(3), Expr::symbol("x")]));
        assert_eq!(eval_src("2 * 3 * x"), Expr::times(vec![Expr::integer(6), Expr::symbol("x")]));
    }

    #[test]
    fn nested_arithmetic_evaluates_bottom_up() {
        // (1 + 1) * (2 + 2) -> 8
        assert_eq!(eval_src("(1 + 1) * (2 + 2)"), Expr::integer(8));
    }

    #[test]
    fn power_and_sqrt_exact_cases() {
        assert_eq!(eval_src("2^10"), Expr::integer(1024));
        assert_eq!(eval_src("Sqrt[4]"), Expr::integer(2));
        assert_eq!(eval_src("Sin[0]"), Expr::integer(0));
    }

    #[test]
    fn hold_all_prevents_argument_evaluation() {
        // Hold[1 + 1] should not fold the sum inside.
        let e = Expr::call("Hold", vec![Expr::plus(vec![Expr::integer(1), Expr::integer(1)])]);
        let result = Evaluator::new().eval(&e);
        assert_eq!(result, Expr::call("Hold", vec![Expr::plus(vec![Expr::integer(1), Expr::integer(1)])]));
        assert_eq!(result.to_string(), "Hold[1 + 1]");
    }

    #[test]
    fn n_forces_numeric_evaluation_recursively() {
        // N[1 + Sqrt[4]] -> 3. (integers become reals, then re-evaluate)
        let e = eval_src("N[1 + Sqrt[4]]");
        assert_eq!(e, Expr::real(3.0));
    }

    #[test]
    fn n_evaluates_transcendental_functions() {
        let e = eval_src("N[Sqrt[2]]");
        match e {
            Expr::Real(x) => assert!((x - std::f64::consts::SQRT_2).abs() < 1e-12),
            other => panic!("expected Real, got {:?}", other),
        }
    }

    #[test]
    fn symbolic_function_call_stays_unevaluated() {
        assert_eq!(eval_src("f[x, y]"), Expr::call("f", vec![Expr::symbol("x"), Expr::symbol("y")]));
    }

    #[test]
    fn division_by_integer_reduces_exactly() {
        assert_eq!(eval_src("4/2"), Expr::integer(2));
        assert_eq!(eval_src("1/2").to_string(), "1/2");
    }

    #[test]
    fn overflow_promotes_to_real() {
        let e = eval_src("1000000000000 * 1000000000000");
        assert!(matches!(e, Expr::Real(_)));
    }

    #[test]
    fn pendulum_terms_from_architecture_md_stay_symbolic() {
        // x''[t] + c x'[t] + Sin[x[t]]: nothing numeric to fold, so evaluation
        // only re-sorts the sum into canonical (string) order.
        let e = eval_src("x''[t] + c x'[t] + Sin[x[t]]");
        assert_eq!(e.to_string(), "Sin[x[t]] + c*x'[t] + x''[t]");
    }
}
