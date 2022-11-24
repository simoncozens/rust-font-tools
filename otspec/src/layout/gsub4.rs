use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};
use otspec_macros::tables;

tables!(
  LigatureSubstFormat1 {
    [offset_base]
    uint16 substFormat
    Offset16(Coverage) coverage
    CountedOffset16(LigatureSet)  ligatureSet
  }
  LigatureSet {
    [offset_base]
    CountedOffset16(Ligature) ligatureOffsets
  }
);

// We can't use the magic tables here because the component count is the array
// length MINUS ONE.
/// Internal representation of a ligature substitution for serialization/deserialization
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Ligature {
    pub ligatureGlyph: uint16,
    pub componentGlyphIDs: Vec<uint16>,
}

impl Serialize for Ligature {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        data.put(self.ligatureGlyph)?;
        data.put(self.componentGlyphIDs.len() as uint16 + 1)?;
        data.put(&self.componentGlyphIDs)
    }
}

impl Deserialize for Ligature {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let ligature_glyph: uint16 = c.de()?;
        let component_count: uint16 = c.de()?;
        let components: Vec<uint16> = c.de_counted(component_count as usize - 1)?;
        Ok(Ligature {
            ligatureGlyph: ligature_glyph,
            componentGlyphIDs: components,
        })
    }
}
