use bitflags::bitflags;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_remainder};
use otspec_macros::tables;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

tables!(
    ScriptListInternal {
        Counted(ScriptRecord) scriptRecords
    }
    ScriptRecord {
        Tag scriptTag
        uint16 scriptOffset
    }
    ScriptInternal {
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

#[derive(Debug, PartialEq, Clone)]
pub struct ScriptList {
    scripts: HashMap<Tag, Script>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Script {
    default_language_system: Option<LanguageSystem>,
    language_systems: HashMap<Tag, LanguageSystem>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LanguageSystem {
    required_feature: Option<usize>,
    feature_indices: Vec<usize>,
}

deserialize_visitor!(
    ScriptList,
    ScriptListVisitor,
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<ScriptList, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let sl = read_field!(seq, ScriptListInternal, "A script list");
        let remainder = read_remainder!(seq, "Script records");
        let base = 2 + (4 * sl.scriptRecords.len());
        let scripts = HashMap::new();
        for rec in sl.scriptRecords {
            let script_base = rec.scriptOffset as usize - base;
            let si: ScriptInternal = otspec::de::from_bytes(&remainder[script_base..]).unwrap();
            if si.defaultLangSysOffset > 0 {
                //
            }
            // XXX
        }
        Ok(ScriptList { scripts })
    }
);
