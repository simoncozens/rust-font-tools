use otspec::layout::classdef::ClassDef;
use otspec::layout::coverage::Coverage;
use otspec::layout::valuerecord::{highest_format, ValueRecord, ValueRecordFlags};
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};

use otspec_macros::Serialize;
use std::collections::BTreeMap;

use crate::format_switching_lookup;

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct PairPosFormat1 {
    #[otspec(offset_base)]
    pub posFormat: uint16,
    pub coverage: Offset16<Coverage>,
    pub valueFormat1: ValueRecordFlags,
    pub valueFormat2: ValueRecordFlags,
    #[otspec(with = "Counted")]
    pub pairSets: VecOffset16<PairSet>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct PairSet {
    #[otspec(offset_base)]
    #[otspec(with = "Counted")]
    pub pairValueRecords: Vec<PairValueRecord>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct PairValueRecord {
    pub secondGlyph: uint16,
    #[otspec(embed)]
    pub valueRecord1: ValueRecord,
    #[otspec(embed)]
    pub valueRecord2: ValueRecord,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct PairPosFormat2 {
    #[otspec(offset_base)]
    pub posFormat: uint16,
    pub coverage: Offset16<Coverage>,
    pub valueFormat1: ValueRecordFlags,
    pub valueFormat2: ValueRecordFlags,
    pub classDef1: Offset16<ClassDef>,
    pub classDef2: Offset16<ClassDef>,
    pub classCount1: uint16,
    pub classCount2: uint16,
    pub class1Records: Vec<Class1Record>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct Class1Record {
    pub class2Records: Vec<Class2Record>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct Class2Record {
    #[otspec(embed)]
    pub valueRecord1: ValueRecord,
    #[otspec(embed)]
    pub valueRecord2: ValueRecord,
}

format_switching_lookup!(PairPos { Format1, Format2 });

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

impl Deserialize for PairPos {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let mut mapping = BTreeMap::new();
        let format: uint16 = c.de()?;

        let coverage: Offset16<Coverage> = c.de()?;
        let value_format1: ValueRecordFlags = c.de()?;
        let value_format2: ValueRecordFlags = c.de()?;
        match format {
            1 => {
                let pair_set_count: uint16 = c.de()?;
                let offsets: Vec<uint16> = c.de_counted(pair_set_count.into())?;
                for (left_glyph_id, &offset) in
                    coverage.as_ref().unwrap().glyphs.iter().zip(offsets.iter())
                {
                    c.ptr = c.top_of_table() + offset as usize;
                    let pair_vr_count: uint16 = c.de()?;
                    for _ in 0..pair_vr_count {
                        let right_glyph_id: uint16 = c.de()?;
                        let mut vr1 = ValueRecord::from_bytes(c, value_format1)?;
                        vr1.simplify();
                        let mut vr2 = ValueRecord::from_bytes(c, value_format2)?;
                        vr2.simplify();
                        mapping.insert((*left_glyph_id, right_glyph_id), (vr1, vr2));
                    }
                }
            }
            2 => {
                let classdef_1_off: Offset16<ClassDef> = c.de()?;
                let classdef_2_off: Offset16<ClassDef> = c.de()?;
                let class1_count: uint16 = c.de()?;
                let class2_count: uint16 = c.de()?;
                let classdef_1 = classdef_1_off.link.unwrap_or_default();
                let classdef_2 = classdef_2_off.link.unwrap_or_default();

                // All covered glyphs not in any other class
                let mut left_class0_glyphs: Vec<GlyphID> = coverage.link.unwrap_or_default().glyphs;
                left_class0_glyphs.retain(|g| !classdef_1.classes.contains_key(g));

                for c1 in 0..class1_count {
                    // I'm sure clever people could just store a Box<dyn Iterator> here,
                    // but I can't get the second one to live long enough.
                    let glyphs1: Vec<GlyphID> = if c1 == 0 {
                        left_class0_glyphs.iter().copied().collect()
                    } else {
                        classdef_1.get_glyphs(c1).iter().copied().collect()
                    };

                    for c2 in 0..class2_count {
                        let mut vr1 = ValueRecord::from_bytes(c, value_format1)?;
                        vr1.simplify();
                        let mut vr2 = ValueRecord::from_bytes(c, value_format2)?;
                        vr2.simplify();
                        if !(vr1.has_any() || vr2.has_any()) {
                            continue;
                        }
                        if c2 == 0 {
                            // This is unfortunate. Really unfortunate. A font
                            // has decided to kern against class 0 ("Every other
                            // glyph in the font"), which is legal but extremely
                            // rare, and because we're decoding this subtable in a
                            // context-free way, at this stage we don't have a list
                            // of all the glyph IDs in the font. So we can't tell
                            // what glyphs are affected.
                            panic!("Our architectural assumptions don't allow for class 0 in pair positioning rules");
                        }
                        for left_glyph_id in &glyphs1 {
                            for right_glyph_id in classdef_2.get_glyphs(c2).iter() {
                                mapping.insert(
                                    (*left_glyph_id, *right_glyph_id),
                                    (vr1.clone(), vr2.clone()),
                                );
                            }
                        }
                    }
                }
            }
            _ => panic!("Bad pair pos format {:?}", format),
        }
        c.pop();
        Ok(PairPos { mapping })
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

impl From<&PairPos> for PairPosInternal {
    fn from(val: &PairPos) -> Self {
        let mut mapping = val.mapping.clone();
        for (_, (val1, val2)) in mapping.iter_mut() {
            (*val1).simplify();
            (*val2).simplify();
        }
        let fmt = best_format(&mapping);
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

        if fmt == 1 {
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
            PairPosInternal::Format1(format1)
        } else {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use otspec::{btreemap, valuerecord};
    use std::iter::FromIterator;

    #[test]
    fn some_kerns_de() {
        let binary_pos = vec![
            0x00, 0x01, 0x00, 0x0e, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02, 0x00, 0x16, 0x00, 0x20,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x01, 0x4c, 0x00, 0x02, 0x01, 0x21, 0xff, 0xa6,
            0x01, 0x4c, 0xff, 0x6a, 0x00, 0x01, 0x03, 0x41, 0x00, 0x64,
        ];
        let de: PairPos = otspec::de::from_bytes(&binary_pos).unwrap();
        assert_eq!(
            de,
            PairPos {
                mapping: btreemap!(
                    (0,289)   => (valuerecord!(xAdvance=-90),  valuerecord!()),
                    (0,332)   => (valuerecord!(xAdvance=-150), valuerecord!()),
                    (332,833) => (valuerecord!(xAdvance=100),  valuerecord!()),
                )
            }
        );
    }

    #[test]
    fn some_kerns_ser() {
        let binary_pos = vec![
            0x00, 0x01, 0x00, 0x0e, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02, 0x00, 0x16, 0x00, 0x20,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x01, 0x4c, 0x00, 0x02, 0x01, 0x21, 0xff, 0xa6,
            0x01, 0x4c, 0xff, 0x6a, 0x00, 0x01, 0x03, 0x41, 0x00, 0x64,
        ];
        let kerntable = PairPos {
            mapping: btreemap!(
                (0,289)   => (valuerecord!(xAdvance=-90),  valuerecord!()),
                (0,332)   => (valuerecord!(xAdvance=-150), valuerecord!()),
                (332,833) => (valuerecord!(xAdvance=100),  valuerecord!()),
            ),
        };
        let serialized = otspec::ser::to_bytes(&kerntable).unwrap();
        assert_eq!(serialized, binary_pos);
    }

    #[test]
    fn class_kerns_de() {
        let binary_pos = vec![
            /* pos [D O Q] [T V W] -26; */
            0x00, 0x02, 0x00, 0x14, 0x00, 0x04, 0x00, 0x00, 0x00, 0x1e, 0x00, 0x22, 0x00, 0x01,
            0x00, 0x02, 0x00, 0x00, 0xff, 0xe6, 0x00, 0x01, 0x00, 0x03, 0x00, 0x25, 0x00, 0x30,
            0x00, 0x32, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x35, 0x00, 0x04, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
        ];
        let de: PairPos = otspec::de::from_bytes(&binary_pos).unwrap();
        assert_eq!(
            de,
            PairPos {
                mapping: btreemap!(
                    (37,53)   => (valuerecord!(xAdvance=-26),  valuerecord!()),
                    (37,55)   => (valuerecord!(xAdvance=-26),  valuerecord!()),
                    (37,56)   => (valuerecord!(xAdvance=-26),  valuerecord!()),
                    (48,53)   => (valuerecord!(xAdvance=-26),  valuerecord!()),
                    (48,55)   => (valuerecord!(xAdvance=-26),  valuerecord!()),
                    (48,56)   => (valuerecord!(xAdvance=-26),  valuerecord!()),
                    (50,53)   => (valuerecord!(xAdvance=-26),  valuerecord!()),
                    (50,55)   => (valuerecord!(xAdvance=-26),  valuerecord!()),
                    (50,56)   => (valuerecord!(xAdvance=-26),  valuerecord!()),
                ),
            },
        );
    }
}
