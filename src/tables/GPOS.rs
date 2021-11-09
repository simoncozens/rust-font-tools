use crate::convert_outgoing_subtables;
use crate::layout::common::{Lookup, GPOSGSUB};
use crate::layout::contextual::{ChainedSequenceContext, SequenceContext};
use crate::layout::gpos1::SinglePos;
use crate::layout::gpos2::PairPos;
use crate::layout::gpos3::CursivePos;
use crate::layout::gpos4::MarkBasePos;
use otspec::layout::common::{
    gsubgpos as gsubgposoutgoing, FeatureList, FeatureRecord, FeatureTable,
    Lookup as LookupInternal, LookupList as LookupListOutgoing,
};
use otspec::tables::GPOS::{GPOSSubtable, GPOS10, GPOS11};

// use crate::layout::gpos5::{MarkLigPos, MarkLigPosFormat1};
// use crate::layout::gpos6::{MarkMarkPos, MarkMarkPosFormat1};
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use std::convert::TryInto;

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

#[allow(clippy::upper_case_acronyms)]
/// The Glyph Positioning table
pub type GPOS = GPOSGSUB<Positioning>;

impl Deserialize for GPOS {
    // It's possible we're going to need to know more context about this.
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        match c.peek(4)? {
            [0x00, 0x01, 0x00, 0x00] => {
                let internal: GPOS10 = c.de()?;
                Ok(internal.into())
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
}

impl Into<GPOS> for GPOS10 {
    fn into(self) -> GPOS {
        let lookup_list_lowlevel = self.lookupList.link.unwrap_or_default();
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
                    1 => Positioning::Single(subtables.into_iter().map(SinglePos::from).collect()),
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
            scripts: self.scriptList.link.unwrap_or_default().into(),
            features: self.featureList.link.unwrap_or_default().into(),
        }
    }
}

// Unneeded, for reference
// impl<'a> From<&Lookup<Positioning>> for LookupInternal {
//     fn from(val: &Lookup<Positioning>) -> Self {
//         let subtables: Vec<Box<dyn OffsetMarkerTrait>> = convert_outgoing_subtables!(
//             val.rule.clone(),
//             (Positioning::Single, SinglePosInternal),
//             (Positioning::Pair, PairPosInternal),
//             (Positioning::Cursive, CursivePosFormat1),
//             (Positioning::MarkToBase, MarkBasePosFormat1),
//             // (Positioning::MarkToLig, MarkLigPosFormat1),
//             // (Positioning::MarkToMark, MarkMarkPosFormat1),
//         );

//         LookupInternal {
//             flags: val.flags,
//             lookupType: val.lookup_type(),
//             mark_filtering_set: val.mark_filtering_set,
//             subtables,
//         }
//     }
// }

// Will be needed soon

impl Serialize for GPOS {
    fn to_bytes(&self, _data: &mut Vec<u8>) -> Result<(), SerializationError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            0x6b, 0x65, 0x72, 0x6e, //FeatureRecord.featureTag = kern
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
        let gpos: GPOS = otspec::de::from_bytes(&binary_gpos).unwrap();
        println!("{:#?}", gpos);
        panic!();
    }
}
