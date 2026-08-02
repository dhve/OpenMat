//! `Plot`/`ListPlot` dispatch: turns `Plot[f, {x, a, b}]`,
//! `Plot[{f, g, ...}, {x, a, b}]`, and `ListPlot[data]` into plottable
//! [`PlotOutcome`]s.
//!
//! ## Sampling the expression
//!
//! Rather than symbolically differentiating or otherwise special-casing the
//! plotted expression, each curve gets one reusable closure ([`make_sampler`]):
//! substitute the plot variable for a concrete `Real` via `replace_all`, force
//! full numeric evaluation with `N[...]`, and read the resulting `Real`/`Integer`
//! leaf back out as `f64`. That closure is called repeatedly by the adaptive
//! sampler below; nothing about the plotted expression's structure is
//! inspected beyond the initial free-symbol preflight check.
//!
//! ## Adaptive sampling and curve breaks
//!
//! Sampling starts from [`INITIAL_SAMPLES`] uniform points, then recursively
//! bisects each interval ([`refine_interval`]) while either:
//! - the turn angle between the two segments meeting at the new midpoint
//!   exceeds [`ANGLE_THRESHOLD_RADIANS`] (ordinary curvature refinement), or
//! - the three points disagree on whether the function is even defined there
//!   (a possible discontinuity boundary, worth narrowing down further).
//!
//! up to [`MAX_SAMPLES_PER_CURVE`] points and [`MAX_REFINE_DEPTH`] levels per
//! interval. A sample counts as undefined, and so `None`, when evaluation
//! does not fold to a plain number (`Sqrt[-1]` under real arithmetic, a
//! symbolic leftover, `NaN`) or when it "explodes": its magnitude clears a
//! per-curve cutoff computed from the spread seen in the initial coarse pass
//! ([`EXPLODE_MULTIPLIER`] times that spread, floored at [`EXPLODE_FLOOR`]).
//! That cutoff is relative rather than a fixed constant on purpose: an
//! absolute cutoff would either clip a legitimately large-valued curve
//! (`Exp[x]` over a wide range) or fail to catch a pole on a curve whose
//! ordinary values are tiny. `Tan[x]` near an asymptote fails this cutoff
//! within a handful of extra bisections, well inside the depth budget; a
//! smooth curve's values never run 50x past what the initial pass already
//! saw, so it is never clipped.
//!
//! `Display::Plot`'s `curves: Vec<Curve>` shape has no field for "this run
//! continues the previous one", so a discontinuity is represented by ending
//! the current point run and starting a fresh `Curve` that shares the same
//! `label` ([`split_into_runs`]); the app groups by label for one legend
//! entry while still rendering each run without a connecting segment. That is
//! the shape chosen over inserting `NaN`-sentinel points into a single
//! `Curve`, since `Curve::points` is typed `Vec<(f64, f64)>` with no room for
//! a sentinel that would not also look like a plottable point to a naive
//! consumer.

use openmat_core::{replace_all, to_latex, Evaluator, Expr};

use crate::Curve;

const INITIAL_SAMPLES: usize = 60;
const MAX_SAMPLES_PER_CURVE: usize = 600;
const ANGLE_THRESHOLD_RADIANS: f64 = 0.15;
const MAX_REFINE_DEPTH: usize = 24;
const EXPLODE_MULTIPLIER: f64 = 50.0;
const EXPLODE_FLOOR: f64 = 1e6;
const Y_QUANTILE_LOW: f64 = 0.02;
const Y_QUANTILE_HIGH: f64 = 0.98;

/// What a successful `Plot`/`ListPlot` produces: the typeset expression(s)
/// and the sampled curves, ready for the kernel to wrap into `Display`s.
#[derive(Debug)]
pub(crate) struct PlotOutcome {
    pub latex: String,
    pub curves: Vec<Curve>,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
}

/// Function names this module knows are builtins rather than free
/// parameters, when scanning a plotted expression for unbound symbols. Kept
/// deliberately small, matching `ndsolve.rs`'s own `BUILTIN_NAMES`: anything
/// else (a stray coefficient like `a` in `a*Sin[x]`) is reported as an
/// unbound parameter before any sampling happens.
const BUILTIN_NAMES: &[&str] = &["Plus", "Times", "Power", "Sin", "Cos", "Tan", "Exp", "Log", "Sqrt", "Abs", "N", "List"];

fn is_known_symbol(name: &str, var: &str) -> bool {
    name == var || BUILTIN_NAMES.contains(&name)
}

/// Scan `e` for a bare symbol that is neither the plot variable `var` nor a
/// known builtin name. Probing this numerically instead (evaluate at a
/// sample point and see what comes back) is unsound for the same reason
/// `ndsolve.rs` gives: a coefficient that happens to multiply a zero at the
/// probed point would hide the free symbol. A symbolic scan up front, before
/// any sampling, cannot be fooled by which numbers happen to get tried.
fn find_free_symbol(evaluator: &Evaluator, e: &Expr, var: &str) -> Option<String> {
    match e {
        Expr::Symbol(s) if !is_known_symbol(s, var) && !evaluator.has_definition(s) => Some(s.clone()),
        Expr::Normal { head, args } => {
            find_free_symbol(evaluator, head, var).or_else(|| args.iter().find_map(|a| find_free_symbol(evaluator, a, var)))
        }
        _ => None,
    }
}

fn to_f64(e: &Expr) -> Option<f64> {
    match e {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Real(x) => Some(*x),
        _ => None,
    }
}

fn list_items(e: &Expr) -> Option<Vec<Expr>> {
    let (head, args) = e.as_normal()?;
    if head.as_symbol() == Some("List") {
        Some(args.to_vec())
    } else {
        None
    }
}

/// Read `{x, a, b}` into the plot variable's name and its numeric bounds.
fn parse_plot_range(e: &Expr) -> Result<(String, f64, f64), String> {
    let items = list_items(e).ok_or_else(|| "Plot: second argument must be {x, a, b}".to_string())?;
    if items.len() != 3 {
        return Err("Plot: second argument must be {x, a, b}".to_string());
    }
    let var = items[0].as_symbol().ok_or_else(|| "Plot: {x, a, b}: x must be a symbol".to_string())?.to_string();
    let a = to_f64(&items[1]).ok_or_else(|| "Plot: {x, a, b}: a must be a number".to_string())?;
    let b = to_f64(&items[2]).ok_or_else(|| "Plot: {x, a, b}: b must be a number".to_string())?;
    if !(a < b) {
        return Err(format!("Plot: {{x, a, b}}: expected a < b, got a = {a}, b = {b}"));
    }
    Ok((var, a, b))
}

/// Read the first argument of `Plot[...]` as one or more `(expression,
/// label)` pairs. `Plot[{f, g}, ...]` gives one entry per list item; a bare
/// `Plot[f, ...]` gives a single entry. The label is the expression's own
/// `InputForm` text, i.e. exactly what the user typed (after any Manipulate
/// bindings have already been substituted in by the caller), matching
/// "labels from the input forms".
fn plot_targets(e: &Expr) -> Vec<(Expr, String)> {
    if let Some((head, items)) = e.as_normal() {
        if head.as_symbol() == Some("List") {
            return items.iter().map(|it| (it.clone(), it.to_string())).collect();
        }
    }
    vec![(e.clone(), e.to_string())]
}

fn plot_latex(targets: &[(Expr, String)]) -> String {
    if targets.len() == 1 {
        to_latex(&targets[0].0)
    } else {
        to_latex(&Expr::list(targets.iter().map(|(e, _)| e.clone()).collect()))
    }
}

/// Force full numeric evaluation of an already-bound expression via
/// `N[...]`. `None` means it did not fold to a plain number: a leftover
/// symbolic term (an unrecognized function name, caught upfront by
/// [`find_free_symbol`] for the top-level free-parameter case, but also
/// possible transiently mid-expression) or a non-numeric result.
fn eval_numeric(evaluator: &Evaluator, expr: &Expr) -> Option<f64> {
    let wrapped = Expr::call("N", vec![expr.clone()]);
    match evaluator.eval(&wrapped) {
        Expr::Real(v) => Some(v),
        Expr::Integer(v) => Some(v as f64),
        _ => None,
    }
}

/// Build the one reusable closure a curve is sampled through: substitute
/// `var -> Real(x)` via `replace_all`, then run the substituted tree through
/// `eval_numeric`. Called once per curve, then invoked at every sample point
/// the adaptive sampler picks.
fn make_sampler<'a>(evaluator: &'a Evaluator, expr: &'a Expr, var: &'a str) -> impl Fn(f64) -> Option<f64> + 'a {
    move |x: f64| {
        let rule = Expr::rule(Expr::symbol(var), Expr::real(x));
        let bound = replace_all(expr, &Expr::list(vec![rule]));
        eval_numeric(evaluator, &bound)
    }
}

/// Clip a sample to `None` if it is non-finite or its magnitude clears
/// `cutoff`, the per-curve "this counts as exploding" threshold.
fn clip_sample(y: Option<f64>, cutoff: f64) -> Option<f64> {
    y.filter(|v| v.is_finite() && v.abs() <= cutoff)
}

/// The angle, in `[0, pi]`, between the segment from `(x0,y0)` to `(x1,y1)`
/// and the segment from `(x1,y1)` to `(x2,y2)`, after scaling both axes by
/// `x_scale`/`y_scale` so the comparison is aspect-ratio independent: without
/// that normalization a curve running 0..1e6 would trip the threshold on
/// ordinary float noise, while one running 0..1e-6 would never trip it.
/// `0` means the three points are collinear (a straight continuation); `pi`
/// means a full reversal, the shape a curve takes right next to a pole.
fn turn_angle(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64, x_scale: f64, y_scale: f64) -> f64 {
    let v1 = ((x1 - x0) / x_scale, (y1 - y0) / y_scale);
    let v2 = ((x2 - x1) / x_scale, (y2 - y1) / y_scale);
    let dot = v1.0 * v2.0 + v1.1 * v2.1;
    let mag1 = (v1.0 * v1.0 + v1.1 * v1.1).sqrt();
    let mag2 = (v2.0 * v2.0 + v2.1 * v2.1).sqrt();
    if mag1 < 1e-15 || mag2 < 1e-15 {
        return 0.0;
    }
    (dot / (mag1 * mag2)).clamp(-1.0, 1.0).acos()
}

/// Recursively refine the interval `(x0, x1)`, appending every sample point
/// from `x1` on (the caller is responsible for `x0` already being the last
/// entry in `out_x`/`out_y`) so a chain of calls builds one contiguous
/// sample stream. Splits on either ordinary curvature (all three points
/// defined, turn angle over threshold) or a possible discontinuity boundary
/// (the three points disagree on definedness), each bounded by `budget` and
/// `depth`. When a split is declined but the freshly sampled midpoint turned
/// out undefined, that midpoint is still recorded (as `None`) rather than
/// silently dropped, so a discontinuity discovered right at the depth/budget
/// limit still produces a break instead of a connecting segment.
#[allow(clippy::too_many_arguments)]
fn refine_interval(
    f: &impl Fn(f64) -> Option<f64>,
    cutoff: f64,
    x0: f64,
    y0: Option<f64>,
    x1: f64,
    y1: Option<f64>,
    x_scale: f64,
    y_scale: f64,
    depth: usize,
    budget: &mut usize,
    out_x: &mut Vec<f64>,
    out_y: &mut Vec<Option<f64>>,
) {
    if *budget == 0 || depth >= MAX_REFINE_DEPTH {
        out_x.push(x1);
        out_y.push(y1);
        return;
    }

    let xm = (x0 + x1) / 2.0;
    let ym = clip_sample(f(xm), cutoff);

    let all_defined = y0.is_some() && ym.is_some() && y1.is_some();
    let any_defined = y0.is_some() || ym.is_some() || y1.is_some();
    let mixed_definedness = any_defined && !all_defined;

    let curvature_trigger = match (y0, ym, y1) {
        (Some(a), Some(m), Some(c)) => turn_angle(x0, a, xm, m, x1, c, x_scale, y_scale) > ANGLE_THRESHOLD_RADIANS,
        _ => false,
    };

    if curvature_trigger || mixed_definedness {
        *budget -= 1;
        refine_interval(f, cutoff, x0, y0, xm, ym, x_scale, y_scale, depth + 1, budget, out_x, out_y);
        refine_interval(f, cutoff, xm, ym, x1, y1, x_scale, y_scale, depth + 1, budget, out_x, out_y);
    } else {
        if ym.is_none() {
            out_x.push(xm);
            out_y.push(None);
        }
        out_x.push(x1);
        out_y.push(y1);
    }
}

/// Split a flat `(x, Option<y>)` sample stream into contiguous runs of
/// defined points, dropping `None` gaps: the mechanism behind curve breaks
/// (see the module docs).
fn split_into_runs(xs: &[f64], ys: &[Option<f64>]) -> Vec<Vec<(f64, f64)>> {
    let mut runs = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    for (&x, &y) in xs.iter().zip(ys.iter()) {
        match y {
            Some(v) => current.push((x, v)),
            None => {
                if !current.is_empty() {
                    runs.push(std::mem::take(&mut current));
                }
            }
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    runs
}

/// Sample `f` over `[a, b]` adaptively, returning the resulting point runs
/// (more than one iff a discontinuity was detected).
fn adaptive_curve(f: &impl Fn(f64) -> Option<f64>, a: f64, b: f64) -> Vec<Vec<(f64, f64)>> {
    let n0 = INITIAL_SAMPLES.max(2);
    let xs: Vec<f64> = (0..n0).map(|i| a + (b - a) * (i as f64) / ((n0 - 1) as f64)).collect();
    let raw: Vec<Option<f64>> = xs.iter().map(|&x| f(x)).collect();

    // The explode cutoff scales with this curve's own spread (see module
    // docs); computed once from the coarse initial pass and held fixed
    // through refinement, so it reflects "how this curve normally behaves"
    // rather than drifting as refinement approaches a singularity.
    let y_scale = {
        let finite: Vec<f64> = raw.iter().filter_map(|y| *y).filter(|y| y.is_finite()).collect();
        let lo = finite.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if lo.is_finite() && hi.is_finite() && hi > lo {
            hi - lo
        } else {
            1.0
        }
    };
    let cutoff = (y_scale * EXPLODE_MULTIPLIER).max(EXPLODE_FLOOR);
    let ys: Vec<Option<f64>> = raw.into_iter().map(|y| clip_sample(y, cutoff)).collect();

    let x_scale = (b - a).abs().max(1e-12);
    let mut out_x: Vec<f64> = vec![xs[0]];
    let mut out_y: Vec<Option<f64>> = vec![ys[0]];
    let mut budget = MAX_SAMPLES_PER_CURVE.saturating_sub(xs.len());

    for i in 0..xs.len() - 1 {
        refine_interval(f, cutoff, xs[i], ys[i], xs[i + 1], ys[i + 1], x_scale, y_scale, 0, &mut budget, &mut out_x, &mut out_y);
    }

    split_into_runs(&out_x, &out_y)
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = pos - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

/// `y_range` from the actual plotted points: clip to the 2nd/98th
/// percentile rather than the literal min/max, then pad by 5%. Quantile
/// clipping (instead of a plain min/max) is what keeps a curve like
/// `Tan[x]` readable: the handful of points sampled right up against an
/// asymptote have huge magnitude by construction, and a literal min/max
/// range would flatten the rest of the curve to a near-flat line to fit them
/// in.
fn quantile_padded_range(ys: &[f64]) -> (f64, f64) {
    let mut finite: Vec<f64> = ys.iter().copied().filter(|y| y.is_finite()).collect();
    if finite.is_empty() {
        return (-1.0, 1.0);
    }
    finite.sort_by(|a, b| a.total_cmp(b));
    let lo = quantile(&finite, Y_QUANTILE_LOW);
    let hi = quantile(&finite, Y_QUANTILE_HIGH);
    let (lo, hi) = if hi > lo { (lo, hi) } else { (finite[0], *finite.last().unwrap()) };
    let span = hi - lo;
    let pad = if span.abs() < 1e-9 { 1.0 } else { span * 0.05 };
    (lo - pad, hi + pad)
}

fn min_max_padded(values: &[f64]) -> (f64, f64) {
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !lo.is_finite() || !hi.is_finite() {
        return (-1.0, 1.0);
    }
    if hi > lo {
        (lo, hi)
    } else {
        (lo - 1.0, hi + 1.0)
    }
}

/// Handle `expr` (already confirmed to have head `Plot`), returning either a
/// ready [`PlotOutcome`] or a human readable error message. The session
/// `evaluator` is used for sampling, so user definitions (`f[x_] := x^2`,
/// `a = 2`) resolve inside the plotted expression.
pub(crate) fn plot(evaluator: &Evaluator, expr: &Expr) -> Result<PlotOutcome, String> {
    let (_head, args) = expr.as_normal().expect("caller checked head is Plot");
    if args.len() != 2 {
        return Err(format!("Plot expects 2 arguments (an expression or list of expressions, and {{x, a, b}}), got {}", args.len()));
    }

    let (var, a, b) = parse_plot_range(&args[1])?;
    let targets = plot_targets(&args[0]);
    if targets.is_empty() {
        return Err("Plot: no expressions given to plot".to_string());
    }

    // Resolve session definitions up front (Plot[y, ...] after y = x^2 must
    // plot x^2): the sampler substitutes the plot variable textually before
    // evaluating, so a definition that only unfolds during evaluation would
    // otherwise never contain the variable to substitute. Labels keep the
    // form the user typed.
    let targets: Vec<(Expr, String)> = targets.into_iter().map(|(target, label)| (evaluator.eval(&target), label)).collect();

    for (target, _label) in &targets {
        if let Some(stray) = find_free_symbol(evaluator, target, &var) {
            return Err(format!(
                "Plot: '{stray}' is not bound to a number; substitute a numeric value for '{stray}' before calling Plot"
            ));
        }
    }

    let mut curves: Vec<Curve> = Vec::new();
    let mut all_ys: Vec<f64> = Vec::new();

    for (target, label) in &targets {
        let sampler = make_sampler(&evaluator, target, &var);
        let runs = adaptive_curve(&sampler, a, b);
        for run in &runs {
            all_ys.extend(run.iter().map(|p| p.1));
        }
        for run in runs {
            curves.push(Curve { points: run, label: Some(label.clone()) });
        }
    }

    let y_range = quantile_padded_range(&all_ys);
    let latex = plot_latex(&targets);

    Ok(PlotOutcome { latex, curves, x_range: (a, b), y_range })
}

/// Handle `expr` (already confirmed to have head `ListPlot`): either
/// `ListPlot[{{x1,y1}, ...}]` or `ListPlot[{y1, y2, ...}]` (x defaults to
/// `1..n`), returning a single unlabeled curve.
pub(crate) fn list_plot(evaluator: &Evaluator, expr: &Expr) -> Result<PlotOutcome, String> {
    let (_head, args) = expr.as_normal().expect("caller checked head is ListPlot");
    if args.len() != 1 {
        return Err(format!("ListPlot expects 1 argument (a list of data), got {}", args.len()));
    }

    // Resolve session definitions (ListPlot[data] after data = {...}).
    let data = evaluator.eval(&args[0]);
    let items = list_items(&data).ok_or_else(|| "ListPlot: argument must be a list".to_string())?;
    if items.is_empty() {
        return Err("ListPlot: given an empty list".to_string());
    }

    let is_pairs = items.iter().all(|it| list_items(it).is_some_and(|p| p.len() == 2));

    let points: Vec<(f64, f64)> = if is_pairs {
        items
            .iter()
            .map(|it| {
                let pair = list_items(it).unwrap();
                let x = eval_numeric(&evaluator, &pair[0])
                    .ok_or_else(|| "ListPlot: list entries must be numeric {x, y} pairs".to_string())?;
                let y = eval_numeric(&evaluator, &pair[1])
                    .ok_or_else(|| "ListPlot: list entries must be numeric {x, y} pairs".to_string())?;
                Ok((x, y))
            })
            .collect::<Result<Vec<_>, String>>()?
    } else {
        items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                let y = eval_numeric(&evaluator, it)
                    .ok_or_else(|| "ListPlot: list entries must all be numbers, or all be {x, y} pairs".to_string())?;
                Ok(((i + 1) as f64, y))
            })
            .collect::<Result<Vec<_>, String>>()?
    };

    let xs: Vec<f64> = points.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.1).collect();
    let x_range = min_max_padded(&xs);
    let y_range = quantile_padded_range(&ys);
    let latex = to_latex(&args[0]);

    Ok(PlotOutcome { latex, curves: vec![Curve { points, label: None }], x_range, y_range })
}

#[cfg(test)]
mod tests {
    use super::*;
    use openmat_core::parse;

    fn e(src: &str) -> Expr {
        parse(src).unwrap()
    }

    fn ev() -> Evaluator {
        Evaluator::new()
    }

    #[test]
    fn sin_curve_point_sanity() {
        let outcome = plot(&ev(), &e("Plot[Sin[x], {x, -3.14159265, 3.14159265}]")).expect("should plot");
        assert_eq!(outcome.curves.len(), 1, "sin is continuous, expected one unbroken curve");
        let pts = &outcome.curves[0].points;
        assert!(pts.len() >= INITIAL_SAMPLES, "expected at least the initial sample count, got {}", pts.len());
        assert!(pts.len() <= MAX_SAMPLES_PER_CURVE, "expected at most the per-curve cap, got {}", pts.len());

        let (x0, y0) = pts[0];
        assert!((x0 - (-3.14159265)).abs() < 1e-9);
        assert!(y0.abs() < 1e-6, "sin(-pi) should be ~0, got {y0}");

        let (xn, yn) = *pts.last().unwrap();
        assert!((xn - 3.14159265).abs() < 1e-9);
        assert!(yn.abs() < 1e-6, "sin(pi) should be ~0, got {yn}");

        for &(_, y) in pts {
            assert!(y >= -1.0001 && y <= 1.0001, "sin(x) out of range: {y}");
        }
        // x values are strictly increasing (adaptive refinement never
        // reorders or duplicates a sample).
        for w in pts.windows(2) {
            assert!(w[1].0 > w[0].0);
        }
    }

    #[test]
    fn multi_curve_labels_from_input_forms() {
        let outcome = plot(&ev(), &e("Plot[{Sin[x], Cos[x]}, {x, 0, 1}]")).expect("should plot");
        assert_eq!(outcome.curves.len(), 2, "Sin and Cos are both continuous on [0, 1]");
        assert_eq!(outcome.curves[0].label.as_deref(), Some("Sin[x]"));
        assert_eq!(outcome.curves[1].label.as_deref(), Some("Cos[x]"));
        assert!(!outcome.curves[0].points.is_empty());
        assert!(!outcome.curves[1].points.is_empty());
    }

    #[test]
    fn tan_splits_into_branches_at_the_asymptotes() {
        let outcome = plot(&ev(), &e("Plot[Tan[x], {x, -4, 4}]")).expect("should plot");
        // Two asymptotes inside [-4, 4] (at -pi/2 and pi/2) split the curve
        // into three branches.
        assert!(outcome.curves.len() >= 3, "expected at least 3 branches, got {}", outcome.curves.len());

        let pi_2 = std::f64::consts::FRAC_PI_2;
        let side = |x: f64| -> i32 {
            if x < -pi_2 {
                -1
            } else if x > pi_2 {
                1
            } else {
                0
            }
        };
        for curve in &outcome.curves {
            assert!(!curve.points.is_empty());
            let sides: std::collections::HashSet<i32> = curve.points.iter().map(|p| side(p.0)).collect();
            assert_eq!(sides.len(), 1, "a single curve run must not straddle a Tan asymptote: {:?}", curve.points);
        }
        // All curves share the label (same underlying expression, split for
        // the discontinuity).
        for curve in &outcome.curves {
            assert_eq!(curve.label.as_deref(), Some("Tan[x]"));
        }
    }

    #[test]
    fn list_plot_xy_pairs() {
        let outcome = list_plot(&ev(), &e("ListPlot[{{1, 2}, {2, 4}, {3, 9}}]")).expect("should plot");
        assert_eq!(outcome.curves.len(), 1);
        assert_eq!(outcome.curves[0].points, vec![(1.0, 2.0), (2.0, 4.0), (3.0, 9.0)]);
        assert!(outcome.curves[0].label.is_none());
    }

    #[test]
    fn list_plot_bare_values_default_to_index_x() {
        let outcome = list_plot(&ev(), &e("ListPlot[{10, 20, 30}]")).expect("should plot");
        assert_eq!(outcome.curves.len(), 1);
        assert_eq!(outcome.curves[0].points, vec![(1.0, 10.0), (2.0, 20.0), (3.0, 30.0)]);
    }

    #[test]
    fn plot_unbound_parameter_is_a_clear_error() {
        let err = plot(&ev(), &e("Plot[a*Sin[x], {x, 0, 1}]")).unwrap_err();
        assert!(err.contains('a'), "error should mention the unbound symbol: {err}");
        assert!(err.to_lowercase().contains("substitute"), "error should tell the user to substitute a value: {err}");
    }
}
