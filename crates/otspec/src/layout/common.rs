use crate::{Serialize, Serializer, Tag};
use bitflags::bitflags;
use otspec::layout::anchor::Anchor;
use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::{tables, Deserialize, Serialize};
use std::fmt::Debug;

tables!(
    ScriptList {
        [offset_base]
        [embed]
        Counted(ScriptRecord) scriptRecords
    }
    ScriptRecord [embedded] [nodebug] {
        Tag scriptTag
        Offset16(Script) script
    }
    Script {
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

    MarkArray {
        [offset_base]
        [embed]
        Counted(MarkRecord) markRecords
    }

    MarkRecord [embedded] {
        uint16 markClass
        Offset16(Anchor) markAnchor
    }
    FeatureVariations {
        uint16 majorVersion
        uint16 minorVersion
        Counted32(FeatureVariationRecord) featureVariationRecords
    }
    FeatureVariationRecord {
        Offset32(ConditionSet) conditionSet
        Offset32(FeatureTableSubstitution) featureTableSubstitution
    }
    ConditionSet {
        CountedOffset32(ConditionFormat1) conditions
    }
    ConditionFormat1 {
        uint16 format
        uint16 axisIndex
        F2DOT14 filterRangeMinValue
        F2DOT14 filterRangeMaxValue
    }
    FeatureTableSubstitution {
        uint16 majorVersion
        uint16 minorVersion
        Counted(FeatureTableSubstitutionRecord) substitutions
    }
    FeatureTableSubstitutionRecord {
        uint16  featureIndex
        Offset32(FeatureTable) alternateFeature
    }

);

impl Default for ScriptList {
    fn default() -> Self {
        ScriptList {
            scriptRecords: vec![],
        }
    }
}
impl Default for FeatureList {
    fn default() -> Self {
        FeatureList {
            featureRecords: vec![],
        }
    }
}

impl Debug for ScriptRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{} => {:#?}", self.scriptTag, self.script.link)
        } else {
            write!(f, "{} => {:?}", self.scriptTag, self.script.link)
        }
    }
}

impl ScriptRecord {
    pub fn default_with_indices(feature_indices: Vec<uint16>) -> Self {
        ScriptRecord {
            scriptTag: Tag::from_raw("DFLT").unwrap(), // Not sure I can use tag! macro here?
            script: Offset16::to(Script {
                defaultLangSys: Offset16::to(LangSys {
                    lookupOrderOffset: 0,
                    requiredFeatureIndex: 65535,
                    featureIndices: feature_indices,
                }),
                langSysRecords: vec![],
            }),
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

impl Default for LookupFlags {
    fn default() -> Self {
        LookupFlags::empty()
    }
}

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug, Serialize)]
pub struct gsubgpos {
    pub majorVersion: uint16,
    pub minorVersion: uint16,
    pub scriptList: Offset16<ScriptList>,
    pub featureList: Offset16<FeatureList>,
    pub lookupList: Offset16<LookupList>,
}

// We have to do horrible things for the Lookup table because it has
// a heterogenous subtable vec field.
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug)]
pub struct Lookup {
    pub lookupType: uint16,
    pub flags: LookupFlags,
    pub subtables: Vec<Box<dyn OffsetMarkerTrait>>,
    pub mark_filtering_set: Option<uint16>,
}

impl otspec::Serialize for Lookup {
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

impl Clone for Lookup {
    fn clone(&self) -> Self {
        panic!("Can't clone this")
    }
}

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug)]
pub struct LookupList {
    pub lookups: VecOffset16<Lookup>,
}

impl Serialize for LookupList {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        let obj = otspec::offsetmanager::resolve_offsets(self);
        self.to_bytes_shallow(data)?;
        otspec::offsetmanager::resolve_offsets_and_serialize(obj, data, false)?;
        Ok(())
    }
    fn to_bytes_shallow(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        data.put(self.lookups.v.len() as uint16)?;
        self.lookups.v.to_bytes_shallow(data)?;
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        2 + 2 * self.lookups.v.len()
    }
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        let mut v: Vec<&dyn OffsetMarkerTrait> = Vec::new();
        v.extend(self.lookups.offset_fields());
        v
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::tag;
//     use otspec::offsetmanager::OffsetManager;
//     use std::iter::FromIterator;

//     macro_rules! hashmap {
//             ($($k:expr => $v:expr),* $(,)?) => {
//                 std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
//             };
//         }

//     #[test]
//     fn test_scriptlist_de() {
//         let binary_scriptlist = vec![
//             0x00, 0x02, 0x61, 0x72, 0x61, 0x62, 0x00, 0x0E, 0x6C, 0x61, 0x74, 0x6E, 0x00, 0x40,
//             0x00, 0x0A, 0x00, 0x01, 0x55, 0x52, 0x44, 0x20, 0x00, 0x1E, 0x00, 0x00, 0xFF, 0xFF,
//             0x00, 0x07, 0x00, 0x01, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00, 0x07,
//             0x00, 0x08, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x04,
//             0x00, 0x05, 0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00,
//             0xFF, 0xFF, 0x00, 0x07, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06,
//             0x00, 0x07, 0x00, 0x08,
//         ];
//         let deserialized: ScriptList = otspec::de::from_bytes(&binary_scriptlist).unwrap();
//         let script_list: ScriptList = ScriptList {
//             scripts: hashmap!(
//                 tag!("arab") => Script {
//                     default_language_system: Some(
//                         LanguageSystem {
//                             required_feature: None,
//                             feature_indices: vec![
//                                 1,
//                                 3,
//                                 4,
//                                 5,
//                                 6,
//                                 7,
//                                 8,
//                             ],
//                         },
//                     ),
//                     language_systems: hashmap!(tag!("URD ") =>
//                         LanguageSystem {
//                             required_feature: None,
//                             feature_indices: vec![
//                                 0,
//                                 3,
//                                 4,
//                                 5,
//                                 6,
//                                 7,
//                                 8,
//                             ],
//                         },
//                     ),
//                 },
//                 tag!("latn") => Script {
//                     default_language_system: Some(
//                         LanguageSystem {
//                             required_feature: None,
//                             feature_indices: vec![
//                                 2,
//                                 3,
//                                 4,
//                                 5,
//                                 6,
//                                 7,
//                                 8,
//                             ],
//                         },
//                     ),
//                     language_systems: hashmap!(),
//                 },
//             ),
//         };
//         assert_eq!(deserialized, script_list);
//     }

//     #[test]
//     fn test_scriptlist_ser() {
//         let binary_scriptlist = vec![
//             0x00, 0x02, 0x61, 0x72, 0x61, 0x62, 0x00, 0x0E, 0x6C, 0x61, 0x74, 0x6E, 0x00, 0x40,
//             0x00, 0x0A, 0x00, 0x01, 0x55, 0x52, 0x44, 0x20, 0x00, 0x1E, 0x00, 0x00, 0xFF, 0xFF,
//             0x00, 0x07, 0x00, 0x01, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00, 0x07,
//             0x00, 0x08, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x04,
//             0x00, 0x05, 0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00,
//             0xFF, 0xFF, 0x00, 0x07, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06,
//             0x00, 0x07, 0x00, 0x08,
//         ];
//         let script_list: ScriptList = ScriptList {
//             scripts: hashmap!(
//                 tag!("arab") => Script {
//                     default_language_system: Some(
//                         LanguageSystem {
//                             required_feature: None,
//                             feature_indices: vec![
//                                 1,
//                                 3,
//                                 4,
//                                 5,
//                                 6,
//                                 7,
//                                 8,
//                             ],
//                         },
//                     ),
//                     language_systems: hashmap!(tag!("URD ") =>
//                         LanguageSystem {
//                             required_feature: None,
//                             feature_indices: vec![
//                                 0,
//                                 3,
//                                 4,
//                                 5,
//                                 6,
//                                 7,
//                                 8,
//                             ],
//                         },
//                     ),
//                 },
//                 tag!("latn") => Script {
//                     default_language_system: Some(
//                         LanguageSystem {
//                             required_feature: None,
//                             feature_indices: vec![
//                                 2,
//                                 3,
//                                 4,
//                                 5,
//                                 6,
//                                 7,
//                                 8,
//                             ],
//                         },
//                     ),
//                     language_systems: hashmap!(),
//                 },
//             ),
//         };

//         let mut serialized = vec![];
//         let sli: ScriptListLowLevel = (&script_list).into();
//         let root = Offset16::to(sli);
//         let mut mgr = OffsetManager::new(&root);
//         mgr.resolve();
//         mgr.dump_graph();
//         mgr.serialize(&mut serialized, true).unwrap();
//         assert_eq!(serialized, binary_scriptlist);
//     }
// }
