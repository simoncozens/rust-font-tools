use crate::layout::common::{FeatureList, FeatureVariations, LookupFlags, ScriptList};
use crate::layout::contextual::{
    deserialize_gpos7, deserialize_gpos8, ChainedSequenceContextFormat1,
    ChainedSequenceContextFormat2, ChainedSequenceContextFormat3, SequenceContextFormat1,
    SequenceContextFormat2, SequenceContextFormat3,
};
use crate::layout::gpos1::{deserialize_gpos1, SinglePosFormat1, SinglePosFormat2};
use crate::layout::gpos2::{deserialize_gpos2, PairPosFormat1, PairPosFormat2};
use crate::layout::gpos3::CursivePosFormat1;
use crate::layout::gpos4::MarkBasePosFormat1;
use crate::layout::gpos5::MarkLigPosFormat1;
use crate::layout::gpos6::MarkMarkPosFormat1;
use crate::{Deserialize, Serialize, Serializer};
use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::tables;
tables! {
    GPOS10 {
        uint16 majorVersion
        uint16 minorVersion
        Offset16(ScriptList) scriptList
        Offset16(FeatureList) featureList
        Offset16(GPOSLookupList) lookupList
    }
    GPOS11 {
        uint16 majorVersion
        uint16 minorVersion
        Offset16(ScriptList) scriptList
        Offset16(FeatureList) featureList
        Offset16(GPOSLookupList) lookupList
        Offset32(FeatureVariations) featureVariations
    }
    GPOSLookupList {
        [offset_base]
        CountedOffset16(GPOSLookup) lookups
    }
}

impl Default for GPOSLookupList {
    fn default() -> Self {
        GPOSLookupList {
            lookups: vec![].into(),
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct GPOSLookup {
    pub lookupType: uint16,
    pub lookupFlag: LookupFlags,
    pub subtables: VecOffset16<GPOSSubtable>,
    pub markFilteringSet: Option<uint16>,
}

impl Serialize for GPOSLookup {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), crate::SerializationError> {
        data.put(self.lookupType)?;
        data.put(self.lookupFlag)?;
        data.put(self.subtables.v.len() as uint16)?;
        data.put(&self.subtables)?;
        if self
            .lookupFlag
            .contains(LookupFlags::USE_MARK_FILTERING_SET)
        {
            data.put(self.markFilteringSet)?;
        }
        Ok(())
    }

    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        self.subtables.offset_fields()
    }

    fn ot_binary_size(&self) -> usize {
        4 + 2
            + 2 * self.subtables.v.len()
            + if self
                .lookupFlag
                .contains(LookupFlags::USE_MARK_FILTERING_SET)
            {
                2
            } else {
                0
            }
    }
}
impl Deserialize for GPOSLookup {
    fn from_bytes(c: &mut crate::ReaderContext) -> Result<Self, crate::DeserializationError>
    where
        Self: std::marker::Sized,
    {
        c.push();
        let lookup_type: uint16 = c.de()?;
        let lookup_flag: LookupFlags = c.de()?;
        let subtable_count: uint16 = c.de()?;
        let mut subtables: Vec<Offset16<GPOSSubtable>> = vec![];
        for _ in 0..subtable_count {
            let off: uint16 = c.de()?;
            let save = c.ptr;
            c.ptr = c.top_of_table() + off as usize;
            let subtable = match lookup_type {
                1 => deserialize_gpos1(c)?,
                2 => deserialize_gpos2(c)?,
                3 => {
                    let cursive: CursivePosFormat1 = c.de()?;
                    GPOSSubtable::GPOS3_1(cursive)
                }
                4 => {
                    let markbase: MarkBasePosFormat1 = c.de()?;
                    GPOSSubtable::GPOS4_1(markbase)
                }
                5 => {
                    let marklig: MarkLigPosFormat1 = c.de()?;
                    GPOSSubtable::GPOS5_1(marklig)
                }
                6 => {
                    let markmark: MarkMarkPosFormat1 = c.de()?;
                    GPOSSubtable::GPOS6_1(markmark)
                }
                7 => deserialize_gpos7(c)?,
                8 => deserialize_gpos8(c)?,
                _ => {
                    unimplemented!()
                }
            };
            subtables.push(Offset16::new(off, subtable));
            c.ptr = save;
        }
        let mark_filtering_set = if lookup_flag.contains(LookupFlags::USE_MARK_FILTERING_SET) {
            let mfs: uint16 = c.de()?;
            Some(mfs)
        } else {
            None
        };
        c.pop();
        Ok(GPOSLookup {
            lookupType: lookup_type,
            lookupFlag: lookup_flag,
            subtables: subtables.into(),
            markFilteringSet: mark_filtering_set,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum GPOSSubtable {
    /// Contains a single positioning rule.
    GPOS1_1(SinglePosFormat1),
    GPOS1_2(SinglePosFormat2),
    GPOS2_1(PairPosFormat1),
    GPOS2_2(PairPosFormat2),
    GPOS3_1(CursivePosFormat1),
    GPOS4_1(MarkBasePosFormat1),
    GPOS5_1(MarkLigPosFormat1),
    GPOS6_1(MarkMarkPosFormat1),
    GPOS7_1(SequenceContextFormat1),
    GPOS7_2(SequenceContextFormat2),
    GPOS7_3(SequenceContextFormat3),
    GPOS8_1(ChainedSequenceContextFormat1),
    GPOS8_2(ChainedSequenceContextFormat2),
    GPOS8_3(ChainedSequenceContextFormat3),
}

fn smash_it(g: &GPOSSubtable) -> &dyn Serialize {
    match &g {
        GPOSSubtable::GPOS1_1(x) => x,
        GPOSSubtable::GPOS1_2(x) => x,
        GPOSSubtable::GPOS2_1(x) => x,
        GPOSSubtable::GPOS2_2(x) => x,
        GPOSSubtable::GPOS3_1(x) => x,
        GPOSSubtable::GPOS4_1(x) => x,
        GPOSSubtable::GPOS5_1(x) => x,
        GPOSSubtable::GPOS6_1(x) => x,
        GPOSSubtable::GPOS7_1(x) => x,
        GPOSSubtable::GPOS7_2(x) => x,
        GPOSSubtable::GPOS7_3(x) => x,
        GPOSSubtable::GPOS8_1(x) => x,
        GPOSSubtable::GPOS8_2(x) => x,
        GPOSSubtable::GPOS8_3(x) => x,
    }
}

impl Serialize for GPOSSubtable {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), crate::SerializationError> {
        smash_it(self).to_bytes(data)
    }

    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        smash_it(self).offset_fields()
    }

    fn ot_binary_size(&self) -> usize {
        smash_it(self).ot_binary_size()
    }

    fn to_bytes_shallow(&self, data: &mut Vec<u8>) -> Result<(), crate::SerializationError> {
        smash_it(self).to_bytes_shallow(data)
    }
}
