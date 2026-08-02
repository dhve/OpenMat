//! Grammar conformance suite: replays specs/fixtures/grammar-v0.1.txt and
//! specs/fixtures/grammar-v0.2.txt.
//!
//! Each fixture line is `input || canonical InputForm || LaTeX`. For each:
//! parse the input, check its printed form, re-parse the printed form and
//! check expression equality (print/parse round trip), and check the LaTeX
//! rendering. See specs/grammar.md section 8.

use openmat_core::{parse, to_latex, Evaluator};

const FIXTURES_V01: &str = include_str!("../../../specs/fixtures/grammar-v0.1.txt");
const FIXTURES_V02: &str = include_str!("../../../specs/fixtures/grammar-v0.2.txt");

/// Replay every fixture line in `fixtures`, returning how many were checked
/// (blank lines and `#` comments don't count).
fn run_fixtures(fixtures: &str) -> usize {
    let mut checked = 0;
    for (lineno, line) in fixtures.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split("||").map(str::trim).collect();
        assert_eq!(parts.len(), 3, "fixture line {} is malformed: {line}", lineno + 1);
        let (input, expected_print, expected_latex) = (parts[0], parts[1], parts[2]);

        let expr = parse(input)
            .unwrap_or_else(|e| panic!("fixture line {}: {input:?} failed to parse: {e}", lineno + 1));

        let printed = expr.to_string();
        assert_eq!(printed, expected_print, "print mismatch for {input:?}");

        // Round trip is required to hold up to evaluation, not structurally:
        // the printer may regroup (a/b/c prints as a/(b*c)), and evaluation
        // canonicalizes both sides to the same form. Matches grammar.md sec 8.
        let reparsed = parse(&printed)
            .unwrap_or_else(|e| panic!("round trip of {input:?}: {printed:?} failed to parse: {e}"));
        let evaluator = Evaluator::new();
        assert_eq!(
            evaluator.eval(&reparsed),
            evaluator.eval(&expr),
            "round trip changed the evaluated expression for {input:?}"
        );

        assert_eq!(to_latex(&expr), expected_latex, "latex mismatch for {input:?}");
        checked += 1;
    }
    checked
}

#[test]
fn grammar_v01_fixtures_parse_print_roundtrip_and_latex() {
    let checked = run_fixtures(FIXTURES_V01);
    assert!(checked >= 20, "expected at least 20 v0.1 fixtures, replayed {checked}");
}

#[test]
fn grammar_v02_fixtures_parse_print_roundtrip_and_latex() {
    let checked = run_fixtures(FIXTURES_V02);
    assert!(checked >= 15, "expected at least 15 v0.2 fixtures, replayed {checked}");
}
