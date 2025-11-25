//==================================================
// File: symbol.rs
//==================================================
// Author: ZobieLabs
// License: Duality Public License (DPL v1.0)
// Goal: Restore missing symbol interning layer
// Objective: Provide Symbol struct and intern_symbol()
//==================================================

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Symbol(pub String);

impl Symbol {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for Symbol {
    fn from(value: String) -> Self {
        Symbol(value)
    }
}

impl From<&str> for Symbol {
    fn from(value: &str) -> Self {
        Symbol(value.to_string())
    }
}

impl From<Symbol> for String {
    fn from(symbol: Symbol) -> Self {
        symbol.0
    }
}

impl Deref for Symbol {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn intern_symbol(name: &str) -> Symbol {
    Symbol::from(name)
}

//==================================================
// End of file
//==================================================
