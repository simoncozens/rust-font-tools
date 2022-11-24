use crate::layout::common::{coverage_or_nah, FromLowlevel, ToLowlevel};
use crate::layout::contextual::{coverage_to_slot, Slot};
use otspec::layout::coverage::Coverage;
use otspec::layout::gsub8::ReverseChainSingleSubstFormat1;
use otspec::tables::GSUB::GSUBSubtable;
use otspec::types::*;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
/// A reverse chaining substitution subtable.
pub struct ReverseChainSubst {
    /// The mapping of input glyph IDs to substitute.
    pub mapping: BTreeMap<GlyphID, GlyphID>,
    /// Glyphs which must appear before the input glyph
    pub backtrack: Vec<Slot>,
    /// Glyphs which must appear after the input glyph
    pub lookahead: Vec<Slot>,
}

impl ToLowlevel<GSUBSubtable> for ReverseChainSubst {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GSUBSubtable {
        let coverage = Coverage {
            glyphs: self.mapping.keys().copied().collect(),
        };

        let lookahead_coverages: Vec<Offset16<Coverage>> = self
            .lookahead
            .iter()
            .map(|slot| {
                Offset16::to(Coverage {
                    glyphs: slot.iter().copied().collect(),
                })
            })
            .collect();
        let backtrack_coverages: Vec<Offset16<Coverage>> = self
            .backtrack
            .iter()
            .map(|slot| {
                Offset16::to(Coverage {
                    glyphs: slot.iter().copied().collect(),
                })
            })
            .collect();

        GSUBSubtable::GSUB8_1(ReverseChainSingleSubstFormat1 {
            substFormat: 1,
            coverage: Offset16::to(coverage),
            backtrackCoverages: backtrack_coverages.into(),
            lookaheadCoverages: lookahead_coverages.into(),
            substituteGlyphIDs: self.mapping.values().copied().collect(),
        })
    }
}

impl FromLowlevel<GSUBSubtable> for ReverseChainSubst {
    fn from_lowlevel(st: GSUBSubtable, _max_glyph_id: GlyphID) -> Self {
        let mut mapping = BTreeMap::new();
        match st {
            GSUBSubtable::GSUB8_1(rs1) => {
                let coverage = coverage_or_nah(rs1.coverage);
                for (input, output) in coverage.into_iter().zip(rs1.substituteGlyphIDs) {
                    mapping.insert(input, output);
                }
                ReverseChainSubst {
                    lookahead: rs1
                        .lookaheadCoverages
                        .v
                        .into_iter()
                        .map(coverage_to_slot)
                        .collect(),
                    backtrack: rs1
                        .backtrackCoverages
                        .v
                        .into_iter()
                        .map(coverage_to_slot)
                        .collect(),
                    mapping,
                }
            }
            _ => panic!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::{Lookup, LookupFlags};
    use crate::tables::GSUB::tests::{assert_can_roundtrip, expected_gsub};
    use crate::tables::GSUB::Substitution;
    use otspec::{btreemap, btreeset};
    use std::iter::FromIterator;

    #[test]
    fn test_gsub8_deser() {
        let binary_gsub = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x14, 0x00, 0x02, 0x00, 0x1a, 0x00, 0x20, 0x00, 0x02, 0x00, 0x2a,
            0x00, 0x32, 0x00, 0x01, 0x00, 0x59, 0x00, 0x01, 0x00, 0x01, 0x00, 0x46, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x45, 0x00, 0x01, 0x00, 0x03, 0x00, 0x42, 0x00, 0x43, 0x00, 0x44,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x46, 0x00, 0x47, 0x00, 0x01, 0x00, 0x01, 0x00, 0x48,
        ];
        let expected = expected_gsub(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Substitution::ReverseChainContextual(vec![ReverseChainSubst {
                backtrack: vec![btreeset!(69), btreeset!(66, 67, 68)],
                lookahead: vec![btreeset!(70, 71), btreeset!(72)],
                mapping: btreemap!(70 => 89),
            }]),
        }]);
        assert_can_roundtrip(binary_gsub, &expected);
    }
}
