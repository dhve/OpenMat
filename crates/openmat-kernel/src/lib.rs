//! OpenMat kernel facade: the kernel service (`ARCHITECTURE.md`, "Kernel
//! service and transport adapters"). Owns evaluation semantics and result
//! formatting; every transport (the Tauri command today, a Jupyter adapter
//! later) calls [`evaluate`] or [`evaluate_with_bindings`] and serializes the
//! [`KernelResult`] it gets back without reinterpreting it.

mod ndsolve;
mod plot;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

use openmat_core::{parse, replace_all, to_latex, Evaluator, Expr, ParseError};

/// The result of one evaluation request. Structured data, not presentation:
/// see `ARCHITECTURE.md`'s "Kernel API" section, which this mirrors field
/// for field (including the `Display` tag) since it is serialized straight
/// to JSON for the Tauri bridge.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KernelResult {
    pub request_id: u64,
    pub status: KernelStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_form: Option<String>,
    pub displays: Vec<Display>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<KernelError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KernelStatus {
    Ok,
    Error,
}

/// A derived presentation of the evaluated expression: LaTeX for typeset
/// output, or a plot. Tagged on `kind` so the app can switch on the JSON
/// directly. The expression itself, as `input_form`, is the canonical
/// payload; displays are derived from it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Display {
    Latex { latex: String },
    Plot { curves: Vec<Curve>, x_range: (f64, f64), y_range: (f64, f64) },
}

/// A single curve within a plot display: sampled `(x, y)` points and an
/// optional legend label.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Curve {
    pub points: Vec<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A warning or note that may accompany an `ok` result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Message {
    pub severity: Severity,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Note,
}

/// Set iff `status == "error"`. `kind` distinguishes where evaluation failed
/// so the UI can render each distinctly, per `specs/m0-milestone.md`'s
/// cross-cutting error-shape budget.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KernelError {
    pub kind: ErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    Parse,
    Eval,
    Solve,
}

impl KernelResult {
    fn ok(request_id: u64, input_form: String, displays: Vec<Display>) -> Self {
        KernelResult { request_id, status: KernelStatus::Ok, input_form: Some(input_form), displays, messages: Vec::new(), error: None }
    }

    fn error(request_id: u64, kind: ErrorKind, message: String, position: Option<usize>) -> Self {
        KernelResult {
            request_id,
            status: KernelStatus::Error,
            input_form: None,
            displays: Vec::new(),
            messages: Vec::new(),
            error: Some(KernelError { kind, message, position }),
        }
    }
}

/// Evaluate one input with no bound parameters. Equivalent to
/// `evaluate_with_bindings` with an empty bindings map.
pub fn evaluate(input: &str, request_id: u64) -> KernelResult {
    evaluate_with_bindings(input, &HashMap::new(), request_id)
}

/// Evaluate one input, first substituting each `bindings` entry for the
/// matching bare symbol as a typed `Expr::Real`, never as text
/// (`ARCHITECTURE.md`, "Manipulate: typed bindings, not text substitution").
/// The app parses a cell's source once per edit and keeps re-issuing this
/// call with that same `input` string and a new `c` on every slider tick, so
/// the parser is skipped on repeat calls via [`parse_cached`]
/// (`specs/m0-milestone.md` row 3).
pub fn evaluate_with_bindings(input: &str, bindings: &HashMap<String, f64>, request_id: u64) -> KernelResult {
    let expr = match parse_cached(input) {
        Ok(expr) => expr,
        Err(err) => return KernelResult::error(request_id, ErrorKind::Parse, err.message, Some(err.pos)),
    };
    let bound = apply_bindings(&expr, bindings);

    if bound.has_head("NDSolve") {
        match ndsolve::solve(&bound) {
            Ok(outcome) => KernelResult::ok(
                request_id,
                ndsolve_input_form(&bound),
                vec![
                    Display::Latex { latex: outcome.latex },
                    Display::Plot { curves: outcome.curves, x_range: outcome.x_range, y_range: outcome.y_range },
                ],
            ),
            Err(message) => KernelResult::error(request_id, ErrorKind::Solve, message, None),
        }
    } else if bound.has_head("Plot") {
        match plot::plot(&bound) {
            Ok(outcome) => KernelResult::ok(
                request_id,
                plot_input_form(&bound),
                vec![
                    Display::Latex { latex: outcome.latex },
                    Display::Plot { curves: outcome.curves, x_range: outcome.x_range, y_range: outcome.y_range },
                ],
            ),
            Err(message) => KernelResult::error(request_id, ErrorKind::Eval, message, None),
        }
    } else if bound.has_head("ListPlot") {
        match plot::list_plot(&bound) {
            Ok(outcome) => KernelResult::ok(
                request_id,
                plot_input_form(&bound),
                vec![
                    Display::Latex { latex: outcome.latex },
                    Display::Plot { curves: outcome.curves, x_range: outcome.x_range, y_range: outcome.y_range },
                ],
            ),
            Err(message) => KernelResult::error(request_id, ErrorKind::Eval, message, None),
        }
    } else {
        let evaluator = Evaluator::new();
        let result = clean_tree(&evaluator.eval(&bound));
        KernelResult::ok(request_id, result.to_string(), vec![Display::Latex { latex: to_latex(&result) }])
    }
}

/// `input_form` for a `Plot`/`ListPlot` call: the InputForm of the plotted
/// expression(s) or data (bindings already substituted in), mirroring
/// `ndsolve_input_form`'s reasoning: the whole `Plot[...]`/`ListPlot[...]`
/// call is not itself a reduced expression, but its first argument is what
/// the latex display actually typesets.
fn plot_input_form(bound: &Expr) -> String {
    match bound.as_normal() {
        Some((_, args)) if !args.is_empty() => args[0].to_string(),
        _ => bound.to_string(),
    }
}

/// Substitute each binding for the matching bare symbol, everywhere it
/// appears, as a typed `Expr::Real`. Runs on the parsed tree after
/// `parse_cached` and before NDSolve/eval dispatch; never touches source
/// text.
fn apply_bindings(expr: &Expr, bindings: &HashMap<String, f64>) -> Expr {
    if bindings.is_empty() {
        return expr.clone();
    }
    let rules: Vec<Expr> = bindings.iter().map(|(name, value)| Expr::rule(Expr::symbol(name.clone()), Expr::real(*value))).collect();
    replace_all(expr, &Expr::list(rules))
}

/// `input_form` for an NDSolve call: the InputForm of the equation list
/// (bindings already substituted in), matching what the latex display
/// renders, rather than the whole `NDSolve[...]` call, which is not itself
/// a reduced expression.
fn ndsolve_input_form(bound: &Expr) -> String {
    match bound.as_normal() {
        Some((_, args)) if !args.is_empty() => args[0].to_string(),
        _ => bound.to_string(),
    }
}

/// Parse `input`, reusing the previous parse when `input` is identical to
/// the last call. Backs "parsed once per edit, not once per slider tick"
/// (`specs/m0-milestone.md` row 3): the app keeps a cell's WL source text
/// fixed across a slider drag and only the bindings change, so every tick
/// after the first is a cache hit. One slot only, as specified: this caches
/// the single most recent (input, parsed expression) pair, not a general
/// LRU.
fn parse_cached(input: &str) -> Result<Expr, ParseError> {
    static CACHE: Mutex<Option<(String, Expr)>> = Mutex::new(None);
    let mut cache = CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((cached_input, cached_expr)) = cache.as_ref() {
        if cached_input == input {
            return Ok(cached_expr.clone());
        }
    }
    let expr = parse(input)?;
    *cache = Some((input.to_string(), expr.clone()));
    Ok(expr)
}

/// Round `Real` leaves to 12 significant decimal places so ordinary
/// floating point noise (`0.1 + 0.2` landing on `0.30000000000000004`
/// instead of `0.3`) does not leak into the typeset output.
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

    const PENDULUM: &str = "NDSolve[{x''[t] + c x'[t] + Sin[x[t]] == 0, x[0] == 2, x'[0] == 0}, x, {t, 0, 20}]";

    fn find_latex(displays: &[Display]) -> Option<&str> {
        displays.iter().find_map(|d| match d {
            Display::Latex { latex } => Some(latex.as_str()),
            _ => None,
        })
    }

    fn find_plot(displays: &[Display]) -> Option<(&[Curve], (f64, f64), (f64, f64))> {
        displays.iter().find_map(|d| match d {
            Display::Plot { curves, x_range, y_range } => Some((curves.as_slice(), *x_range, *y_range)),
            _ => None,
        })
    }

    #[test]
    fn arithmetic_evaluates_and_typesets() {
        let result = evaluate("2 + 3 * 4", 1);
        assert_eq!(result.status, KernelStatus::Ok);
        assert_eq!(result.input_form.as_deref(), Some("14"));
        assert_eq!(find_latex(&result.displays), Some("14"));
        assert!(result.error.is_none());
    }

    #[test]
    fn sin_zero_drops_out_of_the_sum() {
        let result = evaluate("Sin[0] + x", 1);
        assert_eq!(find_latex(&result.displays), Some("x"));
        assert_eq!(result.status, KernelStatus::Ok);
    }

    #[test]
    fn parse_error_surfaces_with_position() {
        let result = evaluate("1 + ", 1);
        assert_eq!(result.status, KernelStatus::Error);
        assert!(result.displays.is_empty());
        assert!(result.input_form.is_none());
        let err = result.error.expect("expected a parse error");
        assert_eq!(err.kind, ErrorKind::Parse);
        assert!(err.position.is_some());
    }

    #[test]
    fn float_noise_is_trimmed() {
        let result = evaluate("N[0.1 + 0.2]", 1);
        assert_eq!(find_latex(&result.displays), Some("0.3"));
    }

    #[test]
    fn request_id_is_echoed() {
        assert_eq!(evaluate("1 + 1", 42).request_id, 42);
        assert_eq!(evaluate_with_bindings("1 + 1", &HashMap::new(), 7).request_id, 7);
    }

    #[test]
    fn pendulum_with_c_bound_solves() {
        let mut bindings = HashMap::new();
        bindings.insert("c".to_string(), 0.5);
        let result = evaluate_with_bindings(PENDULUM, &bindings, 1);
        assert_eq!(result.status, KernelStatus::Ok);
        assert!(result.error.is_none());
        let (curves, x_range, _) = find_plot(&result.displays).expect("expected a plot display");
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0].points.len(), 400);
        assert_eq!(x_range, (0.0, 20.0));
    }

    #[test]
    fn pendulum_with_c_unbound_errors_naming_c() {
        // Same input as the bound case above, reused deliberately: this
        // exercises parse_cached's cache-hit path (same text, no bindings
        // this time) as well as the unbound-parameter error.
        let result = evaluate(PENDULUM, 1);
        assert_eq!(result.status, KernelStatus::Error);
        assert!(result.displays.is_empty());
        let err = result.error.expect("expected an error");
        assert!(matches!(err.kind, ErrorKind::Solve | ErrorKind::Eval), "unexpected error kind: {:?}", err.kind);
        assert!(err.message.contains('c'), "error should name the unbound symbol: {}", err.message);
    }

    #[test]
    fn plot_dispatches_through_evaluate() {
        let result = evaluate("Plot[Sin[x], {x, 0, 1}]", 1);
        assert_eq!(result.status, KernelStatus::Ok);
        assert!(result.error.is_none());
        assert_eq!(find_latex(&result.displays), Some("\\sin\\left(x\\right)"));
        let (curves, x_range, _) = find_plot(&result.displays).expect("expected a plot display");
        assert_eq!(curves.len(), 1);
        assert_eq!(x_range, (0.0, 1.0));
    }

    #[test]
    fn plot_with_unbound_coefficient_bound_via_manipulate_solves() {
        let mut bindings = HashMap::new();
        bindings.insert("a".to_string(), 2.0);
        let result = evaluate_with_bindings("Plot[a*Sin[x], {x, 0, 1}]", &bindings, 1);
        assert_eq!(result.status, KernelStatus::Ok);
        let (curves, ..) = find_plot(&result.displays).expect("expected a plot display");
        assert_eq!(curves.len(), 1);
        // Manipulate bindings are substituted as typed Expr::Real, never as
        // text (ARCHITECTURE.md), so the bound coefficient prints as "2."
        assert_eq!(curves[0].label.as_deref(), Some("2.*Sin[x]"));
    }

    #[test]
    fn plot_with_unbound_coefficient_errors_naming_it() {
        let result = evaluate("Plot[a*Sin[x], {x, 0, 1}]", 1);
        assert_eq!(result.status, KernelStatus::Error);
        assert!(result.displays.is_empty());
        let err = result.error.expect("expected an error");
        assert_eq!(err.kind, ErrorKind::Eval);
        assert!(err.message.contains('a'), "error should name the unbound symbol: {}", err.message);
    }

    #[test]
    fn list_plot_dispatches_through_evaluate() {
        let result = evaluate("ListPlot[{1, 4, 9}]", 1);
        assert_eq!(result.status, KernelStatus::Ok);
        let (curves, ..) = find_plot(&result.displays).expect("expected a plot display");
        assert_eq!(curves[0].points, vec![(1.0, 1.0), (2.0, 4.0), (3.0, 9.0)]);
    }

    #[test]
    fn display_serializes_with_kind_tag_and_contract_field_names() {
        let result = KernelResult::ok(
            9,
            "x^2".to_string(),
            vec![
                Display::Latex { latex: "x^{2}".to_string() },
                Display::Plot {
                    curves: vec![Curve { points: vec![(0.0, 1.0), (1.0, 2.0)], label: Some("x(t)".to_string()) }],
                    x_range: (0.0, 1.0),
                    y_range: (1.0, 2.0),
                },
            ],
        );
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["request_id"], 9);
        assert_eq!(json["status"], "ok");
        assert_eq!(json["input_form"], "x^2");
        assert_eq!(json["displays"][0]["kind"], "latex");
        assert_eq!(json["displays"][0]["latex"], "x^{2}");
        assert_eq!(json["displays"][1]["kind"], "plot");
        assert_eq!(json["displays"][1]["x_range"], serde_json::json!([0.0, 1.0]));
        assert_eq!(json["displays"][1]["y_range"], serde_json::json!([1.0, 2.0]));
        assert_eq!(json["displays"][1]["curves"][0]["points"], serde_json::json!([[0.0, 1.0], [1.0, 2.0]]));
        assert_eq!(json["displays"][1]["curves"][0]["label"], "x(t)");
        assert!(json.get("error").is_none() || json["error"].is_null());
    }

    #[test]
    fn error_result_serializes_with_error_kind_tag() {
        let result = KernelResult::error(3, ErrorKind::Parse, "unexpected end of input".to_string(), Some(4));
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["status"], "error");
        assert_eq!(json["error"]["kind"], "parse");
        assert_eq!(json["error"]["message"], "unexpected end of input");
        assert_eq!(json["error"]["position"], 4);
        assert!(json["displays"].as_array().unwrap().is_empty());
        assert!(json.get("input_form").is_none() || json["input_form"].is_null());
    }
}
