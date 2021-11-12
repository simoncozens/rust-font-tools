use crate::tables::GPOS::GPOSSubtable;
use otspec::layout::classdef::ClassDef;
use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::{DeserializationError, Deserialize, Deserializer, ReaderContext, Serialize};
use otspec_macros::{tables, Serialize};

tables!(
    SequenceLookupRecord {
        uint16 sequenceIndex
        uint16 lookupIndex
    }
    SequenceContextFormat1 {
        [offset_base]
        uint16 format
        Offset16(Coverage) coverage
        CountedOffset16(SequenceRuleSet) seqRuleSets
    }
    SequenceRuleSet {
        [offset_base]
        CountedOffset16(SequenceRule) sequenceRules
    }
    SequenceContextFormat2 {
        [offset_base]
        uint16 format
        Offset16(Coverage) coverage
        Offset16(ClassDef) classDef
        // In theory, this should be a ClassSequenceRuleset. But a
        // ClassSequenceRuleset is just a counted array of ClassSequenceRule
        // offsets, and the layout of ClassSequenceRule is identical to
        // that of SequenceRule
        CountedOffset16(SequenceRuleSet) classSeqRuleSets
    }
    ChainedSequenceContextFormat1 {
        [offset_base]
        uint16 format
        Offset16(Coverage) coverage
        CountedOffset16(ChainedSequenceRuleSet) chainedSeqRuleSets
    }
    ChainedSequenceRuleSet {
        [offset_base]
        CountedOffset16(ChainedSequenceRule) chainedSequenceRules
    }
    ChainedSequenceContextFormat2 {
        [offset_base]
        uint16 format
        Offset16(Coverage) coverage
        Offset16(ClassDef) backtrackClassDef
        Offset16(ClassDef) inputClassDef
        Offset16(ClassDef) lookaheadClassDef
        // See above - should be a ChainedClassSequenceRuleset, but they're the same
        CountedOffset16(ChainedSequenceRuleSet) chainedClassSeqRuleSets
    }
    ChainedSequenceContextFormat3 {
        [offset_base]
        uint16 format
        CountedOffset16(Coverage) backtrackCoverages
        CountedOffset16(Coverage) inputCoverages
        CountedOffset16(Coverage) lookaheadCoverages
        Counted(SequenceLookupRecord) sequenceLookupRecords
    }
);

// Needs to be handled manually because of awkward layout and [glyphcount-1] array
#[allow(missing_docs, non_snake_case)]
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct SequenceRule {
    pub glyphCount: uint16,
    pub seqLookupCount: uint16,
    pub inputSequence: Vec<uint16>,
    pub seqLookupRecords: Vec<SequenceLookupRecord>,
}

impl Deserialize for SequenceRule {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let glyphCount: uint16 = c.de()?;
        let seqLookupCount: uint16 = c.de()?;
        let inputSequence: Vec<uint16> = c.de_counted(glyphCount as usize - 1)?;
        let seqLookupRecords: Vec<SequenceLookupRecord> = c.de_counted(seqLookupCount as usize)?;
        Ok(SequenceRule {
            glyphCount,
            seqLookupCount,
            inputSequence,
            seqLookupRecords,
        })
    }
}

// Needs to be handled manually because of awkward layout
#[allow(missing_docs, non_snake_case)]
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct SequenceContextFormat3 {
    pub format: uint16,
    pub glyphCount: uint16,
    pub seqLookupCount: uint16,
    pub coverages: Vec<Offset16<Coverage>>,
    pub seqLookupRecords: Vec<SequenceLookupRecord>,
}

impl Deserialize for SequenceContextFormat3 {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let format: uint16 = c.de()?;
        let glyphCount: uint16 = c.de()?;
        let seqLookupCount: uint16 = c.de()?;
        let coverages: Vec<Offset16<Coverage>> = c.de_counted(glyphCount as usize)?;
        let seqLookupRecords: Vec<SequenceLookupRecord> = c.de_counted(seqLookupCount as usize)?;
        c.pop();
        Ok(SequenceContextFormat3 {
            format,
            glyphCount,
            seqLookupCount,
            coverages,
            seqLookupRecords,
        })
    }
}

// Needs to be handled manually because of awkward layout and [glyphcount-1] array
#[allow(missing_docs, non_snake_case)]
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ChainedSequenceRule {
    #[otspec(with = "Counted")]
    pub backtrackSequence: Vec<uint16>,
    pub inputGlyphCount: uint16,
    pub inputSequence: Vec<uint16>,
    #[otspec(with = "Counted")]
    pub lookaheadSequence: Vec<uint16>,
    #[otspec(with = "Counted")]
    pub seqLookupRecords: Vec<SequenceLookupRecord>,
}

impl Deserialize for ChainedSequenceRule {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError>
    where
        Self: std::marker::Sized,
    {
        let backtrackGlyphCount: uint16 = c.de()?;
        let backtrackSequence: Vec<uint16> = c.de_counted(backtrackGlyphCount as usize)?;
        let inputGlyphCount: uint16 = c.de()?;
        let inputSequence: Vec<uint16> = c.de_counted(inputGlyphCount as usize - 1)?;
        let lookaheadGlyphCount: uint16 = c.de()?;
        let lookaheadSequence: Vec<uint16> = c.de_counted(lookaheadGlyphCount as usize)?;
        let seqLookupRecordCount: uint16 = c.de()?;
        let seqLookupRecords: Vec<SequenceLookupRecord> =
            c.de_counted(seqLookupRecordCount as usize)?;
        Ok(ChainedSequenceRule {
            backtrackSequence,
            inputGlyphCount,
            lookaheadSequence,
            inputSequence,
            seqLookupRecords,
        })
    }
}

pub(crate) fn deserialize_gpos7(
    c: &mut crate::ReaderContext,
) -> Result<GPOSSubtable, crate::DeserializationError> {
    match c.peek(2)? {
        [0x00, 0x01] => {
            let st: SequenceContextFormat1 = c.de()?;
            Ok(GPOSSubtable::GPOS7_1(st))
        }
        [0x00, 0x02] => {
            let st: SequenceContextFormat2 = c.de()?;
            Ok(GPOSSubtable::GPOS7_2(st))
        }
        [0x00, 0x03] => {
            let st: SequenceContextFormat3 = c.de()?;
            Ok(GPOSSubtable::GPOS7_3(st))
        }
        _ => Err(crate::DeserializationError("Bad GPOS7 format".to_string())),
    }
}

pub(crate) fn deserialize_gpos8(
    c: &mut crate::ReaderContext,
) -> Result<GPOSSubtable, crate::DeserializationError> {
    match c.peek(2)? {
        [0x00, 0x01] => {
            let st: ChainedSequenceContextFormat1 = c.de()?;
            Ok(GPOSSubtable::GPOS8_1(st))
        }
        [0x00, 0x02] => {
            let st: ChainedSequenceContextFormat2 = c.de()?;
            Ok(GPOSSubtable::GPOS8_2(st))
        }
        [0x00, 0x03] => {
            let st: ChainedSequenceContextFormat3 = c.de()?;
            Ok(GPOSSubtable::GPOS8_3(st))
        }
        _ => Err(crate::DeserializationError("Bad GPOS8 format".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::{
        FeatureList, FeatureRecord, FeatureTable, LookupFlags, ScriptList, ScriptRecord,
    };
    use crate::layout::coverage::Coverage;
    use crate::layout::gpos1::SinglePosFormat1;
    use crate::layout::valuerecord::{ValueRecord, ValueRecordFlags};
    use crate::tables::GPOS::{GPOSLookup, GPOSLookupList, GPOS10};
    use crate::valuerecord;

    #[test]
    fn test_gpos_7() {
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x67, 0x70, 0x37, 0x31, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x02, 0x00, 0x06, 0x00, 0x26, 0x00, 0x07, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x08, 0x00, 0x01, 0x00, 0x28, 0x00, 0x01, 0x00, 0x08, 0x00, 0x01, 0x00, 0x04,
            0x00, 0x03, 0x00, 0x01, 0x00, 0x43, 0x00, 0x44, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x01, 0x00, 0x08, 0x00, 0x04, 0x00, 0x14,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x42,
        ];
        let gpos: GPOS10 = otspec::de::from_bytes(&binary_gpos).unwrap();
        let contextual = SequenceContextFormat1 {
            format: 1,
            coverage: Offset16::to(Coverage { glyphs: vec![66] }),
            seqRuleSets: vec![Offset16::to(SequenceRuleSet {
                sequenceRules: vec![Offset16::to(SequenceRule {
                    glyphCount: 3,
                    seqLookupCount: 1,
                    inputSequence: vec![67, 68],
                    seqLookupRecords: vec![SequenceLookupRecord {
                        sequenceIndex: 0,
                        lookupIndex: 1,
                    }],
                })]
                .into(),
            })]
            .into(),
        };
        let reposition = SinglePosFormat1 {
            posFormat: 1,
            coverage: Offset16::to(Coverage { glyphs: vec![66] }),
            valueFormat: ValueRecordFlags::X_ADVANCE,
            valueRecord: valuerecord!(xAdvance = 20),
        };
        assert_eq!(
            gpos,
            GPOS10 {
                majorVersion: 1,
                minorVersion: 0,
                scriptList: Offset16::to(ScriptList {
                    scriptRecords: vec![ScriptRecord::default_with_indices(vec![0])],
                }),
                featureList: Offset16::to(FeatureList {
                    featureRecords: vec![FeatureRecord {
                        featureTag: Tag::from_raw("gp71").unwrap(),
                        feature: Offset16::to(FeatureTable {
                            featureParamsOffset: 0,
                            lookupListIndices: vec![0]
                        })
                    }],
                }),
                lookupList: Offset16::to(GPOSLookupList {
                    lookups: vec![
                        Offset16::to(GPOSLookup {
                            lookupType: 7,
                            lookupFlag: LookupFlags::empty(),
                            markFilteringSet: None,
                            subtables: vec![Offset16::to(GPOSSubtable::GPOS7_1(contextual)),]
                                .into()
                        }),
                        Offset16::to(GPOSLookup {
                            lookupType: 1,
                            lookupFlag: LookupFlags::empty(),
                            markFilteringSet: None,
                            subtables: vec![Offset16::to(GPOSSubtable::GPOS1_1(reposition)),]
                                .into()
                        })
                    ]
                    .into()
                })
            }
        );

        // We can't currently round-trip because fonttools shares offsets for
        // equivalent subtables (in this case, the coverage table), and we don't

        // let gpos_ser = otspec::ser::to_bytes(&gpos).unwrap();
        // assert_eq!(gpos_ser, binary_gpos);
    }

    #[test]
    fn test_gpos_8_1() {
        /*feature test {
            pos x a' lookup one lookup two b' c' lookup three;
            pos a' lookup two y z;
            pos x c' lookup three y;
        } test;*/
        let binary_lookup = vec![
            0x00, 0x02, 0x00, 0x12, 0x00, 0x1c, 0x00, 0x24, 0x00, 0x34, 0x00, 0x03, 0x00, 0x3e,
            0x00, 0x40, 0x00, 0x70, 0x00, 0x02, 0x00, 0x01, 0x00, 0x42, 0x00, 0x47, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x59, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x42,
            0x00, 0x44, 0x00, 0x02, 0x00, 0x45, 0x00, 0x47, 0x00, 0x01, 0x00, 0x01, 0x00, 0x5a,
            0x00, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x06, 0x00, 0x1a,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x02, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x04, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x01,
            0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        ];
        let cscf1: ChainedSequenceContextFormat2 = otspec::de::from_bytes(&binary_lookup).unwrap();
        println!("{:#?}", cscf1);
        panic!();
    }
}
