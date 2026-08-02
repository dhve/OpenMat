//! The universal expression type.
//!
//! Everything in OpenMat is `Head[arg1, arg2, ...]`, exactly as in Wolfram Language.
//! `2 + 3 x` is really `Plus[2, Times[3, x]]`; a list is `List[a, b, c]`; a rule
//! `x -> 1` is `Rule[x, 1]`. Atoms (integers, reals, symbols, strings) are the
//! leaves of that tree.
//!
//! Symbols are plain `String`s rather than interned handles. Interning would
//! speed up comparisons and cut memory, but it needs a global or thread-local
//! table to stay ergonomic, and this crate has no dependencies to reach for one.
//! It is a reasonable future optimization, not a correctness concern.

use std::fmt;

/// A single OpenMat expression: an atom, or `head[args...]`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer(i64),
    Real(f64),
    Symbol(String),
    Str(String),
    Normal { head: Box<Expr>, args: Vec<Expr> },
}

impl Expr {
    pub fn integer(n: i64) -> Expr {
        Expr::Integer(n)
    }

    pub fn real(x: f64) -> Expr {
        Expr::Real(x)
    }

    pub fn symbol(name: impl Into<String>) -> Expr {
        Expr::Symbol(name.into())
    }

    pub fn string(s: impl Into<String>) -> Expr {
        Expr::Str(s.into())
    }

    /// Build `head[args...]`.
    pub fn normal(head: Expr, args: Vec<Expr>) -> Expr {
        Expr::Normal { head: Box::new(head), args }
    }

    /// Build `name[args...]` where `name` is a plain symbol head, the common case.
    pub fn call(name: impl Into<String>, args: Vec<Expr>) -> Expr {
        Expr::normal(Expr::symbol(name), args)
    }

    pub fn plus(args: Vec<Expr>) -> Expr {
        Expr::call("Plus", args)
    }

    pub fn times(args: Vec<Expr>) -> Expr {
        Expr::call("Times", args)
    }

    pub fn power(base: Expr, exp: Expr) -> Expr {
        Expr::call("Power", vec![base, exp])
    }

    pub fn list(items: Vec<Expr>) -> Expr {
        Expr::call("List", items)
    }

    pub fn equal(lhs: Expr, rhs: Expr) -> Expr {
        Expr::call("Equal", vec![lhs, rhs])
    }

    pub fn rule(lhs: Expr, rhs: Expr) -> Expr {
        Expr::call("Rule", vec![lhs, rhs])
    }

    /// `Set[lhs, rhs]`, the assignment `lhs = rhs`.
    pub fn set(lhs: Expr, rhs: Expr) -> Expr {
        Expr::call("Set", vec![lhs, rhs])
    }

    /// `SetDelayed[lhs, rhs]`, the delayed assignment `lhs := rhs`.
    pub fn set_delayed(lhs: Expr, rhs: Expr) -> Expr {
        Expr::call("SetDelayed", vec![lhs, rhs])
    }

    pub fn less(lhs: Expr, rhs: Expr) -> Expr {
        Expr::call("Less", vec![lhs, rhs])
    }

    pub fn greater(lhs: Expr, rhs: Expr) -> Expr {
        Expr::call("Greater", vec![lhs, rhs])
    }

    pub fn less_equal(lhs: Expr, rhs: Expr) -> Expr {
        Expr::call("LessEqual", vec![lhs, rhs])
    }

    pub fn greater_equal(lhs: Expr, rhs: Expr) -> Expr {
        Expr::call("GreaterEqual", vec![lhs, rhs])
    }

    pub fn unequal(lhs: Expr, rhs: Expr) -> Expr {
        Expr::call("Unequal", vec![lhs, rhs])
    }

    /// `Blank[]`, the pattern `_`, matches anything.
    pub fn blank() -> Expr {
        Expr::call("Blank", vec![])
    }

    /// `Blank[head]`, the pattern `_head` (e.g. `_Integer`), matches anything
    /// whose [`head_name`] equals `head`.
    pub fn blank_typed(head: impl Into<String>) -> Expr {
        Expr::call("Blank", vec![Expr::symbol(head)])
    }

    /// `Pattern[name, sub]`, the pattern `name_` or `name_head`, binds `name`
    /// to whatever `sub` matches.
    pub fn named_pattern(name: impl Into<String>, sub: Expr) -> Expr {
        Expr::call("Pattern", vec![Expr::symbol(name), sub])
    }

    /// `BlankSequence[]`, the pattern `__`, matches one or more expressions.
    /// Parse-only in this pass: [`crate::pattern`] does not yet match it.
    pub fn blank_sequence() -> Expr {
        Expr::call("BlankSequence", vec![])
    }

    /// `BlankSequence[head]`, the pattern `__head`.
    pub fn blank_sequence_typed(head: impl Into<String>) -> Expr {
        Expr::call("BlankSequence", vec![Expr::symbol(head)])
    }

    /// `BlankNullSequence[]`, the pattern `___`, matches zero or more
    /// expressions. Parse-only in this pass, as with [`Expr::blank_sequence`].
    pub fn blank_null_sequence() -> Expr {
        Expr::call("BlankNullSequence", vec![])
    }

    /// `BlankNullSequence[head]`, the pattern `___head`.
    pub fn blank_null_sequence_typed(head: impl Into<String>) -> Expr {
        Expr::call("BlankNullSequence", vec![Expr::symbol(head)])
    }

    /// True for `Integer`/`Real` atoms.
    pub fn is_numeric(&self) -> bool {
        matches!(self, Expr::Integer(_) | Expr::Real(_))
    }

    /// True for `Integer(0)` or `Real(0.0)`.
    pub fn is_zero(&self) -> bool {
        matches!(self, Expr::Integer(0)) || matches!(self, Expr::Real(x) if *x == 0.0)
    }

    /// True for `Integer(1)`.
    pub fn is_one(&self) -> bool {
        matches!(self, Expr::Integer(1))
    }

    /// The symbol name if this expression is a bare `Symbol`.
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Expr::Symbol(s) => Some(s),
            _ => None,
        }
    }

    /// The head and args if this expression is `Normal`.
    pub fn as_normal(&self) -> Option<(&Expr, &[Expr])> {
        match self {
            Expr::Normal { head, args } => Some((head, args)),
            _ => None,
        }
    }

    /// True if this expression is `Normal` with a plain symbol head equal to `name`.
    pub fn has_head(&self, name: &str) -> bool {
        matches!(self, Expr::Normal { head, .. } if head.as_symbol() == Some(name))
    }

    /// The `FullForm` type head used by `Blank[head]` pattern matching:
    /// `"Integer"`, `"Real"`, `"Symbol"`, `"String"`, or (for `Normal` expressions)
    /// the name of the expression's own head.
    pub fn head_name(&self) -> String {
        match self {
            Expr::Integer(_) => "Integer".to_string(),
            Expr::Real(_) => "Real".to_string(),
            Expr::Symbol(_) => "Symbol".to_string(),
            Expr::Str(_) => "String".to_string(),
            Expr::Normal { head, .. } => match head.as_symbol() {
                Some(s) => s.to_string(),
                None => head.to_string(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Display: Wolfram InputForm
// ---------------------------------------------------------------------------

// Precedence tiers used to decide when a subexpression needs parentheses.
// Higher binds tighter. Kept private: this is a rendering concern only.
const PREC_ASSIGN: u8 = 1;
const PREC_RULE: u8 = 2;
const PREC_EQUAL: u8 = 3;
const PREC_ADD: u8 = 4;
const PREC_MUL: u8 = 5;
const PREC_UNARY: u8 = 6;
const PREC_POW: u8 = 7;
const PREC_ATOM: u8 = 8;

/// A `Power[base, exp]` where `exp` is a negative numeric literal displays as
/// a reciprocal (`1/base` or `.../base^n`). Returns the positive magnitude of
/// the exponent when this applies.
fn negative_power_exponent(e: &Expr) -> Option<&Expr> {
    if let Expr::Normal { head, args } = e {
        if head.as_symbol() == Some("Power") && args.len() == 2 {
            let is_neg = match &args[1] {
                Expr::Integer(n) => *n < 0,
                Expr::Real(x) => *x < 0.0,
                _ => false,
            };
            if is_neg {
                return Some(&args[0]);
            }
        }
    }
    None
}

fn negated_exponent(e: &Expr) -> Expr {
    match e {
        Expr::Integer(n) => Expr::Integer(-n),
        Expr::Real(x) => Expr::Real(-x),
        other => Expr::normal(Expr::symbol("Times"), vec![Expr::Integer(-1), other.clone()]),
    }
}

/// Split a `Times[...]` (or a single non-Times factor) into numerator and
/// denominator factors, based on which factors are `Power[_, negative]`.
fn split_fraction(e: &Expr) -> (Vec<Expr>, Vec<Expr>) {
    let factors: Vec<Expr> = match e {
        Expr::Normal { head, args } if head.as_symbol() == Some("Times") => args.clone(),
        other => vec![other.clone()],
    };
    let mut num = Vec::new();
    let mut den = Vec::new();
    for f in factors {
        if let Some(base) = negative_power_exponent(&f) {
            let (_h, args) = f.as_normal().unwrap();
            let mag = negated_exponent(&args[1]);
            if mag.is_one() {
                den.push(base.clone());
            } else {
                den.push(Expr::power(base.clone(), mag));
            }
        } else {
            num.push(f);
        }
    }
    (num, den)
}

fn own_precedence(e: &Expr) -> u8 {
    match e {
        Expr::Integer(n) if *n < 0 => PREC_UNARY,
        Expr::Real(x) if *x < 0.0 => PREC_UNARY,
        Expr::Integer(_) | Expr::Real(_) | Expr::Symbol(_) | Expr::Str(_) => PREC_ATOM,
        Expr::Normal { head, args } => match head.as_symbol() {
            Some("Plus") => PREC_ADD,
            Some("Set") | Some("SetDelayed") if args.len() == 2 => PREC_ASSIGN,
            Some("Rule") if args.len() == 2 => PREC_RULE,
            Some("Equal") | Some("Less") | Some("Greater") | Some("LessEqual") | Some("GreaterEqual") | Some("Unequal")
                if args.len() == 2 =>
            {
                PREC_EQUAL
            }
            Some("Times") => {
                let (_num, den) = split_fraction(e);
                if den.is_empty() {
                    if is_negation(e) {
                        PREC_UNARY
                    } else {
                        PREC_MUL
                    }
                } else {
                    PREC_MUL
                }
            }
            Some("Power") if args.len() == 2 => {
                if negative_power_exponent(e).is_some() {
                    PREC_MUL
                } else {
                    PREC_POW
                }
            }
            Some("List") => PREC_ATOM,
            _ => {
                if derivative_form(e).is_some() {
                    PREC_ATOM
                } else {
                    PREC_ATOM
                }
            }
        },
    }
}

/// `Times[-1, rest...]` with nothing else numeric: displays as `-rest`.
fn is_negation(e: &Expr) -> bool {
    if let Expr::Normal { head, args } = e {
        if head.as_symbol() == Some("Times") && args.len() >= 2 {
            if let Expr::Integer(-1) = args[0] {
                return true;
            }
        }
    }
    false
}

/// The type-restriction name for `Blank[head]` / `BlankSequence[head]` /
/// `BlankNullSequence[head]` printing (`_Integer` etc.), or empty for the
/// untyped form.
fn blank_type_suffix(args: &[Expr]) -> String {
    match args {
        [Expr::Symbol(s)] => s.clone(),
        _ => String::new(),
    }
}

/// Detects the triple-nested `Derivative[n][f][t1, t2, ...]` shape built by
/// the parser's postfix-prime syntax, returning `(n, f, call_args)`.
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

fn write_paren_if(f: &mut fmt::Formatter<'_>, e: &Expr, min_prec: u8) -> fmt::Result {
    if own_precedence(e) < min_prec {
        write!(f, "(")?;
        fmt_expr(e, f)?;
        write!(f, ")")
    } else {
        fmt_expr(e, f)
    }
}

fn fmt_real(x: f64, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if x.is_nan() {
        return write!(f, "Indeterminate");
    }
    if x.is_infinite() {
        return write!(f, "{}Infinity", if x < 0.0 { "-" } else { "" });
    }
    let s = format!("{}", x);
    if s.contains('.') || s.contains('e') || s.contains('E') {
        write!(f, "{}", s)
    } else {
        write!(f, "{}.", s)
    }
}

fn fmt_string_literal(s: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "\"")?;
    for c in s.chars() {
        match c {
            '"' => write!(f, "\\\"")?,
            '\\' => write!(f, "\\\\")?,
            _ => write!(f, "{}", c)?,
        }
    }
    write!(f, "\"")
}

fn fmt_plus(args: &[Expr], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if args.is_empty() {
        return write!(f, "0");
    }
    for (i, term) in args.iter().enumerate() {
        let (sign_neg, printable) = plus_term_sign(term);
        if i == 0 {
            if sign_neg {
                write!(f, "-")?;
            }
        } else {
            write!(f, " {} ", if sign_neg { "-" } else { "+" })?;
        }
        write_paren_if(f, &printable, PREC_ADD + 1)?;
    }
    Ok(())
}

/// Returns `(true, positive_form)` when `term` should be displayed as a
/// subtracted term (a negative literal, or a `Times` with negative leading
/// coefficient).
fn plus_term_sign(term: &Expr) -> (bool, Expr) {
    match term {
        Expr::Integer(n) if *n < 0 => (true, Expr::Integer(-n)),
        Expr::Real(x) if *x < 0.0 => (true, Expr::Real(-x)),
        Expr::Normal { head, args } if head.as_symbol() == Some("Times") && !args.is_empty() => {
            match &args[0] {
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
            }
        }
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

fn fmt_times_or_power(e: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let (num, den) = split_fraction(e);
    if den.is_empty() {
        fmt_factor_list(&num, f)
    } else {
        if num.is_empty() {
            write!(f, "1")?;
        } else {
            fmt_factor_list(&num, f)?;
        }
        write!(f, "/")?;
        if den.len() == 1 {
            write_paren_if(f, &den[0], PREC_MUL + 1)?;
        } else {
            write!(f, "(")?;
            fmt_factor_list(&den, f)?;
            write!(f, ")")?;
        }
        Ok(())
    }
}

fn fmt_factor_list(factors: &[Expr], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if factors.is_empty() {
        return write!(f, "1");
    }
    for (i, factor) in factors.iter().enumerate() {
        if i > 0 {
            write!(f, "*")?;
        }
        write_paren_if(f, factor, PREC_MUL + 1)?;
    }
    Ok(())
}

fn fmt_expr(e: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match e {
        Expr::Integer(n) => write!(f, "{}", n),
        Expr::Real(x) => fmt_real(*x, f),
        Expr::Symbol(s) => write!(f, "{}", s),
        Expr::Str(s) => fmt_string_literal(s, f),
        Expr::Normal { head, args } => {
            if let Some(name) = head.as_symbol() {
                match name {
                    "Plus" => return fmt_plus(args, f),
                    "Times" => return fmt_times_or_power(e, f),
                    "Power" if args.len() == 2 => {
                        if negative_power_exponent(e).is_some() {
                            return fmt_times_or_power(e, f);
                        }
                        write_paren_if(f, &args[0], PREC_POW + 1)?;
                        write!(f, "^")?;
                        return write_paren_if(f, &args[1], PREC_POW);
                    }
                    "List" => {
                        write!(f, "{{")?;
                        for (i, a) in args.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            fmt_expr(a, f)?;
                        }
                        return write!(f, "}}");
                    }
                    "Rule" if args.len() == 2 => {
                        write_paren_if(f, &args[0], PREC_RULE + 1)?;
                        write!(f, " -> ")?;
                        return write_paren_if(f, &args[1], PREC_RULE + 1);
                    }
                    "Equal" if args.len() == 2 => {
                        write_paren_if(f, &args[0], PREC_EQUAL + 1)?;
                        write!(f, " == ")?;
                        return write_paren_if(f, &args[1], PREC_EQUAL + 1);
                    }
                    "Less" if args.len() == 2 => {
                        write_paren_if(f, &args[0], PREC_EQUAL + 1)?;
                        write!(f, " < ")?;
                        return write_paren_if(f, &args[1], PREC_EQUAL + 1);
                    }
                    "Greater" if args.len() == 2 => {
                        write_paren_if(f, &args[0], PREC_EQUAL + 1)?;
                        write!(f, " > ")?;
                        return write_paren_if(f, &args[1], PREC_EQUAL + 1);
                    }
                    "LessEqual" if args.len() == 2 => {
                        write_paren_if(f, &args[0], PREC_EQUAL + 1)?;
                        write!(f, " <= ")?;
                        return write_paren_if(f, &args[1], PREC_EQUAL + 1);
                    }
                    "GreaterEqual" if args.len() == 2 => {
                        write_paren_if(f, &args[0], PREC_EQUAL + 1)?;
                        write!(f, " >= ")?;
                        return write_paren_if(f, &args[1], PREC_EQUAL + 1);
                    }
                    "Unequal" if args.len() == 2 => {
                        write_paren_if(f, &args[0], PREC_EQUAL + 1)?;
                        write!(f, " != ")?;
                        return write_paren_if(f, &args[1], PREC_EQUAL + 1);
                    }
                    "Set" if args.len() == 2 => {
                        write_paren_if(f, &args[0], PREC_ASSIGN + 1)?;
                        write!(f, " = ")?;
                        return write_paren_if(f, &args[1], PREC_ASSIGN + 1);
                    }
                    "SetDelayed" if args.len() == 2 => {
                        write_paren_if(f, &args[0], PREC_ASSIGN + 1)?;
                        write!(f, " := ")?;
                        return write_paren_if(f, &args[1], PREC_ASSIGN + 1);
                    }
                    "Blank" => return write!(f, "_{}", blank_type_suffix(args)),
                    "BlankSequence" => return write!(f, "__{}", blank_type_suffix(args)),
                    "BlankNullSequence" => return write!(f, "___{}", blank_type_suffix(args)),
                    "Pattern" if args.len() == 2 => {
                        fmt_expr(&args[0], f)?;
                        return fmt_expr(&args[1], f);
                    }
                    _ => {}
                }
            }
            if let Some((n, func, call_args)) = derivative_form(e) {
                fmt_expr(func, f)?;
                for _ in 0..n {
                    write!(f, "'")?;
                }
                write!(f, "[")?;
                for (i, a) in call_args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    fmt_expr(a, f)?;
                }
                return write!(f, "]");
            }
            fmt_expr(head, f)?;
            write!(f, "[")?;
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                fmt_expr(a, f)?;
            }
            write!(f, "]")
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_expr(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atoms_display() {
        assert_eq!(Expr::integer(3).to_string(), "3");
        assert_eq!(Expr::integer(-3).to_string(), "-3");
        assert_eq!(Expr::real(2.0).to_string(), "2.");
        assert_eq!(Expr::real(3.14).to_string(), "3.14");
        assert_eq!(Expr::symbol("x").to_string(), "x");
        assert_eq!(Expr::string("hi").to_string(), "\"hi\"");
    }

    #[test]
    fn plus_times_display_from_spec_example() {
        // Plus[a, Times[2, b]] displays as a + 2*b
        let e = Expr::plus(vec![Expr::symbol("a"), Expr::times(vec![Expr::integer(2), Expr::symbol("b")])]);
        assert_eq!(e.to_string(), "a + 2*b");
    }

    #[test]
    fn power_display() {
        let e = Expr::power(Expr::symbol("x"), Expr::integer(2));
        assert_eq!(e.to_string(), "x^2");
    }

    #[test]
    fn power_of_sum_needs_parens() {
        let e = Expr::power(Expr::plus(vec![Expr::symbol("x"), Expr::integer(1)]), Expr::integer(2));
        assert_eq!(e.to_string(), "(x + 1)^2");
    }

    #[test]
    fn negative_term_displays_as_subtraction() {
        let e = Expr::plus(vec![Expr::symbol("x"), Expr::times(vec![Expr::integer(-1), Expr::symbol("y")])]);
        assert_eq!(e.to_string(), "x - y");
    }

    #[test]
    fn list_display() {
        let e = Expr::list(vec![Expr::integer(1), Expr::integer(2), Expr::integer(3)]);
        assert_eq!(e.to_string(), "{1, 2, 3}");
    }

    #[test]
    fn equal_and_rule_display() {
        assert_eq!(Expr::equal(Expr::symbol("x"), Expr::integer(1)).to_string(), "x == 1");
        assert_eq!(Expr::rule(Expr::symbol("x"), Expr::integer(1)).to_string(), "x -> 1");
    }

    #[test]
    fn function_call_display() {
        let e = Expr::call("Sin", vec![Expr::symbol("t")]);
        assert_eq!(e.to_string(), "Sin[t]");
    }

    #[test]
    fn pattern_forms_display_compactly() {
        assert_eq!(Expr::blank().to_string(), "_");
        assert_eq!(Expr::blank_typed("Integer").to_string(), "_Integer");
        assert_eq!(Expr::blank_sequence().to_string(), "__");
        assert_eq!(Expr::blank_null_sequence().to_string(), "___");
        assert_eq!(Expr::named_pattern("x", Expr::blank()).to_string(), "x_");
        assert_eq!(Expr::named_pattern("x", Expr::blank_typed("Integer")).to_string(), "x_Integer");
    }

    #[test]
    fn comparison_and_assignment_display() {
        assert_eq!(Expr::less(Expr::symbol("a"), Expr::symbol("b")).to_string(), "a < b");
        assert_eq!(Expr::greater_equal(Expr::symbol("a"), Expr::symbol("b")).to_string(), "a >= b");
        assert_eq!(Expr::unequal(Expr::symbol("a"), Expr::symbol("b")).to_string(), "a != b");
        assert_eq!(Expr::set(Expr::symbol("a"), Expr::integer(5)).to_string(), "a = 5");
        assert_eq!(
            Expr::set_delayed(Expr::call("f", vec![Expr::named_pattern("x", Expr::blank())]), Expr::power(Expr::symbol("x"), Expr::integer(2)))
                .to_string(),
            "f[x_] := x^2"
        );
    }

    #[test]
    fn head_name_for_blank_matching() {
        assert_eq!(Expr::integer(3).head_name(), "Integer");
        assert_eq!(Expr::real(1.0).head_name(), "Real");
        assert_eq!(Expr::symbol("x").head_name(), "Symbol");
        assert_eq!(Expr::string("x").head_name(), "String");
        assert_eq!(Expr::call("Plus", vec![]).head_name(), "Plus");
    }
}
