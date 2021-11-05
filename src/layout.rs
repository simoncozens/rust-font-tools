/// Common tables for OpenType Layout
pub mod common;
/// Common tables for contextual lookup subtables
pub mod contextual;
/// GPOS1 single positioning
pub mod gpos1;
/// GPOS2 pair positioning
pub mod gpos2;
/// GPOS3 cursive positioning
pub mod gpos3;
/// GPOS4 mark-to-base positioning
pub mod gpos4;
/// GSUB1 single substitution
pub mod gsub1;
/// GSUB2 multiple substitution
pub mod gsub2;
/// GSUB3 alternate substitution
pub mod gsub3;
/// GSUB4 ligature substitution
pub mod gsub4;
pub(crate) mod macros;
