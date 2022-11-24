use crate::layout::common::{coverage_or_nah, FromLowlevel, ToLowlevel};
use otspec::layout::coverage::Coverage;
use otspec::layout::gsub2::{MultipleSubstFormat1, Sequence};
use otspec::tables::GSUB::GSUBSubtable;
use otspec::types::*;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
/// A multiple substitution (one-to-many) subtable.
pub struct MultipleSubst {
    /// The mapping of input glyph IDs to sequence of replacement glyph IDs.
    pub mapping: BTreeMap<GlyphID, Vec<GlyphID>>,
}

impl FromLowlevel<GSUBSubtable> for MultipleSubst {
    fn from_lowlevel(st: GSUBSubtable, _max_glyph_id: GlyphID) -> Self {
        let mut multsubst = MultipleSubst::default();
        match st {
            GSUBSubtable::GSUB2_1(multsubst1) => {
                for (glyph, sequence) in coverage_or_nah(multsubst1.coverage)
                    .iter()
                    .zip(multsubst1.sequences.v.into_iter())
                {
                    multsubst
                        .mapping
                        .insert(*glyph, sequence.link.unwrap_or_default().substituteGlyphIDs);
                }
            }
            _ => panic!(),
        }
        multsubst
    }
}

impl ToLowlevel<GSUBSubtable> for MultipleSubst {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GSUBSubtable {
        let coverage = Offset16::to(Coverage {
            glyphs: self.mapping.keys().copied().collect(),
        });
        let mut sequences = vec![];
        for right in self.mapping.values() {
            sequences.push(Offset16::to(Sequence {
                substituteGlyphIDs: right.to_vec(),
            }));
        }
        GSUBSubtable::GSUB2_1(MultipleSubstFormat1 {
            substFormat: 1,
            coverage,
            sequences: sequences.into(),
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
    fn test_gsub2_deser() {
        let binary_gsub = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x0a, 0x00, 0x02, 0x00, 0x12, 0x00, 0x18, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x42, 0x00, 0x45, 0x00, 0x02, 0x00, 0x43, 0x00, 0x44, 0x00, 0x02, 0x00, 0x48,
            0x00, 0x49,
        ];
        let expected = expected_gsub(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Substitution::Multiple(vec![MultipleSubst {
                mapping: btreemap!(
                    66 => vec![67, 68],
                    69 => vec![72, 73]
                ),
            }]),
        }]);
        assert_can_roundtrip(binary_gsub, &expected);
    }
}
