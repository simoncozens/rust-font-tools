use crate::layout::common::{coverage_or_nah, FromLowlevel, ToLowlevel};
use otspec::layout::anchor::Anchor;
use otspec::layout::common::{MarkArray, MarkRecord};
use otspec::layout::coverage::Coverage;
use otspec::layout::gpos5::{ComponentRecord, LigatureArray, LigatureAttach, MarkLigPosFormat1};
use otspec::tables::GPOS::GPOSSubtable;
use otspec::types::*;

use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
/// A mark-to-ligature subtable.
pub struct MarkLigPos {
    /// Ligature glyphs to be attached in this subtable
    /// Each ligature glyph is mapped to a vec (one-per-component) of mappings of (anchor class, anchor position)
    pub ligatures: BTreeMap<GlyphID, Vec<BTreeMap<uint16, Anchor>>>,
    /// Mark glyphs to be attached in this subtable
    /// Each mark glyph is associated with an anchor class and anchor position
    pub marks: BTreeMap<GlyphID, (uint16, Anchor)>,
}

impl FromLowlevel<GPOSSubtable> for MarkLigPos {
    fn from_lowlevel(st: GPOSSubtable, _max_glyph_id: GlyphID) -> Self {
        match st {
            GPOSSubtable::GPOS5_1(markligature1) => {
                let mark_glyphs = coverage_or_nah(markligature1.markCoverage);
                let ligature_glyphs = coverage_or_nah(markligature1.ligatureCoverage);
                let mut ligatures: BTreeMap<GlyphID, Vec<BTreeMap<uint16, Anchor>>> =
                    BTreeMap::new();
                let mut marks: BTreeMap<GlyphID, (uint16, Anchor)> = BTreeMap::new();
                let mark_array = markligature1.markArray.link.unwrap_or_default();
                let ligature_array = markligature1.ligatureArray.link.unwrap_or_default();
                for (&mark_glyph, mark_record) in
                    mark_glyphs.iter().zip(mark_array.markRecords.iter())
                {
                    marks.insert(
                        mark_glyph,
                        (
                            mark_record.markClass,
                            mark_record.markAnchor.link.unwrap_or_default(),
                        ),
                    );
                }
                for (&ligature_glyph, ligature_attach) in ligature_glyphs
                    .iter()
                    .zip(ligature_array.ligatureAttach.v.iter())
                {
                    let mut component_records = vec![];
                    for component in ligature_attach
                        .link
                        .as_ref()
                        .map_or_else(Vec::new, |x| x.componentRecords.clone())
                    // XXX clone
                    {
                        let mut anchor_list: BTreeMap<uint16, Anchor> = BTreeMap::new();
                        for (class, ligature_anchor) in
                            component.ligatureAnchors.iter().map(|x| x.link).enumerate()
                        {
                            if let Some(anchor) = ligature_anchor {
                                anchor_list.insert(class as u16, anchor);
                            }
                        }
                        component_records.push(anchor_list);
                    }
                    ligatures.insert(ligature_glyph, component_records);
                }
                MarkLigPos { marks, ligatures }
            }
            _ => panic!(),
        }
    }
}
impl ToLowlevel<GPOSSubtable> for MarkLigPos {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GPOSSubtable {
        let mut mark_class_count = 0;
        let mark_array = Offset16::to(MarkArray {
            markRecords: self
                .marks
                .values()
                .map(|&(class, anchor)| {
                    if class + 1 > mark_class_count {
                        mark_class_count = class + 1;
                    }
                    MarkRecord {
                        markClass: class,
                        markAnchor: Offset16::to(anchor),
                    }
                })
                .collect(),
        });

        let ligature_records: Vec<Offset16<LigatureAttach>> = self
            .ligatures
            .values()
            .map(|ligature| {
                Offset16::to(LigatureAttach {
                    componentRecords: ligature
                        .iter()
                        .map(|component| ComponentRecord {
                            ligatureAnchors: (0..mark_class_count)
                                .map(|i| {
                                    component
                                        .get(&i)
                                        .copied()
                                        .map(Offset16::to)
                                        .unwrap_or_else(Offset16::to_nothing)
                                })
                                .collect(),
                        })
                        .collect(),
                })
            })
            .collect();

        GPOSSubtable::GPOS5_1(MarkLigPosFormat1 {
            posFormat: 1,
            markCoverage: Offset16::to(Coverage {
                glyphs: self.marks.keys().copied().collect(),
            }),
            ligatureCoverage: Offset16::to(Coverage {
                glyphs: self.ligatures.keys().copied().collect(),
            }),
            ligatureArray: Offset16::to(LigatureArray {
                ligatureAttach: ligature_records.into(),
            }),
            markArray: mark_array,
            markClassCount: mark_class_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::{Lookup, LookupFlags};
    use crate::tables::GPOS::tests::{assert_can_roundtrip, expected_gpos};
    use crate::tables::GPOS::Positioning;
    use otspec::btreemap;
    use std::iter::FromIterator;

    #[test]
    fn test_markbase_de() {
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
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
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::MarkToLig(vec![MarkLigPos {
                ligatures: btreemap!(564 => vec![
                   btreemap!(0 => Anchor { xCoordinate: 625, yCoordinate: 1800, anchorPoint: None },
                    ),

                   btreemap!(
                    1 => Anchor { xCoordinate: 376, yCoordinate: -368, anchorPoint: None },
                   ),
                   btreemap!(),
                ]),
                marks: btreemap!(
                    828 => (0, Anchor { xCoordinate: 346, yCoordinate: -98, anchorPoint: None }),
                    831 => (1, Anchor { xCoordinate: 261, yCoordinate: 488, anchorPoint: None })
                ),
            }]),
        }]);
        assert_can_roundtrip(binary_gpos, &expected);
    }
}
