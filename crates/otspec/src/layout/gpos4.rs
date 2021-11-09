use crate::layout::anchor::Anchor;
use crate::{DeserializationError, Deserialize, ReaderContext, SerializationError, Serializer};
use otspec::layout::common::MarkArray;
use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::{Deserializer, Serialize};
use otspec_macros::tables;

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

// We need to deserialize this thing manually because of the data dependency:
// BaseRecord needs to know markClassCount.
impl Deserialize for MarkBasePosFormat1 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let pos_format: uint16 = c.de()?;
        let mark_coverage: Offset16<Coverage> = c.de()?;
        let base_coverage: Offset16<Coverage> = c.de()?;
        let mark_class_count: uint16 = c.de()?;
        let mark_array_offset: Offset16<MarkArray> = c.de()?;

        // Now it gets tricky.
        let base_array_offset: uint16 = c.de()?;
        let mut base_records = vec![];
        c.ptr = c.top_of_table() + base_array_offset as usize;
        // We are now at the start of the base array table
        c.push();

        let count: uint16 = c.de()?;
        for _ in 0..count {
            let base_record: Vec<Offset16<Anchor>> = c.de_counted(mark_class_count.into())?;
            base_records.push(BaseRecord {
                baseAnchors: base_record,
            })
        }
        c.pop();
        c.pop();
        Ok(MarkBasePosFormat1 {
            posFormat: pos_format,
            markCoverage: mark_coverage,
            baseCoverage: base_coverage,
            markClassCount: mark_class_count,
            markArray: mark_array_offset,
            baseArray: Offset16::new(
                base_array_offset,
                BaseArray {
                    baseRecords: base_records,
                },
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::MarkRecord;

    #[test]
    fn test_markbase_deser() {
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
        let markbase: MarkBasePosFormat1 = otspec::de::from_bytes(&binary_markbase).unwrap();
        let expected = MarkBasePosFormat1 {
            posFormat: 1,
            markCoverage: Offset16::to(Coverage {
                glyphs: vec![819, 831],
            }),
            baseCoverage: Offset16::to(Coverage { glyphs: vec![400] }),
            markClassCount: 2,
            markArray: Offset16::to(MarkArray {
                markRecords: vec![
                    MarkRecord {
                        markClass: 0,
                        markAnchor: Offset16::to(Anchor {
                            xCoordinate: 346,
                            yCoordinate: -98,
                            anchorPoint: None,
                        }),
                    },
                    MarkRecord {
                        markClass: 1,
                        markAnchor: Offset16::to(Anchor {
                            xCoordinate: 261,
                            yCoordinate: 88,
                            anchorPoint: None,
                        }),
                    },
                ],
            }),
            baseArray: Offset16::to(BaseArray {
                baseRecords: vec![BaseRecord {
                    baseAnchors: vec![
                        Offset16::to(Anchor {
                            xCoordinate: 830,
                            yCoordinate: 1600,
                            anchorPoint: None,
                        }),
                        Offset16::to(Anchor {
                            xCoordinate: 830,
                            yCoordinate: -83,
                            anchorPoint: None,
                        }),
                    ],
                }],
            }),
        };
        assert_eq!(markbase, expected);

        let output: Vec<u8> = otspec::ser::to_bytes(&markbase).unwrap();
        assert_eq!(output, binary_markbase);
    }
}
