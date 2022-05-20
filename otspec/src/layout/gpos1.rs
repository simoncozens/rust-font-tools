use crate::layout::coverage::Coverage;
use crate::layout::valuerecord::{ValueRecord, ValueRecordFlags};
use crate::tables::GPOS::GPOSSubtable;
use crate::types::*;
use crate::Deserializer;
use otspec_macros::Serialize;

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct SinglePosFormat1 {
    #[otspec(offset_base)]
    pub posFormat: uint16,
    pub coverage: Offset16<Coverage>,
    pub valueFormat: ValueRecordFlags,
    #[otspec(embed)]
    pub valueRecord: ValueRecord,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct SinglePosFormat2 {
    #[otspec(offset_base)]
    pub posFormat: uint16,
    pub coverage: Offset16<Coverage>,
    pub valueFormat: ValueRecordFlags,
    #[otspec(with = "Counted")]
    pub valueRecords: Vec<ValueRecord>,
}

pub(crate) fn deserialize_gpos1(
    c: &mut crate::ReaderContext,
) -> Result<GPOSSubtable, crate::DeserializationError> {
    c.push();
    let format: uint16 = c.de()?;
    let coverage: Offset16<Coverage> = c.de()?;
    let value_format: ValueRecordFlags = c.de()?;
    match format {
        1 => {
            let vr: ValueRecord = ValueRecord::from_bytes(c, value_format)?;
            let spf1 = SinglePosFormat1 {
                posFormat: 1,
                coverage,
                valueFormat: value_format,
                valueRecord: vr,
            };
            c.pop();
            Ok(GPOSSubtable::GPOS1_1(spf1))
        }
        2 => {
            let vr_count: uint16 = c.de()?;
            let mut vrs: Vec<ValueRecord> = vec![];
            for _ in 0..vr_count {
                vrs.push(ValueRecord::from_bytes(c, value_format)?);
            }
            let spf2 = SinglePosFormat2 {
                posFormat: 1,
                coverage,
                valueFormat: value_format,
                valueRecords: vrs,
            };
            c.pop();
            Ok(GPOSSubtable::GPOS1_2(spf2))
        }
        f => Err(crate::DeserializationError(format!(
            "Bad single pos format {}",
            f
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::{
        FeatureList, FeatureRecord, FeatureTable, LookupFlags, ScriptList, ScriptRecord,
    };
    use crate::layout::coverage::Coverage;
    use crate::layout::valuerecord::ValueRecord;
    use crate::tables::GPOS::{GPOSLookup, GPOSLookupList, GPOS10};
    use crate::valuerecord;

    #[test]
    fn test_gpos_1() {
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, // GPOS 1.0
            0x00, 0x0a, // scriptlist offset
            0x00, 0x1e, // featurelist offset
            0x00, 0x2c, // lookuplist offset
            /* 0x0a */ 0x00, 0x01, // ScriptList.scriptCount
            0x44, 0x46, 0x4c, 0x54, // ScriptRecord.scriptTag = DFLT
            0x00, 0x08, // ScriptRecord.scriptOffset
            0x00, 0x04, // Script.defaultLangSysOffset
            0x00, 0x00, // Script.langSysCount
            0x00, 0x00, // LangSys.lookupOrderOffset
            0xff, 0xff, // LangSys.requiredFeatureIndex
            0x00, 0x01, // LangSys.featureIndexCount
            0x00, 0x00, // LangSys.featureIndices
            /* 0x1e */ 0x00, 0x01, // FeatureList.featureCount
            0x6b, 0x65, 0x72, 0x6e, //FeatureRecord.featureTag = kern
            0x00, 0x08, // FeatureRecord.featureOffset
            0x00, 0x00, // Feature.featureParamsOffset
            0x00, 0x01, // Feature.lookupIndexCount
            0x00, 0x00, // Feature.lookupListIndices
            /* 0x2c */ 0x00, 0x01, // LookupList.lookupCount
            0x00, 0x04, // LookupList.lookupOffsets
            0x00, 0x01, // Lookup.lookupType
            0x00, 0x00, // Lookup.lookupFlags
            0x00, 0x01, // Lookup.subtableCount
            0x00, 0x08, // Lookup.subtableOffsets
            0x00, 0x01, 0x00, 0x08, 0x00, 0x04, 0x00, 0x23, 0x00, 0x01, 0x00, 0x03, 0x00, 0x25,
            0x00, 0x30, 0x00, 0x32,
        ];
        let gpos: GPOS10 = otspec::de::from_bytes(&binary_gpos).unwrap();
        let pos = SinglePosFormat1 {
            posFormat: 1,
            coverage: Offset16::to(Coverage {
                glyphs: vec![37, 48, 50],
            }),
            valueFormat: ValueRecordFlags::X_ADVANCE,
            valueRecord: valuerecord!(xAdvance = 35),
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
                        featureTag: Tag::from_raw("kern").unwrap(),
                        feature: Offset16::to(FeatureTable {
                            featureParamsOffset: 0,
                            lookupListIndices: vec![0]
                        })
                    }],
                }),
                lookupList: Offset16::to(GPOSLookupList {
                    lookups: vec![Offset16::to(GPOSLookup {
                        lookupType: 1,
                        lookupFlag: LookupFlags::empty(),
                        markFilteringSet: None,
                        subtables: vec![Offset16::to(GPOSSubtable::GPOS1_1(pos))].into()
                    })]
                    .into()
                })
            }
        );
        let gpos_ser = otspec::ser::to_bytes(&gpos).unwrap();
        assert_eq!(gpos_ser, binary_gpos);
    }
}
