//! Numeric folding and canonicalization for `Plus`, `Times`, and `Power`,
//! plus the small numeric builtin library (`Sin`, `Cos`, `Tan`, `Exp`, `Log`,
//! `Sqrt`, `Abs`).
//!
//! There is no `Rational` variant on [`Expr`]; exact fractions are
//! represented the same way Wolfram Language's own `FullForm` shows them,
//! `Times[num, Power[den, -1]]`, and integer overflow anywhere in this module
//! promotes to `f64` rather than growing to a bignum. Both are deliberate
//! scope cuts for this first pass (see the crate root docs); they are
//! internally consistent and exact for anything that fits in an `i64`.

use crate::expr::Expr;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Shared numeric helpers
// ---------------------------------------------------------------------------

fn add_numeric(a: &Expr, b: &Expr) -> Expr {
    match (a, b) {
        (Expr::Integer(x), Expr::Integer(y)) => match x.checked_add(*y) {
            Some(s) => Expr::Integer(s),
            None => Expr::Real(*x as f64 + *y as f64),
        },
        (Expr::Integer(x), Expr::Real(y)) | (Expr::Real(y), Expr::Integer(x)) => Expr::Real(*x as f64 + y),
        (Expr::Real(x), Expr::Real(y)) => Expr::Real(x + y),
        _ => panic!("add_numeric called with a non-numeric operand"),
    }
}

fn gcd_i64(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    if a == 0 {
        1
    } else {
        a
    }
}

fn checked_ipow(base: i64, exp: i64) -> Option<i64> {
    debug_assert!(exp >= 0);
    let mut result: i64 = 1;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = result.checked_mul(b)?;
        }
        e >>= 1;
        if e > 0 {
            b = b.checked_mul(b)?;
        }
    }
    Some(result)
}

/// Sort key used for canonical (Orderless) ordering: numeric atoms sort
/// first, everything else follows in the order given by its own `InputForm`
/// text. This is a deliberate simplification of Wolfram's actual `Order[]`
/// algorithm, chosen because it is simple, deterministic, and gives sensible
/// results (`5 + x`, `x + x^2`) without needing a full canonical-ordering
/// implementation, which is out of scope for this pass.
fn canonical_key(e: &Expr) -> (u8, String) {
    if e.is_numeric() {
        (0, String::new())
    } else {
        (1, e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Plus
// ---------------------------------------------------------------------------

/// Flatten nested `Plus`, fold numeric terms together, collect like terms
/// (`2 + x + 3` becomes `5 + x`; `x + 2 x` becomes `3 x`), and sort the result
/// into canonical order.
pub fn canonicalize_plus(args: &[Expr]) -> Expr {
    let mut flat = Vec::new();
    for a in args {
        if a.has_head("Plus") {
            let (_, sub_args) = a.as_normal().unwrap();
            flat.extend(sub_args.iter().cloned());
        } else {
            flat.push(a.clone());
        }
    }

    let mut numeric_sum: Option<Expr> = None;
    // (canonical-string key, the term with its coefficient stripped, accumulated coefficient)
    let mut groups: Vec<(String, Expr, Expr)> = Vec::new();

    for term in flat {
        if term.is_numeric() {
            numeric_sum = Some(match numeric_sum {
                None => term,
                Some(s) => add_numeric(&s, &term),
            });
            continue;
        }
        let (coeff, rest) = strip_coefficient(&term);
        let key = rest.to_string();
        match groups.iter_mut().find(|(k, _, _)| *k == key) {
            Some(entry) => entry.2 = add_numeric(&entry.2, &coeff),
            None => groups.push((key, rest, coeff)),
        }
    }

    let mut terms: Vec<Expr> = Vec::new();
    for (_, rest, coeff) in groups {
        if coeff.is_zero() {
            continue;
        }
        if coeff.is_one() {
            terms.push(rest);
        } else {
            terms.push(canonicalize_times(&[coeff, rest]));
        }
    }
    terms.sort_by(|a, b| canonical_key(a).cmp(&canonical_key(b)));

    let mut result = Vec::new();
    if let Some(n) = numeric_sum {
        if !n.is_zero() || terms.is_empty() {
            result.push(n);
        }
    }
    result.extend(terms);

    match result.len() {
        0 => Expr::integer(0),
        1 => result.into_iter().next().unwrap(),
        _ => Expr::plus(result),
    }
}

/// Split `coeff * rest` (a `Times` with a leading numeral) into its pieces;
/// anything else is treated as `1 * itself`.
fn strip_coefficient(term: &Expr) -> (Expr, Expr) {
    if let Some((head, args)) = term.as_normal() {
        if head.as_symbol() == Some("Times") && args.first().map_or(false, |a| a.is_numeric()) {
            let coeff = args[0].clone();
            let rest_factors = args[1..].to_vec();
            let rest = match rest_factors.len() {
                1 => rest_factors.into_iter().next().unwrap(),
                _ => Expr::times(rest_factors),
            };
            return (coeff, rest);
        }
    }
    (Expr::integer(1), term.clone())
}

// ---------------------------------------------------------------------------
// Times
// ---------------------------------------------------------------------------

/// `Power[Integer(d), Integer(-1)]`: the canonical shape [`eval_power`] leaves
/// an exact integer reciprocal in.
fn integer_reciprocal(e: &Expr) -> Option<i64> {
    let (head, args) = e.as_normal()?;
    if head.as_symbol() == Some("Power") && args.len() == 2 {
        if let (Expr::Integer(d), Expr::Integer(-1)) = (&args[0], &args[1]) {
            return Some(*d);
        }
    }
    None
}

/// Flatten nested `Times`, fold the numeric coefficient (as an exact
/// fraction where possible), combine same-base powers when both exponents
/// are numeric literals, and sort into canonical order.
pub fn canonicalize_times(args: &[Expr]) -> Expr {
    let mut flat = Vec::new();
    for a in args {
        if a.has_head("Times") {
            let (_, sub_args) = a.as_normal().unwrap();
            flat.extend(sub_args.iter().cloned());
        } else {
            flat.push(a.clone());
        }
    }

    let mut num: i64 = 1;
    let mut den: i64 = 1;
    let mut real_coeff: Option<f64> = None;
    let mut symbolic: Vec<Expr> = Vec::new();

    for factor in flat {
        match &factor {
            Expr::Integer(n) => match real_coeff {
                Some(r) => real_coeff = Some(r * (*n as f64)),
                None => match num.checked_mul(*n) {
                    Some(p) => num = p,
                    None => real_coeff = Some((num as f64 / den as f64) * (*n as f64)),
                },
            },
            Expr::Real(x) => {
                let base = real_coeff.unwrap_or(num as f64 / den as f64);
                real_coeff = Some(base * x);
            }
            _ => {
                if let Some(d) = integer_reciprocal(&factor) {
                    if d == 0 {
                        symbolic.push(factor);
                    } else {
                        match real_coeff {
                            Some(r) => real_coeff = Some(r / d as f64),
                            None => match den.checked_mul(d) {
                                Some(nd) => den = nd,
                                None => real_coeff = Some((num as f64 / den as f64) / d as f64),
                            },
                        }
                    }
                } else {
                    symbolic.push(factor);
                }
            }
        }
    }

    if real_coeff == Some(0.0) || (real_coeff.is_none() && num == 0) {
        return Expr::integer(0);
    }

    let (leading, extra_factor): (Option<Expr>, Option<Expr>) = if let Some(r) = real_coeff {
        (Some(Expr::real(r)), None)
    } else {
        let g = gcd_i64(num, den);
        let mut n = num / g;
        let mut d = den / g;
        if d < 0 {
            n = -n;
            d = -d;
        }
        let lead = if n == 1 { None } else { Some(Expr::integer(n)) };
        let extra = if d == 1 { None } else { Some(Expr::power(Expr::integer(d), Expr::integer(-1))) };
        (lead, extra)
    };

    // Group remaining symbolic factors by base, combining exponents when
    // both are numeric literals (x*x -> x^2). Factors with a symbolic
    // exponent, or whose base collides with a differently-exponented
    // symbolic power, are left distinct: combining them is not generally
    // valid without more care than this first pass takes (see module docs
    // in pattern.rs about Orderless/Flat matching being out of scope).
    let mut grouped: HashMap<String, (Expr, Expr)> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut passthrough: Vec<Expr> = Vec::new();

    for factor in symbolic {
        if let Some((head, pargs)) = factor.as_normal() {
            if head.as_symbol() == Some("Power") && pargs.len() == 2 && pargs[1].is_numeric() {
                let base = pargs[0].clone();
                let exp = pargs[1].clone();
                let key = base.to_string();
                match grouped.get_mut(&key) {
                    Some((_, e)) => *e = add_numeric(e, &exp),
                    None => {
                        order.push(key.clone());
                        grouped.insert(key, (base, exp));
                    }
                }
                continue;
            }
            if head.as_symbol() == Some("Power") {
                // symbolic exponent: not safe to combine automatically
                passthrough.push(factor);
                continue;
            }
        }
        let key = factor.to_string();
        match grouped.get_mut(&key) {
            Some((_, e)) => *e = add_numeric(e, &Expr::integer(1)),
            None => {
                order.push(key.clone());
                grouped.insert(key, (factor, Expr::integer(1)));
            }
        }
    }

    let mut factor_list: Vec<Expr> = Vec::new();
    for key in order {
        let (base, exp) = grouped.remove(&key).unwrap();
        if exp.is_zero() {
            continue;
        } else if exp.is_one() {
            factor_list.push(base);
        } else {
            factor_list.push(Expr::power(base, exp));
        }
    }
    factor_list.extend(passthrough);
    if let Some(e) = extra_factor {
        factor_list.push(e);
    }
    factor_list.sort_by(|a, b| canonical_key(a).cmp(&canonical_key(b)));

    let mut result: Vec<Expr> = Vec::new();
    if let Some(l) = leading {
        result.push(l);
    }
    result.extend(factor_list);

    match result.len() {
        0 => Expr::integer(1),
        1 => result.into_iter().next().unwrap(),
        _ => Expr::times(result),
    }
}

// ---------------------------------------------------------------------------
// Power
// ---------------------------------------------------------------------------

/// Numeric folding for `Power[base, exp]`. Integer bases with a negative
/// integer exponent are left in the canonical reciprocal shape
/// `Power[base^|exp|, -1]` (sign normalized onto a `Times[-1, ...]` wrapper
/// when needed) rather than folded to a float, so `1/2`, `2^-3`, etc. stay
/// exact; [`canonicalize_times`] knows how to fold that shape further
/// (`4^-1 * 4 -> 1`) when it appears inside a product.
pub fn eval_power(args: &[Expr]) -> Expr {
    if args.len() != 2 {
        return Expr::normal(Expr::symbol("Power"), args.to_vec());
    }
    let base = &args[0];
    let exp = &args[1];

    if let Expr::Integer(0) = exp {
        return Expr::integer(1);
    }
    if let Expr::Integer(1) = base {
        return Expr::integer(1);
    }
    if let Expr::Integer(1) = exp {
        return base.clone();
    }

    // Distribute a reciprocal over a product: Power[Times[a, b], -1] becomes
    // Times[Power[a, -1], Power[b, -1]], so the two ways a quotient can parse
    // (a/b/c vs a/(b*c)) converge to one canonical form. The evaluator's
    // fixed-point loop re-canonicalizes the resulting product.
    if *exp == Expr::Integer(-1) {
        if let Expr::Normal { head, args: factors } = base {
            if head.as_symbol() == Some("Times") {
                let recips = factors
                    .iter()
                    .map(|f| Expr::normal(Expr::symbol("Power"), vec![f.clone(), Expr::integer(-1)]))
                    .collect();
                return Expr::normal(Expr::symbol("Times"), recips);
            }
        }
    }

    match (base, exp) {
        (Expr::Real(b), Expr::Integer(e)) => Expr::real(b.powi(*e as i32)),
        (Expr::Real(b), Expr::Real(e)) => Expr::real(b.powf(*e)),
        (Expr::Integer(b), Expr::Real(e)) => Expr::real((*b as f64).powf(*e)),
        (Expr::Integer(b), Expr::Integer(e)) => eval_integer_power(*b, *e),
        _ => Expr::normal(Expr::symbol("Power"), vec![base.clone(), exp.clone()]),
    }
}

fn eval_integer_power(base: i64, exp: i64) -> Expr {
    if exp >= 0 {
        match checked_ipow(base, exp) {
            Some(v) => Expr::integer(v),
            None => Expr::real((base as f64).powi(exp as i32)),
        }
    } else if base == 0 {
        // Division by zero: leave symbolic rather than fail.
        Expr::normal(Expr::symbol("Power"), vec![Expr::integer(base), Expr::integer(exp)])
    } else {
        match checked_ipow(base, -exp) {
            Some(1) => Expr::integer(1),
            Some(-1) => Expr::integer(-1),
            Some(v) if v > 0 => Expr::power(Expr::integer(v), Expr::integer(-1)),
            Some(v) => Expr::times(vec![Expr::integer(-1), Expr::power(Expr::integer(-v), Expr::integer(-1))]),
            None => Expr::real((base as f64).powi(exp as i32)),
        }
    }
}

// ---------------------------------------------------------------------------
// Numeric builtins: Sin, Cos, Tan, Exp, Log, Sqrt, Abs
// ---------------------------------------------------------------------------

/// Evaluate a one-argument numeric builtin. `Real` arguments always fold
/// numerically; `Integer` arguments fold only for exact cases (`Sin[0] -> 0`,
/// perfect squares under `Sqrt`, `Abs` always); anything else is left
/// symbolic (e.g. `Sin[3]` stays `Sin[3]`, `Sin[x]` stays `Sin[x]`).
pub fn eval_numeric_builtin(name: &str, arg: &Expr) -> Expr {
    match arg {
        Expr::Real(x) => Expr::real(apply_real(name, *x)),
        Expr::Integer(n) => eval_integer_exact_case(name, *n).unwrap_or_else(|| Expr::call(name, vec![arg.clone()])),
        _ => Expr::call(name, vec![arg.clone()]),
    }
}

fn apply_real(name: &str, x: f64) -> f64 {
    match name {
        "Sin" => x.sin(),
        "Cos" => x.cos(),
        "Tan" => x.tan(),
        "Exp" => x.exp(),
        "Log" => x.ln(),
        "Sqrt" => x.sqrt(),
        "Abs" => x.abs(),
        _ => unreachable!("eval_numeric_builtin called with unknown builtin {}", name),
    }
}

fn eval_integer_exact_case(name: &str, n: i64) -> Option<Expr> {
    match name {
        "Sin" if n == 0 => Some(Expr::integer(0)),
        "Cos" if n == 0 => Some(Expr::integer(1)),
        "Tan" if n == 0 => Some(Expr::integer(0)),
        "Exp" if n == 0 => Some(Expr::integer(1)),
        "Log" if n == 1 => Some(Expr::integer(0)),
        "Sqrt" => integer_sqrt(n).map(Expr::integer),
        "Abs" => Some(Expr::integer(n.abs())),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plus_folds_numerics_and_collects_like_terms() {
        // 2 + x + 3 -> 5 + x
        let e = canonicalize_plus(&[Expr::integer(2), Expr::symbol("x"), Expr::integer(3)]);
        assert_eq!(e, Expr::plus(vec![Expr::integer(5), Expr::symbol("x")]));

        // x + 2x -> 3x
        let e2 = canonicalize_plus(&[Expr::symbol("x"), Expr::times(vec![Expr::integer(2), Expr::symbol("x")])]);
        assert_eq!(e2, Expr::times(vec![Expr::integer(3), Expr::symbol("x")]));

        // x - x -> 0
        let e3 = canonicalize_plus(&[Expr::symbol("x"), Expr::times(vec![Expr::integer(-1), Expr::symbol("x")])]);
        assert_eq!(e3, Expr::integer(0));
    }

    #[test]
    fn plus_flattens_nested() {
        let inner = Expr::plus(vec![Expr::symbol("a"), Expr::symbol("b")]);
        let e = canonicalize_plus(&[inner, Expr::symbol("c")]);
        assert_eq!(e, Expr::plus(vec![Expr::symbol("a"), Expr::symbol("b"), Expr::symbol("c")]));
    }

    #[test]
    fn times_folds_numerics_and_combines_powers() {
        // 2 * 3 * x -> 6 x
        let e = canonicalize_times(&[Expr::integer(2), Expr::integer(3), Expr::symbol("x")]);
        assert_eq!(e, Expr::times(vec![Expr::integer(6), Expr::symbol("x")]));

        // x * x -> x^2
        let e2 = canonicalize_times(&[Expr::symbol("x"), Expr::symbol("x")]);
        assert_eq!(e2, Expr::power(Expr::symbol("x"), Expr::integer(2)));
    }

    #[test]
    fn times_zero_short_circuits() {
        let e = canonicalize_times(&[Expr::integer(0), Expr::symbol("x")]);
        assert_eq!(e, Expr::integer(0));
    }

    #[test]
    fn exact_fraction_folds_when_it_divides_evenly() {
        // 4 * (2^-1) -> 2
        let e = canonicalize_times(&[Expr::integer(4), Expr::power(Expr::integer(2), Expr::integer(-1))]);
        assert_eq!(e, Expr::integer(2));
    }

    #[test]
    fn exact_fraction_stays_symbolic_otherwise() {
        // 1 * (2^-1) -> 1/2, represented as Power[2,-1]
        let e = canonicalize_times(&[Expr::integer(1), Expr::power(Expr::integer(2), Expr::integer(-1))]);
        assert_eq!(e, Expr::power(Expr::integer(2), Expr::integer(-1)));
        assert_eq!(e.to_string(), "1/2");
    }

    #[test]
    fn power_numeric_folding() {
        assert_eq!(eval_power(&[Expr::integer(2), Expr::integer(10)]), Expr::integer(1024));
        assert_eq!(eval_power(&[Expr::real(2.0), Expr::integer(3)]), Expr::real(8.0));
        assert_eq!(eval_power(&[Expr::symbol("x"), Expr::integer(0)]), Expr::integer(1));
        assert_eq!(eval_power(&[Expr::symbol("x"), Expr::integer(1)]), Expr::symbol("x"));
    }

    #[test]
    fn power_negative_integer_exponent_stays_exact() {
        // 2^-1 -> Power[2,-1], displays as 1/2
        let e = eval_power(&[Expr::integer(2), Expr::integer(-1)]);
        assert_eq!(e, Expr::power(Expr::integer(2), Expr::integer(-1)));
        assert_eq!(e.to_string(), "1/2");
    }

    #[test]
    fn power_overflow_promotes_to_real() {
        let e = eval_power(&[Expr::integer(10), Expr::integer(30)]);
        assert!(matches!(e, Expr::Real(_)));
    }

    #[test]
    fn numeric_builtins_exact_cases() {
        assert_eq!(eval_numeric_builtin("Sin", &Expr::integer(0)), Expr::integer(0));
        assert_eq!(eval_numeric_builtin("Cos", &Expr::integer(0)), Expr::integer(1));
        assert_eq!(eval_numeric_builtin("Sqrt", &Expr::integer(4)), Expr::integer(2));
        assert_eq!(eval_numeric_builtin("Sqrt", &Expr::integer(9)), Expr::integer(3));
        assert_eq!(eval_numeric_builtin("Abs", &Expr::integer(-5)), Expr::integer(5));
    }

    #[test]
    fn numeric_builtins_real_arg_folds() {
        let e = eval_numeric_builtin("Sin", &Expr::real(0.0));
        assert_eq!(e, Expr::real(0.0));
    }

    #[test]
    fn numeric_builtins_leave_symbolic_input_alone() {
        assert_eq!(eval_numeric_builtin("Sin", &Expr::symbol("x")), Expr::call("Sin", vec![Expr::symbol("x")]));
        assert_eq!(eval_numeric_builtin("Sin", &Expr::integer(3)), Expr::call("Sin", vec![Expr::integer(3)]));
    }
}
