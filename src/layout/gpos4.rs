use crate::layout::anchor::Anchor;
use crate::layout::common::MarkArray;
use crate::layout::coverage::Coverage;

use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::tables;

use std::collections::BTreeMap;

tables!(
  MarkBasePosFormat1 [nodeserialize] {
    [offset_base]
    uint16 posFormat
    Offset16(Coverage) markCoverage
    Offset16(Coverage) baseCoverage
    uint16 markClassCount
    Offset16(MarkArray) markArray
    Offset16(BaseArray) baseArray
  }

  BaseArray [nodeserialize] {
    [offset_base]
    [embed]
    Counted(BaseRecord) baseRecords
  }
);

#[derive(Debug, Clone, PartialEq)]
pub struct BaseRecord {
    pub baseAnchors: Vec<Anchor>,
}

impl Serialize for BaseRecord {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        todo!()
    }
}

// MarkBasePosFormat1 needs manual deserialization because of the the data dependency:
// BaseRecord needs to know the mark class count.
impl Deserialize for MarkBasePosFormat1 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError>
    where
        Self: std::marker::Sized,
    {
        todo!()
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
/// A mark-to-base subtable.
pub struct MarkBasePos {
    pub marks: BTreeMap<GlyphID, (uint16, Anchor)>,
    pub bases: BTreeMap<GlyphID, BTreeMap<uint16, Anchor>>,
}

impl Deserialize for MarkBasePos {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let cursivepos1: MarkBasePosFormat1 = c.de()?;
        unimplemented!()
    }
}

impl From<&MarkBasePos> for MarkBasePosFormat1 {
    fn from(lookup: &MarkBasePos) -> Self {
        unimplemented!()
        // MarkBaseFormat1 {
        //     posFormat: 1,
        //     coverage,
        //     entryExitRecord: anchors,
        // }
    }
}

impl Serialize for MarkBasePos {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let i: MarkBasePosFormat1 = self.into();
        i.to_bytes(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btreemap;
    use otspec::offsetmanager::OffsetManager;
    use std::iter::FromIterator;
}
