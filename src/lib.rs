#![warn(missing_docs, missing_crate_level_docs)]
//! A library for parsing, manipulating and writing OpenType fonts
//!
//! *This is a prerelease version; it is not feature complete.*
//! *Notably, variable fonts are supported, but GPOS/GSUB (OpenType Layout) is not.*
//!
//! # Example usage
//! ```no_run
//! # // we need an explicit main fn to use macros:
//! # #[macro_use] extern crate otspec;
//! # fn main() {
//! use fonttools::font::{self, Font};
//! use fonttools::tables::name::{name, NameRecord, NameRecordID};
//!
//! // Load a font (tables are lazy-loaded)
//! let mut myfont = Font::load("Test.otf").expect("Could not load font");
//!
//! // Access an existing table
//! let mut name = myfont.tables.name()
//!    .expect("Error reading name table")
//!    .expect("There was no name table");
//!
//!     // Manipulate the table (table-specific)
//! name.records.push(NameRecord::windows_unicode(
//!     NameRecordID::LicenseURL,
//!     "http://opensource.org/licenses/OFL-1.1"
//! ));
//!
//! myfont.tables.insert(name);
//!
//! myfont.save("Test-with-OFL.otf").expect("Could not create file");
//! # }
//! ```
//! For information about creating and manipulating structures for
//! each specific OpenType table, see the modules below. See
//! the [font] module as the entry point to creating, parsing and
//! saving an OpenType font.

/// The main font object. Start here.
pub mod font;
/// OpenType Layout common tables
pub mod layout;
/// OpenType Variations common tables
pub mod otvar;
mod table_store;
/// OpenType table definitions.
pub mod tables;
/// Useful utilities
pub mod utils;

pub use otspec::types;
pub use otspec_macros::tag;

// lets us use the tag! macro from otspec_macros within this crate
extern crate self as fonttools;
