use crate::layout::common::{FromLowlevel, ToLowlevel};
use otspec::layout::coverage::Coverage;
use otspec::layout::gsub4::{Ligature, LigatureSet, LigatureSubstFormat1};
use otspec::tables::GSUB::GSUBSubtable;
use otspec::types::*;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
/// A ligature substitution (many-to-one) subtable.
pub struct LigatureSubst {
    /// The mapping of sequences of input glyphs IDs to replacement glyph IDs.
    pub mapping: BTreeMap<Vec<GlyphID>, GlyphID>,
}

impl ToLowlevel<GSUBSubtable> for LigatureSubst {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GSUBSubtable {
        let mut split_map: BTreeMap<u16, Vec<Vec<u16>>> = BTreeMap::new();
        let mut mapping_keys: Vec<&Vec<uint16>> = self.mapping.keys().collect();
        mapping_keys.sort_by_key(|a| -(a.len() as isize));
        for left in mapping_keys {
            let covered = left.first().unwrap();
            split_map
                .entry(*covered)
                .or_insert_with(std::vec::Vec::new)
                .push(left.clone());
        }
        // println!("Split map {:?}", split_map);

        let coverage = Coverage {
            glyphs: split_map.keys().copied().collect(),
        };
        let mut ligature_sets: Vec<Offset16<LigatureSet>> = vec![];
        for first in &coverage.glyphs {
            // println!("For covered glyph {:?}", first);
            let relevant_keys = split_map.get(first).unwrap();
            let ligature_offsets: Vec<Offset16<Ligature>> = relevant_keys
                .iter()
                .map(|k| {
                    Offset16::to(Ligature {
                        ligatureGlyph: *self.mapping.get(k).unwrap(),
                        componentGlyphIDs: k[1..].to_vec(),
                    })
                })
                .collect();

            let ls = LigatureSet {
                ligatureOffsets: ligature_offsets.into(),
            };
            ligature_sets.push(Offset16::to(ls));
        }
        GSUBSubtable::GSUB4_1(LigatureSubstFormat1 {
            substFormat: 1,
            coverage: Offset16::to(coverage),
            ligatureSet: VecOffset16 { v: ligature_sets },
        })
    }
}

impl FromLowlevel<GSUBSubtable> for LigatureSubst {
    fn from_lowlevel(st: GSUBSubtable, _max_glyph_id: GlyphID) -> Self {
        let mut mapping = BTreeMap::new();
        match st {
            GSUBSubtable::GSUB4_1(lsf1) => {
                for (input, lig_set) in lsf1
                    .coverage
                    .link
                    .unwrap()
                    .glyphs
                    .iter()
                    .zip(lsf1.ligatureSet.v.iter())
                {
                    for ligature in lig_set.link.as_ref().unwrap().ligatureOffsets.v.iter() {
                        let ligature = ligature.link.as_ref().unwrap();
                        let mut input_sequence: Vec<u16> = vec![*input];
                        input_sequence.extend(ligature.componentGlyphIDs.clone());
                        mapping.insert(input_sequence, ligature.ligatureGlyph);
                    }
                }
            }
            _ => panic!(),
        }
        LigatureSubst { mapping }
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
    fn test_gsub4_deser() {
        let binary_gsub = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x08, 0x00, 0x01, 0x00, 0x0e, 0x00, 0x01, 0x00, 0x01, 0x00, 71, 0x00,
            0x02, 0x00, 0x06, 0x00, 0x0e, 0x00, 240, 0x00, 0x03, 0x00, 71, 0x00, 77, 0x00, 109,
            0x00, 0x02, 0x00, 74,
        ];
        let expected = expected_gsub(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Substitution::Ligature(vec![LigatureSubst {
                mapping: btreemap!(
                    vec![71, 71, 77] => 240,
                    vec![71, 74] => 109,
                ),
            }]),
        }]);
        assert_can_roundtrip(binary_gsub, &expected);
    }
}
