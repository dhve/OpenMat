//! Symbol table: per-symbol attributes that steer the evaluator.
//!
//! Real Wolfram Language has a much larger attribute set (`SequenceHold`,
//! `NHoldAll`, `Protected`, `Locked`, ...). This first slice covers the five
//! attributes the evaluator actually consults: the two hold flavors it needs
//! for `Hold`, `Flat`/`Orderless` for canonicalizing `Plus`/`Times`, and
//! `Listable` for the numeric builtins.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Attribute {
    HoldAll,
    HoldFirst,
    Flat,
    Orderless,
    Listable,
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    attrs: HashMap<String, HashSet<Attribute>>,
}

impl SymbolTable {
    /// A table pre-populated with the attributes builtins in this crate rely on.
    pub fn new() -> Self {
        let mut table = SymbolTable { attrs: HashMap::new() };
        table.set_attributes("Plus", &[Attribute::Flat, Attribute::Orderless]);
        table.set_attributes("Times", &[Attribute::Flat, Attribute::Orderless]);
        table.set_attributes("Hold", &[Attribute::HoldAll]);
        for f in ["Sin", "Cos", "Tan", "Exp", "Log", "Sqrt", "Abs"] {
            table.set_attributes(f, &[Attribute::Listable]);
        }
        table
    }

    /// An empty table with no attributes registered, useful for tests that
    /// want full control over the environment.
    pub fn empty() -> Self {
        SymbolTable { attrs: HashMap::new() }
    }

    pub fn set_attributes(&mut self, symbol: &str, attrs: &[Attribute]) {
        let entry = self.attrs.entry(symbol.to_string()).or_default();
        for a in attrs {
            entry.insert(*a);
        }
    }

    pub fn clear_attributes(&mut self, symbol: &str) {
        self.attrs.remove(symbol);
    }

    pub fn has_attribute(&self, symbol: &str, attr: Attribute) -> bool {
        self.attrs.get(symbol).map_or(false, |s| s.contains(&attr))
    }

    pub fn attributes(&self, symbol: &str) -> Vec<Attribute> {
        self.attrs.get(symbol).map(|s| s.iter().copied().collect()).unwrap_or_default()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        SymbolTable::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_attributes_preset() {
        let t = SymbolTable::new();
        assert!(t.has_attribute("Plus", Attribute::Flat));
        assert!(t.has_attribute("Plus", Attribute::Orderless));
        assert!(t.has_attribute("Hold", Attribute::HoldAll));
        assert!(!t.has_attribute("Hold", Attribute::Flat));
        assert!(!t.has_attribute("Unknown", Attribute::Flat));
    }

    #[test]
    fn set_and_clear_attributes() {
        let mut t = SymbolTable::empty();
        assert!(!t.has_attribute("f", Attribute::HoldFirst));
        t.set_attributes("f", &[Attribute::HoldFirst]);
        assert!(t.has_attribute("f", Attribute::HoldFirst));
        t.clear_attributes("f");
        assert!(!t.has_attribute("f", Attribute::HoldFirst));
    }
}
