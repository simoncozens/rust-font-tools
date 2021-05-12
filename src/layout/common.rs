use bitflags::bitflags;
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
            LookupFlags	lookupFlag
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

bitflags! {
    /// Lookup qualifiers
    #[derive(Serialize, Deserialize)]
    pub struct LookupFlags: u16 {
        /// Position the last glyph of a cursive positioning sequence on the baseline
        const RIGHT_TO_LEFT = 0x0001;
        /// Skip over base glyphs
        const IGNORE_BASE_GLYPHS = 0x0002;
        /// Skip over ligatures
        const IGNORE_LIGATURES = 0x0004;
        /// Skip over all combining marks
        const IGNORE_MARKS = 0x0008;
        /// Indicates that the lookup table structure is followed by a MarkFilteringSet field
        const USE_MARK_FILTERING_SET = 0x0010;
        /// Mask off the high bits to reveal a mark class defined in the GDEF table
        const MARK_ATTACHMENT_TYPE_MASK = 0xFF00;
    }
}
