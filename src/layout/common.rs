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
);
