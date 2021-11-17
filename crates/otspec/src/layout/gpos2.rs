use crate::layout::classdef::ClassDef;
use crate::layout::coverage::Coverage;
use crate::layout::valuerecord::{ValueRecord, ValueRecordFlags};
use crate::tables::GPOS::GPOSSubtable;
use crate::types::*;
use crate::Deserializer;
use otspec_macros::Serialize;

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct PairPosFormat1 {
    #[otspec(offset_base)]
    pub posFormat: GlyphID,
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
    pub secondGlyph: GlyphID,
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

pub(crate) fn deserialize_gpos2(
    c: &mut crate::ReaderContext,
) -> Result<GPOSSubtable, crate::DeserializationError> {
    c.push();
    let format: uint16 = c.de()?;

    let coverage: Offset16<Coverage> = c.de()?;
    let value_format1: ValueRecordFlags = c.de()?;
    let value_format2: ValueRecordFlags = c.de()?;
    match format {
        1 => {
            let pair_set_count: uint16 = c.de()?;
            let offsets: Vec<uint16> = c.de_counted(pair_set_count.into())?;
            let mut pair_sets = vec![];
            for (_left_glyph_id, &offset) in
                coverage.as_ref().unwrap().glyphs.iter().zip(offsets.iter())
            {
                c.ptr = c.top_of_table() + offset as usize;
                let pair_vr_count: uint16 = c.de()?;
                let mut pair_value_records = vec![];
                for _ in 0..pair_vr_count {
                    let right_glyph_id: uint16 = c.de()?;
                    let vr1 = ValueRecord::from_bytes(c, value_format1)?;
                    let vr2 = ValueRecord::from_bytes(c, value_format2)?;
                    pair_value_records.push(PairValueRecord {
                        secondGlyph: right_glyph_id,
                        valueRecord1: vr1,
                        valueRecord2: vr2,
                    })
                }
                pair_sets.push(Offset16::new(
                    offset,
                    PairSet {
                        pairValueRecords: pair_value_records,
                    },
                ))
            }

            c.pop();
            Ok(GPOSSubtable::GPOS2_1(PairPosFormat1 {
                posFormat: 1,
                coverage,
                valueFormat1: value_format1,
                valueFormat2: value_format2,
                pairSets: pair_sets.into(),
            }))
        }
        2 => {
            let classdef_1_off: Offset16<ClassDef> = c.de()?;
            let classdef_2_off: Offset16<ClassDef> = c.de()?;
            let class1_count: uint16 = c.de()?;
            let class2_count: uint16 = c.de()?;
            let mut class1_records = vec![];

            for _c1 in 0..class1_count {
                let mut class2_records: Vec<Class2Record> = vec![];
                for _c2 in 0..class2_count {
                    let vr1 = ValueRecord::from_bytes(c, value_format1)?;
                    let vr2 = ValueRecord::from_bytes(c, value_format2)?;
                    class2_records.push(Class2Record {
                        valueRecord1: vr1,
                        valueRecord2: vr2,
                    })
                }
                class1_records.push(Class1Record {
                    class2Records: class2_records,
                })
            }
            c.pop();
            Ok(GPOSSubtable::GPOS2_2(PairPosFormat2 {
                posFormat: 2,
                coverage,
                valueFormat1: value_format1,
                valueFormat2: value_format2,
                classDef1: classdef_1_off,
                classDef2: classdef_2_off,
                classCount1: class1_count,
                classCount2: class2_count,
                class1Records: class1_records,
            }))
        }
        _ => panic!("Bad pair pos format {:?}", format),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::{
        FeatureList, FeatureRecord, FeatureTable, LookupFlags, ScriptList, ScriptRecord,
    };
    use crate::layout::coverage::Coverage;
    use crate::layout::valuerecord::{ValueRecord, ValueRecordFlags};
    use crate::offsetmanager::OffsetManager;
    use crate::tables::GPOS::{GPOSLookup, GPOSLookupList, GPOS10};
    use crate::{btreemap, valuerecord};
    use std::iter::FromIterator;

    #[test]
    fn test_gpos_22_ser() {
        let pairpos2 = PairPosFormat2 {
            posFormat: 2,
            coverage: Offset16::to(Coverage {
                glyphs: vec![37, 48, 50],
            }),
            valueFormat1: ValueRecordFlags::X_ADVANCE,
            valueFormat2: ValueRecordFlags::empty(),
            classDef1: Offset16::to(ClassDef {
                classes: btreemap!(),
            }),
            classDef2: Offset16::to(ClassDef {
                classes: btreemap!(34 => 2, 35 => 2, 36 => 2, 37 => 1, 38 => 1, 39 => 1 ),
            }),
            classCount1: 1,
            classCount2: 3,
            class1Records: vec![Class1Record {
                class2Records: vec![
                    Class2Record {
                        valueRecord1: valuerecord!(xAdvance = 0),
                        valueRecord2: valuerecord!(),
                    },
                    Class2Record {
                        valueRecord1: valuerecord!(xAdvance = 10),
                        valueRecord2: valuerecord!(),
                    },
                    Class2Record {
                        valueRecord1: valuerecord!(xAdvance = 5),
                        valueRecord2: valuerecord!(),
                    },
                ],
            }],
        };
        let binary_pairpos = vec![
            0x00, 0x02, 0x00, 0x16, // offset to coverage
            0x00, 0x04, // ValueFormat1
            0x00, 0x00, // ValueFormat2
            0x00, 0x20, // classDef1Offset
            0x00, 0x24, // classDef2Offset
            0x00, 0x01, // classDef1Count = 1
            0x00, 0x03, // classDef2Count = 3
            0x00, 0x00, // Class1Record[0] -> Class2Record[0] -> ValueRecord1
            0x00, 0x0a, // Class1Record[0] -> Class2Record[1] -> ValueRecord1
            0x00, 0x05, // Class1Record[0] -> Class2Record[2] -> ValueRecord1
            0x00, 0x01, // Coverage Format
            0x00, 0x03, // Coverage count
            0x00, 0x25, 0x00, 0x30, 0x00, 0x32, // coverage glyphs
            0x00, 0x02, // Class def format 2
            0x00, 0x00, //  number of range records
            0x00, 0x02, // Class def format 2
            0x00, 0x02, //  number of range records
            0x00, 0x22, 0x00, 0x24, 0x00, 0x02, 0x00, 0x25, 0x00, 0x27, 0x00, 0x01,
        ];
        let pairpos2_ser = otspec::ser::to_bytes(&pairpos2).unwrap();
        assert_eq!(binary_pairpos, pairpos2_ser);
    }
    #[test]
    fn test_gpos_2() {
        /* feature gp21 {
            pos A B 5;
            pos C D 10;
        } gp21;

        feature gp22 {
            pos [D O Q] [A B C] 15;
            pos [D O Q] [D E F] 20;
        } gp22;
        */
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x20, 0x00, 0x3a, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x67, 0x70, 0x32, 0x31, 0x00, 0x0e, 0x67, 0x70,
            0x32, 0x32, 0x00, 0x14, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x06, 0x00, 0x30, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x08, 0x00, 0x01, 0x00, 0x0e, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02, 0x00, 0x16,
            0x00, 0x1c, 0x00, 0x01, 0x00, 0x02, 0x00, 0x22, 0x00, 0x24, 0x00, 0x01, 0x00, 0x23,
            0x00, 0x05, 0x00, 0x01, 0x00, 0x25, 0x00, 0x0a, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x08, 0x00, 0x02, 0x00, 0x16, 0x00, 0x04, 0x00, 0x00, 0x00, 0x20, 0x00, 0x24,
            0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x05, 0x00, 0x01, 0x00, 0x03,
            0x00, 0x25, 0x00, 0x30, 0x00, 0x32, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02,
            0x00, 0x22, 0x00, 0x24, 0x00, 0x02, 0x00, 0x25, 0x00, 0x27, 0x00, 0x01,
        ];
        let gpos: GPOS10 = otspec::de::from_bytes(&binary_gpos).unwrap();
        let lookup1 = GPOSLookup {
            lookupType: 2,
            lookupFlag: LookupFlags::empty(),
            subtables: vec![Offset16::to(GPOSSubtable::GPOS2_1(PairPosFormat1 {
                posFormat: 1,
                coverage: Offset16::to(Coverage {
                    glyphs: vec![34, 36],
                }),
                valueFormat1: ValueRecordFlags::X_ADVANCE,
                valueFormat2: ValueRecordFlags::empty(),
                pairSets: vec![
                    Offset16::to(PairSet {
                        pairValueRecords: vec![PairValueRecord {
                            secondGlyph: 35,
                            valueRecord1: valuerecord!(xAdvance = 5),
                            valueRecord2: valuerecord!(),
                        }],
                    }),
                    Offset16::to(PairSet {
                        pairValueRecords: vec![PairValueRecord {
                            secondGlyph: 37,
                            valueRecord1: valuerecord!(xAdvance = 10),
                            valueRecord2: valuerecord!(),
                        }],
                    }),
                ]
                .into(),
            }))]
            .into(),
            markFilteringSet: None,
        };
        let lookup2 = GPOSLookup {
            lookupType: 2,
            lookupFlag: LookupFlags::empty(),
            subtables: vec![Offset16::to(GPOSSubtable::GPOS2_2(PairPosFormat2 {
                posFormat: 2,
                coverage: Offset16::to(Coverage {
                    glyphs: vec![37, 48, 50],
                }),
                valueFormat1: ValueRecordFlags::X_ADVANCE,
                valueFormat2: ValueRecordFlags::empty(),
                classDef1: Offset16::to(ClassDef {
                    classes: btreemap!(),
                }),
                classDef2: Offset16::to(ClassDef {
                    classes: btreemap!(34 => 2, 35 => 2, 36 => 2, 37 => 1, 38 => 1, 39 => 1 ),
                }),
                classCount1: 1,
                classCount2: 3,
                class1Records: vec![Class1Record {
                    class2Records: vec![
                        Class2Record {
                            valueRecord1: valuerecord!(xAdvance = 0),
                            valueRecord2: valuerecord!(),
                        },
                        Class2Record {
                            valueRecord1: valuerecord!(xAdvance = 10),
                            valueRecord2: valuerecord!(),
                        },
                        Class2Record {
                            valueRecord1: valuerecord!(xAdvance = 5),
                            valueRecord2: valuerecord!(),
                        },
                    ],
                }],
            }))]
            .into(),
            markFilteringSet: None,
        };

        let expected = GPOS10 {
            majorVersion: 1,
            minorVersion: 0,
            scriptList: Offset16::to(ScriptList {
                scriptRecords: vec![ScriptRecord::default_with_indices(vec![0, 1])],
            }),
            featureList: Offset16::to(FeatureList {
                featureRecords: vec![
                    FeatureRecord {
                        featureTag: Tag::from_raw("gp21").unwrap(),
                        feature: Offset16::to(FeatureTable {
                            featureParamsOffset: 0,
                            lookupListIndices: vec![0],
                        }),
                    },
                    FeatureRecord {
                        featureTag: Tag::from_raw("gp22").unwrap(),
                        feature: Offset16::to(FeatureTable {
                            featureParamsOffset: 0,
                            lookupListIndices: vec![1],
                        }),
                    },
                ],
            }),
            lookupList: Offset16::to(GPOSLookupList {
                lookups: vec![Offset16::to(lookup1), Offset16::to(lookup2)].into(),
            }),
        };
        assert_eq!(gpos, expected);
        let gpos_ser = otspec::ser::to_bytes(&gpos).unwrap();
        let root = Offset16::to(gpos);
        let mut mgr = OffsetManager::new(&root);
        mgr.resolve();
        mgr.dump_graph();
        assert_eq!(gpos_ser, binary_gpos);
    }
}
