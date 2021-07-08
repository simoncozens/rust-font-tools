use bitflags::bitflags;
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};
use otspec_macros::{tables, Deserialize, Serialize};
use std::collections::BTreeMap; // For predictable ordering
use std::fmt::Debug;

tables!(
    ScriptListInternal {
        [offset_base]
        [embed]
        Counted(ScriptRecord) scriptRecords
    }
    ScriptRecord [embedded] [nodebug] {
        Tag scriptTag
        Offset16(ScriptInternal) scriptOffset
    }
    ScriptInternal {
        [offset_base]
        Offset16(LangSys) defaultLangSys
        [embed]
        Counted(LangSysRecord) langSysRecords
    }
    LangSysRecord [embedded] {
        Tag langSysTag
        Offset16(LangSys) langSys
    }
    LangSys {
        uint16	lookupOrderOffset // Null, ignore it
        uint16	requiredFeatureIndex
        Counted(uint16) featureIndices
    }
    FeatureList {
        [offset_base]
        [embed]
        Counted(FeatureRecord) featureRecords
    }
    FeatureRecord [embedded] {
            Tag	featureTag
            Offset16(FeatureTable)	feature
    }
    FeatureTable {
            uint16	featureParamsOffset
            Counted(uint16) lookupListIndices
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

impl Debug for ScriptRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(
                f,
                "{} => {:#?}",
                std::str::from_utf8(&self.scriptTag).unwrap(),
                self.scriptOffset.link
            )
        } else {
            write!(
                f,
                "{} => {:?}",
                std::str::from_utf8(&self.scriptTag).unwrap(),
                self.scriptOffset.link
            )
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// Feature parameter data.
///
/// Certain OpenType features may have various ancillary data attached to them.
/// The format of this data varies from feature to feature, so this container
/// wraps the general concept of feature parameter data.
pub enum FeatureParams {
    /// The stylistic set features (`ss01`-`ss20`) may provide two parameters: a
    /// parameter data version, currently set to zero, and a name table ID
    /// which is used to display the stylistic set name to the user.
    StylisticSet(uint16, uint16),
    /// Feature parameter information for the `size` feature, including the
    /// design size, subfamily identifier and name ID, and largest and smallest
    /// intended sizes. This has been superseded by the `STAT` table.
    SizeFeature(sizeFeatureParams),
    /// The character variant features (`cv01`-`cv99`) provide various name
    /// parameters to display information to the user.
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

/// A script list
#[derive(Debug, PartialEq, Clone)]
pub struct ScriptList {
    /// A mapping between script tags and `Script` tables.
    pub scripts: BTreeMap<Tag, Script>,
}

/// A Script table, containing information about language systems for a certain script.
#[derive(Debug, PartialEq, Clone)]
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
            let si: &ScriptInternal = &rec.scriptOffset.as_ref().unwrap();
            let script: Script = si.into();
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
                    scriptOffset: Offset16::to(si),
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

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug, Serialize)]
pub struct gsubgposoutgoing {
    pub majorVersion: uint16,
    pub minorVersion: uint16,
    pub scriptList: Offset16<ScriptList>,
    pub featureList: Offset16<FeatureList>,
    pub lookupList: Offset16<LookupListOutgoing>,
}

// We have to do horrible things for the Lookup table because it has
// a heterogenous subtable vec field.
#[derive(Debug)]
pub struct LookupInternal {
    pub lookupType: uint16,
    pub flags: LookupFlags,
    pub subtables: Vec<Box<dyn OffsetMarkerTrait>>,
    pub mark_filtering_set: Option<uint16>,
}

impl otspec::Serialize for LookupInternal {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        let obj = otspec::offsetmanager::resolve_offsets(self);
        self.to_bytes_shallow(data)?;
        otspec::offsetmanager::resolve_offsets_and_serialize(obj, data, false)?;
        Ok(())
    }
    fn to_bytes_shallow(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        let obj = self;
        obj.lookupType.to_bytes(data)?;
        obj.flags.to_bytes(data)?;
        (obj.subtables.len() as uint16).to_bytes(data)?;
        for st in &obj.subtables {
            st.to_bytes_shallow(data)?;
        }
        obj.mark_filtering_set.to_bytes(data)?;
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        self.lookupType.ot_binary_size()
            + self.flags.ot_binary_size()
            + 2
            + 2 * self.subtables.len()
            + self.mark_filtering_set.ot_binary_size()
    }
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        self.subtables.iter().map(|x| x.as_ref()).collect()
    }
}

impl Clone for LookupInternal {
    fn clone(&self) -> Self {
        panic!("Can't clone this")
    }
}

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug)]
pub struct LookupListOutgoing {
    pub(crate) lookups: VecOffset16<LookupInternal>,
}

impl Serialize for LookupListOutgoing {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        let obj = otspec::offsetmanager::resolve_offsets(self);
        self.to_bytes_shallow(data)?;
        otspec::offsetmanager::resolve_offsets_and_serialize(obj, data, false)?;
        Ok(())
    }
    fn to_bytes_shallow(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        data.put(self.lookups.0.len() as uint16)?;
        self.lookups.0.to_bytes_shallow(data)?;
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        2 + 2 * self.lookups.0.len()
    }
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        let mut v: Vec<&dyn OffsetMarkerTrait> = Vec::new();
        v.extend(self.lookups.offset_fields());
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use otspec::offsetmanager::OffsetManager;
    use std::iter::FromIterator;

    macro_rules! hashmap {
            ($($k:expr => $v:expr),* $(,)?) => {
                std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
            };
        }

    #[test]
    fn test_scriptlist_de() {
        let binary_scriptlist = vec![
            0x00, 0x02, 0x61, 0x72, 0x61, 0x62, 0x00, 0x0E, 0x6C, 0x61, 0x74, 0x6E, 0x00, 0x40,
            0x00, 0x0A, 0x00, 0x01, 0x55, 0x52, 0x44, 0x20, 0x00, 0x1E, 0x00, 0x00, 0xFF, 0xFF,
            0x00, 0x07, 0x00, 0x01, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00, 0x07,
            0x00, 0x08, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x04,
            0x00, 0x05, 0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00,
            0xFF, 0xFF, 0x00, 0x07, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06,
            0x00, 0x07, 0x00, 0x08,
        ];
        let deserialized: ScriptList = otspec::de::from_bytes(&binary_scriptlist).unwrap();
        let script_list: ScriptList = ScriptList {
            scripts: hashmap!(
                *b"arab" => Script {
                    default_language_system: Some(
                        LanguageSystem {
                            required_feature: None,
                            feature_indices: vec![
                                1,
                                3,
                                4,
                                5,
                                6,
                                7,
                                8,
                            ],
                        },
                    ),
                    language_systems: hashmap!(*b"URD " =>
                        LanguageSystem {
                            required_feature: None,
                            feature_indices: vec![
                                0,
                                3,
                                4,
                                5,
                                6,
                                7,
                                8,
                            ],
                        },
                    ),
                },
                *b"latn" => Script {
                    default_language_system: Some(
                        LanguageSystem {
                            required_feature: None,
                            feature_indices: vec![
                                2,
                                3,
                                4,
                                5,
                                6,
                                7,
                                8,
                            ],
                        },
                    ),
                    language_systems: hashmap!(),
                },
            ),
        };
        assert_eq!(deserialized, script_list);
    }

    #[test]
    fn test_scriptlist_ser() {
        let binary_scriptlist = vec![
            0x00, 0x02, 0x61, 0x72, 0x61, 0x62, 0x00, 0x0E, 0x6C, 0x61, 0x74, 0x6E, 0x00, 0x40,
            0x00, 0x0A, 0x00, 0x01, 0x55, 0x52, 0x44, 0x20, 0x00, 0x1E, 0x00, 0x00, 0xFF, 0xFF,
            0x00, 0x07, 0x00, 0x01, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00, 0x07,
            0x00, 0x08, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x04,
            0x00, 0x05, 0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00,
            0xFF, 0xFF, 0x00, 0x07, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06,
            0x00, 0x07, 0x00, 0x08,
        ];
        let script_list: ScriptList = ScriptList {
            scripts: hashmap!(
                *b"arab" => Script {
                    default_language_system: Some(
                        LanguageSystem {
                            required_feature: None,
                            feature_indices: vec![
                                1,
                                3,
                                4,
                                5,
                                6,
                                7,
                                8,
                            ],
                        },
                    ),
                    language_systems: hashmap!(*b"URD " =>
                        LanguageSystem {
                            required_feature: None,
                            feature_indices: vec![
                                0,
                                3,
                                4,
                                5,
                                6,
                                7,
                                8,
                            ],
                        },
                    ),
                },
                *b"latn" => Script {
                    default_language_system: Some(
                        LanguageSystem {
                            required_feature: None,
                            feature_indices: vec![
                                2,
                                3,
                                4,
                                5,
                                6,
                                7,
                                8,
                            ],
                        },
                    ),
                    language_systems: hashmap!(),
                },
            ),
        };

        let mut serialized = vec![];
        let sli: ScriptListInternal = (&script_list).into();
        let root = Offset16::to(sli);
        let mut mgr = OffsetManager::new(&root);
        mgr.resolve();
        mgr.dump_graph();
        mgr.serialize(&mut serialized, true).unwrap();
        assert_eq!(serialized, binary_scriptlist);
    }
}
