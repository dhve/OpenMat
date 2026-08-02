//! Small helpers shared across the calculus/algebra submodules: checking
//! whether an expression depends on a given variable, and pulling the linear
//! form `a*x + b` out of an expression when it has one. Both are used by
//! `diff`, `integrate`, `solve`, and `factor`.

use crate::expr::Expr;

/// True if `var` appears anywhere in `e`, as a bare `Symbol` leaf. Function
/// heads (`Sin`, `Plus`, ...) are themselves `Symbol` nodes in head position,
/// but callers only care about variable occurrences in argument position, so
/// this deliberately does not recurse into the head of a `Normal` node.
pub fn depends_on(e: &Expr, var: &str) -> bool {
    match e {
        Expr::Symbol(s) => s == var,
        Expr::Integer(_) | Expr::Real(_) | Expr::Str(_) => false,
        // Deliberately does not look at `head`: for an ordinary call like
        // `Sin[x]` the head is `Symbol("Sin")`, which is not a variable
        // occurrence even if `var` happened to be named "Sin".
        Expr::Normal { args, .. } => args.iter().any(|a| depends_on(a, var)),
    }
}

/// Build `Plus[terms...]`, collapsing away literal zeros and the
/// zero/one/many-term cases, without needing a full evaluator pass. The
/// coefficient-extraction helpers below (`linear_form`, `poly_coeffs`) use
/// this instead of the raw [`Expr::plus`] constructor so their results are
/// already-clean literals (`Integer(3)`, not `Plus[0, 3]`) that the callers
/// in `solve.rs`/`factor.rs` can pattern-match on directly with [`int_val`]
/// before any evaluator ever sees them.
fn simple_sum(terms: Vec<Expr>) -> Expr {
    let kept: Vec<Expr> = terms.into_iter().filter(|t| !t.is_zero()).collect();
    match kept.len() {
        0 => Expr::integer(0),
        1 => kept.into_iter().next().unwrap(),
        _ => Expr::plus(kept),
    }
}

/// Build `Times[factors...]`, collapsing away literal ones (and short
/// circuiting to `0` on a literal zero factor), for the same reason as
/// [`simple_sum`].
fn simple_product(factors: Vec<Expr>) -> Expr {
    if factors.iter().any(|f| f.is_zero()) {
        return Expr::integer(0);
    }
    let kept: Vec<Expr> = factors.into_iter().filter(|f| !f.is_one()).collect();
    match kept.len() {
        0 => Expr::integer(1),
        1 => kept.into_iter().next().unwrap(),
        _ => Expr::times(kept),
    }
}

/// Collect every bare `Symbol` that appears in argument position (never head
/// position, for the same reason as [`depends_on`]) into `set`.
pub fn collect_symbols(e: &Expr, set: &mut std::collections::HashSet<String>) {
    match e {
        Expr::Symbol(s) => {
            set.insert(s.clone());
        }
        Expr::Normal { args, .. } => {
            for a in args {
                collect_symbols(a, set);
            }
        }
        _ => {}
    }
}

/// The single distinct symbol appearing in `e`, if there is exactly one.
pub fn only_symbol(e: &Expr) -> Option<String> {
    let mut set = std::collections::HashSet::new();
    collect_symbols(e, &mut set);
    if set.len() == 1 {
        set.into_iter().next()
    } else {
        None
    }
}

/// Decompose `e` as `a*var + b` with `a`, `b` free of `var`, when `e` is
/// actually linear in `var`. Used for the "simple linear substitution" cases
/// in `Integrate` (`Sin[a x + b]`, `Exp[a x + b]`, `(a x + b)^n`, ...).
///
/// Handles `var` itself, `Plus` of linear/constant terms, and `Times` with at
/// most one factor equal to `var` (anything nonlinear in `var`, like `x^2` or
/// `x*Sin[x]` as a factor, is rejected by returning `None`).
pub fn linear_form(e: &Expr, var: &str) -> Option<(Expr, Expr)> {
    if !depends_on(e, var) {
        return Some((Expr::integer(0), e.clone()));
    }
    match e {
        Expr::Symbol(s) if s == var => Some((Expr::integer(1), Expr::integer(0))),
        Expr::Normal { head, args } if head.as_symbol() == Some("Plus") => {
            let mut a_terms = Vec::new();
            let mut b_terms = Vec::new();
            for t in args {
                let (a, b) = linear_form(t, var)?;
                a_terms.push(a);
                b_terms.push(b);
            }
            Some((simple_sum(a_terms), simple_sum(b_terms)))
        }
        Expr::Normal { head, args } if head.as_symbol() == Some("Times") => {
            let mut coeff_factors = Vec::new();
            let mut saw_var = false;
            for f in args {
                if !depends_on(f, var) {
                    coeff_factors.push(f.clone());
                } else if f.as_symbol() == Some(var) && !saw_var {
                    saw_var = true;
                } else {
                    return None;
                }
            }
            if !saw_var {
                return None;
            }
            Some((simple_product(coeff_factors), Expr::integer(0)))
        }
        _ => None,
    }
}

/// `Some(n)` when `e` is a plain `Integer(n)`.
pub fn int_val(e: &Expr) -> Option<i64> {
    match e {
        Expr::Integer(n) => Some(*n),
        _ => None,
    }
}

/// `(degree, coefficient)` for a single additive term, when it is a plain
/// monomial in `var`: a constant, `var` itself, `var^k` for a non-negative
/// integer `k`, or a `Times` of one such power with constant factors.
/// Anything else (`var` inside a function call, a negative or fractional
/// power of `var`, two separate `var` factors, ...) returns `None`, which
/// [`poly_coeffs`] treats as "not a polynomial" and bails out on.
fn term_degree_coeff(term: &Expr, var: &str) -> Option<(i64, Expr)> {
    if !depends_on(term, var) {
        return Some((0, term.clone()));
    }
    match term {
        Expr::Symbol(s) if s == var => Some((1, Expr::integer(1))),
        Expr::Normal { head, args } if head.as_symbol() == Some("Power") && args.len() == 2 => {
            var_power_degree(&args[0], &args[1], var).map(|k| (k, Expr::integer(1)))
        }
        Expr::Normal { head, args } if head.as_symbol() == Some("Times") => {
            let mut coeff_factors = Vec::new();
            let mut degree = 0i64;
            let mut saw_var = false;
            for f in args {
                if !depends_on(f, var) {
                    coeff_factors.push(f.clone());
                    continue;
                }
                if f.as_symbol() == Some(var) {
                    degree += 1;
                    saw_var = true;
                    continue;
                }
                if let Some((h, a)) = f.as_normal() {
                    if h.as_symbol() == Some("Power") && a.len() == 2 {
                        if let Some(k) = var_power_degree(&a[0], &a[1], var) {
                            degree += k;
                            saw_var = true;
                            continue;
                        }
                    }
                }
                return None;
            }
            if !saw_var {
                return None;
            }
            Some((degree, simple_product(coeff_factors)))
        }
        _ => None,
    }
}

/// The exponent `k` when `base^exp` is `var^k` for a non-negative integer `k`.
fn var_power_degree(base: &Expr, exp: &Expr, var: &str) -> Option<i64> {
    if base.as_symbol() != Some(var) {
        return None;
    }
    match exp {
        Expr::Integer(k) if *k >= 0 => Some(*k),
        _ => None,
    }
}

/// Read `expr` (already `Plus`-flattened, or a single term) as a univariate
/// polynomial in `var`, returning `[c0, c1, c2, ...]` such that
/// `expr == c0 + c1*var + c2*var^2 + ...`. `None` when `expr` isn't a
/// polynomial in `var` at all (`var` inside a `Sin`, a negative power of
/// `var`, and so on).
pub fn poly_coeffs(expr: &Expr, var: &str) -> Option<Vec<Expr>> {
    let terms: Vec<Expr> = match expr.as_normal() {
        Some((h, a)) if h.as_symbol() == Some("Plus") => a.to_vec(),
        _ => vec![expr.clone()],
    };
    let mut by_degree: std::collections::HashMap<i64, Vec<Expr>> = std::collections::HashMap::new();
    let mut max_degree = 0i64;
    for t in terms {
        let (deg, coeff) = term_degree_coeff(&t, var)?;
        max_degree = max_degree.max(deg);
        by_degree.entry(deg).or_default().push(coeff);
    }
    let mut coeffs = Vec::with_capacity((max_degree + 1) as usize);
    for d in 0..=max_degree {
        coeffs.push(by_degree.remove(&d).map(simple_sum).unwrap_or_else(|| Expr::integer(0)));
    }
    Some(coeffs)
}

/// `Sqrt[n]` for a non-negative integer `n`, with the largest perfect-square
/// factor pulled out (`Sqrt[8]` -> `2 Sqrt[2]`, not left as `Sqrt[8]`).
/// `eval_numeric_builtin` in `canon.rs` only folds `Sqrt` exactly when the
/// whole argument is a perfect square; `Solve`'s quadratic formula needs the
/// partial-factoring case too (`x^2 - 2 == 0` must give `Sqrt[2]`, not
/// `Sqrt[8]/2`), so this fills that gap locally rather than reaching into
/// `canon.rs`.
pub fn simplify_sqrt_of_integer(n: i64) -> Expr {
    if n < 0 {
        return Expr::call("Sqrt", vec![Expr::integer(n)]);
    }
    if n == 0 {
        return Expr::integer(0);
    }
    let mut remaining = n;
    let mut coeff: i64 = 1;
    let mut factor: i64 = 2;
    while factor * factor <= remaining {
        while remaining % (factor * factor) == 0 {
            remaining /= factor * factor;
            coeff *= factor;
        }
        factor += 1;
    }
    if remaining == 1 {
        Expr::integer(coeff)
    } else if coeff == 1 {
        Expr::call("Sqrt", vec![Expr::integer(remaining)])
    } else {
        Expr::times(vec![Expr::integer(coeff), Expr::call("Sqrt", vec![Expr::integer(remaining)])])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depends_on_ignores_head_position() {
        // Sin[y] does not depend on "Sin" as a variable.
        assert!(!depends_on(&Expr::call("Sin", vec![Expr::symbol("y")]), "Sin"));
        assert!(depends_on(&Expr::call("Sin", vec![Expr::symbol("x")]), "x"));
        assert!(!depends_on(&Expr::symbol("y"), "x"));
    }

    #[test]
    fn linear_form_basic_cases() {
        assert_eq!(linear_form(&Expr::symbol("x"), "x"), Some((Expr::integer(1), Expr::integer(0))));
        let e = Expr::plus(vec![Expr::times(vec![Expr::integer(2), Expr::symbol("x")]), Expr::integer(3)]);
        assert_eq!(linear_form(&e, "x"), Some((Expr::integer(2), Expr::integer(3))));
        // x^2 is not linear.
        assert_eq!(linear_form(&Expr::power(Expr::symbol("x"), Expr::integer(2)), "x"), None);
    }

    #[test]
    fn only_symbol_finds_the_lone_variable() {
        let e = Expr::plus(vec![Expr::power(Expr::symbol("x"), Expr::integer(2)), Expr::integer(1)]);
        assert_eq!(only_symbol(&e), Some("x".to_string()));
        let two_vars = Expr::plus(vec![Expr::symbol("x"), Expr::symbol("y")]);
        assert_eq!(only_symbol(&two_vars), None);
    }

    #[test]
    fn poly_coeffs_reads_off_a_quadratic() {
        // x^2 - 5x + 6
        let e = Expr::plus(vec![
            Expr::power(Expr::symbol("x"), Expr::integer(2)),
            Expr::times(vec![Expr::integer(-5), Expr::symbol("x")]),
            Expr::integer(6),
        ]);
        assert_eq!(poly_coeffs(&e, "x"), Some(vec![Expr::integer(6), Expr::integer(-5), Expr::integer(1)]));
    }

    #[test]
    fn poly_coeffs_rejects_non_polynomial_use() {
        // Sin[x] is not a polynomial in x.
        assert_eq!(poly_coeffs(&Expr::call("Sin", vec![Expr::symbol("x")]), "x"), None);
        // 1/x is not a polynomial in x (negative power).
        assert_eq!(poly_coeffs(&Expr::power(Expr::symbol("x"), Expr::integer(-1)), "x"), None);
    }

    #[test]
    fn sqrt_of_integer_pulls_out_perfect_square_factors() {
        assert_eq!(simplify_sqrt_of_integer(8), Expr::times(vec![Expr::integer(2), Expr::call("Sqrt", vec![Expr::integer(2)])]));
        assert_eq!(simplify_sqrt_of_integer(4), Expr::integer(2));
        assert_eq!(simplify_sqrt_of_integer(2), Expr::call("Sqrt", vec![Expr::integer(2)]));
        assert_eq!(simplify_sqrt_of_integer(12), Expr::times(vec![Expr::integer(2), Expr::call("Sqrt", vec![Expr::integer(3)])]));
    }
}
