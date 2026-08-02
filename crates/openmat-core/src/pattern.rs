//! Pattern matching, first slice.
//!
//! Patterns are ordinary expressions, exactly as in Wolfram Language's own
//! `FullForm`: `_` is `Blank[]`, `_Integer` is `Blank[Integer]`, `x_` is
//! `Pattern[x, Blank[]]`, `x_Integer` is `Pattern[x, Blank[Integer]]`. Build
//! them with [`Expr::blank`], [`Expr::blank_typed`], and [`Expr::named_pattern`].
//!
//! Supported here: `Blank[]`, `Blank[head]`, named patterns, and literal
//! structural matching (a pattern with a concrete head matches an expression
//! with the same head and arity, recursing arg by arg). A name used twice in
//! one pattern must match the same subexpression both times.
//!
//! Out of scope for this pass, left for later: `BlankSequence`/`BlankNullSequence`
//! (`__`/`___`), `PatternTest`/`Condition` (`?`/`/;`), and critically
//! `Orderless`/`Flat`-aware matching against `Plus`/`Times` (matching `a_ + b_`
//! against `x + y + z` needs backtracking search over which addends fill which
//! pattern variable, and matching under commutativity generally). That belongs
//! here once it's built; the evaluator's canonicalization in `eval.rs` covers
//! numeric folding and canonical ordering only, not general Orderless matching.

use crate::expr::Expr;
use std::collections::HashMap;

pub type Bindings = HashMap<String, Expr>;

/// Try to match `pattern` against `expr`, returning the variable bindings on success.
pub fn pattern_match(pattern: &Expr, expr: &Expr) -> Option<Bindings> {
    let mut bindings = Bindings::new();
    if match_into(pattern, expr, &mut bindings) {
        Some(bindings)
    } else {
        None
    }
}

fn match_into(pattern: &Expr, expr: &Expr, bindings: &mut Bindings) -> bool {
    if let Some((head, args)) = pattern.as_normal() {
        if head.as_symbol() == Some("Blank") {
            return match args {
                [] => true,
                [Expr::Symbol(type_name)] => &expr.head_name() == type_name,
                _ => false,
            };
        }
        if head.as_symbol() == Some("Pattern") && args.len() == 2 {
            let name = match &args[0] {
                Expr::Symbol(s) => s,
                _ => return false,
            };
            if !match_into(&args[1], expr, bindings) {
                return false;
            }
            return match bindings.get(name) {
                Some(existing) => existing == expr,
                None => {
                    bindings.insert(name.clone(), expr.clone());
                    true
                }
            };
        }
        // Structural pattern: same head, same arity, each arg matches.
        return match expr.as_normal() {
            Some((e_head, e_args)) => {
                args.len() == e_args.len()
                    && match_into(head, e_head, bindings)
                    && args.iter().zip(e_args).all(|(p, a)| match_into(p, a, bindings))
            }
            None => false,
        };
    }
    pattern == expr
}

/// Rebuild `template` with every free occurrence of a bound symbol replaced
/// by its binding. Exposed beyond this module (not just used by
/// [`replace_all`]) because `eval.rs` reuses it for the same substitution
/// job when applying a matched downvalue (`f[x_] := x^2`) and when
/// instantiating a `Table` body per iteration.
pub fn substitute(template: &Expr, bindings: &Bindings) -> Expr {
    match template {
        Expr::Symbol(name) => bindings.get(name).cloned().unwrap_or_else(|| template.clone()),
        Expr::Normal { head, args } => {
            Expr::normal(substitute(head, bindings), args.iter().map(|a| substitute(a, bindings)).collect())
        }
        _ => template.clone(),
    }
}

/// Pull `(lhs, rhs)` pairs out of a `Rule[lhs, rhs]` or a `List` of rules.
fn rule_pairs(rules: &Expr) -> Vec<(Expr, Expr)> {
    if let Some((head, args)) = rules.as_normal() {
        if head.as_symbol() == Some("Rule") && args.len() == 2 {
            return vec![(args[0].clone(), args[1].clone())];
        }
        if head.as_symbol() == Some("List") {
            return args.iter().flat_map(rule_pairs).collect();
        }
    }
    Vec::new()
}

/// `expr /. rules`: apply the first matching rule at every subexpression,
/// top-down, without recursing into a freshly substituted replacement.
pub fn replace_all(expr: &Expr, rules: &Expr) -> Expr {
    let pairs = rule_pairs(rules);
    replace_all_with_pairs(expr, &pairs)
}

fn replace_all_with_pairs(expr: &Expr, pairs: &[(Expr, Expr)]) -> Expr {
    for (lhs, rhs) in pairs {
        if let Some(bindings) = pattern_match(lhs, expr) {
            return substitute(rhs, &bindings);
        }
    }
    match expr {
        Expr::Normal { head, args } => Expr::normal(
            replace_all_with_pairs(head, pairs),
            args.iter().map(|a| replace_all_with_pairs(a, pairs)).collect(),
        ),
        _ => expr.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_matches_anything() {
        let bindings = pattern_match(&Expr::blank(), &Expr::integer(5)).unwrap();
        assert!(bindings.is_empty());
        assert!(pattern_match(&Expr::blank(), &Expr::symbol("x")).is_some());
    }

    #[test]
    fn typed_blank_checks_head() {
        assert!(pattern_match(&Expr::blank_typed("Integer"), &Expr::integer(5)).is_some());
        assert!(pattern_match(&Expr::blank_typed("Integer"), &Expr::real(5.0)).is_none());
        assert!(pattern_match(&Expr::blank_typed("Plus"), &Expr::plus(vec![Expr::integer(1), Expr::symbol("x")])).is_some());
    }

    #[test]
    fn named_pattern_binds() {
        let pat = Expr::named_pattern("x", Expr::blank());
        let bindings = pattern_match(&pat, &Expr::integer(7)).unwrap();
        assert_eq!(bindings.get("x"), Some(&Expr::integer(7)));
    }

    #[test]
    fn repeated_name_requires_same_match() {
        // f[x_, x_] should match f[a, a] but not f[a, b]
        let pat = Expr::call("f", vec![Expr::named_pattern("x", Expr::blank()), Expr::named_pattern("x", Expr::blank())]);
        let same = Expr::call("f", vec![Expr::symbol("a"), Expr::symbol("a")]);
        let diff = Expr::call("f", vec![Expr::symbol("a"), Expr::symbol("b")]);
        assert!(pattern_match(&pat, &same).is_some());
        assert!(pattern_match(&pat, &diff).is_none());
    }

    #[test]
    fn replace_all_simple_substitution() {
        // {a, b, c} /. b -> x
        let list = Expr::list(vec![Expr::symbol("a"), Expr::symbol("b"), Expr::symbol("c")]);
        let rule = Expr::rule(Expr::symbol("b"), Expr::symbol("x"));
        let result = replace_all(&list, &rule);
        assert_eq!(result, Expr::list(vec![Expr::symbol("a"), Expr::symbol("x"), Expr::symbol("c")]));
    }

    #[test]
    fn replace_all_with_list_of_rules() {
        let expr = Expr::plus(vec![Expr::symbol("x"), Expr::symbol("y")]);
        let rules = Expr::list(vec![
            Expr::rule(Expr::symbol("x"), Expr::integer(1)),
            Expr::rule(Expr::symbol("y"), Expr::integer(2)),
        ]);
        let result = replace_all(&expr, &rules);
        assert_eq!(result, Expr::plus(vec![Expr::integer(1), Expr::integer(2)]));
    }

    #[test]
    fn replace_all_with_blank_pattern() {
        // f[1, x, 2] /. _Integer -> 0
        let expr = Expr::call("f", vec![Expr::integer(1), Expr::symbol("x"), Expr::integer(2)]);
        let rule = Expr::rule(Expr::blank_typed("Integer"), Expr::integer(0));
        let result = replace_all(&expr, &rule);
        assert_eq!(result, Expr::call("f", vec![Expr::integer(0), Expr::symbol("x"), Expr::integer(0)]));
    }

    #[test]
    fn replace_all_named_pattern_rebuilds_rhs() {
        // f[x_] -> x*x applied to f[3]
        let rule = Expr::rule(
            Expr::call("f", vec![Expr::named_pattern("x", Expr::blank())]),
            Expr::times(vec![Expr::symbol("x"), Expr::symbol("x")]),
        );
        let result = replace_all(&Expr::call("f", vec![Expr::integer(3)]), &rule);
        assert_eq!(result, Expr::times(vec![Expr::integer(3), Expr::integer(3)]));
    }

    #[test]
    fn replace_all_recurses_into_subexpressions() {
        // g[f[1], f[2]] /. f[x_] -> x
        let rule = Expr::rule(Expr::call("f", vec![Expr::named_pattern("x", Expr::blank())]), Expr::symbol("x"));
        let expr = Expr::call("g", vec![Expr::call("f", vec![Expr::integer(1)]), Expr::call("f", vec![Expr::integer(2)])]);
        let result = replace_all(&expr, &rule);
        assert_eq!(result, Expr::call("g", vec![Expr::integer(1), Expr::integer(2)]));
    }
}
