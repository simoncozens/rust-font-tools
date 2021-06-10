#![warn(missing_docs, missing_crate_level_docs)]
//! A library for parsing, manipulating and writing OpenType fonts
//!
//! *This is a prerelease version; it is not feature complete.*
//! *Notably, variable fonts are supported, but GPOS/GSUB (OpenType Layout) is not.*
//!
//! # Example usage
//! ```no_run
//! use fonttools::font::{self, Font, Table};
//! use fonttools::name::{name, NameRecord, NameRecordID};
//!
//! // Load a font (tables are lazy-loaded)
//! let fontfile = File::open("Test.otf").unwrap();
//! use std::fs::File;
//! let mut myfont = font::load(fontfile).expect("Could not load font");
//!
//! // Access an existing table
//! if let Table::Name(name_table) = myfont.get_table(b"name")
//!         .expect("Error reading name table")
//!         .expect("There was no name table") {
//!     // Manipulate the table (table-specific)
//!         name_table.records.push(NameRecord::windows_unicode(
//!             NameRecordID::LicenseURL,
//!             "http://opensource.org/licenses/OFL-1.1"
//!         ));
//! }
//! let mut outfile = File::create("Test-with-OFL.otf").expect("Could not create file");
//! myfont.save(&mut outfile);
//! ```
//! For information about creating and manipulating structures for
//! each specific OpenType table, see the modules below. See
//! the [font] module as the entry point to creating, parsing and
//! saving an OpenType font.

#[allow(non_snake_case)]
// pub mod GSUB;

/// The `avar` (Axis variations) table
pub mod avar;
/// The `cmap` (Character To Glyph Index Mapping) table
pub mod cmap;
/// The main font object. Start here.
pub mod font;
/// The `fvar` (Font variations) table
pub mod fvar;
/// The `gasp` (Grid-fitting and Scan-conversion Procedure) table
pub mod gasp;
/// The `glyf` (Glyf data) table
pub mod glyf;
/// The `gvar` (Glyph variations) table
pub mod gvar;
/// The `head` (Header) table
pub mod head;
/// The `hhea` (Horizontal header) table
pub mod hhea;
/// The `hmtx` (Horizontal metrics) table
pub mod hmtx;
/// OpenType Layout common tables
pub mod layout;
mod loca;
/// The `maxp` (Maximum profile) table
pub mod maxp;
/// The `name` (Naming) table
pub mod name;
/// The `OS/2` (OS/2 and Windows Metrics) table
pub mod os2;
/// OpenType Variations common tables
pub mod otvar;
/// The `post` (PostScript) table
pub mod post;
/// Useful utilities
pub mod utils;
