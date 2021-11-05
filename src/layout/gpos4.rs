use crate::layout::common::{MarkArray, MarkRecord};
use otspec::layout::anchor::Anchor;
use otspec::layout::coverage::Coverage;

use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
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
#[allow(non_snake_case)]
/// Information about anchor positioning on a base
pub struct BaseRecord {
    /// A list of base anchors
    pub baseAnchors: Vec<Offset16<Anchor>>,
}

impl Serialize for BaseRecord {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        for a in &self.baseAnchors {
            data.put(a)?
        }
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        2 * self.baseAnchors.len()
    }

    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        self.baseAnchors
            .iter()
            .map(|x| {
                let erase_type: &dyn OffsetMarkerTrait = x;
                erase_type
            })
            .collect()
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
/// A mark-to-base subtable.
pub struct MarkBasePos {
    /// Base glyphs to be attached in this subtable
    /// Each base glyph is mapped to a mapping of (anchor class, anchor position)
    pub bases: BTreeMap<GlyphID, BTreeMap<uint16, Anchor>>,
    /// Mark glyphs to be attached in this subtable
    /// Each mark glyph is associated with an anchor class and anchor position
    pub marks: BTreeMap<GlyphID, (uint16, Anchor)>,
}

// We need to deserialize this thing manually because of the data dependency:
// BaseRecord needs to know markClassCount.
impl Deserialize for MarkBasePos {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let _pos_format: uint16 = c.de()?;
        let mark_coverage: Offset16<Coverage> = c.de()?;
        let mark_glyphs: Coverage = mark_coverage.link.ok_or_else(|| {
            DeserializationError("Dangling offset in GPOS4 mark coverage".to_string())
        })?;
        let base_coverage: Offset16<Coverage> = c.de()?;
        let base_glyphs: Coverage = base_coverage.link.ok_or_else(|| {
            DeserializationError("Dangling offset in GPOS4 base coverage".to_string())
        })?;
        let mark_class_count: uint16 = c.de()?;
        let mark_array_offset: Offset16<MarkArray> = c.de()?;
        let mark_array: MarkArray = mark_array_offset.link.ok_or_else(|| {
            DeserializationError("Dangling offset in GPOS4 mark array".to_string())
        })?;

        let mut marks: BTreeMap<GlyphID, (uint16, Anchor)> = BTreeMap::new();
        for (&mark_glyph, mark_record) in
            mark_glyphs.glyphs.iter().zip(mark_array.markRecords.iter())
        {
            marks.insert(
                mark_glyph,
                (
                    mark_record.markClass,
                    mark_record.markAnchor.link.ok_or_else(|| {
                        DeserializationError("Dangling offset in GPOS4 mark record".to_string())
                    })?,
                ),
            );
        }

        // Now it gets tricky.
        let offset: uint16 = c.de()?;
        c.ptr = c.top_of_table() + offset as usize;

        // We are now at the start of the base array table
        c.push();
        let mut bases: BTreeMap<GlyphID, BTreeMap<uint16, Anchor>> = BTreeMap::new();
        let count: uint16 = c.de()?;
        if count != base_glyphs.glyphs.len() as uint16 {
            return Err(DeserializationError(
                "Base coverage length didn't match base array count".to_string(),
            ));
        }
        for base_glyph in base_glyphs.glyphs {
            let base_record: Vec<Offset16<Anchor>> = c.de_counted(mark_class_count.into())?;
            let mut anchor_list: BTreeMap<uint16, Anchor> = BTreeMap::new();
            for (class, anchor) in base_record.iter().enumerate() {
                anchor_list.insert(
                    class as u16,
                    anchor.link.ok_or_else(|| {
                        DeserializationError("Dangling offset in GPOS4 anchor".to_string())
                    })?,
                );
            }
            bases.insert(base_glyph, anchor_list);
        }
        c.pop();
        c.pop();
        Ok(MarkBasePos { marks, bases })
    }
}

impl From<&MarkBasePos> for MarkBasePosFormat1 {
    #[allow(non_snake_case)]
    fn from(lookup: &MarkBasePos) -> Self {
        let mut markClassCount = 0;
        let markArray = Offset16::to(MarkArray {
            markRecords: lookup
                .marks
                .values()
                .map(|&(class, anchor)| {
                    if class + 1 > markClassCount {
                        markClassCount = class + 1;
                    }
                    MarkRecord {
                        markClass: class,
                        markAnchor: Offset16::to(anchor),
                    }
                })
                .collect(),
        });

        let base_records = lookup
            .bases
            .values()
            .map(|base| BaseRecord {
                baseAnchors: (0..markClassCount)
                    .map(|i| {
                        base.get(&i)
                            .copied()
                            .map(Offset16::to)
                            .unwrap_or_else(Offset16::to_nothing)
                    })
                    .collect(),
            })
            .collect();

        MarkBasePosFormat1 {
            posFormat: 1,
            markCoverage: Offset16::to(Coverage {
                glyphs: lookup.marks.keys().copied().collect(),
            }),
            baseCoverage: Offset16::to(Coverage {
                glyphs: lookup.bases.keys().copied().collect(),
            }),
            baseArray: Offset16::to(BaseArray {
                baseRecords: base_records,
            }),
            markArray,
            markClassCount,
        }
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
    use otspec::btreemap;
    use pretty_assertions::assert_eq;
    use std::iter::FromIterator;

    #[test]
    fn test_markbase_de() {
        let binary_markbase = vec![
            // MarkBasePosFormat1
            // MarkBaseAttachSubTable   MarkBasePos subtable definition
            0x00, 0x01, // 1    posFormat
            0x00, 0x0C, // MarkGlyphsCoverage   offset to markCoverage table
            0x00, 0x14, // BaseGlyphsCoverage   offset to baseCoverage table
            0x00, 0x02, // 2    markClassCount
            0x00, 0x1A, // MarkGlyphsArray  offset to MarkArray table
            0x00, 0x30, // BaseGlyphsArray  offset to BaseArray table
            // CoverageFormat1
            // MarkGlyphsCoverage   Coverage table definition
            0x00, 0x01, // 1    coverageFormat: lists
            0x00, 0x02, // 2    glyphCount
            0x03, 0x33, // fathatanMarkGlyphID  glyphArray[0]
            0x03, 0x3F, // kasraMarkGlyphID glyphArray[1]
            // CoverageFormat1
            // BaseGlyphsCoverage   Coverage table definition
            0x00, 0x01, // 1    coverageFormat: lists
            0x00, 0x01, // 1    glyphCount
            0x01, 0x90, // tahBaseGlyphID   glyphArray[0]
            // MarkArray
            // MarkGlyphsArray  MarkArray table definition
            0x00, 0x02, // 2    markCount
            // markRecords[0]   MarkRecords in Coverage index order
            0x00, 0x00, // 0    markClass, for marks over base
            0x00, 0x0A, // fathatanMarkAnchor   markAnchorOffset
            // markRecords[1]
            0x00, 0x01, // 1    markClass, for marks under
            0x00, 0x10, // kasraMarkAnchor  markAnchorOffset
            // AnchorFormat1
            // fathatanMarkAnchor   Anchor table definition
            0x00, 0x01, // 1    anchorFormat: design units only
            0x01, 0x5A, // 346  xCoordinate
            0xFF, 0x9E, // -98  yCoordinate
            // AnchorFormat1
            // kasraMarkAnchor  Anchor table definition
            0x00, 0x01, // 1    anchorFormat: design units only
            0x01, 0x05, // 261  xCoordinate
            0x00, 0x58, // 88   yCoordinate
            // BaseArray
            // BaseGlyphsArray  BaseArray table definition
            0x00, 0x01, // 1    baseCount
            // baseRecords[0]
            0x00, 0x06, // AboveBaseAnchor  baseAnchorOffsets[0]
            0x00, 0x0C, // BelowBaseAnchor  baseAnchorOffsets[1]
            // AnchorFormat1
            // AboveBaseAnchor  Anchor table definition
            0x00, 0x01, // 1    anchorFormat: design units only
            0x03, 0x3E, // 830  xCoordinate
            0x06, 0x40, // 1600 yCoordinate
            // AnchorFormat1
            // BelowBaseAnchor  Anchor table definition
            0x00, 0x01, // 1    anchorFormat: design units only
            0x03, 0x3E, // 830  xCoordinate
            0xFF, 0xAD, // -83  yCoordinate
        ];
        let markbase: MarkBasePos = otspec::de::from_bytes(&binary_markbase).unwrap();
        let expected_tah_marks: BTreeMap<uint16, Anchor> =
            btreemap!(0 => Anchor::new(830,1600), 1 => Anchor::new(830,-83));
        assert_eq!(
            markbase,
            MarkBasePos {
                marks: btreemap!(819 => (0, Anchor::new(346,-98)), 831 => (1, Anchor::new(261, 88))),
                bases: btreemap!(400 => expected_tah_marks),
            },
        );

        let output: Vec<u8> = otspec::ser::to_bytes(&markbase).unwrap();
        assert_eq!(output, binary_markbase);
    }
}
