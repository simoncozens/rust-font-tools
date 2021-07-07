use std::fmt::{self, Display};
use syn::{Ident, Path};

#[derive(Copy, Clone)]
pub struct Symbol(&'static str);

pub const DESERIALIZE_WITH: Symbol = Symbol("deserialize_with");
pub const OFFSET_BASE: Symbol = Symbol("offset_base");
pub const EMBED: Symbol = Symbol("embed");
pub const SERDE: Symbol = Symbol("serde");
pub const SERIALIZE_WITH: Symbol = Symbol("serialize_with");
pub const SKIP_DESERIALIZING: Symbol = Symbol("skip_deserializing");
pub const WITH: Symbol = Symbol("with");

impl PartialEq<Symbol> for Ident {
    fn eq(&self, word: &Symbol) -> bool {
        self == word.0
    }
}

impl<'a> PartialEq<Symbol> for &'a Ident {
    fn eq(&self, word: &Symbol) -> bool {
        *self == word.0
    }
}

impl PartialEq<Symbol> for Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl<'a> PartialEq<Symbol> for &'a Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl Display for Symbol {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}
