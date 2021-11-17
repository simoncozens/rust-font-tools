use crate::layout::common::{FromLowlevel, Lookup, ToLowlevel, GPOSGSUB};
use crate::layout::contextual::{ChainedSequenceContext, SequenceContext};
use crate::layout::gsub1::SingleSubst;
use otspec::tables::GSUB::{GSUBLookup as GSUBLookupLowlevel, GSUBSubtable, GSUB10};
use otspec::types::*;
use otspec::{DeserializationError, Deserializer, ReaderContext, SerializationError, Serialize};

/// The 'GSUB' OpenType tag.
pub const TAG: Tag = crate::tag!("GSUB");

/// A container which represents a generic substitution rule
///
/// Each rule is expressed as a vector of subtables.
#[derive(Debug, PartialEq, Clone)]
pub enum Substitution {
    /// Contains a single substitution rule.
    Single(Vec<SingleSubst>),
    /// Contains a contextual substitution rule.
    Contextual(Vec<SequenceContext>),
    /// Contains a chained contextual substitution rule.
    ChainedContextual(Vec<ChainedSequenceContext>),
    /// Contains an extension subtable.
    Extension,
}

impl Substitution {
    /// Adds a subtable break to this rule
    pub fn add_subtable_break(&mut self) {
        match self {
            Substitution::Single(v) => v.push(SingleSubst::default()),
            Substitution::Contextual(v) => v.push(SequenceContext::default()),
            Substitution::ChainedContextual(v) => v.push(ChainedSequenceContext::default()),
            Substitution::Extension => todo!(),
        }
    }
}

impl Lookup<Substitution> {
    /// Returns the GSUB lookup type for this subtable
    pub fn lookup_type(&self) -> u16 {
        match &self.rule {
            Substitution::Single(_) => 1,
            Substitution::Contextual(_) => 7,
            Substitution::ChainedContextual(_) => 8,
            Substitution::Extension => 9,
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
/// The Glyph Substitution table
pub type GSUB = GPOSGSUB<Substitution>;

pub(crate) fn from_bytes(
    c: &mut ReaderContext,
    max_glyph_id: GlyphID,
) -> Result<GSUB, DeserializationError> {
    match c.peek(4)? {
        [0x00, 0x01, 0x00, 0x00] => {
            let internal: GSUB10 = c.de()?;
            Ok(GSUB::from_lowlevel(internal, max_glyph_id))
        }
        // [0x00, 0x01, 0x00, 0x01] => {
        //     let internal: GSUB11 = c.de()?;
        //     Ok(internal.into())
        // }
        _ => Err(DeserializationError(
            "Invalid GSUB table version".to_string(),
        )),
    }
}

impl FromLowlevel<GSUB10> for GSUB {
    fn from_lowlevel(val: GSUB10, max_glyph_id: GlyphID) -> Self {
        let lookup_list_lowlevel = val.lookupList.link.unwrap_or_default();
        let mut lookups: Vec<Lookup<Substitution>> = vec![];
        for lookup_off in lookup_list_lowlevel.lookups.v {
            if let Some(lookup_lowlevel) = lookup_off.link {
                let subtables: Vec<GSUBSubtable> = lookup_lowlevel
                    .subtables
                    .v
                    .iter()
                    .map(|x| x.link.clone())
                    .flatten()
                    .collect();
                let theirs = match lookup_lowlevel.lookupType {
                    1 => Substitution::Single(
                        subtables
                            .into_iter()
                            .map(|st| SingleSubst::from_lowlevel(st, max_glyph_id))
                            .collect(),
                    ),
                    5 => Substitution::Contextual(
                        subtables
                            .into_iter()
                            .map(|st| SequenceContext::from_lowlevel(st, max_glyph_id))
                            .collect(),
                    ),
                    6 => Substitution::ChainedContextual(
                        subtables
                            .into_iter()
                            .map(|st| ChainedSequenceContext::from_lowlevel(st, max_glyph_id))
                            .collect(),
                    ),
                    x => panic!("Unknown GSUB lookup type {:?}", x),
                };

                let lookup_highlevel: Lookup<Substitution> = Lookup {
                    flags: lookup_lowlevel.lookupFlag,
                    mark_filtering_set: lookup_lowlevel.markFilteringSet,
                    rule: theirs,
                };
                lookups.push(lookup_highlevel)
            }
        }
        GSUB {
            lookups,
            scripts: val.scriptList.link.unwrap_or_default().into(),
            features: val.featureList.link.unwrap_or_default().into(),
        }
    }
}

impl ToLowlevel<GSUBLookupLowlevel> for Lookup<Substitution> {
    fn to_lowlevel(&self, max_glyph_id: GlyphID) -> GSUBLookupLowlevel {
        let subtables: Vec<Offset16<GSUBSubtable>> = match &self.rule {
            Substitution::Single(sp) => sp
                .iter()
                .map(|subtable| Offset16::to(subtable.to_lowlevel(max_glyph_id)))
                .collect(),
            Substitution::Contextual(contextual) => contextual
                .iter()
                .map(|subtable| {
                    subtable
                        .to_lowlevel_subtables_gsub(max_glyph_id)
                        .into_iter()
                        .map(Offset16::to)
                })
                .flatten()
                .collect(),
            Substitution::ChainedContextual(chainedcontextual) => chainedcontextual
                .iter()
                .map(|subtable| {
                    subtable
                        .to_lowlevel_subtables_gsub(max_glyph_id)
                        .into_iter()
                        .map(Offset16::to)
                })
                .flatten()
                .collect(),
            Substitution::Extension => panic!("This can't happen"),
        };
        GSUBLookupLowlevel {
            lookupType: self.lookup_type(),
            lookupFlag: self.flags,
            subtables: subtables.into(),
            markFilteringSet: self.mark_filtering_set,
        }
    }
}
impl ToLowlevel<GSUB10> for GSUB {
    fn to_lowlevel(&self, max_glyph_id: GlyphID) -> GSUB10 {
        let lookups: Vec<Offset16<GSUBLookupLowlevel>> = self
            .lookups
            .iter()
            .map(|x| Offset16::to(x.to_lowlevel(max_glyph_id)))
            .collect();
        GSUB10 {
            majorVersion: 1,
            minorVersion: 0,
            scriptList: Offset16::to((&self.scripts).into()),
            featureList: Offset16::to((&self.features).into()),
            lookupList: Offset16::to(otspec::tables::GSUB::GSUBLookupList {
                lookups: lookups.into(),
            }),
        }
    }
}
pub(crate) fn to_bytes(
    gsub: &GSUB,
    data: &mut Vec<u8>,
    max_glyph_id: GlyphID,
) -> Result<(), SerializationError> {
    let gsub10 = gsub.to_lowlevel(max_glyph_id);
    gsub10.to_bytes(data)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::layout::common::{
        FeatureList, LanguageSystem, LookupFlags, Script, ScriptList, ValueRecord,
    };
    use crate::tag;
    use otspec::{btreemap, valuerecord};
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    pub fn expected_gsub(lookups: Vec<Lookup<Substitution>>) -> GSUB {
        GSUB {
            lookups,
            scripts: ScriptList {
                scripts: btreemap!(tag!("DFLT") =>  Script {
                        default_language_system: Some(
                            LanguageSystem {
                                required_feature: None,
                                feature_indices: vec![0],
                            },
                        ),
                        language_systems: BTreeMap::new(),
                    },
                ),
            },
            features: FeatureList::new(vec![(tag!("test"), vec![0], None)]),
        }
    }

    pub fn assert_can_deserialize(binary_gsub: Vec<u8>, expected: &GSUB) {
        let mut rc = ReaderContext::new(binary_gsub);
        let gsub: GSUB = from_bytes(&mut rc, 200).unwrap();
        assert_eq!(&gsub, expected);
    }

    pub fn assert_can_roundtrip(binary_gsub: Vec<u8>, expected: &GSUB) {
        let mut rc = ReaderContext::new(binary_gsub.clone());
        let gsub: GSUB = from_bytes(&mut rc, 200).unwrap();
        assert_eq!(&gsub, expected);
        let mut gsub_data = vec![];
        to_bytes(&gsub, &mut gsub_data, 200).unwrap();
        assert_eq!(gsub_data, binary_gsub);
    }

    #[test]
    fn test_gsub1_highlevel_de() {
        let binary_gsub = vec![
            0x00, 0x01, 0x00, 0x00, // GSUB 1.0
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
            0x74, 0x65, 0x73, 0x74, //FeatureRecord.featureTag = test
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
        let expected = expected_gsub(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Substitution::Single(vec![SinglePos {
                mapping: btreemap!(
                    37 => valuerecord!(xAdvance = 35),
                    48 => valuerecord!(xAdvance = 35),
                    50 => valuerecord!(xAdvance = 35)
                ),
            }]),
        }]);
        assert_can_roundtrip(binary_gsub, &expected);
    }
}
