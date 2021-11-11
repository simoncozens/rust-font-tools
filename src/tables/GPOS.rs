use crate::layout::common::{FromLowlevel, Lookup, ToLowlevel, GPOSGSUB};
use crate::layout::contextual::{ChainedSequenceContext, SequenceContext};
use crate::layout::gpos1::SinglePos;
use crate::layout::gpos2::PairPos;
use crate::layout::gpos3::CursivePos;
use crate::layout::gpos4::MarkBasePos;
use otspec::tables::GPOS::{GPOSLookup as GPOSLookupLowlevel, GPOSSubtable, GPOS10};
// use otspec::tables::GPOS::GPOS11;
// use crate::layout::gpos5::{MarkLigPos, MarkLigPosFormat1};
// use crate::layout::gpos6::{MarkMarkPos, MarkMarkPosFormat1};
use otspec::types::*;
use otspec::{DeserializationError, Deserializer, ReaderContext, SerializationError, Serialize};

/// The 'GPOS' OpenType tag.
pub const TAG: Tag = crate::tag!("GPOS");

/// A container which represents a generic positioning rule
///
/// Each rule is expressed as a vector of subtables.
#[derive(Debug, PartialEq, Clone)]
pub enum Positioning {
    /// Contains a single positioning rule.
    Single(Vec<SinglePos>),
    /// Contains a pair positioning rule.
    Pair(Vec<PairPos>),
    /// Contains an cursive positioning rule.
    Cursive(Vec<CursivePos>),
    /// Contains a mark-to-base rule.
    MarkToBase(Vec<MarkBasePos>),
    /// Contains a mark-to-lig rule.
    // MarkToLig(Vec<MarkLigPos>),
    MarkToLig,
    /// Contains a mark-to-mark rule.
    // MarkToMark(Vec<MarkMarkPos>),
    MarkToMark,
    /// Contains a contextual positioning rule.
    Contextual(Vec<SequenceContext>),
    /// Contains a chained contextual positioning rule.
    ChainedContextual(Vec<ChainedSequenceContext>),
    /// Contains an extension subtable.
    Extension,
}

impl Positioning {
    /// Adds a subtable break to this rule
    pub fn add_subtable_break(&mut self) {
        match self {
            Positioning::Single(v) => v.push(SinglePos::default()),
            Positioning::Pair(v) => v.push(PairPos::default()),
            Positioning::Cursive(v) => v.push(CursivePos::default()),
            Positioning::MarkToBase(v) => v.push(MarkBasePos::default()),
            Positioning::MarkToLig => todo!(),
            Positioning::MarkToMark => todo!(),
            // Positioning::MarkToLig(v) => v.push(MarkLigPos::default()),
            // Positioning::MarkToMark(v) => v.push(MarkMarkPos::default()),
            Positioning::Contextual(v) => v.push(SequenceContext::default()),
            Positioning::ChainedContextual(v) => v.push(ChainedSequenceContext::default()),
            Positioning::Extension => todo!(),
        }
    }
}

impl Lookup<Positioning> {
    /// Returns the GPOS lookup type for this subtable
    pub fn lookup_type(&self) -> u16 {
        match &self.rule {
            Positioning::Single(_) => 1,
            Positioning::Pair(_) => 2,
            Positioning::Cursive(_) => 3,
            Positioning::MarkToBase(_) => 4,
            Positioning::MarkToLig => 5,
            Positioning::MarkToMark => 6,
            Positioning::Contextual(_) => 7,
            Positioning::ChainedContextual(_) => 8,
            Positioning::Extension => 9,
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
/// The Glyph Positioning table
pub type GPOS = GPOSGSUB<Positioning>;

pub(crate) fn from_bytes(
    c: &mut ReaderContext,
    max_glyph_id: GlyphID,
) -> Result<GPOS, DeserializationError> {
    match c.peek(4)? {
        [0x00, 0x01, 0x00, 0x00] => {
            let internal: GPOS10 = c.de()?;
            Ok(GPOS::from_lowlevel(internal, max_glyph_id))
        }
        // [0x00, 0x01, 0x00, 0x01] => {
        //     let internal: GPOS11 = c.de()?;
        //     Ok(internal.into())
        // }
        _ => Err(DeserializationError(
            "Invalid GPOS table version".to_string(),
        )),
    }
}

impl FromLowlevel<GPOS10> for GPOS {
    fn from_lowlevel(val: GPOS10, max_glyph_id: GlyphID) -> Self {
        let lookup_list_lowlevel = val.lookupList.link.unwrap_or_default();
        let mut lookups: Vec<Lookup<Positioning>> = vec![];
        for lookup_off in lookup_list_lowlevel.lookups.v {
            if let Some(lookup_lowlevel) = lookup_off.link {
                let subtables: Vec<GPOSSubtable> = lookup_lowlevel
                    .subtables
                    .v
                    .iter()
                    .map(|x| x.link.clone())
                    .flatten()
                    .collect();
                let theirs = match lookup_lowlevel.lookupType {
                    1 => Positioning::Single(
                        subtables
                            .into_iter()
                            .map(|st| SinglePos::from_lowlevel(st, max_glyph_id))
                            .collect(),
                    ),
                    2 => Positioning::Pair(
                        subtables
                            .into_iter()
                            .map(|st| PairPos::from_lowlevel(st, max_glyph_id))
                            .collect(),
                    ),
                    3 => Positioning::Cursive(
                        subtables
                            .into_iter()
                            .map(|st| CursivePos::from_lowlevel(st, max_glyph_id))
                            .collect(),
                    ),
                    _ => unimplemented!(),
                };

                let lookup_highlevel: Lookup<Positioning> = Lookup {
                    flags: lookup_lowlevel.lookupFlag,
                    mark_filtering_set: lookup_lowlevel.markFilteringSet,
                    rule: theirs,
                };
                lookups.push(lookup_highlevel)
            }
        }
        GPOS {
            lookups,
            scripts: val.scriptList.link.unwrap_or_default().into(),
            features: val.featureList.link.unwrap_or_default().into(),
        }
    }
}

// Will be needed soon

impl ToLowlevel<GPOSLookupLowlevel> for Lookup<Positioning> {
    fn to_lowlevel(&self, max_glyph_id: GlyphID) -> GPOSLookupLowlevel {
        let subtables: Vec<Offset16<GPOSSubtable>> = match &self.rule {
            Positioning::Single(sp) => sp
                .iter()
                .map(|subtable| Offset16::to(subtable.to_lowlevel(max_glyph_id)))
                .collect(),
            Positioning::Pair(pp) => pp
                .iter()
                .map(|subtable| Offset16::to(subtable.to_lowlevel(max_glyph_id)))
                .collect(),
            Positioning::Cursive(curs) => curs
                .iter()
                .map(|subtable| Offset16::to(subtable.to_lowlevel(max_glyph_id)))
                .collect(),
            Positioning::MarkToBase(_) => todo!(),
            Positioning::MarkToLig => todo!(),
            Positioning::MarkToMark => todo!(),
            Positioning::Contextual(_) => todo!(),
            Positioning::ChainedContextual(_) => todo!(),
            Positioning::Extension => todo!(),
        };
        GPOSLookupLowlevel {
            lookupType: self.lookup_type(),
            lookupFlag: self.flags,
            subtables: subtables.into(),
            markFilteringSet: self.mark_filtering_set,
        }
    }
}
impl ToLowlevel<GPOS10> for GPOS {
    fn to_lowlevel(&self, max_glyph_id: GlyphID) -> GPOS10 {
        let lookups: Vec<Offset16<GPOSLookupLowlevel>> = self
            .lookups
            .iter()
            .map(|x| Offset16::to(x.to_lowlevel(max_glyph_id)))
            .collect();
        GPOS10 {
            majorVersion: 1,
            minorVersion: 0,
            scriptList: Offset16::to((&self.scripts).into()),
            featureList: Offset16::to((&self.features).into()),
            lookupList: Offset16::to(otspec::tables::GPOS::GPOSLookupList {
                lookups: lookups.into(),
            }),
        }
    }
}
pub(crate) fn to_bytes(
    gpos: &GPOS,
    data: &mut Vec<u8>,
    max_glyph_id: GlyphID,
) -> Result<(), SerializationError> {
    let gpos10 = gpos.to_lowlevel(max_glyph_id);
    gpos10.to_bytes(data)
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

    pub fn expected_gpos(lookups: Vec<Lookup<Positioning>>) -> GPOS {
        GPOS {
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

    pub fn assert_can_deserialize(binary_gpos: Vec<u8>, expected: &GPOS) {
        let mut rc = ReaderContext::new(binary_gpos);
        let gpos: GPOS = from_bytes(&mut rc, 200).unwrap();
        assert_eq!(&gpos, expected);
    }

    pub fn assert_can_roundtrip(binary_gpos: Vec<u8>, expected: &GPOS) {
        let mut rc = ReaderContext::new(binary_gpos.clone());
        let gpos: GPOS = from_bytes(&mut rc, 200).unwrap();
        assert_eq!(&gpos, expected);
        let mut gpos_data = vec![];
        to_bytes(&gpos, &mut gpos_data, 200).unwrap();
        assert_eq!(gpos_data, binary_gpos);
    }

    #[test]
    fn test_gpos1_highlevel_de() {
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, // GPOS 1.0
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
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::Single(vec![SinglePos {
                mapping: btreemap!(
                    37 => valuerecord!(xAdvance = 35),
                    48 => valuerecord!(xAdvance = 35),
                    50 => valuerecord!(xAdvance = 35)
                ),
            }]),
        }]);
        assert_can_roundtrip(binary_gpos, &expected);
    }
}
