//! Calculus/algebra conformance suite for `crates/openmat-core/src/mathfns/`:
//! `D`, `Integrate`, `Expand`, `Solve`, `Factor`, `Simplify`.
//!
//! Every case compares two independently parsed-and-evaluated expressions
//! (`check(actual_src, expected_src)`) rather than a raw string, per the
//! evaluator's own convention (see `tests/conformance.rs`): the printed
//! *form* Mathematica happens to pick isn't the contract, equality after the
//! same canonicalizing evaluator is. A fresh `Evaluator` is used for each
//! side since none of these builtins depend on evaluator state.

use openmat_core::{parse, Evaluator};

fn check(actual_src: &str, expected_src: &str) {
    let actual = Evaluator::new().eval(&parse(actual_src).unwrap_or_else(|e| panic!("parse failed for {actual_src:?}: {e}")));
    let expected =
        Evaluator::new().eval(&parse(expected_src).unwrap_or_else(|e| panic!("parse failed for {expected_src:?}: {e}")));
    assert_eq!(actual, expected, "{actual_src:?} evaluated to {actual}, expected {expected_src:?} ({expected})");
}

/// For inputs outside a builtin's supported subset: the call must stay
/// symbolic (still headed by `name`) rather than produce a wrong answer.
fn check_stays_symbolic(src: &str, name: &str) {
    let result = Evaluator::new().eval(&parse(src).unwrap());
    assert!(result.has_head(name), "{src:?} should have stayed a symbolic {name}[...] call, got {result}");
}

// ---------------------------------------------------------------------------
// D: constants, x itself, linearity
// ---------------------------------------------------------------------------

#[test]
fn d_of_x_is_one() {
    check("D[x, x]", "1");
}

#[test]
fn d_of_constant_is_zero() {
    check("D[5, x]", "0");
}

#[test]
fn d_of_unrelated_symbol_is_zero() {
    check("D[y, x]", "0");
}

#[test]
fn d_linearity_over_plus() {
    check("D[x^3 + 2x, x]", "3x^2 + 2");
}

// ---------------------------------------------------------------------------
// D: power rule
// ---------------------------------------------------------------------------

#[test]
fn d_power_rule_square() {
    check("D[x^2, x]", "2x");
}

#[test]
fn d_power_rule_cube() {
    check("D[x^3, x]", "3x^2");
}

/// A symbolic (but x-independent) exponent still gets the power rule.
#[test]
fn d_power_rule_symbolic_exponent() {
    check("D[x^n, x]", "n*x^(n-1)");
}

// ---------------------------------------------------------------------------
// D: product rule (and, via Times[a, Power[b,-1]], the quotient rule for free)
// ---------------------------------------------------------------------------

#[test]
fn d_product_rule() {
    check("D[x*Sin[x], x]", "Sin[x] + x*Cos[x]");
}

#[test]
fn d_quotient_rule_via_product_and_power() {
    // d/dx [x/(x+1)] = 1/(x+1) - x/(x+1)^2, left as two terms since this
    // crate has no Together/Cancel to recombine them into one fraction.
    check("D[x/(x+1), x]", "1/(x+1) - x*(x+1)^(-2)");
}

// ---------------------------------------------------------------------------
// D: chain rule through Power, Sin, Cos, Tan, Exp, Log, Sqrt
// ---------------------------------------------------------------------------

#[test]
fn d_chain_rule_sin_of_square() {
    // The worked example from the task spec.
    check("D[Sin[x^2], x]", "2 x Cos[x^2]");
}

#[test]
fn d_chain_rule_cos_of_linear() {
    check("D[Cos[3x], x]", "-3 Sin[3x]");
}

#[test]
fn d_chain_rule_tan() {
    check("D[Tan[x], x]", "Cos[x]^(-2)");
}

#[test]
fn d_chain_rule_exp() {
    check("D[Exp[x], x]", "Exp[x]");
    check("D[Exp[2x], x]", "2 Exp[2x]");
}

#[test]
fn d_chain_rule_log() {
    check("D[Log[x], x]", "1/x");
    check("D[Log[3x+1], x]", "3/(3x+1)");
}

#[test]
fn d_chain_rule_sqrt() {
    check("D[Sqrt[x], x]", "1/(2 Sqrt[x])");
    check("D[Sqrt[x^2+1], x]", "x/Sqrt[x^2+1]");
}

#[test]
fn d_chain_rule_power_of_sum() {
    check("D[(x^2+1)^3, x]", "6 x (x^2+1)^2");
}

/// The exponential-rule branch of the Power chain rule: a^g(x).
#[test]
fn d_exponential_rule() {
    check("D[a^x, x]", "Log[a] * a^x");
}

// ---------------------------------------------------------------------------
// D: honest failure outside the supported set
// ---------------------------------------------------------------------------

#[test]
fn d_of_unknown_function_stays_symbolic() {
    check_stays_symbolic("D[f[x], x]", "D");
}

// ---------------------------------------------------------------------------
// Integrate: constants, power rule (including 1/x -> Log[x]), linearity
// ---------------------------------------------------------------------------

#[test]
fn integrate_constant() {
    check("Integrate[5, x]", "5x");
}

#[test]
fn integrate_power_rule() {
    check("Integrate[x, x]", "x^2/2");
    check("Integrate[x^3, x]", "x^4/4");
}

#[test]
fn integrate_negative_power() {
    check("Integrate[x^(-2), x]", "-1/x");
}

#[test]
fn integrate_reciprocal_gives_log() {
    check("Integrate[1/x, x]", "Log[x]");
}

#[test]
fn integrate_linearity() {
    check("Integrate[2x + 3, x]", "x^2 + 3x");
}

// ---------------------------------------------------------------------------
// Integrate: Sin, Cos, Exp, and the linear-substitution form f[a x + b]
// ---------------------------------------------------------------------------

#[test]
fn integrate_sin_cos_exp() {
    check("Integrate[Sin[x], x]", "-Cos[x]");
    check("Integrate[Cos[x], x]", "Sin[x]");
    check("Integrate[Exp[x], x]", "Exp[x]");
}

#[test]
fn integrate_linear_substitution() {
    check("Integrate[Exp[2x], x]", "Exp[2x]/2");
    check("Integrate[Sin[3x+1], x]", "-Cos[3x+1]/3");
}

// ---------------------------------------------------------------------------
// Integrate: polynomial products/powers via Expand, then termwise
// ---------------------------------------------------------------------------

#[test]
fn integrate_product_via_expand() {
    check("Integrate[x*(x+1), x]", "x^2/2 + x^3/3");
}

#[test]
fn integrate_linear_power_directly() {
    // Base is already linear in x, so this takes the direct power-rule
    // path (matching real Mathematica's (1+x)^3/3) rather than expanding.
    check("Integrate[(x+1)^2, x]", "(x+1)^3/3");
}

#[test]
fn integrate_nonlinear_power_via_expand() {
    check("Integrate[(x^2+x)^2, x]", "x^5/5 + x^4/2 + x^3/3");
}

// ---------------------------------------------------------------------------
// Integrate: honest failure outside the supported set
// ---------------------------------------------------------------------------

#[test]
fn integrate_by_parts_case_stays_symbolic() {
    check_stays_symbolic("Integrate[x*Sin[x], x]", "Integrate");
}

#[test]
fn integrate_log_stays_symbolic() {
    // Log[x] isn't in the antiderivative table (it needs integration by
    // parts: x Log[x] - x), so this must stay unevaluated, not guess.
    check_stays_symbolic("Integrate[Log[x], x]", "Integrate");
}

// ---------------------------------------------------------------------------
// Expand: distribute products over sums, integer powers of sums
// ---------------------------------------------------------------------------

#[test]
fn expand_square_of_binomial() {
    check("Expand[(x+1)^2]", "1 + 2x + x^2");
}

#[test]
fn expand_cube_of_binomial_two_variables() {
    check("Expand[(x+y)^3]", "x^3 + 3x^2*y + 3x*y^2 + y^3");
}

#[test]
fn expand_product_of_three_factors() {
    check("Expand[x*(x+1)*(x-1)]", "x^3 - x");
}

// ---------------------------------------------------------------------------
// Solve: linear, quadratic (exact via Sqrt), factored polynomial input
// ---------------------------------------------------------------------------

#[test]
fn solve_linear() {
    check("Solve[2x - 4 == 0, x]", "{{x -> 2}}");
}

#[test]
fn solve_quadratic_rational_roots() {
    check("Solve[x^2 - 5x + 6 == 0, x]", "{{x -> 2}, {x -> 3}}");
}

/// The task spec's own worked example: irrational roots must come out as an
/// exact Sqrt, and the perfect-square factor buried in the discriminant
/// (8 = 4*2) must be pulled out, not left as Sqrt[8]/2.
#[test]
fn solve_quadratic_irrational_roots_exact() {
    check("Solve[x^2 - 2 == 0, x]", "{{x -> -Sqrt[2]}, {x -> Sqrt[2]}}");
}

#[test]
fn solve_factored_polynomial_input() {
    check("Solve[(x-2)(x-3) == 0, x]", "{{x -> 2}, {x -> 3}}");
}

#[test]
fn solve_unsupported_degree_stays_symbolic() {
    check_stays_symbolic("Solve[x^3 - x == 0, x]", "Solve");
}

// ---------------------------------------------------------------------------
// Factor: quadratics with integer roots
// ---------------------------------------------------------------------------

#[test]
fn factor_quadratic_with_integer_roots() {
    check("Factor[x^2 - 5x + 6]", "(x-2)*(x-3)");
}

#[test]
fn factor_difference_of_squares() {
    check("Factor[x^2 - 4]", "(x-2)*(x+2)");
}

#[test]
fn factor_irrational_roots_stays_symbolic() {
    check_stays_symbolic("Factor[x^2 - 2]", "Factor");
}

// ---------------------------------------------------------------------------
// Simplify: evaluate, Expand, keep the shorter form
// ---------------------------------------------------------------------------

#[test]
fn simplify_folds_numerics() {
    check("Simplify[2 + x + 3]", "5 + x");
}

#[test]
fn simplify_combines_like_terms() {
    check("Simplify[x + x]", "2x");
}

#[test]
fn simplify_keeps_unexpanded_form_when_shorter() {
    // (x+1)^2 (9 chars) is shorter than its expansion 1 + 2x + x^2 (13
    // chars), so Simplify should leave it factored rather than distribute it.
    check("Simplify[(x+1)^2]", "(x+1)^2");
}
