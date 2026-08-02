//! OpenMat expression kernel: expressions, parsing, evaluation, patterns.
//!
//! Everything is `Head[args...]`, exactly as in Wolfram Language: see
//! [`expr::Expr`]. The pieces on top of that tree:
//!
//! - [`parser::parse`]: turns WL-shaped linear syntax text into an [`expr::Expr`].
//! - [`eval::Evaluator`]: fixed-point rewriter with Hold-attribute-aware
//!   evaluation, numeric folding, and `Plus`/`Times` canonicalization.
//! - [`pattern`]: `Blank`/named-pattern matching and `ReplaceAll`.
//! - [`latex::to_latex`]: KaTeX-ready LaTeX rendering.
//!
//! Two scope cuts apply throughout and are documented where they bite:
//! symbols are plain `String`s, not interned; there is no `Rational` type, so
//! exact fractions live as `Times[num, Power[den, -1]]` and integer overflow
//! anywhere promotes to `f64` rather than an arbitrary-precision integer.
//! Both are called out in `canon.rs` and `expr.rs`.
//!
//! Full `Orderless`/`Flat` pattern matching (matching `a_ + b_` against an
//! arbitrary sum) is out of scope for this pass; see the module doc in
//! `pattern.rs` for where it plugs in.

pub mod canon;
pub mod eval;
pub mod expr;
pub mod latex;
pub mod lexer;
pub mod parser;
pub mod pattern;
pub mod symtab;

pub use eval::Evaluator;
pub use expr::Expr;
pub use latex::to_latex;
pub use parser::{parse, ParseError};
pub use pattern::{pattern_match, replace_all, Bindings};
pub use symtab::{Attribute, SymbolTable};
