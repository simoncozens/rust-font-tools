use crate::layout::coverage::Coverage;
use crate::tables::GSUB::GSUBSubtable;
use crate::types::*;
use crate::Deserializer;
use otspec_macros::tables;

tables!(
  SingleSubstFormat1 {
    [offset_base]
    uint16 substFormat
    Offset16(Coverage) coverage // Offset to Coverage table, from beginning of substitution subtable
    int16 deltaGlyphID  // Add to original glyph ID to get substitute glyph ID
  }
  SingleSubstFormat2 {
    [offset_base]
    uint16 substFormat
    Offset16(Coverage)  coverage  // Offset to Coverage table, from beginning of substitution subtable
    Counted(uint16)  substituteGlyphIDs // Array of substitute glyph IDs — ordered by Coverage index
  }
);

pub(crate) fn deserialize_gsub1(
    c: &mut crate::ReaderContext,
) -> Result<GSUBSubtable, crate::DeserializationError> {
    match c.peek(2)? {
        [0x00, 0x01] => {
            let ssf1: SingleSubstFormat1 = c.de()?;
            Ok(GSUBSubtable::GSUB1_1(ssf1))
        }
        [0x00, 0x02] => {
            let ssf2: SingleSubstFormat2 = c.de()?;
            Ok(GSUBSubtable::GSUB1_2(ssf2))
        }
        f => Err(crate::DeserializationError(format!(
            "Bad single sub format {:?}",
            f
        ))),
    }
}
