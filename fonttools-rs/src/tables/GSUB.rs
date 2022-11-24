use crate::layout::common::{FromLowlevel, Lookup, ToLowlevel, GPOSGSUB};
use crate::layout::contextual::{ChainedSequenceContext, SequenceContext};
use crate::layout::gsub1::SingleSubst;
use crate::layout::gsub2::MultipleSubst;
use crate::layout::gsub3::AlternateSubst;
use crate::layout::gsub4::LigatureSubst;
use crate::layout::gsub8::ReverseChainSubst;
use otspec::tables::GSUB::{
    ExtensionSubstFormat1, GSUBLookup as GSUBLookupLowlevel, GSUBSubtable, GSUB10,
};
use otspec::types::*;
use otspec::utils::is_all_the_same;
use otspec::{DeserializationError, Deserializer, ReaderContext, SerializationError, Serialize};

/// The 'GSUB' OpenType tag.
pub const TAG: Tag = crate::tag!("GSUB");

/// A container which represents a generic substitution rule
///
/// Each rule is expressed as a vector of subtables.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Substitution {
    /// Contains a single substitution rule.
    Single(Vec<SingleSubst>),
    /// Contains a multiple substitution rule.
    Multiple(Vec<MultipleSubst>),
    /// Contains an alternate substitution rule.
    Alternate(Vec<AlternateSubst>),
    /// Contains an ligature substitution rule.
    Ligature(Vec<LigatureSubst>),

    /// Contains a contextual substitution rule.
    Contextual(Vec<SequenceContext>),
    /// Contains a chained contextual substitution rule.
    ChainedContextual(Vec<ChainedSequenceContext>),
    /// Contains a reverse chaining contextual single substitution rules
    ReverseChainContextual(Vec<ReverseChainSubst>),
}

impl Substitution {
    /// Adds a subtable break to this rule
    pub fn add_subtable_break(&mut self) {
        match self {
            Substitution::Single(v) => v.push(SingleSubst::default()),
            Substitution::Multiple(v) => v.push(MultipleSubst::default()),
            Substitution::Alternate(v) => v.push(AlternateSubst::default()),
            Substitution::Ligature(v) => v.push(LigatureSubst::default()),
            Substitution::Contextual(v) => v.push(SequenceContext::default()),
            Substitution::ChainedContextual(v) => v.push(ChainedSequenceContext::default()),
            Substitution::ReverseChainContextual(v) => v.push(ReverseChainSubst::default()),
        }
    }
}

impl Lookup<Substitution> {
    /// Returns the GSUB lookup type for this subtable
    pub fn lookup_type(&self) -> u16 {
        match &self.rule {
            Substitution::Single(_) => 1,
            Substitution::Multiple(_) => 2,
            Substitution::Alternate(_) => 3,
            Substitution::Ligature(_) => 4,
            Substitution::Contextual(_) => 5,
            Substitution::ChainedContextual(_) => 6,
            Substitution::ReverseChainContextual(_) => 8,
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

fn extension_from_lowlevel(subtables: Vec<GSUBSubtable>, max_glyph_id: GlyphID) -> Substitution {
    // Unwrap the subtable enum
    let extension_tables: Vec<ExtensionSubstFormat1> = subtables
        .into_iter()
        .map(|st| {
            if let GSUBSubtable::GSUB7_1(boxed_st) = st {
                *boxed_st
            } else {
                panic!("Found a thing in an extension lookup which wasn't an extension subtable")
            }
        })
        .collect();
    if !is_all_the_same(extension_tables.iter().map(|st| st.extensionLookupType)) {
        panic!("Mismatched extension lookup types in extension subtable")
    }
    let lookup_type = extension_tables
        .iter()
        .map(|st| st.extensionLookupType)
        .next()
        .expect("No extension subtables in extension lookup");
    let inner_subtables: Vec<GSUBSubtable> = extension_tables
        .into_iter()
        .map(|st| st.extension.link.unwrap())
        .collect();
    subtables_from_lowlevel(lookup_type, inner_subtables, max_glyph_id)
}

fn subtables_from_lowlevel(
    lookup_type: uint16,
    subtables: Vec<GSUBSubtable>,
    max_glyph_id: GlyphID,
) -> Substitution {
    match lookup_type {
        1 => Substitution::Single(
            subtables
                .into_iter()
                .map(|st| SingleSubst::from_lowlevel(st, max_glyph_id))
                .collect(),
        ),
        2 => Substitution::Multiple(
            subtables
                .into_iter()
                .map(|st| MultipleSubst::from_lowlevel(st, max_glyph_id))
                .collect(),
        ),
        3 => Substitution::Alternate(
            subtables
                .into_iter()
                .map(|st| AlternateSubst::from_lowlevel(st, max_glyph_id))
                .collect(),
        ),
        4 => Substitution::Ligature(
            subtables
                .into_iter()
                .map(|st| LigatureSubst::from_lowlevel(st, max_glyph_id))
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
        7 => extension_from_lowlevel(subtables, max_glyph_id),
        8 => Substitution::ReverseChainContextual(
            subtables
                .into_iter()
                .map(|st| ReverseChainSubst::from_lowlevel(st, max_glyph_id))
                .collect(),
        ),
        x => panic!("Unknown GSUB lookup type {:?}", x),
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
                    .flat_map(|x| x.link.clone())
                    .collect();
                let theirs =
                    subtables_from_lowlevel(lookup_lowlevel.lookupType, subtables, max_glyph_id);
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
            Substitution::Single(ss) => ss
                .iter()
                .map(|subtable| Offset16::to(subtable.to_lowlevel(max_glyph_id)))
                .collect(),
            Substitution::Multiple(ms) => ms
                .iter()
                .map(|subtable| Offset16::to(subtable.to_lowlevel(max_glyph_id)))
                .collect(),
            Substitution::Alternate(alts) => alts
                .iter()
                .map(|subtable| Offset16::to(subtable.to_lowlevel(max_glyph_id)))
                .collect(),
            Substitution::Ligature(ls) => ls
                .iter()
                .map(|subtable| Offset16::to(subtable.to_lowlevel(max_glyph_id)))
                .collect(),
            Substitution::Contextual(contextual) => contextual
                .iter()
                .flat_map(|subtable| {
                    subtable
                        .to_lowlevel_subtables_gsub(max_glyph_id)
                        .into_iter()
                        .map(Offset16::to)
                })
                .collect(),
            Substitution::ChainedContextual(chainedcontextual) => chainedcontextual
                .iter()
                .flat_map(|subtable| {
                    subtable
                        .to_lowlevel_subtables_gsub(max_glyph_id)
                        .into_iter()
                        .map(Offset16::to)
                })
                .collect(),
            Substitution::ReverseChainContextual(rs) => rs
                .iter()
                .map(|subtable| Offset16::to(subtable.to_lowlevel(max_glyph_id)))
                .collect(),
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
    use crate::layout::common::{FeatureList, LanguageSystem, LookupFlags, Script, ScriptList};
    use crate::tag;
    use otspec::btreemap;
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
    fn test_extension_deserialize() {
        let binary_gsub = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x07, /* lookup type 7 = extension */
            0x00, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x06, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x42,
        ];
        let expected = expected_gsub(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Substitution::Single(vec![SingleSubst {
                mapping: btreemap!(
                    66 => 67
                ),
            }]),
        }]);
        assert_can_deserialize(binary_gsub, &expected);
    }
}
