//! OpenMat kernel facade: ties core evaluation to solvers, exposes the app API.
//!
//! [`evaluate`] is the one function the Tauri app calls (see
//! `ARCHITECTURE.md`'s app-to-kernel contract). It parses `input`, dispatches
//! `NDSolve[...]` to [`ndsolve::solve`], and otherwise evaluates and typesets
//! the expression with `openmat-core`.

mod ndsolve;

use serde::Serialize;

use openmat_core::{parse, to_latex, Evaluator, Expr};

/// The result of evaluating one input cell: typeset output, an optional
/// plot, and an optional error message. Field names match the app-to-kernel
/// contract in `ARCHITECTURE.md` exactly, since this is serialized straight
/// to JSON for the Tauri bridge.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvalResult {
    pub latex: String,
    pub plot: Option<PlotData>,
    pub error: Option<String>,
}

/// A plottable result: one or more curves and the axis ranges to draw them in.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlotData {
    pub curves: Vec<Curve>,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
}

/// A single curve: sampled `(x, y)` points and an optional legend label.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Curve {
    pub points: Vec<(f64, f64)>,
    pub label: Option<String>,
}

impl EvalResult {
    fn error(message: impl Into<String>) -> Self {
        EvalResult { latex: String::new(), plot: None, error: Some(message.into()) }
    }
}

/// Evaluate one line of input and return a render-ready result.
///
/// Parse failures and NDSolve-specific failures (an unbound parameter, a
/// malformed problem) are reported through `EvalResult::error` rather than
/// as a `Result`, since that is the shape the app's single Tauri command
/// needs: always a value to render, never a thrown exception.
pub fn evaluate(input: &str) -> EvalResult {
    let expr = match parse(input) {
        Ok(expr) => expr,
        Err(err) => return EvalResult::error(err.to_string()),
    };

    if expr.has_head("NDSolve") {
        return match ndsolve::solve(&expr) {
            Ok(result) => result,
            Err(message) => EvalResult::error(message),
        };
    }

    let evaluator = Evaluator::new();
    let result = evaluator.eval(&expr);
    EvalResult { latex: to_latex(&clean_tree(&result)), plot: None, error: None }
}

/// Round `Real` leaves to 12 significant decimal places so ordinary
/// floating point noise (`0.1 + 0.2` landing on `0.30000000000000004`
/// instead of `0.3`) does not leak into the typeset output. Leaves anything
/// that survives evaluation as an exact `Integer` untouched.
fn clean_tree(e: &Expr) -> Expr {
    match e {
        Expr::Real(x) => Expr::Real(clean_number(*x)),
        Expr::Normal { head, args } => Expr::normal(clean_tree(head), args.iter().map(clean_tree).collect()),
        _ => e.clone(),
    }
}

fn clean_number(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let scaled = x * 1e12;
    if !scaled.is_finite() {
        return x;
    }
    scaled.round() / 1e12
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_evaluates_and_typesets() {
        let result = evaluate("2 + 3 * 4");
        assert_eq!(result.latex, "14");
        assert!(result.error.is_none());
        assert!(result.plot.is_none());
    }

    #[test]
    fn sin_zero_drops_out_of_the_sum() {
        let result = evaluate("Sin[0] + x");
        assert_eq!(result.latex, "x");
        assert!(result.error.is_none());
    }

    #[test]
    fn parse_error_surfaces_with_position() {
        let result = evaluate("1 + ");
        assert!(result.latex.is_empty());
        let err = result.error.expect("expected a parse error");
        assert!(err.contains("position"), "error should mention a position: {err}");
    }

    #[test]
    fn float_noise_is_trimmed() {
        let result = evaluate("N[0.1 + 0.2]");
        assert_eq!(result.latex, "0.3");
    }

    #[test]
    fn eval_result_serializes_with_contract_field_names() {
        let result = EvalResult {
            latex: "x^{2}".to_string(),
            plot: Some(PlotData {
                curves: vec![Curve { points: vec![(0.0, 1.0), (1.0, 2.0)], label: Some("x(t)".to_string()) }],
                x_range: (0.0, 1.0),
                y_range: (1.0, 2.0),
            }),
            error: None,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["latex"], "x^{2}");
        assert_eq!(json["error"], serde_json::Value::Null);
        let plot = &json["plot"];
        assert_eq!(plot["x_range"], serde_json::json!([0.0, 1.0]));
        assert_eq!(plot["y_range"], serde_json::json!([1.0, 2.0]));
        assert_eq!(plot["curves"][0]["label"], "x(t)");
        assert_eq!(plot["curves"][0]["points"], serde_json::json!([[0.0, 1.0], [1.0, 2.0]]));
    }
}
