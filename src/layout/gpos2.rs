use crate::layout::common::{coverage_or_nah, FromLowlevel};
use otspec::layout::coverage::Coverage;
use otspec::layout::gpos2::{PairPosFormat1, PairSet, PairValueRecord};
use otspec::layout::valuerecord::{highest_format, ValueRecord};
use otspec::tables::GPOS::GPOSSubtable;
use otspec::types::*;
use std::collections::BTreeMap;

/// User-friendly mapping between glyph pairs and value record adjustments
pub type PairPositioningMap = BTreeMap<(GlyphID, GlyphID), (ValueRecord, ValueRecord)>;
/// Internal mapping between glyph pairs and value record adjustments, used for serialization
pub type SplitPairPositioningMap = BTreeMap<GlyphID, BTreeMap<GlyphID, (ValueRecord, ValueRecord)>>;

#[derive(Debug, PartialEq, Clone, Default)]
/// A pair positioning subtable.
pub struct PairPos {
    /// The mapping of pair glyph IDs to pairs of value records.
    pub mapping: PairPositioningMap,
}

impl FromLowlevel<GPOSSubtable> for PairPos {
    fn from_lowlevel(st: GPOSSubtable, max_glyph_id: GlyphID) -> Self {
        let mut pairpos = PairPos::default();
        match st {
            GPOSSubtable::GPOS2_1(pairpos1) => {
                let left_glyphs = coverage_or_nah(pairpos1.coverage);
                for (left_glyph, off) in left_glyphs.iter().zip(pairpos1.pairSets.v.into_iter()) {
                    let pair_value_records = off
                        .link
                        .map_or_else(Vec::new, |pairset| pairset.pairValueRecords);
                    for p in pair_value_records {
                        pairpos.mapping.insert(
                            (*left_glyph, p.secondGlyph),
                            (p.valueRecord1, p.valueRecord2),
                        );
                    }
                }
            }
            GPOSSubtable::GPOS2_2(pairpos2) => {
                let classdef_1 = pairpos2.classDef1.link.unwrap_or_default();
                let classdef_2 = pairpos2.classDef2.link.unwrap_or_default();

                for (c1, class1_record) in pairpos2.class1Records.iter().enumerate() {
                    let left_glyphs: Vec<GlyphID> = classdef_1
                        .get_glyphs(c1 as u16, max_glyph_id)
                        .iter()
                        .copied()
                        .collect();
                    for (c2, class2_record) in class1_record.class2Records.iter().enumerate() {
                        let mut vr1 = class2_record.valueRecord1.clone();
                        vr1.simplify();
                        let mut vr2 = class2_record.valueRecord2.clone();
                        vr2.simplify();
                        if !(vr1.has_any() || vr2.has_any()) {
                            continue;
                        }
                        let right_glyphs: Vec<GlyphID> = classdef_2
                            .get_glyphs(c2 as u16, max_glyph_id)
                            .iter()
                            .copied()
                            .collect();
                        for left_glyph_id in &left_glyphs {
                            for right_glyph_id in &right_glyphs {
                                pairpos.mapping.insert(
                                    (*left_glyph_id, *right_glyph_id),
                                    (vr1.clone(), vr2.clone()),
                                );
                            }
                        }
                    }
                }
            }
            _ => panic!(),
        }
        pairpos
    }
}

fn split_into_two_layer(in_hash: PairPositioningMap) -> SplitPairPositioningMap {
    let mut out_hash = BTreeMap::new();
    for (&(l, r), vs) in in_hash.iter() {
        out_hash
            .entry(l)
            .or_insert_with(BTreeMap::new)
            .insert(r, vs.clone());
    }
    out_hash
}

fn best_format(_: &PairPositioningMap) -> uint16 {
    1
}

// We may generate more than one subtable if we go down the format2 route.
impl PairPos {
    pub(crate) fn to_lowlevel_subtables(&self, _max_glyph_id: GlyphID) -> Vec<GPOSSubtable> {
        if best_format(&self.mapping) == 1 {
            return vec![self.to_format_1()];
        }
        // let mut subtables: Vec<GPOSSubtable> = vec![];
        unimplemented!()
    }

    fn to_format_1(&self) -> GPOSSubtable {
        let mut mapping = self.mapping.clone();
        for (_, (val1, val2)) in mapping.iter_mut() {
            (*val1).simplify();
            (*val2).simplify();
        }
        let split_mapping = split_into_two_layer(mapping);
        let coverage = Coverage {
            glyphs: split_mapping.keys().copied().collect(),
        };
        let all_pair_vrs: Vec<&(ValueRecord, ValueRecord)> = split_mapping
            .values()
            .map(|x| x.values())
            .flatten()
            .collect();
        let value_format_1 = highest_format(all_pair_vrs.iter().map(|x| &x.0));
        let value_format_2 = highest_format(all_pair_vrs.iter().map(|x| &x.1));

        let mut pair_sets: Vec<Offset16<PairSet>> = vec![];
        for left in &coverage.glyphs {
            let mut pair_value_records: Vec<PairValueRecord> = vec![];
            for (right, (vr1, vr2)) in split_mapping.get(left).unwrap() {
                pair_value_records.push(PairValueRecord {
                    secondGlyph: *right,
                    valueRecord1: vr1.clone(),
                    valueRecord2: vr2.clone(),
                })
            }
            pair_sets.push(Offset16::to(PairSet {
                pairValueRecords: pair_value_records,
            }));
        }
        let format1: PairPosFormat1 = PairPosFormat1 {
            posFormat: 1,
            coverage: Offset16::to(coverage),
            valueFormat1: value_format_1,
            valueFormat2: value_format_2,
            pairSets: VecOffset16 { v: pair_sets },
        };
        GPOSSubtable::GPOS2_1(format1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::{Lookup, LookupFlags};
    use crate::tables::GPOS::tests::{assert_can_roundtrip, expected_gpos};
    use crate::tables::GPOS::Positioning;
    use otspec::{btreemap, valuerecord};
    use std::iter::FromIterator;

    #[test]
    fn gpos21_deser() {
        /*
        feature test {
            pos A -20 B;
            pos B -30 A;
        } test;
        */
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x0e, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02, 0x00, 0x16, 0x00, 0x1c,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x22, 0x00, 0x23, 0x00, 0x01, 0x00, 0x23, 0xff, 0xec,
            0x00, 0x01, 0x00, 0x22, 0xff, 0xe2,
        ];
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::Pair(vec![PairPos {
                mapping: btreemap!(
                    (34,35) => (valuerecord!(xAdvance = -20),valuerecord!()),
                    (35,34) => (valuerecord!(xAdvance = -30), valuerecord!())
                ),
            }]),
        }]);
        assert_can_roundtrip(binary_gpos, &expected);
    }
}
