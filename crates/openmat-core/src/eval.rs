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
//! User definitions (`Set`/`SetDelayed`, i.e. `=` and `:=`) live here too:
//! `OwnValues` (`a = 5`) and `DownValues` (`f[x_] := x^2`) are stored in
//! [`Evaluator`]'s private `definitions` table and consulted for, respectively,
//! bare symbol evaluation and unrecognized function calls. The table sits
//! behind a `RefCell` rather than requiring `&mut self`: `openmat-kernel` and
//! `openmat-solve::ndsolve` both hold a plain `&Evaluator` (the latter inside
//! an `Fn` closure invoked many times per solve step), so `eval` has to stay
//! callable through a shared reference. `Clear[f]` removes both value kinds
//! for a symbol.
//!
//! Everything else new in this pass (`Table`, `Range`, `Map`, `Length`,
//! `First`, `Rest`, `Total`, the comparison operators, `If`) is an ordinary
//! builtin dispatched from [`Evaluator::apply_rules`], the same place
//! `Plus`/`Times`/`Power` and the numeric function library already live.
//! Downvalue matching reuses [`crate::pattern::pattern_match`] and
//! [`crate::pattern::substitute`] by wrapping a call's arguments in a
//! synthetic `f[...]` on both sides, rather than duplicating the Blank/
//! Pattern matcher here.

use crate::canon::{canonicalize_plus, canonicalize_times, eval_numeric_builtin, eval_power};
use crate::expr::Expr;
use crate::pattern::{pattern_match, substitute, Bindings};
use crate::symtab::{Attribute, SymbolTable};
use std::cell::RefCell;
use std::collections::HashMap;

/// Cap on evaluation-loop iterations per subexpression. Generous for
/// anything this crate's builtins can produce (none of them oscillate), but
/// present so a future rule set with a non-terminating rewrite can't hang.
const MAX_ITERATIONS: usize = 4096;

/// One user-defined downvalue: `f[pattern_args...] := rhs` (rhs stored
/// unevaluated) or `f[pattern_args...] = rhs` (rhs already evaluated once,
/// at definition time). Matched against a call's actual arguments by
/// wrapping both the stored patterns and the actual arguments in a
/// synthetic `f[...]` call and running the ordinary pattern matcher over it.
#[derive(Clone)]
struct DownValue {
    pattern_args: Vec<Expr>,
    rhs: Expr,
}

/// Mutable state that `Set`/`SetDelayed`/`Clear` write to. Definitions are
/// matched in the order they were made, first match wins; unlike real
/// Wolfram Language, rules are not reordered by pattern specificity (see the
/// module doc in `pattern.rs` for the same scope cut on matching itself).
#[derive(Default, Clone)]
struct Definitions {
    own_values: HashMap<String, Expr>,
    down_values: HashMap<String, Vec<DownValue>>,
}

pub struct Evaluator {
    pub symtab: SymbolTable,
    definitions: RefCell<Definitions>,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator { symtab: SymbolTable::new(), definitions: RefCell::new(Definitions::default()) }
    }

    pub fn with_symtab(symtab: SymbolTable) -> Self {
        Evaluator { symtab, definitions: RefCell::new(Definitions::default()) }
    }

    /// Whether `name` carries a user definition (an OwnValue from `=` or a
    /// DownValue from `:=`). Lets callers doing free-symbol analysis (e.g.
    /// Plot/NDSolve's unbound-parameter checks) treat session-defined
    /// symbols as bound rather than stray.
    pub fn has_definition(&self, name: &str) -> bool {
        let defs = self.definitions.borrow();
        defs.own_values.contains_key(name) || defs.down_values.contains_key(name)
    }

    /// A snapshot copy: a new Evaluator carrying a clone of the current
    /// definitions. Used where an owned evaluator must move into a
    /// `Send + 'static` closure (NDSolve's ODE right-hand side) while still
    /// seeing the session's definitions at the moment of the call.
    pub fn fork(&self) -> Evaluator {
        Evaluator { symtab: SymbolTable::new(), definitions: RefCell::new(self.definitions.borrow().clone()) }
    }

    /// Evaluate `expr` to a fixed point.
    pub fn eval(&self, expr: &Expr) -> Expr {
        match expr {
            Expr::Integer(_) | Expr::Real(_) | Expr::Str(_) => expr.clone(),
            Expr::Symbol(name) => self.eval_symbol(name, expr),
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

    /// A bare symbol self-evaluates unless an `OwnValue` (`x = ...`) is on
    /// file for it, in which case it evaluates to the (further evaluated)
    /// bound value.
    fn eval_symbol(&self, name: &str, expr: &Expr) -> Expr {
        let bound = self.definitions.borrow().own_values.get(name).cloned();
        match bound {
            Some(value) if value != *expr => self.eval(&value),
            Some(value) => value,
            None => expr.clone(),
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
                exact_trig_at_pi(name, &args[0]).unwrap_or_else(|| eval_numeric_builtin(name, &args[0]))
            }
            "Less" | "Greater" | "LessEqual" | "GreaterEqual" | "Unequal" if args.len() == 2 => {
                eval_comparison(name, &args[0], &args[1])
            }
            "Set" if args.len() == 2 => self.do_set(&args[0], &args[1], false),
            "SetDelayed" if args.len() == 2 => self.do_set(&args[0], &args[1], true),
            "Clear" => {
                self.do_clear(args);
                Expr::symbol("Null")
            }
            "Table" if args.len() == 2 => self.eval_table(&args[0], &args[1]),
            "Range" => eval_range(args),
            "Map" if args.len() == 2 => self.eval_map(&args[0], &args[1]),
            "Length" if args.len() == 1 => eval_length(&args[0]),
            "First" if args.len() == 1 => eval_first(&args[0]),
            "Rest" if args.len() == 1 => eval_rest(&args[0]),
            "Total" if args.len() == 1 => eval_total(&args[0]),
            "If" if args.len() == 3 => self.eval_if(&args[0], &args[1], &args[2]),
            _ => {
                // User-defined downvalues (`f[x_] := ...`) get first refusal
                // on any name not already claimed by a builtin above. Math
                // builtins (calculus, algebra, solving) live in their own
                // module so the core loop stays small; None from either
                // means no rule fired, so the call stays symbolic.
                if let Some(result) = self.try_downvalue(name, args) {
                    return result;
                }
                match crate::mathfns::dispatch(name, args, self) {
                    Some(result) => result,
                    None => expr.clone(),
                }
            }
        }
    }

    // -- user definitions: Set, SetDelayed, Clear ---------------------------

    /// Record an `OwnValue` or `DownValue` for `lhs = rhs` / `lhs := rhs`.
    /// For the immediate form (`delayed == false`) `rhs` is evaluated once,
    /// now; for the delayed form it is stored as-is and evaluated fresh
    /// every time the definition fires. Returns the value `Set`/`SetDelayed`
    /// itself evaluates to: the assigned value for `Set` (so `a = b = 5`
    /// chains sensibly), `Null` for `SetDelayed` (nothing to hand back yet).
    fn do_set(&self, lhs: &Expr, rhs: &Expr, delayed: bool) -> Expr {
        let value = if delayed { rhs.clone() } else { self.eval(rhs) };
        match lhs {
            Expr::Symbol(name) => {
                self.definitions.borrow_mut().own_values.insert(name.clone(), value.clone());
            }
            Expr::Normal { head, args: pattern_args } => {
                if let Some(name) = head.as_symbol() {
                    let mut defs = self.definitions.borrow_mut();
                    let rules = defs.down_values.entry(name.to_string()).or_default();
                    match rules.iter_mut().find(|dv| dv.pattern_args.as_slice() == pattern_args.as_slice()) {
                        Some(existing) => existing.rhs = value.clone(),
                        None => rules.push(DownValue { pattern_args: pattern_args.clone(), rhs: value.clone() }),
                    }
                }
            }
            _ => {}
        }
        if delayed {
            Expr::symbol("Null")
        } else {
            value
        }
    }

    fn do_clear(&self, args: &[Expr]) {
        let mut defs = self.definitions.borrow_mut();
        for a in args {
            if let Expr::Symbol(name) = a {
                defs.own_values.remove(name);
                defs.down_values.remove(name);
            }
        }
    }

    /// Look up and apply a user-defined downvalue for the call `name[args...]`.
    fn try_downvalue(&self, name: &str, args: &[Expr]) -> Option<Expr> {
        let substituted = self.match_downvalue(name, args)?;
        Some(self.eval(&substituted))
    }

    fn match_downvalue(&self, name: &str, args: &[Expr]) -> Option<Expr> {
        let defs = self.definitions.borrow();
        let rules = defs.down_values.get(name)?;
        for dv in rules {
            let pattern_call = Expr::call(name, dv.pattern_args.clone());
            let actual_call = Expr::call(name, args.to_vec());
            if let Some(bindings) = pattern_match(&pattern_call, &actual_call) {
                return Some(substitute(&dv.rhs, &bindings));
            }
        }
        None
    }

    // -- structural builtins: Table, Map, If ---------------------------------

    /// `Table[body, {i, n}]` / `Table[body, {i, a, b}]`. `Table` carries
    /// `HoldAll` (see `symtab.rs`) so `body` and the iterator spec arrive
    /// here unevaluated; the iterator bounds are evaluated explicitly below
    /// (so `Table[i, {i, 1, 2+3}]` works), but the loop variable name never
    /// is, so an unrelated existing binding for that name can't leak in.
    fn eval_table(&self, body: &Expr, iter_spec: &Expr) -> Expr {
        match self.parse_iterator(iter_spec) {
            Some((var, start, end)) => {
                let items = (start..=end)
                    .map(|i| {
                        let bindings: Bindings = [(var.clone(), Expr::integer(i))].into_iter().collect();
                        self.eval(&substitute(body, &bindings))
                    })
                    .collect();
                Expr::list(items)
            }
            None => Expr::call("Table", vec![body.clone(), iter_spec.clone()]),
        }
    }

    fn parse_iterator(&self, iter_spec: &Expr) -> Option<(String, i64, i64)> {
        let (head, items) = iter_spec.as_normal()?;
        if head.as_symbol() != Some("List") {
            return None;
        }
        match items {
            [Expr::Symbol(var), n] => Some((var.clone(), 1, self.eval_to_int(n)?)),
            [Expr::Symbol(var), a, b] => Some((var.clone(), self.eval_to_int(a)?, self.eval_to_int(b)?)),
            _ => None,
        }
    }

    fn eval_to_int(&self, e: &Expr) -> Option<i64> {
        match self.eval(e) {
            Expr::Integer(n) => Some(n),
            Expr::Real(x) if x.fract() == 0.0 => Some(x as i64),
            _ => None,
        }
    }

    /// `Map[f, expr]`: apply `f` to each of `expr`'s arguments (level 1),
    /// preserving `expr`'s own head, so `Map[f, {a, b}]` is `{f[a], f[b]}`
    /// but `Map[f, g[a, b]]` is `g[f[a], f[b]]`.
    fn eval_map(&self, f: &Expr, list: &Expr) -> Expr {
        match list.as_normal() {
            Some((head, items)) => {
                let mapped: Vec<Expr> =
                    items.iter().map(|item| self.eval(&Expr::normal(f.clone(), vec![item.clone()]))).collect();
                Expr::normal(head.clone(), mapped)
            }
            None => Expr::call("Map", vec![f.clone(), list.clone()]),
        }
    }

    /// `If[cond, then, else]`. `If` carries `HoldAll`, so only the branch
    /// actually taken is ever evaluated; when `cond` does not reduce to
    /// `True`/`False`, the call stays symbolic with `cond` evaluated but
    /// both branches still held, matching Wolfram Language.
    fn eval_if(&self, cond: &Expr, then_branch: &Expr, else_branch: &Expr) -> Expr {
        let cond_val = self.eval(cond);
        if cond_val == Expr::symbol("True") {
            self.eval(then_branch)
        } else if cond_val == Expr::symbol("False") {
            self.eval(else_branch)
        } else {
            Expr::call("If", vec![cond_val, then_branch.clone(), else_branch.clone()])
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Evaluator::new()
    }
}

// ---------------------------------------------------------------------------
// Free-standing builtins that need no access to definitions: Range, Length,
// First, Rest, Total, the comparison operators.
// ---------------------------------------------------------------------------

fn eval_range(args: &[Expr]) -> Expr {
    let bounds: Option<(i64, i64)> = match args {
        [n] => as_int(n).map(|n| (1, n)),
        [a, b] => match (as_int(a), as_int(b)) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        },
        _ => None,
    };
    match bounds {
        Some((a, b)) => Expr::list((a..=b).map(Expr::integer).collect()),
        None => Expr::call("Range", args.to_vec()),
    }
}

fn as_int(e: &Expr) -> Option<i64> {
    match e {
        Expr::Integer(n) => Some(*n),
        _ => None,
    }
}

fn eval_length(e: &Expr) -> Expr {
    match e.as_normal() {
        Some((_, args)) => Expr::integer(args.len() as i64),
        None => Expr::integer(0),
    }
}

fn eval_first(e: &Expr) -> Expr {
    match e.as_normal() {
        Some((_, args)) if !args.is_empty() => args[0].clone(),
        _ => Expr::call("First", vec![e.clone()]),
    }
}

fn eval_rest(e: &Expr) -> Expr {
    match e.as_normal() {
        Some((head, args)) if !args.is_empty() => Expr::normal(head.clone(), args[1..].to_vec()),
        _ => Expr::call("Rest", vec![e.clone()]),
    }
}

/// `Total[list]` reuses [`canonicalize_plus`] (the same numeric folding
/// `Plus` gets during ordinary evaluation) rather than re-implementing
/// numeric summation.
fn eval_total(e: &Expr) -> Expr {
    match e.as_normal() {
        Some((_, args)) => canonicalize_plus(args),
        None => e.clone(),
    }
}

fn eval_comparison(name: &str, lhs: &Expr, rhs: &Expr) -> Expr {
    match (numeric_value(lhs), numeric_value(rhs)) {
        (Some(a), Some(b)) => {
            let result = match name {
                "Less" => a < b,
                "Greater" => a > b,
                "LessEqual" => a <= b,
                "GreaterEqual" => a >= b,
                "Unequal" => a != b,
                _ => unreachable!("eval_comparison called with unknown operator {name}"),
            };
            Expr::symbol(if result { "True" } else { "False" })
        }
        _ => Expr::call(name, vec![lhs.clone(), rhs.clone()]),
    }
}

fn numeric_value(e: &Expr) -> Option<f64> {
    match e {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Real(x) => Some(*x),
        _ => None,
    }
}

/// A handful of exact trig identities at the symbol `Pi` that `canon.rs`'s
/// numeric builtin library cannot know about on its own (it only special-
/// cases literal `Integer`/`Real` arguments, never a symbolic constant).
fn exact_trig_at_pi(name: &str, arg: &Expr) -> Option<Expr> {
    if arg.as_symbol() != Some("Pi") {
        return None;
    }
    match name {
        "Sin" => Some(Expr::integer(0)),
        "Cos" => Some(Expr::integer(-1)),
        "Tan" => Some(Expr::integer(0)),
        _ => None,
    }
}

/// Recursively rewrite every `Integer` leaf to the equivalent `Real`, and
/// the named constants `Pi`/`E` to their machine-precision values: the first
/// step of `N[expr]`. The caller re-evaluates the result so any builtin that
/// only folds numerically on `Real` input (see `canon.rs`) gets a chance to
/// run.
fn to_real_tree(e: &Expr) -> Expr {
    match e {
        Expr::Integer(n) => Expr::Real(*n as f64),
        Expr::Symbol(s) if s == "Pi" => Expr::Real(std::f64::consts::PI),
        Expr::Symbol(s) if s == "E" => Expr::Real(std::f64::consts::E),
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

    /// Evaluate each of `sources` in order on the *same* `Evaluator`, so
    /// definitions made by an earlier statement (`f[x_] := x^2`) are visible
    /// to a later one (`f[3]`). Returns the last result. This is the
    /// pattern a stateful session (not this crate's single-shot `eval_src`)
    /// would use.
    fn eval_session(sources: &[&str]) -> Expr {
        let ev = Evaluator::new();
        let mut result = Expr::symbol("Null");
        for src in sources {
            result = ev.eval(&parse(src).unwrap());
        }
        result
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

    // -- constants: Pi, E -----------------------------------------------------

    #[test]
    fn n_of_pi_is_numeric() {
        let e = eval_src("N[Pi]");
        match e {
            Expr::Real(x) => assert!((x - std::f64::consts::PI).abs() < 1e-12),
            other => panic!("expected Real, got {:?}", other),
        }
    }

    #[test]
    fn n_of_e_is_numeric() {
        let e = eval_src("N[E]");
        match e {
            Expr::Real(x) => assert!((x - std::f64::consts::E).abs() < 1e-12),
            other => panic!("expected Real, got {:?}", other),
        }
    }

    #[test]
    fn sin_of_pi_is_exactly_zero() {
        assert_eq!(eval_src("Sin[Pi]"), Expr::integer(0));
    }

    #[test]
    fn exp_of_one_stays_symbolic() {
        assert_eq!(eval_src("Exp[1]"), Expr::call("Exp", vec![Expr::integer(1)]));
    }

    #[test]
    fn bare_pi_stays_symbolic() {
        assert_eq!(eval_src("Pi"), Expr::symbol("Pi"));
        assert_eq!(eval_src("Pi + 1").to_string(), "1 + Pi");
    }

    // -- user definitions: Set, SetDelayed, Clear ----------------------------

    #[test]
    fn own_value_assignment_and_lookup() {
        assert_eq!(eval_session(&["a = 5", "a"]), Expr::integer(5));
        assert_eq!(eval_session(&["a = 5", "a + 1"]), Expr::integer(6));
    }

    #[test]
    fn set_returns_the_assigned_value() {
        assert_eq!(eval_src("a = 5"), Expr::integer(5));
    }

    #[test]
    fn set_delayed_downvalue_definition_and_application() {
        assert_eq!(eval_session(&["f[x_] := x^2", "f[3]"]), Expr::integer(9));
        assert_eq!(eval_session(&["f[x_] := x^2", "f[5]"]), Expr::integer(25));
    }

    #[test]
    fn set_delayed_body_reevaluates_each_call() {
        // Delayed rhs is not folded at definition time: redefining an
        // ownvalue it depends on changes future applications.
        assert_eq!(eval_session(&["c = 2", "f[x_] := c * x", "f[10]"]), Expr::integer(20));
        assert_eq!(eval_session(&["c = 2", "f[x_] := c * x", "c = 3", "f[10]"]), Expr::integer(30));
    }

    #[test]
    fn immediate_set_on_a_function_evaluates_rhs_once() {
        assert_eq!(eval_session(&["f[x_] = x^2", "f[3]"]), Expr::integer(9));
    }

    #[test]
    fn redefining_the_same_pattern_replaces_the_rule() {
        assert_eq!(eval_session(&["f[x_] := x^2", "f[x_] := x^3", "f[2]"]), Expr::integer(8));
    }

    #[test]
    fn clear_removes_both_own_and_down_values() {
        assert_eq!(eval_session(&["a = 5", "Clear[a]", "a"]), Expr::symbol("a"));
        assert_eq!(eval_session(&["f[x_] := x^2", "Clear[f]", "f[3]"]), Expr::call("f", vec![Expr::integer(3)]));
    }

    #[test]
    fn undefined_function_call_stays_symbolic() {
        assert_eq!(eval_src("g[3]"), Expr::call("g", vec![Expr::integer(3)]));
    }

    // -- structural builtins: Table, Range, Map, Length, First, Rest, Total --

    #[test]
    fn table_with_implicit_start() {
        assert_eq!(
            eval_src("Table[i^2, {i, 4}]"),
            Expr::list(vec![Expr::integer(1), Expr::integer(4), Expr::integer(9), Expr::integer(16)])
        );
    }

    #[test]
    fn table_with_explicit_start() {
        assert_eq!(eval_src("Table[i, {i, 2, 5}]"), Expr::list(vec![Expr::integer(2), Expr::integer(3), Expr::integer(4), Expr::integer(5)]));
    }

    #[test]
    fn table_iterator_name_does_not_leak_a_prior_ownvalue() {
        assert_eq!(
            eval_session(&["i = 100", "Table[i, {i, 3}]"]),
            Expr::list(vec![Expr::integer(1), Expr::integer(2), Expr::integer(3)])
        );
    }

    #[test]
    fn range_one_and_two_argument_forms() {
        assert_eq!(eval_src("Range[5]"), Expr::list(vec![1, 2, 3, 4, 5].into_iter().map(Expr::integer).collect()));
        assert_eq!(eval_src("Range[2, 5]"), Expr::list(vec![2, 3, 4, 5].into_iter().map(Expr::integer).collect()));
    }

    #[test]
    fn map_over_list_and_over_a_user_function() {
        assert_eq!(eval_src("Map[Abs, {-1, 2, -3}]"), Expr::list(vec![Expr::integer(1), Expr::integer(2), Expr::integer(3)]));
        assert_eq!(
            eval_session(&["f[x_] := x^2", "Map[f, {1, 2, 3}]"]),
            Expr::list(vec![Expr::integer(1), Expr::integer(4), Expr::integer(9)])
        );
    }

    #[test]
    fn map_infix_operator_matches_prefix_form() {
        assert_eq!(eval_src("Abs /@ {-1, 2, -3}"), eval_src("Map[Abs, {-1, 2, -3}]"));
    }

    #[test]
    fn length_first_rest_total() {
        assert_eq!(eval_src("Length[{1, 2, 3}]"), Expr::integer(3));
        assert_eq!(eval_src("Length[5]"), Expr::integer(0));
        assert_eq!(eval_src("First[{1, 2, 3}]"), Expr::integer(1));
        assert_eq!(eval_src("Rest[{1, 2, 3}]"), Expr::list(vec![Expr::integer(2), Expr::integer(3)]));
        assert_eq!(eval_src("Total[{1, 2, 3}]"), Expr::integer(6));
        assert_eq!(eval_src("Total[{1, x, 2}]").to_string(), "3 + x");
    }

    // -- comparisons and If ---------------------------------------------------

    #[test]
    fn comparisons_fold_for_numeric_args() {
        assert_eq!(eval_src("1 < 2"), Expr::symbol("True"));
        assert_eq!(eval_src("2 < 1"), Expr::symbol("False"));
        assert_eq!(eval_src("2 <= 2"), Expr::symbol("True"));
        assert_eq!(eval_src("3 > 2"), Expr::symbol("True"));
        assert_eq!(eval_src("3 >= 4"), Expr::symbol("False"));
        assert_eq!(eval_src("3 != 4"), Expr::symbol("True"));
        assert_eq!(eval_src("3 != 3"), Expr::symbol("False"));
    }

    #[test]
    fn comparisons_stay_symbolic_for_non_numeric_args() {
        assert_eq!(eval_src("a < b"), Expr::call("Less", vec![Expr::symbol("a"), Expr::symbol("b")]));
    }

    #[test]
    fn if_evaluates_only_the_taken_branch() {
        assert_eq!(eval_src("If[1 < 2, 10, 20]"), Expr::integer(10));
        assert_eq!(eval_src("If[2 < 1, 10, 20]"), Expr::integer(20));
    }

    #[test]
    fn if_with_symbolic_condition_stays_held_and_symbolic() {
        let e = eval_src("If[a < b, 10, 20]");
        assert_eq!(e, Expr::call("If", vec![Expr::call("Less", vec![Expr::symbol("a"), Expr::symbol("b")]), Expr::integer(10), Expr::integer(20)]));
    }
}
