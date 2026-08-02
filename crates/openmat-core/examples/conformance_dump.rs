//! Regenerates the grammar conformance fixture table.
//!
//! Prints `input || canonical InputForm || LaTeX` lines for the inputs below.
//! Review the output by eye, then paste it into
//! specs/fixtures/grammar-v0.1.txt. The conformance test in
//! tests/conformance.rs replays that file.

use openmat_core::{parse, to_latex};

const INPUTS: &[&str] = &[
    "2 + 3",
    "x + y*z",
    "-x^2",
    "(-x)^2",
    "a/b/c",
    "2^3^2",
    "2 x",
    "c x[t]",
    "3 Sin[t]",
    "2 (x + 1)",
    "f[x, y]",
    "{1, 2, 3}",
    "a == b",
    "a -> b",
    "x'[t]",
    "x''[t]",
    "1.5",
    "x/2",
    "(a + b)/c",
    "Sqrt[x]",
    "Sin[x]^2 + Cos[x]^2",
    "x''[t] + c x'[t] + Sin[x[t]] == 0",
];

fn main() {
    for input in INPUTS {
        match parse(input) {
            Ok(expr) => println!("{} || {} || {}", input, expr, to_latex(&expr)),
            Err(err) => println!("{} || PARSE ERROR: {}", input, err),
        }
    }
}
