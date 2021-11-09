use crate::layout::common::{FeatureList, FeatureVariations, LookupFlags, ScriptList};
use crate::layout::contextual::{
    deserialize_gpos7, SequenceContextFormat1, SequenceContextFormat2, SequenceContextFormat3,
};
use crate::layout::gpos1::{deserialize_gpos1, SinglePosFormat1, SinglePosFormat2};
use crate::layout::gpos2::{deserialize_gpos2, PairPosFormat1, PairPosFormat2};
use crate::layout::gpos3::CursivePosFormat1;
use crate::layout::gpos4::MarkBasePosFormat1;
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
                7 => deserialize_gpos7(c)?,
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
}

impl Serialize for GPOSSubtable {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), crate::SerializationError> {
        match self {
            GPOSSubtable::GPOS1_1(x) => x.to_bytes(data),
            GPOSSubtable::GPOS1_2(x) => x.to_bytes(data),
            GPOSSubtable::GPOS2_1(x) => x.to_bytes(data),
            GPOSSubtable::GPOS2_2(x) => x.to_bytes(data),
            GPOSSubtable::GPOS3_1(x) => x.to_bytes(data),
            GPOSSubtable::GPOS4_1(x) => x.to_bytes(data),
        }
    }

    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        match self {
            GPOSSubtable::GPOS1_1(x) => x.offset_fields(),
            GPOSSubtable::GPOS1_2(x) => x.offset_fields(),
            GPOSSubtable::GPOS2_1(x) => x.offset_fields(),
            GPOSSubtable::GPOS2_2(x) => x.offset_fields(),
            GPOSSubtable::GPOS3_1(x) => x.offset_fields(),
            GPOSSubtable::GPOS4_1(x) => x.offset_fields(),
        }
    }

    fn ot_binary_size(&self) -> usize {
        match self {
            GPOSSubtable::GPOS1_1(x) => x.ot_binary_size(),
            GPOSSubtable::GPOS1_2(x) => x.ot_binary_size(),
            GPOSSubtable::GPOS2_1(x) => x.ot_binary_size(),
            GPOSSubtable::GPOS2_2(x) => x.ot_binary_size(),
            GPOSSubtable::GPOS3_1(x) => x.ot_binary_size(),
            GPOSSubtable::GPOS4_1(x) => x.ot_binary_size(),
        }
    }

    fn to_bytes_shallow(&self, data: &mut Vec<u8>) -> Result<(), crate::SerializationError> {
        match self {
            GPOSSubtable::GPOS1_1(x) => x.to_bytes_shallow(data),
            GPOSSubtable::GPOS1_2(x) => x.to_bytes_shallow(data),
            GPOSSubtable::GPOS2_1(x) => x.to_bytes_shallow(data),
            GPOSSubtable::GPOS2_2(x) => x.to_bytes_shallow(data),
            GPOSSubtable::GPOS3_1(x) => x.to_bytes_shallow(data),
            GPOSSubtable::GPOS4_1(x) => x.to_bytes_shallow(data),
        }
    }
}
