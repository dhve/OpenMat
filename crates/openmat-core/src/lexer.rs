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
    EqualEqual,
    Arrow,
    Prime,
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

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace();
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
                self.pos += 1;
                TokenKind::Slash
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
                    return Err(LexError {
                        message: "unexpected '='; did you mean '=='?".to_string(),
                        pos: start,
                    });
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

fn is_symbol_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_symbol_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
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
}
