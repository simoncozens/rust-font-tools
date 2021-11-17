use crate::layout::common::{FeatureList, FeatureVariations, LookupFlags, ScriptList};
use crate::layout::contextual::{
    deserialize_gsub5, deserialize_gsub6, ChainedSequenceContextFormat1,
    ChainedSequenceContextFormat2, ChainedSequenceContextFormat3, SequenceContextFormat1,
    SequenceContextFormat2, SequenceContextFormat3,
};
use crate::layout::gsub1::{deserialize_gsub1, SingleSubstFormat1, SingleSubstFormat2};
use crate::layout::gsub2::MultipleSubstFormat1;
use crate::layout::gsub3::AlternateSubstFormat1;
use crate::layout::gsub4::LigatureSubstFormat1;
use crate::{Deserialize, Serialize, Serializer};
use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::tables;
tables! {
    GSUB10 {
        uint16 majorVersion
        uint16 minorVersion
        Offset16(ScriptList) scriptList
        Offset16(FeatureList) featureList
        Offset16(GSUBLookupList) lookupList
    }
    GSUB11 {
        uint16 majorVersion
        uint16 minorVersion
        Offset16(ScriptList) scriptList
        Offset16(FeatureList) featureList
        Offset16(GSUBLookupList) lookupList
        Offset32(FeatureVariations) featureVariations
    }
    GSUBLookupList {
        [offset_base]
        CountedOffset16(GSUBLookup) lookups
    }

    ExtensionSubstFormat1 [nodeserialize] {
        [offset_base]
        uint16  substFormat
        uint16  extensionLookupType
        Offset32(GSUBSubtable) extension
    }

}

impl Default for GSUBLookupList {
    fn default() -> Self {
        GSUBLookupList {
            lookups: vec![].into(),
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct GSUBLookup {
    pub lookupType: uint16,
    pub lookupFlag: LookupFlags,
    pub subtables: VecOffset16<GSUBSubtable>,
    pub markFilteringSet: Option<uint16>,
}

impl Serialize for GSUBLookup {
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

impl GSUBSubtable {
    fn deserialize(
        c: &mut crate::ReaderContext,
        lookup_type: uint16,
    ) -> Result<Self, crate::DeserializationError> {
        Ok(match lookup_type {
            1 => deserialize_gsub1(c)?,
            2 => {
                let multiple: MultipleSubstFormat1 = c.de()?;
                GSUBSubtable::GSUB2_1(multiple)
            }
            3 => {
                let alternate: AlternateSubstFormat1 = c.de()?;
                GSUBSubtable::GSUB3_1(alternate)
            }
            4 => {
                let ligature: LigatureSubstFormat1 = c.de()?;
                GSUBSubtable::GSUB4_1(ligature)
            }
            5 => deserialize_gsub5(c)?,
            6 => deserialize_gsub6(c)?,
            7 => {
                let extension: ExtensionSubstFormat1 = c.de()?;
                GSUBSubtable::GSUB7_1(Box::new(extension))
            }
            _ => panic!(),
        })
    }
}

impl Deserialize for GSUBLookup {
    fn from_bytes(c: &mut crate::ReaderContext) -> Result<Self, crate::DeserializationError>
    where
        Self: std::marker::Sized,
    {
        c.push();
        let lookup_type: uint16 = c.de()?;
        let lookup_flag: LookupFlags = c.de()?;
        let subtable_count: uint16 = c.de()?;
        let mut subtables: Vec<Offset16<GSUBSubtable>> = vec![];
        for _ in 0..subtable_count {
            let off: uint16 = c.de()?;
            let save = c.ptr;
            c.ptr = c.top_of_table() + off as usize;
            let subtable = GSUBSubtable::deserialize(c, lookup_type)?;
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
        Ok(GSUBLookup {
            lookupType: lookup_type,
            lookupFlag: lookup_flag,
            subtables: subtables.into(),
            markFilteringSet: mark_filtering_set,
        })
    }
}

impl Deserialize for ExtensionSubstFormat1 {
    fn from_bytes(c: &mut crate::ReaderContext) -> Result<Self, crate::DeserializationError> {
        c.push();
        let subst_format: uint16 = c.de()?;
        let extension_lookup_type: uint16 = c.de()?;
        if !(0..=6).contains(&extension_lookup_type) {
            return Err(crate::DeserializationError(format!(
                "Bad GSUB extension lookup type {:?}",
                extension_lookup_type
            )));
        }

        let off: uint32 = c.de()?;
        let save = c.ptr;
        c.ptr = c.top_of_table() + off as usize;
        let subtable = GSUBSubtable::deserialize(c, extension_lookup_type)?;
        c.ptr = save;

        c.pop();
        Ok(ExtensionSubstFormat1 {
            substFormat: subst_format,
            extensionLookupType: extension_lookup_type,
            extension: Offset32::new(off, subtable),
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum GSUBSubtable {
    GSUB1_1(SingleSubstFormat1),
    GSUB1_2(SingleSubstFormat2),
    GSUB2_1(MultipleSubstFormat1),
    GSUB3_1(AlternateSubstFormat1),
    GSUB4_1(LigatureSubstFormat1),
    GSUB5_1(SequenceContextFormat1),
    GSUB5_2(SequenceContextFormat2),
    GSUB5_3(SequenceContextFormat3),
    GSUB6_1(ChainedSequenceContextFormat1),
    GSUB6_2(ChainedSequenceContextFormat2),
    GSUB6_3(ChainedSequenceContextFormat3),
    GSUB7_1(Box<ExtensionSubstFormat1>),
}

fn smash_it(g: &GSUBSubtable) -> &dyn Serialize {
    match &g {
        GSUBSubtable::GSUB1_1(x) => x,
        GSUBSubtable::GSUB1_2(x) => x,
        GSUBSubtable::GSUB2_1(x) => x,
        GSUBSubtable::GSUB3_1(x) => x,
        GSUBSubtable::GSUB4_1(x) => x,
        GSUBSubtable::GSUB5_1(x) => x,
        GSUBSubtable::GSUB5_2(x) => x,
        GSUBSubtable::GSUB5_3(x) => x,
        GSUBSubtable::GSUB6_1(x) => x,
        GSUBSubtable::GSUB6_2(x) => x,
        GSUBSubtable::GSUB6_3(x) => x,
        GSUBSubtable::GSUB7_1(x) => x.as_ref(),
    }
}

impl Serialize for GSUBSubtable {
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
