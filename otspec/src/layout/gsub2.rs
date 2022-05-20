use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::tables;

tables!(
  MultipleSubstFormat1 {
    [offset_base]
    uint16 substFormat
    Offset16(Coverage) coverage
    CountedOffset16(Sequence)  sequences
  }
  Sequence [default] {
    Counted(uint16) substituteGlyphIDs
  }
);
