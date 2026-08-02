//! Tokenizer for the WL-shaped linear syntax subset.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Integer(i64),
    Real(f64),
    Symbol(String),
    Str(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Equal,
    EqualEqual,
    ColonEqual,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Arrow,
    MapArrow,
    Prime,
    /// A run of one, two, or three underscores: `_` (Blank), `__`
    /// (BlankSequence), `___` (BlankNullSequence). Lexed as a single token
    /// (rather than as identifier characters) so the parser can recognize
    /// pattern surface syntax; see `specs/grammar.md` v0.2 section 1.
    Blank(u8),
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    /// Byte offset of the start of this token in the source.
    pub pos: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub message: String,
    pub pos: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lex error at position {}: {}", self.pos, self.message)
    }
}

impl std::error::Error for LexError {}

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src, bytes: src.as_bytes(), pos: 0 }
    }

    /// Tokenize the whole input, ending with a single `Eof` token.
    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek_byte() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Skip whitespace and `(* ... *)` comments, alternating between the two
    /// until neither consumes anything, so `  (* a *) (* b *)  x` skips
    /// cleanly to `x`. Comments nest: `(* outer (* inner *) still *)` is one
    /// comment.
    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            let before = self.pos;
            self.skip_whitespace();
            if self.peek_byte() == Some(b'(') && self.peek_at(1) == Some(b'*') {
                self.skip_comment()?;
            }
            if self.pos == before {
                break;
            }
        }
        Ok(())
    }

    fn skip_comment(&mut self) -> Result<(), LexError> {
        let start = self.pos;
        self.pos += 2; // consume the opening "(*"
        let mut depth = 1u32;
        while depth > 0 {
            match (self.peek_byte(), self.peek_at(1)) {
                (Some(b'('), Some(b'*')) => {
                    self.pos += 2;
                    depth += 1;
                }
                (Some(b'*'), Some(b')')) => {
                    self.pos += 2;
                    depth -= 1;
                }
                (Some(_), _) => self.pos += 1,
                (None, _) => return Err(LexError { message: "unterminated comment".to_string(), pos: start }),
            }
        }
        Ok(())
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_trivia()?;
        let start = self.pos;
        let b = match self.peek_byte() {
            None => return Ok(Token { kind: TokenKind::Eof, pos: start }),
            Some(b) => b,
        };

        if b.is_ascii_digit() {
            return self.lex_number();
        }
        if b == b'"' {
            return self.lex_string();
        }
        if b == b'_' {
            return self.lex_underscore();
        }
        if is_symbol_start(b) {
            return self.lex_symbol();
        }

        let kind = match b {
            b'+' => {
                self.pos += 1;
                TokenKind::Plus
            }
            b'-' => {
                if self.peek_at(1) == Some(b'>') {
                    self.pos += 2;
                    TokenKind::Arrow
                } else {
                    self.pos += 1;
                    TokenKind::Minus
                }
            }
            b'*' => {
                self.pos += 1;
                TokenKind::Star
            }
            b'/' => {
                if self.peek_at(1) == Some(b'@') {
                    self.pos += 2;
                    TokenKind::MapArrow
                } else {
                    self.pos += 1;
                    TokenKind::Slash
                }
            }
            b'^' => {
                self.pos += 1;
                TokenKind::Caret
            }
            b'=' => {
                if self.peek_at(1) == Some(b'=') {
                    self.pos += 2;
                    TokenKind::EqualEqual
                } else {
                    self.pos += 1;
                    TokenKind::Equal
                }
            }
            b':' => {
                if self.peek_at(1) == Some(b'=') {
                    self.pos += 2;
                    TokenKind::ColonEqual
                } else {
                    return Err(LexError { message: "unexpected ':'; did you mean ':='?".to_string(), pos: start });
                }
            }
            b'!' => {
                if self.peek_at(1) == Some(b'=') {
                    self.pos += 2;
                    TokenKind::NotEqual
                } else {
                    return Err(LexError { message: "unexpected '!'; did you mean '!='?".to_string(), pos: start });
                }
            }
            b'<' => {
                if self.peek_at(1) == Some(b'=') {
                    self.pos += 2;
                    TokenKind::LessEqual
                } else {
                    self.pos += 1;
                    TokenKind::Less
                }
            }
            b'>' => {
                if self.peek_at(1) == Some(b'=') {
                    self.pos += 2;
                    TokenKind::GreaterEqual
                } else {
                    self.pos += 1;
                    TokenKind::Greater
                }
            }
            b'\'' => {
                self.pos += 1;
                TokenKind::Prime
            }
            b'(' => {
                self.pos += 1;
                TokenKind::LParen
            }
            b')' => {
                self.pos += 1;
                TokenKind::RParen
            }
            b'[' => {
                self.pos += 1;
                TokenKind::LBracket
            }
            b']' => {
                self.pos += 1;
                TokenKind::RBracket
            }
            b'{' => {
                self.pos += 1;
                TokenKind::LBrace
            }
            b'}' => {
                self.pos += 1;
                TokenKind::RBrace
            }
            b',' => {
                self.pos += 1;
                TokenKind::Comma
            }
            other => {
                return Err(LexError {
                    message: format!("unexpected character '{}'", other as char),
                    pos: start,
                })
            }
        };
        Ok(Token { kind, pos: start })
    }

    fn lex_number(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        while matches!(self.peek_byte(), Some(b) if b.is_ascii_digit()) {
            self.pos += 1;
        }
        let mut is_real = false;
        if self.peek_byte() == Some(b'.') && matches!(self.peek_at(1), Some(b) if b.is_ascii_digit()) {
            is_real = true;
            self.pos += 1;
            while matches!(self.peek_byte(), Some(b) if b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek_byte(), Some(b'e') | Some(b'E')) {
            let save = self.pos;
            let mut probe = self.pos + 1;
            if matches!(self.bytes.get(probe), Some(b'+') | Some(b'-')) {
                probe += 1;
            }
            if matches!(self.bytes.get(probe), Some(b) if b.is_ascii_digit()) {
                is_real = true;
                self.pos = probe;
                while matches!(self.peek_byte(), Some(b) if b.is_ascii_digit()) {
                    self.pos += 1;
                }
            } else {
                self.pos = save;
            }
        }
        let text = &self.src[start..self.pos];
        if is_real {
            let value: f64 = text
                .parse()
                .map_err(|_| LexError { message: format!("invalid real literal '{}'", text), pos: start })?;
            Ok(Token { kind: TokenKind::Real(value), pos: start })
        } else {
            match text.parse::<i64>() {
                Ok(n) => Ok(Token { kind: TokenKind::Integer(n), pos: start }),
                // Overflows an i64: fall back to a machine real. Documented in
                // eval.rs alongside the same policy for arithmetic overflow.
                Err(_) => {
                    let value: f64 = text
                        .parse()
                        .map_err(|_| LexError { message: format!("invalid integer literal '{}'", text), pos: start })?;
                    Ok(Token { kind: TokenKind::Real(value), pos: start })
                }
            }
        }
    }

    fn lex_string(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            match self.peek_byte() {
                None => {
                    return Err(LexError { message: "unterminated string literal".to_string(), pos: start })
                }
                Some(b'"') => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek_byte() {
                        Some(b'"') => {
                            out.push('"');
                            self.pos += 1;
                        }
                        Some(b'\\') => {
                            out.push('\\');
                            self.pos += 1;
                        }
                        Some(b'n') => {
                            out.push('\n');
                            self.pos += 1;
                        }
                        Some(b't') => {
                            out.push('\t');
                            self.pos += 1;
                        }
                        _ => {
                            return Err(LexError {
                                message: "invalid escape sequence in string".to_string(),
                                pos: self.pos,
                            })
                        }
                    }
                }
                Some(_) => {
                    // Advance by one full UTF-8 character, not one byte.
                    let ch = self.src[self.pos..].chars().next().unwrap();
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        Ok(Token { kind: TokenKind::Str(out), pos: start })
    }

    /// Lex a run of one to three underscores into `Blank(1|2|3)`: `_`, `__`,
    /// `___`. Real Wolfram Language never treats `_` as an identifier
    /// character, so unlike v0.1 this is not folded into `lex_symbol`.
    fn lex_underscore(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        let mut count: u8 = 0;
        while self.peek_byte() == Some(b'_') {
            self.pos += 1;
            count += 1;
            if count > 3 {
                return Err(LexError {
                    message: "too many consecutive underscores in a pattern (at most ___ is meaningful)".to_string(),
                    pos: start,
                });
            }
        }
        Ok(Token { kind: TokenKind::Blank(count), pos: start })
    }

    fn lex_symbol(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        self.pos += 1;
        while matches!(self.peek_byte(), Some(b) if is_symbol_continue(b)) {
            self.pos += 1;
        }
        let text = self.src[start..self.pos].to_string();
        Ok(Token { kind: TokenKind::Symbol(text), pos: start })
    }
}

/// `_` is deliberately excluded: real Wolfram Language never treats it as an
/// identifier character (it lexes as pattern syntax, [`lex_underscore`]).
/// `$` stays valid, matching WL's context-mark convention.
fn is_symbol_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'$'
}

fn is_symbol_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'$'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src).tokenize().unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn integers_and_reals() {
        assert_eq!(kinds("42"), vec![TokenKind::Integer(42), TokenKind::Eof]);
        assert_eq!(kinds("3.14"), vec![TokenKind::Real(3.14), TokenKind::Eof]);
        assert_eq!(kinds("1e10"), vec![TokenKind::Real(1e10), TokenKind::Eof]);
        assert_eq!(kinds("2.5e-3"), vec![TokenKind::Real(2.5e-3), TokenKind::Eof]);
    }

    #[test]
    fn symbols_and_strings() {
        assert_eq!(kinds("x"), vec![TokenKind::Symbol("x".to_string()), TokenKind::Eof]);
        assert_eq!(kinds("\"hi\""), vec![TokenKind::Str("hi".to_string()), TokenKind::Eof]);
    }

    #[test]
    fn operators() {
        assert_eq!(
            kinds("+-*/^==->'"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Caret,
                TokenKind::EqualEqual,
                TokenKind::Arrow,
                TokenKind::Prime,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn positions_tracked() {
        let toks = Lexer::new("1 + x").tokenize().unwrap();
        assert_eq!(toks[0].pos, 0);
        assert_eq!(toks[1].pos, 2);
        assert_eq!(toks[2].pos, 4);
    }

    #[test]
    fn bad_character_reports_position() {
        let err = Lexer::new("1 @ 2").tokenize().unwrap_err();
        assert_eq!(err.pos, 2);
    }

    #[test]
    fn underscore_no_longer_an_identifier_character() {
        assert_eq!(kinds("_"), vec![TokenKind::Blank(1), TokenKind::Eof]);
        assert_eq!(kinds("__"), vec![TokenKind::Blank(2), TokenKind::Eof]);
        assert_eq!(kinds("___"), vec![TokenKind::Blank(3), TokenKind::Eof]);
        // "x_" is two adjacent tokens now, not one identifier "x_".
        assert_eq!(kinds("x_"), vec![TokenKind::Symbol("x".to_string()), TokenKind::Blank(1), TokenKind::Eof]);
    }

    #[test]
    fn too_many_underscores_is_a_lex_error() {
        let err = Lexer::new("____").tokenize().unwrap_err();
        assert_eq!(err.pos, 0);
    }

    #[test]
    fn dollar_still_an_identifier_character() {
        assert_eq!(kinds("$Context"), vec![TokenKind::Symbol("$Context".to_string()), TokenKind::Eof]);
    }

    #[test]
    fn comparison_and_assignment_operators() {
        assert_eq!(
            kinds("< > <= >= != = :="),
            vec![
                TokenKind::Less,
                TokenKind::Greater,
                TokenKind::LessEqual,
                TokenKind::GreaterEqual,
                TokenKind::NotEqual,
                TokenKind::Equal,
                TokenKind::ColonEqual,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn map_operator() {
        assert_eq!(kinds("/@"), vec![TokenKind::MapArrow, TokenKind::Eof]);
    }

    #[test]
    fn comments_are_skipped_including_nested() {
        assert_eq!(kinds("(* hi *) 1"), vec![TokenKind::Integer(1), TokenKind::Eof]);
        assert_eq!(kinds("1 (* a (* b *) c *) + 2"), kinds("1 + 2"));
    }

    #[test]
    fn unterminated_comment_is_a_lex_error() {
        let err = Lexer::new("(* never closed").tokenize().unwrap_err();
        assert_eq!(err.pos, 0);
    }
}
