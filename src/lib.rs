#![warn(missing_docs, missing_crate_level_docs)]
#![allow(non_camel_case_types, non_snake_case, clippy::upper_case_acronyms)]

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
