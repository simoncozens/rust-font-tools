use crate::layout::anchor::Anchor;
use crate::{DeserializationError, Deserialize, ReaderContext, SerializationError, Serializer};
use otspec::layout::common::MarkArray;
use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::{Deserializer, Serialize};
use otspec_macros::tables;

tables!(
  MarkMarkPosFormat1 [nodeserialize] {
    [offset_base]
    uint16 posFormat
    Offset16(Coverage) mark1Coverage
    Offset16(Coverage) mark2Coverage
    uint16 markClassCount
    Offset16(MarkArray) mark1Array
    Offset16(Mark2Array) mark2Array
  }

  Mark2Array [nodeserialize] [default] {
    [offset_base]
    [embed]
    Counted(Mark2Record) mark2Records
  }
);

#[derive(Debug, Clone, PartialEq)]
#[allow(non_snake_case)]
/// Information about anchor positioning on a base
pub struct Mark2Record {
    /// A list of base anchors
    pub mark2Anchors: Vec<Offset16<Anchor>>,
}

impl Serialize for Mark2Record {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        for a in &self.mark2Anchors {
            data.put(a)?
        }
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        2 * self.mark2Anchors.len()
    }

    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        self.mark2Anchors
            .iter()
            .map(|x| {
                let erase_type: &dyn OffsetMarkerTrait = x;
                erase_type
            })
            .collect()
    }
}

// We need to deserialize this thing manually because of the data dependency:
// Mark2Record needs to know markClassCount.
impl Deserialize for MarkMarkPosFormat1 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let pos_format: uint16 = c.de()?;
        let mark_coverage: Offset16<Coverage> = c.de()?;
        let mark2_coverage: Offset16<Coverage> = c.de()?;
        let mark_class_count: uint16 = c.de()?;
        let mark_array_offset: Offset16<MarkArray> = c.de()?;

        // Now it gets tricky.
        let mark2_array_offset: uint16 = c.de()?;
        let mut mark2_records = vec![];
        c.ptr = c.top_of_table() + mark2_array_offset as usize;
        // We are now at the start of the base array table
        c.push();

        let count: uint16 = c.de()?;
        for _ in 0..count {
            let mark2_record: Vec<Offset16<Anchor>> = c.de_counted(mark_class_count.into())?;
            mark2_records.push(Mark2Record {
                mark2Anchors: mark2_record,
            })
        }
        c.pop();
        c.pop();
        Ok(MarkMarkPosFormat1 {
            posFormat: pos_format,
            mark1Coverage: mark_coverage,
            mark2Coverage: mark2_coverage,
            markClassCount: mark_class_count,
            mark1Array: mark_array_offset,
            mark2Array: Offset16::new(
                mark2_array_offset,
                Mark2Array {
                    mark2Records: mark2_records,
                },
            ),
        })
    }
}
