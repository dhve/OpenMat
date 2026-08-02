//! End-to-end checks across parse -> eval -> latex, the path
//! `openmat-kernel` will actually drive.

use openmat_core::{parse, replace_all, to_latex, Evaluator, Expr};

#[test]
fn parse_eval_latex_pipeline_for_simple_algebra() {
    let parsed = parse("2 + x + 3").unwrap();
    let evaluated = Evaluator::new().eval(&parsed);
    assert_eq!(evaluated.to_string(), "5 + x");
    assert_eq!(to_latex(&evaluated), "5 + x");
}

#[test]
fn parse_eval_latex_pipeline_for_fraction() {
    let parsed = parse("1 / 2").unwrap();
    let evaluated = Evaluator::new().eval(&parsed);
    assert_eq!(evaluated.to_string(), "1/2");
    assert_eq!(to_latex(&evaluated), "\\frac{1}{2}");
}

#[test]
fn damped_pendulum_equation_parses_and_renders() {
    // The flagship demo equation from ARCHITECTURE.md.
    let src = "x''[t] + c x'[t] + Sin[x[t]] == 0";
    let parsed = parse(src).unwrap();
    assert_eq!(parsed.to_string(), "x''[t] + c*x'[t] + Sin[x[t]] == 0");
    let latex = to_latex(&parsed);
    assert_eq!(latex, "x''\\left(t\\right) + c x'\\left(t\\right) + \\sin\\left(x\\left(t\\right)\\right) = 0");
}

#[test]
fn ndsolve_call_shape_parses_for_kernel_dispatch() {
    // NDSolve[{eq1, eq2, eq3}, x, {t, 0, 20}] must parse as a plain function
    // call so openmat-kernel can pattern-match on its head and args.
    let src = "NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]";
    let parsed = parse(src).expect("NDSolve call should parse");
    let (head, args) = parsed.as_normal().expect("should be a Normal expression");
    assert_eq!(head.as_symbol(), Some("NDSolve"));
    assert_eq!(args.len(), 3);
    assert!(args[0].has_head("List"));
    assert_eq!(args[1], Expr::symbol("x"));
    assert!(args[2].has_head("List"));
}

#[test]
fn replace_all_after_evaluation_substitutes_symbol() {
    let parsed = parse("a + b").unwrap();
    let evaluated = Evaluator::new().eval(&parsed);
    let rule = Expr::rule(Expr::symbol("b"), Expr::integer(5));
    let substituted = replace_all(&evaluated, &rule);
    let result = Evaluator::new().eval(&substituted);
    assert_eq!(result.to_string(), "5 + a");
}

#[test]
fn n_of_pi_free_expression_matches_manual_float_eval() {
    let parsed = parse("N[Sin[0] + Sqrt[2]]").unwrap();
    let evaluated = Evaluator::new().eval(&parsed);
    match evaluated {
        Expr::Real(x) => assert!((x - std::f64::consts::SQRT_2).abs() < 1e-12),
        other => panic!("expected a Real, got {}", other),
    }
}

#[test]
fn parse_error_reports_position_for_kernel_error_surface() {
    let err = parse("1 + * 2").unwrap_err();
    assert!(err.pos > 0);
    assert!(err.to_string().contains("position"));
}
