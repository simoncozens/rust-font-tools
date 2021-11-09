use otspec::layout::common::{
    FeatureParams, LangSys, LangSysRecord, LookupFlags, Script as ScriptInternal,
    ScriptList as ScriptListInternal, ScriptRecord,
};
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};

use std::collections::BTreeMap; // For predictable ordering
use std::fmt::Debug;

/// A script list
#[derive(Debug, PartialEq, Clone, Default)]
pub struct ScriptList {
    /// A mapping between script tags and `Script` tables.
    pub scripts: BTreeMap<Tag, Script>,
}

/// A Script table, containing information about language systems for a certain script.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Script {
    /// Optionally, a default language system to be used when no specific
    /// language is selected.
    pub default_language_system: Option<LanguageSystem>,
    /// A mapping between language tags and `LanguageSystem` records.
    pub language_systems: BTreeMap<Tag, LanguageSystem>,
}

/// A LanguageSystem table, selecting which features should be applied in the
/// current script/language combination.
#[derive(Debug, PartialEq, Clone)]
pub struct LanguageSystem {
    /// Each language system can define a required feature which must be processed
    /// for this script/language combination.
    pub required_feature: Option<usize>,
    /// A list of indices into the feature table to be processed for this
    /// script language combination.
    pub feature_indices: Vec<usize>,
}

impl From<&LangSys> for LanguageSystem {
    fn from(langsys: &LangSys) -> Self {
        LanguageSystem {
            required_feature: if langsys.requiredFeatureIndex != 0xFFFF {
                Some(langsys.requiredFeatureIndex.into())
            } else {
                None
            },
            feature_indices: langsys.featureIndices.iter().map(|x| *x as usize).collect(),
        }
    }
}

impl From<&LanguageSystem> for LangSys {
    fn from(ls: &LanguageSystem) -> Self {
        LangSys {
            lookupOrderOffset: 0,
            requiredFeatureIndex: ls.required_feature.unwrap_or(0xFFFF) as u16,
            featureIndices: ls.feature_indices.iter().map(|x| *x as uint16).collect(),
        }
    }
}

impl From<&ScriptInternal> for Script {
    fn from(si: &ScriptInternal) -> Self {
        let mut script = Script {
            default_language_system: (*si.defaultLangSys).as_ref().map(|langsys| langsys.into()),
            language_systems: BTreeMap::new(),
        };
        for langsysrecord in &si.langSysRecords {
            let lang_tag = langsysrecord.langSysTag;
            let ls: LanguageSystem = langsysrecord.langSys.as_ref().unwrap().into();
            script.language_systems.insert(lang_tag, ls);
        }
        script
    }
}

impl From<&Script> for ScriptInternal {
    fn from(script: &Script) -> Self {
        let default_lang_sys = if script.default_language_system.is_some() {
            let langsys: LangSys = script.default_language_system.as_ref().unwrap().into();
            Offset16::to(langsys)
        } else {
            Offset16::to_nothing()
        };
        let lang_sys_records: Vec<LangSysRecord> = script
            .language_systems
            .iter()
            .map(|(k, v)| {
                let ls: LangSys = v.into();
                LangSysRecord {
                    langSysTag: *k,
                    langSys: Offset16::to(ls),
                }
            })
            .collect();
        ScriptInternal {
            defaultLangSys: default_lang_sys,
            langSysRecords: lang_sys_records,
        }
    }
}

impl Deserialize for ScriptList {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let sl: ScriptListInternal = c.de()?;
        let mut scripts = BTreeMap::new();
        for rec in sl.scriptRecords {
            let script = rec.script.as_ref().map(Script::from).unwrap();
            scripts.insert(rec.scriptTag, script);
        }
        Ok(ScriptList { scripts })
    }
}

impl From<&ScriptList> for ScriptListInternal {
    fn from(sl: &ScriptList) -> Self {
        let script_records = sl
            .scripts
            .iter()
            .map(|(k, v)| {
                let si: ScriptInternal = v.into();
                ScriptRecord {
                    scriptTag: *k,
                    script: Offset16::to(si),
                }
            })
            .collect();
        ScriptListInternal {
            scriptRecords: script_records,
        }
    }
}

impl Serialize for ScriptList {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let i: ScriptListInternal = self.into();
        i.to_bytes(data)
    }
}

/// A general lookup rule, of whatever type
#[derive(Debug, PartialEq, Clone)]
pub struct Lookup<T> {
    /// Lookup flags
    pub flags: LookupFlags,
    /// The mark filtering set index in the `GDEF` table.
    pub mark_filtering_set: Option<uint16>,
    /// The concrete rule (set of subtables)
    pub rule: T,
}

// GPOS and GSUB tables

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::upper_case_acronyms)]
/// The Glyph Positioning table
pub struct GPOSGSUB<T> {
    /// A list of positioning lookups
    pub lookups: Vec<Lookup<T>>,
    /// A mapping between script tags and `Script` tables.
    pub scripts: ScriptList,
    /// The association between feature tags and the list of indices into the
    /// lookup table used to process this feature, together with any feature parameters.
    pub features: Vec<(Tag, Vec<usize>, Option<FeatureParams>)>,
}

impl<T> Default for GPOSGSUB<T> {
    fn default() -> Self {
        Self {
            lookups: Default::default(),
            scripts: Default::default(),
            features: Default::default(),
        }
    }
}
