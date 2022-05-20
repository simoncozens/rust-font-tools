use crate::layout::common::{coverage_or_nah, FromLowlevel, ToLowlevel};
use otspec::layout::coverage::Coverage;
use otspec::layout::gsub3::{AlternateSet, AlternateSubstFormat1};
use otspec::tables::GSUB::GSUBSubtable;
use otspec::types::*;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Clone, Default)]
/// A alternate substitution (`sub ... from ...`) subtable.
pub struct AlternateSubst {
    /// The mapping of input glyph IDs to array of possible glyph IDs.
    pub mapping: BTreeMap<GlyphID, Vec<GlyphID>>,
}

impl FromLowlevel<GSUBSubtable> for AlternateSubst {
    fn from_lowlevel(st: GSUBSubtable, _max_glyph_id: GlyphID) -> Self {
        let mut alternatesubst = AlternateSubst::default();
        match st {
            GSUBSubtable::GSUB3_1(alternatesubst1) => {
                for (glyph, sequence) in coverage_or_nah(alternatesubst1.coverage)
                    .iter()
                    .zip(alternatesubst1.alternateSets.v.into_iter())
                {
                    alternatesubst
                        .mapping
                        .insert(*glyph, sequence.link.unwrap_or_default().alternateGlyphIDs);
                }
            }
            _ => panic!(),
        }
        alternatesubst
    }
}

impl ToLowlevel<GSUBSubtable> for AlternateSubst {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GSUBSubtable {
        let coverage = Offset16::to(Coverage {
            glyphs: self.mapping.keys().copied().collect(),
        });
        let mut alternate_sets = vec![];
        for right in self.mapping.values() {
            alternate_sets.push(Offset16::to(AlternateSet {
                alternateGlyphIDs: right.to_vec(),
            }));
        }
        GSUBSubtable::GSUB3_1(AlternateSubstFormat1 {
            substFormat: 1,
            coverage,
            alternateSets: alternate_sets.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::{Lookup, LookupFlags};
    use crate::tables::GSUB::tests::{assert_can_roundtrip, expected_gsub};
    use crate::tables::GSUB::Substitution;
    use otspec::btreemap;
    use std::iter::FromIterator;

    #[test]
    fn test_gsub3_deser() {
        // Note that this is different from the data produced by Python fonttools, which puts the
        // coverage table at the end, before the mapping table.
        let binary_gsub = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x0a, 0x00, 0x02, 0x00, 0x12, 0x00, 0x18, 0x00, 0x01, 0x00, 0x02,
            0x00, 66, 0x00, 69, 0x00, 0x02, 0x00, 67, 0x00, 68, 0x00, 0x02, 0x00, 72, 0x00, 73,
        ];
        let expected = expected_gsub(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Substitution::Alternate(vec![AlternateSubst {
                mapping: btreemap!(
                    66 => vec![67, 68],
                    69 => vec![72, 73]
                ),
            }]),
        }]);
        assert_can_roundtrip(binary_gsub, &expected);
    }
}
