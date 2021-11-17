use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::tables;

tables!(
  ReverseChainSingleSubstFormat1 {
    [offset_base]
    uint16 substFormat
    Offset16(Coverage) coverage
    CountedOffset16(Coverage) backtrackCoverages
    CountedOffset16(Coverage) lookaheadCoverages
    Counted(uint16) substituteGlyphIDs
}
);
