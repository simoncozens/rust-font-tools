use bitflags::bitflags;
use otspec::types::*;
use otspec::DeserializationError;
use otspec::Deserialize;
use otspec::Deserializer;
use otspec::ReaderContext;
use otspec_macros::{tables, Deserialize, Serialize};
use std::collections::HashMap;

tables!(
    ScriptListInternal {
        [offset_base]
        Counted(ScriptRecord) scriptRecords
    }
    ScriptRecord {
        Tag scriptTag
        Offset16(ScriptInternal) scriptOffset
    }
    ScriptInternal {
        [offset_base]
        Offset16(LangSys) defaultLangSys
        Counted(LangSysRecord) langSysRecords
    }
    LangSysRecord {
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
        Counted(FeatureRecord) featureRecords
    }
    FeatureRecord {
            Tag	featureTag
            Offset16(FeatureTable)	feature
    }
    FeatureTable {
            uint16	featureParamsOffset
            Counted(uint16) lookupListIndices
    }
    LookupList {
        [offset_base]
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
    pub scripts: HashMap<Tag, Script>,
}

/// A Script table, containing information about language systems for a certain script.
#[derive(Debug, PartialEq, Clone)]
pub struct Script {
    /// Optionally, a default language system to be used when no specific
    /// language is selected.
    pub default_language_system: Option<LanguageSystem>,
    /// A mapping between language tags and `LanguageSystem` records.
    pub language_systems: HashMap<Tag, LanguageSystem>,
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

impl Deserialize for ScriptList {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let sl: ScriptListInternal = c.de()?;
        let mut scripts = HashMap::new();
        for rec in sl.scriptRecords {
            let si: &ScriptInternal = &rec.scriptOffset.as_ref().unwrap();

            let mut script = Script {
                default_language_system: (*si.defaultLangSys)
                    .as_ref()
                    .map(|langsys| langsys.into()),
                language_systems: HashMap::new(),
            };
            for langsysrecord in &si.langSysRecords {
                let lang_tag = langsysrecord.langSysTag;
                let ls: LanguageSystem = langsysrecord.langSys.as_ref().unwrap().into();
                script.language_systems.insert(lang_tag, ls);
            }
            scripts.insert(rec.scriptTag, script);
        }
        Ok(ScriptList { scripts })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    macro_rules! hashmap {
            ($($k:expr => $v:expr),* $(,)?) => {
                std::collections::HashMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
            };
        }

    #[test]
    fn test_scriptlist_internal() {
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
        assert_eq!(
            deserialized,
            ScriptList {
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
            }
        );
    }
}
