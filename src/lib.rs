#![warn(missing_docs, missing_crate_level_docs)]
#![allow(non_camel_case_types, non_snake_case, clippy::upper_case_acronyms)]

mod avar;
pub mod cmap;
pub mod font;
pub mod fvar;
pub mod gasp;
pub mod glyf;
mod gvar;
pub mod head;
pub mod hhea;
pub mod hmtx;
mod loca;
pub mod maxp;
/// Represents a font's name (Naming) table
pub mod name;
/// Represents a font's OS/2 (OS/2 and Windows Metrics) table
pub mod os2;
/// OpenType Variations common tables
pub mod otvar;
/// Represents the font's post (PostScript) table
pub mod post;
