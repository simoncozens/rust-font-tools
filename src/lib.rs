#![warn(missing_docs, missing_crate_level_docs)]
//! A library for parsing, manipulating and writing OpenType fonts
//!
//! # Example usage
//! ```no_run
//! use fonttools::{font, Table};
//! use fonttools::name::{name, NameRecordID};
//!
//! // Load a font (tables are lazy-loaded)
//! let myfont = font::load("Test.otf");
//!
//! // Access an existing table
//! if let Table::Name(name_table) = myfont.get_table(b"name")
//!         .expect("Error reading name table")
//!         .expect("There was no name table") {
//!     // Manipulate the table (table-specific)
//!         name_table.records.push(NameRecord::windows_unicode(
//!             NameRecordID::LicenseURL,
//!             "http://opensource.org/licenses/OFL-1.1"
//!         );
//! }
//! myfont.save("Test-with-OFL.otf");
//! ```
//! For information about creating and manipulating structures for
//! each specific OpenType table, see the modules below. See
//! the `font` module as the entry point to creating, parsing and
//! saving an OpenType font.

/// The `GSUB` (Glyph substitution) table
#[allow(non_snake_case)]
pub mod GSUB;
/// The `avar` (Axis variations) table
pub mod avar;
/// The `fvar` (Font variations) table
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
