use otspec::types::*;

/// The 'head' OpenType tag.
pub const TAG: Tag = crate::tag!("head");

pub use otspec::tables::head::head;

/// Create a new `head` table, given a float font revision, units-per-em
/// value and the global glyph coordinate maxima/minima.
#[allow(non_snake_case)]
pub fn new(
    fontRevision: f32,
    upm: uint16,
    xMin: int16,
    yMin: int16,
    xMax: int16,
    yMax: int16,
) -> head {
    head {
        majorVersion: 1,
        minorVersion: 0,
        fontRevision,
        checksumAdjustment: 0x0,
        magicNumber: 0x5F0F3CF5,
        flags: 3,
        unitsPerEm: upm,
        created: chrono::Utc::now().naive_local(),
        modified: chrono::Utc::now().naive_local(),
        xMin,
        yMin,
        xMax,
        yMax,
        macStyle: 0,
        lowestRecPPEM: 6,
        fontDirectionHint: 2,
        indexToLocFormat: 0,
        glyphDataFormat: 0,
    }
}
