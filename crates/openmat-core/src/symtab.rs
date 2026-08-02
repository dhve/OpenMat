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
        // Set holds its left side so `f[x_] = ...` never evaluates the
        // pattern being defined (which could otherwise substitute a stray
        // ownvalue into a pattern variable's own name); SetDelayed holds
        // both sides so the right side isn't evaluated until the rule
        // actually fires. Clear holds its symbol arguments for the same
        // reason: `Clear[f]` must not evaluate `f`. Table holds everything
        // so its iterator variable name and bounds stay literal until each
        // iteration substitutes a concrete value; If holds its branches so
        // only the taken one is ever evaluated.
        table.set_attributes("Set", &[Attribute::HoldFirst]);
        table.set_attributes("SetDelayed", &[Attribute::HoldAll]);
        table.set_attributes("Clear", &[Attribute::HoldAll]);
        table.set_attributes("Table", &[Attribute::HoldAll]);
        table.set_attributes("If", &[Attribute::HoldAll]);
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
    fn definition_and_control_form_attributes_preset() {
        let t = SymbolTable::new();
        assert!(t.has_attribute("Set", Attribute::HoldFirst));
        assert!(!t.has_attribute("Set", Attribute::HoldAll));
        assert!(t.has_attribute("SetDelayed", Attribute::HoldAll));
        assert!(t.has_attribute("Clear", Attribute::HoldAll));
        assert!(t.has_attribute("Table", Attribute::HoldAll));
        assert!(t.has_attribute("If", Attribute::HoldAll));
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
