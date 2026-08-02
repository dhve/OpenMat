//! Recursive-descent parser for the WL-shaped linear syntax subset.
//!
//! Precedence, loosest to tightest: `->` (Rule) < `==` (Equal) < `+ -` (Plus) <
//! `* /` and implicit multiplication (Times) < unary `-` < `^` (Power, right
//! associative) < postfix `'` (Derivative) and `[...]` (function application).
//!
//! Implicit multiplication ("2 x", "c x[t]") is handled inside the
//! multiplicative parse level: after consuming a factor, if the next token
//! can start a new primary and isn't an explicit `*`/`/`, a multiplication is
//! inferred.

use crate::expr::Expr;
use crate::lexer::{Lexer, Token, TokenKind};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub pos: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at position {}: {}", self.pos, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse a complete expression from `src`, requiring the whole input to be consumed.
pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let tokens = Lexer::new(src).tokenize().map_err(|e| ParseError { message: e.message, pos: e.pos })?;
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.parse_expr()?;
    parser.expect(&TokenKind::Eof)?;
    Ok(expr)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<Token, ParseError> {
        if self.peek_kind() == kind {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("expected {}, found {}", describe(kind), describe(self.peek_kind())),
                pos: self.peek().pos,
            })
        }
    }

    /// Entry point for a full expression: used at the top level and anywhere
    /// a subexpression is nested (parens, list items, function arguments).
    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_assign()
    }

    /// `lhs = rhs` (`Set`) and `lhs := rhs` (`SetDelayed`): the loosest
    /// binding forms, so `f[x_] := x^2` reads as "assign the whole rest of
    /// the line as the definition." Right associative, so `a = b = 5`
    /// chains as `Set[a, Set[b, 5]]`, matching Wolfram Language.
    fn parse_assign(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_rule()?;
        match self.peek_kind() {
            TokenKind::Equal => {
                self.advance();
                let rhs = self.parse_assign()?;
                Ok(Expr::set(lhs, rhs))
            }
            TokenKind::ColonEqual => {
                self.advance();
                let rhs = self.parse_assign()?;
                Ok(Expr::set_delayed(lhs, rhs))
            }
            _ => Ok(lhs),
        }
    }

    fn parse_rule(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_map()?;
        if *self.peek_kind() == TokenKind::Arrow {
            self.advance();
            let rhs = self.parse_rule()?; // right associative
            return Ok(Expr::rule(lhs, rhs));
        }
        Ok(lhs)
    }

    /// `f /@ expr` (`Map`), a cheap infix spelling of `Map[f, expr]`. Sits
    /// between `->` (looser) and the relational operators (tighter); see
    /// specs/grammar.md v0.2 section 2.
    fn parse_map(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_equal()?;
        if *self.peek_kind() == TokenKind::MapArrow {
            self.advance();
            let rhs = self.parse_map()?; // right associative
            return Ok(Expr::call("Map", vec![lhs, rhs]));
        }
        Ok(lhs)
    }

    fn parse_equal(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_additive()?;
        loop {
            match self.peek_kind() {
                TokenKind::EqualEqual => {
                    self.advance();
                    let rhs = self.parse_additive()?;
                    lhs = Expr::equal(lhs, rhs);
                }
                TokenKind::NotEqual => {
                    self.advance();
                    let rhs = self.parse_additive()?;
                    lhs = Expr::unequal(lhs, rhs);
                }
                TokenKind::Less => {
                    self.advance();
                    let rhs = self.parse_additive()?;
                    lhs = Expr::less(lhs, rhs);
                }
                TokenKind::Greater => {
                    self.advance();
                    let rhs = self.parse_additive()?;
                    lhs = Expr::greater(lhs, rhs);
                }
                TokenKind::LessEqual => {
                    self.advance();
                    let rhs = self.parse_additive()?;
                    lhs = Expr::less_equal(lhs, rhs);
                }
                TokenKind::GreaterEqual => {
                    self.advance();
                    let rhs = self.parse_additive()?;
                    lhs = Expr::greater_equal(lhs, rhs);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut args = vec![self.parse_multiplicative()?];
        loop {
            match self.peek_kind() {
                TokenKind::Plus => {
                    self.advance();
                    args.push(self.parse_multiplicative()?);
                }
                TokenKind::Minus => {
                    self.advance();
                    let rhs = self.parse_multiplicative()?;
                    args.push(negate(rhs));
                }
                _ => break,
            }
        }
        Ok(if args.len() == 1 { args.pop().unwrap() } else { Expr::plus(args) })
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut factors = vec![self.parse_unary()?];
        loop {
            match self.peek_kind() {
                TokenKind::Star => {
                    self.advance();
                    factors.push(self.parse_unary()?);
                }
                TokenKind::Slash => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    factors.push(Expr::power(rhs, Expr::integer(-1)));
                }
                k if starts_primary(k) => {
                    // implicit multiplication: "2 x", "c x[t]", "3 Sin[t]"
                    factors.push(self.parse_unary()?);
                }
                _ => break,
            }
        }
        Ok(if factors.len() == 1 { factors.pop().unwrap() } else { Expr::times(factors) })
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if *self.peek_kind() == TokenKind::Minus {
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(negate(inner));
        }
        if *self.peek_kind() == TokenKind::Plus {
            self.advance();
            return self.parse_unary();
        }
        self.parse_power()
    }

    fn parse_power(&mut self) -> Result<Expr, ParseError> {
        let base = self.parse_postfix()?;
        if *self.peek_kind() == TokenKind::Caret {
            self.advance();
            let exp = self.parse_unary()?; // recursion into parse_power gives right associativity
            return Ok(Expr::power(base, exp));
        }
        Ok(base)
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                TokenKind::Prime => {
                    let mut n = 0i64;
                    while *self.peek_kind() == TokenKind::Prime {
                        self.advance();
                        n += 1;
                    }
                    let deriv_op = Expr::normal(Expr::symbol("Derivative"), vec![Expr::integer(n)]);
                    expr = Expr::normal(deriv_op, vec![expr]);
                }
                TokenKind::LBracket => {
                    self.advance();
                    let args = self.parse_comma_list(&TokenKind::RBracket)?;
                    self.expect(&TokenKind::RBracket)?;
                    expr = Expr::normal(expr, args);
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Integer(n) => {
                self.advance();
                Ok(Expr::integer(n))
            }
            TokenKind::Real(x) => {
                self.advance();
                Ok(Expr::real(x))
            }
            TokenKind::Symbol(s) => {
                self.advance();
                let sym_end = tok.pos + s.len();
                if let TokenKind::Blank(n) = *self.peek_kind() {
                    if self.peek().pos == sym_end {
                        let blank_tok = self.advance();
                        let blank_end = blank_tok.pos + n as usize;
                        let type_name = self.try_adjacent_symbol(blank_end);
                        return Ok(Expr::named_pattern(s, build_blank(n, type_name)));
                    }
                }
                Ok(Expr::symbol(s))
            }
            TokenKind::Blank(n) => {
                self.advance();
                let blank_end = tok.pos + n as usize;
                let type_name = self.try_adjacent_symbol(blank_end);
                Ok(build_blank(n, type_name))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::string(s))
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(inner)
            }
            TokenKind::LBrace => {
                self.advance();
                let items = self.parse_comma_list(&TokenKind::RBrace)?;
                self.expect(&TokenKind::RBrace)?;
                Ok(Expr::list(items))
            }
            other => Err(ParseError { message: format!("unexpected {}", describe(&other)), pos: tok.pos }),
        }
    }

    /// If the next token is a `Symbol` immediately adjacent to `end_pos`
    /// (no whitespace between), consume and return it: the type restriction
    /// in `_Integer`, `x_Integer`, `__Real`, and so on. Adjacency (tracked
    /// via byte positions, not a lexer-level fusion) is what lets `_Integer`
    /// bind tighter than `_  Integer` (implicit multiplication of a bare
    /// blank and a symbol), matching how real Wolfram Language treats
    /// whitespace as significant around patterns.
    fn try_adjacent_symbol(&mut self, end_pos: usize) -> Option<String> {
        if let TokenKind::Symbol(name) = self.peek_kind() {
            if self.peek().pos == end_pos {
                let name = name.clone();
                self.advance();
                return Some(name);
            }
        }
        None
    }

    fn parse_comma_list(&mut self, closing: &TokenKind) -> Result<Vec<Expr>, ParseError> {
        let mut items = Vec::new();
        if self.peek_kind() == closing {
            return Ok(items);
        }
        loop {
            items.push(self.parse_expr()?);
            if *self.peek_kind() == TokenKind::Comma {
                self.advance();
                continue;
            }
            break;
        }
        Ok(items)
    }
}

fn starts_primary(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Integer(_)
            | TokenKind::Real(_)
            | TokenKind::Symbol(_)
            | TokenKind::Str(_)
            | TokenKind::LParen
            | TokenKind::Blank(_)
    )
}

/// Negate a parsed expression for unary minus / binary subtraction, folding
/// into numeric literals and existing `Times` coefficients where possible so
/// the parse tree stays tidy without needing a full evaluation pass.
fn negate(e: Expr) -> Expr {
    match &e {
        Expr::Integer(n) => Expr::Integer(-n),
        Expr::Real(x) => Expr::Real(-x),
        Expr::Normal { head, args } if head.as_symbol() == Some("Times") => {
            let mut new_args = args.clone();
            match new_args.first().cloned() {
                Some(Expr::Integer(n)) => {
                    new_args[0] = Expr::Integer(-n);
                    Expr::times(new_args)
                }
                Some(Expr::Real(x)) => {
                    new_args[0] = Expr::Real(-x);
                    Expr::times(new_args)
                }
                _ => {
                    let mut v = vec![Expr::integer(-1)];
                    v.extend(new_args);
                    Expr::times(v)
                }
            }
        }
        _ => Expr::times(vec![Expr::integer(-1), e]),
    }
}

/// Build the `Blank`/`BlankSequence`/`BlankNullSequence` expression for a
/// lexed `Blank(n)` token (`n` underscores), with an optional adjacent type
/// restriction symbol (`_Integer` and friends).
fn build_blank(n: u8, type_name: Option<String>) -> Expr {
    match (n, type_name) {
        (1, Some(h)) => Expr::blank_typed(h),
        (1, None) => Expr::blank(),
        (2, Some(h)) => Expr::blank_sequence_typed(h),
        (2, None) => Expr::blank_sequence(),
        (3, Some(h)) => Expr::blank_null_sequence_typed(h),
        (3, None) => Expr::blank_null_sequence(),
        _ => unreachable!("lexer only emits Blank(1..=3)"),
    }
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Integer(n) => format!("integer '{}'", n),
        TokenKind::Real(x) => format!("real '{}'", x),
        TokenKind::Symbol(s) => format!("symbol '{}'", s),
        TokenKind::Str(s) => format!("string \"{}\"", s),
        TokenKind::Plus => "'+'".to_string(),
        TokenKind::Minus => "'-'".to_string(),
        TokenKind::Star => "'*'".to_string(),
        TokenKind::Slash => "'/'".to_string(),
        TokenKind::Caret => "'^'".to_string(),
        TokenKind::Equal => "'='".to_string(),
        TokenKind::EqualEqual => "'=='".to_string(),
        TokenKind::ColonEqual => "':='".to_string(),
        TokenKind::NotEqual => "'!='".to_string(),
        TokenKind::Less => "'<'".to_string(),
        TokenKind::Greater => "'>'".to_string(),
        TokenKind::LessEqual => "'<='".to_string(),
        TokenKind::GreaterEqual => "'>='".to_string(),
        TokenKind::Arrow => "'->'".to_string(),
        TokenKind::MapArrow => "'/@'".to_string(),
        TokenKind::Prime => "'\\''".to_string(),
        TokenKind::Blank(n) => match n {
            1 => "'_'".to_string(),
            2 => "'__'".to_string(),
            3 => "'___'".to_string(),
            _ => "'_'".to_string(),
        },
        TokenKind::LParen => "'('".to_string(),
        TokenKind::RParen => "')'".to_string(),
        TokenKind::LBracket => "'['".to_string(),
        TokenKind::RBracket => "']'".to_string(),
        TokenKind::LBrace => "'{'".to_string(),
        TokenKind::RBrace => "'}'".to_string(),
        TokenKind::Comma => "','".to_string(),
        TokenKind::Eof => "end of input".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(src: &str) -> Expr {
        parse(src).unwrap_or_else(|e| panic!("parse failed for {:?}: {}", src, e))
    }

    #[test]
    fn numbers_and_symbols() {
        assert_eq!(p("42"), Expr::integer(42));
        assert_eq!(p("3.14"), Expr::real(3.14));
        assert_eq!(p("x"), Expr::symbol("x"));
        assert_eq!(p("\"hi\""), Expr::string("hi"));
    }

    #[test]
    fn arithmetic_precedence() {
        // 1 + 2 * 3 -> Plus[1, Times[2,3]]
        assert_eq!(p("1 + 2 * 3"), Expr::plus(vec![Expr::integer(1), Expr::times(vec![Expr::integer(2), Expr::integer(3)])]));
        // 2 * 3 + 1 -> Plus[Times[2,3], 1]
        assert_eq!(p("2 * 3 + 1"), Expr::plus(vec![Expr::times(vec![Expr::integer(2), Expr::integer(3)]), Expr::integer(1)]));
    }

    #[test]
    fn power_right_associative() {
        // 2^3^2 -> 2^(3^2)
        let expected = Expr::power(Expr::integer(2), Expr::power(Expr::integer(3), Expr::integer(2)));
        assert_eq!(p("2^3^2"), expected);
    }

    #[test]
    fn unary_minus() {
        assert_eq!(p("-3"), Expr::integer(-3));
        assert_eq!(p("-x"), Expr::times(vec![Expr::integer(-1), Expr::symbol("x")]));
        assert_eq!(p("1 - 2"), Expr::plus(vec![Expr::integer(1), Expr::integer(-2)]));
    }

    #[test]
    fn implicit_multiplication() {
        assert_eq!(p("2 x"), Expr::times(vec![Expr::integer(2), Expr::symbol("x")]));
        assert_eq!(
            p("c x[t]"),
            Expr::times(vec![Expr::symbol("c"), Expr::normal(Expr::symbol("x"), vec![Expr::symbol("t")])])
        );
        assert_eq!(
            p("3 Sin[t]"),
            Expr::times(vec![Expr::integer(3), Expr::normal(Expr::symbol("Sin"), vec![Expr::symbol("t")])])
        );
    }

    #[test]
    fn function_application_nested() {
        assert_eq!(
            p("f[g[x], y]"),
            Expr::normal(
                Expr::symbol("f"),
                vec![Expr::normal(Expr::symbol("g"), vec![Expr::symbol("x")]), Expr::symbol("y")]
            )
        );
    }

    #[test]
    fn lists() {
        assert_eq!(p("{1, 2, 3}"), Expr::list(vec![Expr::integer(1), Expr::integer(2), Expr::integer(3)]));
        assert_eq!(p("{}"), Expr::list(vec![]));
    }

    #[test]
    fn equations_and_rules() {
        assert_eq!(p("x == 1"), Expr::equal(Expr::symbol("x"), Expr::integer(1)));
        assert_eq!(p("x -> 1"), Expr::rule(Expr::symbol("x"), Expr::integer(1)));
    }

    #[test]
    fn derivative_primes() {
        assert_eq!(
            p("x'[t]"),
            Expr::normal(
                Expr::normal(Expr::normal(Expr::symbol("Derivative"), vec![Expr::integer(1)]), vec![Expr::symbol("x")]),
                vec![Expr::symbol("t")]
            )
        );
        assert_eq!(
            p("x''[t]"),
            Expr::normal(
                Expr::normal(Expr::normal(Expr::symbol("Derivative"), vec![Expr::integer(2)]), vec![Expr::symbol("x")]),
                vec![Expr::symbol("t")]
            )
        );
    }

    #[test]
    fn pendulum_equation_from_architecture_md() {
        // x''[t] + c x'[t] + Sin[x[t]] == 0
        let e = p("x''[t] + c x'[t] + Sin[x[t]] == 0");
        assert!(e.has_head("Equal"));
        assert_eq!(e.to_string(), "x''[t] + c*x'[t] + Sin[x[t]] == 0");
    }

    #[test]
    fn parentheses_group() {
        assert_eq!(
            p("2 * (x + 1)"),
            Expr::times(vec![Expr::integer(2), Expr::plus(vec![Expr::symbol("x"), Expr::integer(1)])])
        );
    }

    #[test]
    fn division_builds_negative_power() {
        assert_eq!(p("x / y"), Expr::times(vec![Expr::symbol("x"), Expr::power(Expr::symbol("y"), Expr::integer(-1))]));
    }

    #[test]
    fn error_has_position() {
        let err = parse("1 + ").unwrap_err();
        assert_eq!(err.pos, 4);
        let err2 = parse("(1 + 2").unwrap_err();
        assert!(err2.message.contains("')'"));
    }

    #[test]
    fn error_on_unexpected_token() {
        let err = parse(")").unwrap_err();
        assert_eq!(err.pos, 0);
    }

    #[test]
    fn bare_blank_forms() {
        assert_eq!(p("_"), Expr::blank());
        assert_eq!(p("_Integer"), Expr::blank_typed("Integer"));
        assert_eq!(p("__"), Expr::blank_sequence());
        assert_eq!(p("___"), Expr::blank_null_sequence());
    }

    #[test]
    fn named_pattern_forms() {
        assert_eq!(p("x_"), Expr::named_pattern("x", Expr::blank()));
        assert_eq!(p("x_Integer"), Expr::named_pattern("x", Expr::blank_typed("Integer")));
        assert_eq!(p("x__"), Expr::named_pattern("x", Expr::blank_sequence()));
        assert_eq!(p("x___"), Expr::named_pattern("x", Expr::blank_null_sequence()));
    }

    #[test]
    fn pattern_in_function_definition_shape() {
        assert_eq!(
            p("f[x_]"),
            Expr::normal(Expr::symbol("f"), vec![Expr::named_pattern("x", Expr::blank())])
        );
        assert_eq!(
            p("f[x_Integer, y_]"),
            Expr::normal(
                Expr::symbol("f"),
                vec![
                    Expr::named_pattern("x", Expr::blank_typed("Integer")),
                    Expr::named_pattern("y", Expr::blank()),
                ]
            )
        );
    }

    #[test]
    fn whitespace_before_blank_breaks_adjacency() {
        // "x _" is implicit multiplication of x and a bare blank, not a
        // named pattern, since a space separates them.
        assert_eq!(p("x _"), Expr::times(vec![Expr::symbol("x"), Expr::blank()]));
    }

    #[test]
    fn comparison_operators_parse() {
        assert_eq!(p("a < b"), Expr::less(Expr::symbol("a"), Expr::symbol("b")));
        assert_eq!(p("a > b"), Expr::greater(Expr::symbol("a"), Expr::symbol("b")));
        assert_eq!(p("a <= b"), Expr::less_equal(Expr::symbol("a"), Expr::symbol("b")));
        assert_eq!(p("a >= b"), Expr::greater_equal(Expr::symbol("a"), Expr::symbol("b")));
        assert_eq!(p("a != b"), Expr::unequal(Expr::symbol("a"), Expr::symbol("b")));
    }

    #[test]
    fn set_and_set_delayed_parse() {
        assert_eq!(p("a = 5"), Expr::set(Expr::symbol("a"), Expr::integer(5)));
        assert_eq!(
            p("f[x_] := x^2"),
            Expr::set_delayed(
                Expr::normal(Expr::symbol("f"), vec![Expr::named_pattern("x", Expr::blank())]),
                Expr::power(Expr::symbol("x"), Expr::integer(2))
            )
        );
    }

    #[test]
    fn assignment_is_right_associative_and_looser_than_rule() {
        // a = b = 5 chains as Set[a, Set[b, 5]].
        assert_eq!(p("a = b = 5"), Expr::set(Expr::symbol("a"), Expr::set(Expr::symbol("b"), Expr::integer(5))));
    }

    #[test]
    fn map_operator_parses() {
        assert_eq!(
            p("f /@ {1, 2, 3}"),
            Expr::call("Map", vec![Expr::symbol("f"), Expr::list(vec![Expr::integer(1), Expr::integer(2), Expr::integer(3)])])
        );
    }

    #[test]
    fn comments_are_invisible_to_the_parser() {
        assert_eq!(p("(* a comment *) 1 + 2"), p("1 + 2"));
        assert_eq!(p("1 (* nested (* comment *) here *) + 2"), p("1 + 2"));
    }
}
