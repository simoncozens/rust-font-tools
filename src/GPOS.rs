use crate::deserialize_lookup_match;
use crate::layout::common::*;
use crate::layout::gpos1::{SinglePos, SinglePosInternal};
use crate::layout::gpos2::{PairPos, PairPosInternal};
use crate::layout::gpos3::{CursivePos, CursivePosFormat1};
use otspec::types::*;
use otspec::Counted;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::Deserialize;
use std::convert::TryInto;

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug, Clone, Deserialize)]
pub struct gposcoreincoming {
    pub majorVersion: uint16,
    pub minorVersion: uint16,
    pub scriptList: Offset16<ScriptList>,
    pub featureList: Offset16<FeatureList>,
    pub lookupList: Offset16<PosLookupListIncoming>,
}

#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[derive(Debug, Clone, Deserialize)]
pub struct PosLookupListIncoming {
    #[serde(offset_base)]
    #[serde(with = "Counted")]
    pub lookups: VecOffset16<Lookup<Positioning>>,
}

impl Lookup<Positioning> {
    /// Return the integer GPOS lookup type for this lookup
    pub fn lookup_type(&self) -> u16 {
        match self.rule {
            Positioning::Single(_) => 1,
            Positioning::Pair(_) => 2,
            Positioning::Cursive(_) => 3,
            Positioning::MarkToBase => 4,
            Positioning::MarkToLig => 5,
            Positioning::MarkToMark => 6,
            Positioning::Contextual => 7,
            Positioning::ChainedContextual => 8,
            Positioning::Extension => 9,
        }
    }

    /// Add subtable break
    pub fn add_subtable_break(&mut self) {
        self.rule.add_subtable_break()
    }
}
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
    MarkToBase,
    /// Contains a mark-to-lig rule.
    MarkToLig,
    /// Contains a mark-to-mark rule.
    MarkToMark,
    /// Contains a contextual positioning rule.
    Contextual,
    /// Contains a chained contextual positioning rule.
    ChainedContextual,
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
            Positioning::MarkToBase => todo!(),
            Positioning::MarkToLig => todo!(),
            Positioning::MarkToMark => todo!(),
            Positioning::Contextual => todo!(),
            Positioning::ChainedContextual => todo!(),
            Positioning::Extension => todo!(),
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
/// The Glyph Positioning table
pub type GPOS = GPOSGSUB<Positioning>;

impl Default for GPOS {
    fn default() -> Self {
        GPOS {
            lookups: vec![],
            scripts: ScriptList::default(),
            features: vec![],
        }
    }
}

impl Deserialize for GPOS {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let core: gposcoreincoming = c.de()?;
        if core.minorVersion == 1 {
            let _feature_variations_offset: uint16 = c.de()?;
        }
        let scripts: ScriptList = core
            .scriptList
            .link
            .ok_or_else(|| DeserializationError("Bad script list in GPOS table".to_string()))?;
        let lookups: Vec<Lookup<Positioning>> = core
            .lookupList
            .link
            .ok_or_else(|| DeserializationError("Bad lookup list in GPOS table".to_string()))?
            .lookups
            .try_into()?;
        let feature_records = core
            .featureList
            .link
            .ok_or_else(|| DeserializationError("Bad feature list in GPOS table".to_string()))?
            .featureRecords;
        let mut features = vec![];
        for f in feature_records.iter() {
            let tag = f.featureTag;
            let table = f
                .feature
                .link
                .as_ref()
                .ok_or_else(|| DeserializationError("Bad feature in GPOS table".to_string()))?;
            features.push((
                tag,
                table
                    .lookupListIndices
                    .iter()
                    .map(|x| *x as usize)
                    .collect(),
                None,
            ));
        }

        Ok(GPOS {
            lookups,
            scripts,
            features,
        })
    }
}

impl From<&GPOS> for gsubgposoutgoing {
    fn from(val: &GPOS) -> Self {
        let substlookuplist: LookupListOutgoing = LookupListOutgoing {
            lookups: VecOffset16(val.lookups.iter().map(|x| Offset16::to(x.into())).collect()),
        };
        let featurelist: FeatureList = FeatureList {
            featureRecords: val
                .features
                .iter()
                .map(|f| {
                    let indices: Vec<uint16> = f.1.iter().map(|x| *x as uint16).collect();
                    FeatureRecord {
                        featureTag: f.0,
                        feature: Offset16::to(FeatureTable {
                            featureParamsOffset: 0,
                            lookupListIndices: indices,
                        }),
                    }
                })
                .collect(),
        };
        gsubgposoutgoing {
            majorVersion: 1,
            minorVersion: 0,
            scriptList: Offset16::to(val.scripts.clone()),
            featureList: Offset16::to(featurelist),
            lookupList: Offset16::to(substlookuplist),
        }
    }
}

impl Deserialize for Lookup<Positioning> {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let lookup_type: uint16 = c.de()?;
        let lookup_flag: LookupFlags = c.de()?;
        let rule = deserialize_lookup_match!(
            lookup_type,
            c,
            (1, SinglePos, Positioning::Single),
            (2, PairPos, Positioning::Pair),
            (3, CursivePos, Positioning::Cursive),
        );

        c.pop();
        Ok(Lookup {
            flags: lookup_flag,
            mark_filtering_set: None,
            rule,
        })
    }
}

impl<'a> From<&Lookup<Positioning>> for LookupInternal {
    fn from(val: &Lookup<Positioning>) -> Self {
        let subtables: Vec<Box<dyn OffsetMarkerTrait>> = match &val.rule.clone() {
            Positioning::Single(subs) => {
                let mut v: Vec<Box<dyn OffsetMarkerTrait>> = vec![];
                for s in subs {
                    let si: SinglePosInternal = s.into();
                    v.push(Box::new(Offset16::to(si)));
                }
                v
            }
            Positioning::Pair(subs) => {
                let mut v: Vec<Box<dyn OffsetMarkerTrait>> = vec![];
                for s in subs {
                    let si: PairPosInternal = s.into();
                    v.push(Box::new(Offset16::to(si)));
                }
                v
            }
            Positioning::Cursive(subs) => {
                let mut v: Vec<Box<dyn OffsetMarkerTrait>> = vec![];
                for s in subs {
                    let si: CursivePosFormat1 = s.into();
                    v.push(Box::new(Offset16::to(si)));
                }
                v
            } // Positioning::Multiple(subs) => subs.offset_fields(),
            // Positioning::Alternate(subs) => subs.offset_fields(),
            // Positioning::Ligature(subs) => subs.offset_fields(),
            _ => unimplemented!(),
        };

        LookupInternal {
            flags: val.flags,
            lookupType: val.lookup_type(),
            mark_filtering_set: val.mark_filtering_set,
            subtables,
        }
    }
}

impl Serialize for GPOS {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let gsc: gsubgposoutgoing = self.into();
        gsc.to_bytes(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    macro_rules! hashmap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }

    macro_rules! btreemap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }
}
