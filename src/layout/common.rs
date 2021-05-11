use otspec::types::*;
use otspec_macros::tables;
use serde::{Deserialize, Serialize};

tables!(
    ScriptList {
        Counted(ScriptRecord) scriptRecords
    }
    ScriptRecord {
        Tag scriptTag
        uint16 scriptOffset
    }
    Script {
        uint16 defaultLangSysOffset
        Counted(LangSysRecord) langSysRecords
    }
    LangSysRecord {
        Tag langSysTag
        uint16 langSysOffset
    }
    LangSys {
        uint16	lookupOrderOffset
        uint16	requiredFeatureIndex
        Counted(uint16) featureIndices
    }
    FeatureList {
            Counted(FeatureRecord) featureRecords
    }
    FeatureRecord {
            Tag	featureTag
            uint16	featureOffset
    }
    FeatureTable {
            uint16	featureParamsOffset
            Counted(uint16) lookupListIndices
    }
    LookupList {
            Counted(uint16) lookupOffsets
    }
    Lookup {
            uint16	lookupType
            uint16	lookupFlag
            Counted(uint16)	subtableOffsets
            // Optional markFilteringSet
    }

    cvFeatureParams {
        uint16 format
        uint16  featUiLabelNameId
        uint16  featUiTooltipTextNameId
        uint16  sampleTextNameId
        uint16  numNamedParameters
        uint16  firstParamUiLabelNameId
        // everything is horrible
        // Counted(uint24) character
    }
    sizeFeatureParams {
        uint16 designSize
        uint16 subfamilyIdentifier
        uint16 subfamilyNameID
        uint16 smallest
        uint16 largest
    }

);

#[derive(Debug, PartialEq)]
pub enum FeatureParams {
    StylisticSet(uint16, uint16),
    SizeFeature(sizeFeatureParams),
    CharacterVariant(cvFeatureParams),
}
