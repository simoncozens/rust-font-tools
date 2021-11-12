use crate::layout::anchor::Anchor;
use crate::{
    Counted, DeserializationError, Deserialize, ReaderContext, SerializationError, Serialize,
    Serializer,
};
use otspec::layout::common::MarkArray;
use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::tables;

tables!(
  MarkLigPosFormat1 [nodeserialize] {
    [offset_base]
    uint16 posFormat
    Offset16(Coverage) markCoverage
    Offset16(Coverage) ligatureCoverage
    uint16 markClassCount
    Offset16(MarkArray) markArray
    Offset16(LigatureArray) ligatureArray
  }

  LigatureArray [nodeserialize] [default] {
    [offset_base]
    CountedOffset16(LigatureAttach) ligatureAttach
  }

  LigatureAttach [nodeserialize] [default] {
    [offset_base]
    [embed]
    Counted(ComponentRecord) componentRecords
  }
);

#[derive(Debug, Clone, PartialEq)]
#[allow(non_snake_case)]
/// Information about anchor positioning on a ligature
pub struct ComponentRecord {
    /// A list of ligature anchors
    pub ligatureAnchors: Vec<Offset16<Anchor>>,
}

impl Serialize for ComponentRecord {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        for a in &self.ligatureAnchors {
            data.put(a)?
        }
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        2 * self.ligatureAnchors.len()
    }

    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        self.ligatureAnchors
            .iter()
            .map(|x| {
                let erase_type: &dyn OffsetMarkerTrait = x;
                erase_type
            })
            .collect()
    }
}

// We need to deserialize this thing manually because of the data dependency:
// ComponentRecord needs to know markClassCount.
impl Deserialize for MarkLigPosFormat1 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let pos_format: uint16 = c.de()?;
        let mark_coverage: Offset16<Coverage> = c.de()?;
        let ligature_coverage: Offset16<Coverage> = c.de()?;
        let mark_class_count: uint16 = c.de()?;
        let mark_array_offset: Offset16<MarkArray> = c.de()?;

        let ligature_array_offset: uint16 = c.de()?;
        let mut ligature_array: LigatureArray = LigatureArray::default();
        c.follow_offset(ligature_array_offset);
        {
            // We are now at the start of the ligature array table
            c.push();

            let attach_count: uint16 = c.de()?;
            for _ in 0..attach_count {
                let mut component_records: Vec<ComponentRecord> = vec![];

                let ligature_attach_offset: uint16 = c.de()?;
                c.follow_offset(ligature_attach_offset);
                {
                    // We are now at the start of the ligature attach table
                    c.push();
                    let component_record_count: uint16 = c.de()?;
                    for _ in 0..component_record_count {
                        let ligature_anchors: Counted<Offset16<Anchor>> =
                            c.de_counted(mark_class_count.into())?.into();
                        component_records.push(ComponentRecord {
                            ligatureAnchors: ligature_anchors.into(),
                        });
                    }
                    c.pop();
                }
                ligature_array.ligatureAttach.push(Offset16::new(
                    ligature_array_offset,
                    LigatureAttach {
                        componentRecords: component_records,
                    },
                ));
            }
            c.pop();
        }

        c.pop();
        Ok(MarkLigPosFormat1 {
            posFormat: pos_format,
            markCoverage: mark_coverage,
            markClassCount: mark_class_count,
            markArray: mark_array_offset,
            ligatureArray: Offset16::new(ligature_array_offset, ligature_array),
            ligatureCoverage: ligature_coverage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::MarkRecord;

    #[test]
    fn test_marklig_deser() {
        let binary_marklig = vec![
            // MarkLigPosFormat1
            // MarkLigAttachSubTable  MarkLigPos subtable definition
            0x00, 0x01, //   1  posFormat
            0x00, 0x0C, //   MarkGlyphsCoverage offset to markCoverage table
            0x00, 0x14, //   LigGlyphsCoverage  offset to ligatureCoverage table
            0x00, 0x02, //   2  markClassCount
            0x00, 0x1A, //   MarkGlyphsArray  offset to MarkArray table
            0x00, 0x30, //   LigGlyphsArray offset to LigatureArray table
            // CoverageFormat1
            // MarkGlyphsCoverage Coverage table definition
            0x00, 0x01, //   1  coverageFormat: lists
            0x00, 0x02, //   2  glyphCount
            0x03, 0x3C, //   sukunMarkGlyphID glyphArray[0]
            0x03, 0x3F, //   kasratanMarkGlyphID  glyphArray[1]
            // CoverageFormat1
            // LigGlyphsCoverage  Coverage table definition
            0x00, 0x01, //   1  coverageFormat: lists
            0x00, 0x01, //   1  glyphCount
            0x02, 0x34, //   LamWithMeemWithJeem
            // LigatureGlyphID  glyphArray[0]
            // MarkArray
            // MarkGlyphsArray  MarkArray table definition
            0x00, 0x02, //   2  markCount
            // markRecords[0] MarkRecords in Coverage index order
            0x00, 0x00, //   0  markClass, for marks above components
            0x00, 0x0A, //   sukunMarkAnchor  markAnchorOffset
            // markRecords[1]
            0x00, 0x01, //   1  markClass, for marks below components
            0x00, 0x10, //   kasratanMarkAnchor markAnchorOffset
            // AnchorFormat1
            // sukunMarkAnchor  Anchor table definition
            0x00, 0x01, //   1  anchorFormat: design units only
            0x01, 0x5A, //   346  xCoordinate
            0xFF, 0x9E, //   -98  yCoordinate
            // AnchorFormat1
            // kasratanMarkAnchor Anchor table definition
            0x00, 0x01, //   1  anchorFormat: design units only
            0x01, 0x05, //   261  xCoordinate
            0x01, 0xE8, //   488  yCoordinate
            // LigatureArray
            // LigGlyphsArray LigatureArray table definition
            0x00, 0x01, //   1  ligatureCount
            0x00, 0x04, //   LamWithMeemWithJeemLigAttach ligatureAttachOffsets[0]
            // LigatureAttach
            // LamWithMeemWithJeemLigAttach LigatureAttach table definition
            0x00, 0x03, //   3  componentCount
            // componentRecords[0]  Right-to-left text; ComponentRecords in writing-direction (logical) order: right-most glyph first
            0x00,
            0x0E, //   AboveLamAnchor ligatureAnchorOffsets[0] — offsets ordered by mark class
            0x00,
            0x00, //   NULL ligatureAnchorOffsets[1] — no attachment points for Class1 marks
            // componentRecords[1]
            0x00,
            0x00, //   NULL ligatureAnchorOffsets[0] — no attachment points for Class 0 marks
            0x00,
            0x14, //   BelowMeemAnchor  ligatureAnchorOffsets — for Class 1 marks (below)
            // componentRecords[2]
            0x00,
            0x00, //   NULL ligatureAnchorOffsets — no attachment points for Class 0 marks
            0x00,
            0x00, //   NULL ligatureAnchorOffsets[1] — no attachment points for Class 1 marks
            // AnchorFormat1
            // AboveLamAnchor Anchor table definition
            0x00, 0x01, //   1  anchorFormat: design units only
            0x02, 0x71, //   625  xCoordinate
            0x07, 0x08, //   1800 yCoordinate
            // AnchorFormat1
            // BelowMeemAnchor  Anchor table definition
            0x00, 0x01, //   1  anchorFormat: design units only
            0x01, 0x78, //   376  xCoordinate
            0xFE, 0x90, //   -368 yCoordinate
        ];
        let markbase: MarkLigPosFormat1 = otspec::de::from_bytes(&binary_marklig).unwrap();
        let expected = MarkLigPosFormat1 {
            posFormat: 1,
            markCoverage: Offset16::to(Coverage {
                glyphs: vec![0x33c, 0x33f],
            }),
            ligatureCoverage: Offset16::to(Coverage {
                glyphs: vec![0x234],
            }),
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
                            yCoordinate: 488,
                            anchorPoint: None,
                        }),
                    },
                ],
            }),
            ligatureArray: Offset16::to(LigatureArray {
                ligatureAttach: vec![Offset16::to(LigatureAttach {
                    componentRecords: vec![
                        ComponentRecord {
                            ligatureAnchors: vec![
                                Offset16::to(Anchor {
                                    xCoordinate: 625,
                                    yCoordinate: 1800,
                                    anchorPoint: None,
                                }),
                                Offset16::to_nothing(),
                            ],
                        },
                        ComponentRecord {
                            ligatureAnchors: vec![
                                Offset16::to_nothing(),
                                Offset16::to(Anchor {
                                    xCoordinate: 376,
                                    yCoordinate: -368,
                                    anchorPoint: None,
                                }),
                            ],
                        },
                        ComponentRecord {
                            ligatureAnchors: vec![Offset16::to_nothing(), Offset16::to_nothing()],
                        },
                    ],
                })]
                .into(),
            }),
        };
        assert_eq!(markbase, expected);

        let output: Vec<u8> = otspec::ser::to_bytes(&markbase).unwrap();
        assert_eq!(output, binary_marklig);
    }
}
