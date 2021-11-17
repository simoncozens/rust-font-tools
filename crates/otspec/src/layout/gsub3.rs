use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::tables;

tables!(
  AlternateSubstFormat1 {
    [offset_base]
    uint16 substFormat
    Offset16(Coverage) coverage
    CountedOffset16(AlternateSet) alternateSets
  }
  AlternateSet [default] {
    Counted(uint16) alternateGlyphIDs
  }
);
