//! `to_latex`: render an [`Expr`] as textbook-quality LaTeX for KaTeX.
//!
//! Shares the numerator/denominator and derivative-form detection helpers
//! conceptually with `expr.rs`'s `InputForm` renderer, but LaTeX has its own
//! layout rules (`\frac`, superscripts, `\left(...\right)`), so the logic is
//! kept separate rather than parameterizing one renderer over two output
//! languages.

use crate::expr::Expr;

pub fn to_latex(e: &Expr) -> String {
    render(e)
}

fn render(e: &Expr) -> String {
    match e {
        Expr::Integer(n) => n.to_string(),
        Expr::Real(x) => render_real(*x),
        Expr::Symbol(s) => render_symbol(s),
        Expr::Str(s) => format!("\\text{{{}}}", escape_text(s)),
        Expr::Normal { head, args } => render_normal(head, args, e),
    }
}

fn render_real(x: f64) -> String {
    if x.is_nan() {
        return "\\text{Indeterminate}".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-\\infty".to_string() } else { "\\infty".to_string() };
    }
    let s = format!("{}", x);
    if s.contains('.') || s.contains('e') {
        s
    } else {
        format!("{}.", s)
    }
}

const GREEK: &[(&str, &str)] = &[
    ("alpha", "\\alpha"),
    ("beta", "\\beta"),
    ("gamma", "\\gamma"),
    ("delta", "\\delta"),
    ("epsilon", "\\epsilon"),
    ("zeta", "\\zeta"),
    ("eta", "\\eta"),
    ("theta", "\\theta"),
    ("lambda", "\\lambda"),
    ("mu", "\\mu"),
    ("nu", "\\nu"),
    ("xi", "\\xi"),
    ("pi", "\\pi"),
    ("rho", "\\rho"),
    ("sigma", "\\sigma"),
    ("tau", "\\tau"),
    ("phi", "\\phi"),
    ("chi", "\\chi"),
    ("psi", "\\psi"),
    ("omega", "\\omega"),
];

fn render_symbol(name: &str) -> String {
    // Pi is a named mathematical constant, not a user symbol that happens to
    // share a name with the Greek letter: always the lowercase glyph,
    // regardless of the capital-letter-triggers-\Pi rule below.
    if name == "Pi" {
        return "\\pi".to_string();
    }
    for (plain, cmd) in GREEK {
        if name.eq_ignore_ascii_case(plain) {
            return if name.chars().next().unwrap().is_uppercase() {
                format!("\\{}", capitalize(plain))
            } else {
                cmd.to_string()
            };
        }
    }
    name.to_string()
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn escape_text(s: &str) -> String {
    s.replace('\\', "\\textbackslash{}").replace('_', "\\_").replace('&', "\\&").replace('%', "\\%")
}

/// `Power[base, exp]` with a negative numeric exponent: the magnitude of the
/// exponent, for rendering as a reciprocal.
fn negative_power_exponent(e: &Expr) -> Option<Expr> {
    let (head, args) = e.as_normal()?;
    if head.as_symbol() != Some("Power") || args.len() != 2 {
        return None;
    }
    let is_neg = match &args[1] {
        Expr::Integer(n) => *n < 0,
        Expr::Real(x) => *x < 0.0,
        _ => false,
    };
    if !is_neg {
        return None;
    }
    Some(match &args[1] {
        Expr::Integer(n) => Expr::Integer(-n),
        Expr::Real(x) => Expr::Real(-x),
        other => other.clone(),
    })
}

fn split_fraction(e: &Expr) -> (Vec<Expr>, Vec<Expr>) {
    let factors: Vec<Expr> = match e.as_normal() {
        Some((head, args)) if head.as_symbol() == Some("Times") => args.to_vec(),
        _ => vec![e.clone()],
    };
    let mut num = Vec::new();
    let mut den = Vec::new();
    for f in factors {
        if let Some(mag) = negative_power_exponent(&f) {
            let base = f.as_normal().unwrap().1[0].clone();
            if mag.is_one() {
                den.push(base);
            } else {
                den.push(Expr::power(base, mag));
            }
        } else {
            num.push(f);
        }
    }
    (num, den)
}

/// True for a `Times` whose only "special" factor is a leading `-1`
/// (renders as unary negation rather than a `\frac` or product).
fn is_negation(args: &[Expr]) -> bool {
    matches!(args.first(), Some(Expr::Integer(-1))) && args.len() >= 2
}

fn render_normal(head: &Expr, args: &[Expr], whole: &Expr) -> String {
    if let Some(name) = head.as_symbol() {
        match name {
            "Plus" => return render_plus(args),
            "Times" => return render_times(whole),
            "Power" if args.len() == 2 => return render_power(&args[0], &args[1], whole),
            "List" => {
                let items: Vec<String> = args.iter().map(render).collect();
                return format!("\\left\\{{{}\\right\\}}", items.join(", "));
            }
            "Rule" if args.len() == 2 => return format!("{} \\to {}", render(&args[0]), render(&args[1])),
            "Equal" if args.len() == 2 => return format!("{} = {}", render(&args[0]), render(&args[1])),
            "Less" if args.len() == 2 => return format!("{} < {}", render(&args[0]), render(&args[1])),
            "Greater" if args.len() == 2 => return format!("{} > {}", render(&args[0]), render(&args[1])),
            "LessEqual" if args.len() == 2 => return format!("{} \\leq {}", render(&args[0]), render(&args[1])),
            "GreaterEqual" if args.len() == 2 => return format!("{} \\geq {}", render(&args[0]), render(&args[1])),
            "Unequal" if args.len() == 2 => return format!("{} \\neq {}", render(&args[0]), render(&args[1])),
            "Set" if args.len() == 2 => return format!("{} = {}", render(&args[0]), render(&args[1])),
            "SetDelayed" if args.len() == 2 => return format!("{} := {}", render(&args[0]), render(&args[1])),
            "Blank" => return format!("\\_{}", blank_type_suffix(args)),
            "BlankSequence" => return format!("\\_\\_{}", blank_type_suffix(args)),
            "BlankNullSequence" => return format!("\\_\\_\\_{}", blank_type_suffix(args)),
            "Pattern" if args.len() == 2 => return format!("{}{}", render(&args[0]), render(&args[1])),
            "Sin" | "Cos" | "Tan" | "Cot" | "Sec" | "Csc" if args.len() == 1 => {
                return format!("\\{}\\left({}\\right)", name.to_lowercase(), render(&args[0]));
            }
            "Log" if args.len() == 1 => return format!("\\ln\\left({}\\right)", render(&args[0])),
            "Exp" if args.len() == 1 => return format!("e^{{{}}}", render(&args[0])),
            "Sqrt" if args.len() == 1 => return format!("\\sqrt{{{}}}", render(&args[0])),
            "Abs" if args.len() == 1 => return format!("\\left|{}\\right|", render(&args[0])),
            _ => {}
        }
    }
    if let Some((n, func, call_args)) = derivative_form(whole) {
        let primes = "'".repeat(n.max(0) as usize);
        let arglist: Vec<String> = call_args.iter().map(render).collect();
        if n <= 3 {
            return format!("{}{}\\left({}\\right)", render(func), primes, arglist.join(", "));
        }
        return format!("{}^{{({})}}\\left({}\\right)", render(func), n, arglist.join(", "));
    }
    // Generic function application fallback. A multi-character name reads as
    // a named operator/function (`\operatorname{MyFunc}`), matching how CAS
    // output distinguishes "a function called MyFunc" from letters that
    // would otherwise run together and look like implied multiplication.
    // A single-character head (the common case for a dependent variable
    // like `x[t]` in an ODE) is just the plain italic symbol, `x(t)`.
    let head_str = match head.as_symbol() {
        Some(s) if s.chars().count() > 1 => format!("\\operatorname{{{}}}", s),
        Some(s) => render_symbol(s),
        None => render(head),
    };
    let arglist: Vec<String> = args.iter().map(render).collect();
    format!("{}\\left({}\\right)", head_str, arglist.join(", "))
}

fn render_plus(args: &[Expr]) -> String {
    if args.is_empty() {
        return "0".to_string();
    }
    let mut out = String::new();
    for (i, term) in args.iter().enumerate() {
        let (neg, positive_form) = plus_term_sign(term);
        if i == 0 {
            if neg {
                out.push('-');
            }
        } else {
            out.push_str(if neg { " - " } else { " + " });
        }
        out.push_str(&render(&positive_form));
    }
    out
}

fn plus_term_sign(term: &Expr) -> (bool, Expr) {
    match term {
        Expr::Integer(n) if *n < 0 => (true, Expr::Integer(-n)),
        Expr::Real(x) if *x < 0.0 => (true, Expr::Real(-x)),
        Expr::Normal { head, args } if head.as_symbol() == Some("Times") && !args.is_empty() => match &args[0] {
            Expr::Integer(n) if *n < 0 => {
                let mut rest = args.clone();
                rest[0] = Expr::Integer(-n);
                (true, rebuild_times(rest))
            }
            Expr::Real(x) if *x < 0.0 => {
                let mut rest = args.clone();
                rest[0] = Expr::Real(-x);
                (true, rebuild_times(rest))
            }
            _ => (false, term.clone()),
        },
        _ => (false, term.clone()),
    }
}

fn rebuild_times(mut factors: Vec<Expr>) -> Expr {
    if factors.len() == 1 {
        factors.pop().unwrap()
    } else if factors.first().map(|e| e.is_one()) == Some(true) {
        factors.remove(0);
        rebuild_times(factors)
    } else {
        Expr::times(factors)
    }
}

fn render_times(e: &Expr) -> String {
    let (num, den) = split_fraction(e);
    if den.is_empty() {
        if let Some((_h, args)) = e.as_normal() {
            if is_negation(args) {
                let rest: Vec<Expr> = args[1..].to_vec();
                let inner = if rest.len() == 1 { rest[0].clone() } else { Expr::times(rest) };
                return format!("-{}", render(&inner));
            }
        }
        render_factor_list(&num)
    } else {
        let num_str = if num.is_empty() { "1".to_string() } else { render_factor_list(&num) };
        let den_str = render_factor_list(&den);
        format!("\\frac{{{}}}{{{}}}", num_str, den_str)
    }
}

fn render_factor_list(factors: &[Expr]) -> String {
    if factors.is_empty() {
        return "1".to_string();
    }
    factors.iter().map(|f| render_atomic(f)).collect::<Vec<_>>().join(" ")
}

/// Render a factor, parenthesizing it if it is itself a sum (so `2(x+1)`
/// does not read as `2x+1`).
fn render_atomic(e: &Expr) -> String {
    if e.has_head("Plus") {
        format!("\\left({}\\right)", render(e))
    } else {
        render(e)
    }
}

fn render_power(base: &Expr, exp: &Expr, whole: &Expr) -> String {
    if negative_power_exponent(whole).is_some() {
        return render_times(whole);
    }
    // x^(1/2) -> sqrt(x); x^(-1/2) handled by the reciprocal branch above.
    if let Some((h, a)) = exp.as_normal() {
        if h.as_symbol() == Some("Power") && a.len() == 2 {
            if let (Expr::Integer(2), Expr::Integer(-1)) = (&a[0], &a[1]) {
                return format!("\\sqrt{{{}}}", render(base));
            }
        }
    }
    let base_str = if needs_parens_as_power_base(base) { format!("\\left({}\\right)", render(base)) } else { render(base) };
    format!("{}^{{{}}}", base_str, render(exp))
}

fn needs_parens_as_power_base(e: &Expr) -> bool {
    match e {
        Expr::Integer(n) => *n < 0,
        Expr::Real(x) => *x < 0.0,
        Expr::Normal { head, .. } => matches!(head.as_symbol(), Some("Plus") | Some("Times") | Some("Power")),
        _ => false,
    }
}

/// The type-restriction name for `Blank[head]` etc., or empty for the
/// untyped form. Mirrors `expr.rs`'s helper of the same name; kept separate
/// since LaTeX and InputForm rendering are deliberately independent (see
/// this module's doc comment).
fn blank_type_suffix(args: &[Expr]) -> String {
    match args {
        [Expr::Symbol(s)] => s.clone(),
        _ => String::new(),
    }
}

fn derivative_form(e: &Expr) -> Option<(i64, &Expr, &[Expr])> {
    let (outer_head, call_args) = e.as_normal()?;
    let (mid_head, f_args) = outer_head.as_normal()?;
    if f_args.len() != 1 {
        return None;
    }
    let f = &f_args[0];
    let (deriv_head, n_args) = mid_head.as_normal()?;
    if deriv_head.as_symbol() != Some("Derivative") || n_args.len() != 1 {
        return None;
    }
    let n = match &n_args[0] {
        Expr::Integer(n) => *n,
        _ => return None,
    };
    Some((n, f, call_args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atoms() {
        assert_eq!(to_latex(&Expr::integer(3)), "3");
        assert_eq!(to_latex(&Expr::real(2.0)), "2.");
        assert_eq!(to_latex(&Expr::symbol("x")), "x");
        assert_eq!(to_latex(&Expr::symbol("pi")), "\\pi");
        assert_eq!(to_latex(&Expr::symbol("theta")), "\\theta");
    }

    #[test]
    fn plus_and_negative_terms() {
        let e = Expr::plus(vec![Expr::symbol("x"), Expr::times(vec![Expr::integer(-1), Expr::symbol("y")])]);
        assert_eq!(to_latex(&e), "x - y");
    }

    #[test]
    fn fraction_rendering() {
        let e = Expr::power(Expr::integer(2), Expr::integer(-1));
        assert_eq!(to_latex(&e), "\\frac{1}{2}");

        let e2 = Expr::times(vec![Expr::integer(3), Expr::symbol("x"), Expr::power(Expr::integer(2), Expr::integer(-1))]);
        assert_eq!(to_latex(&e2), "\\frac{3 x}{2}");
    }

    #[test]
    fn power_rendering() {
        assert_eq!(to_latex(&Expr::power(Expr::symbol("x"), Expr::integer(2))), "x^{2}");
        let sum_base = Expr::power(Expr::plus(vec![Expr::symbol("x"), Expr::integer(1)]), Expr::integer(2));
        assert_eq!(to_latex(&sum_base), "\\left(x + 1\\right)^{2}");
    }

    #[test]
    fn sqrt_from_half_exponent() {
        let e = Expr::power(Expr::symbol("x"), Expr::power(Expr::integer(2), Expr::integer(-1)));
        assert_eq!(to_latex(&e), "\\sqrt{x}");
    }

    #[test]
    fn known_functions() {
        assert_eq!(to_latex(&Expr::call("Sin", vec![Expr::symbol("t")])), "\\sin\\left(t\\right)");
        assert_eq!(to_latex(&Expr::call("Sqrt", vec![Expr::integer(2)])), "\\sqrt{2}");
        assert_eq!(to_latex(&Expr::call("Log", vec![Expr::symbol("x")])), "\\ln\\left(x\\right)");
        assert_eq!(to_latex(&Expr::call("Abs", vec![Expr::symbol("x")])), "\\left|x\\right|");
    }

    #[test]
    fn derivative_primes() {
        let d1 = Expr::normal(
            Expr::normal(Expr::normal(Expr::symbol("Derivative"), vec![Expr::integer(1)]), vec![Expr::symbol("x")]),
            vec![Expr::symbol("t")],
        );
        assert_eq!(to_latex(&d1), "x'\\left(t\\right)");

        let d2 = Expr::normal(
            Expr::normal(Expr::normal(Expr::symbol("Derivative"), vec![Expr::integer(2)]), vec![Expr::symbol("x")]),
            vec![Expr::symbol("t")],
        );
        assert_eq!(to_latex(&d2), "x''\\left(t\\right)");
    }

    #[test]
    fn unknown_function_falls_back_to_operatorname() {
        assert_eq!(to_latex(&Expr::call("MyFunc", vec![Expr::symbol("x")])), "\\operatorname{MyFunc}\\left(x\\right)");
    }

    #[test]
    fn list_rendering() {
        let e = Expr::list(vec![Expr::integer(1), Expr::integer(2)]);
        assert_eq!(to_latex(&e), "\\left\\{1, 2\\right\\}");
    }

    #[test]
    fn equal_and_rule() {
        assert_eq!(to_latex(&Expr::equal(Expr::symbol("x"), Expr::integer(1))), "x = 1");
        assert_eq!(to_latex(&Expr::rule(Expr::symbol("x"), Expr::integer(1))), "x \\to 1");
    }

    #[test]
    fn pi_renders_lowercase_regardless_of_capitalization_rule() {
        assert_eq!(to_latex(&Expr::symbol("Pi")), "\\pi");
    }

    #[test]
    fn pattern_forms_render() {
        assert_eq!(to_latex(&Expr::blank()), "\\_");
        assert_eq!(to_latex(&Expr::blank_typed("Integer")), "\\_Integer");
        assert_eq!(to_latex(&Expr::named_pattern("x", Expr::blank())), "x\\_");
    }

    #[test]
    fn comparison_and_assignment_render() {
        assert_eq!(to_latex(&Expr::less(Expr::symbol("a"), Expr::symbol("b"))), "a < b");
        assert_eq!(to_latex(&Expr::less_equal(Expr::symbol("a"), Expr::symbol("b"))), "a \\leq b");
        assert_eq!(to_latex(&Expr::set(Expr::symbol("a"), Expr::integer(5))), "a = 5");
    }

    #[test]
    fn coefficient_times_sum_gets_parens() {
        let e = Expr::times(vec![Expr::integer(2), Expr::plus(vec![Expr::symbol("x"), Expr::integer(1)])]);
        assert_eq!(to_latex(&e), "2 \\left(x + 1\\right)");
    }
}
