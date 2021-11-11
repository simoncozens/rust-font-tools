use crate::layout::common::{coverage_or_nah, FromLowlevel, ToLowlevel};
use otspec::layout::anchor::{self, Anchor};
use otspec::layout::common::{MarkArray, MarkRecord};
use otspec::layout::coverage::Coverage;
use otspec::layout::gpos4::{BaseArray, BaseRecord, MarkBasePosFormat1};
use otspec::tables::GPOS::GPOSSubtable;
use otspec::types::*;

use std::collections::BTreeMap;

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

impl FromLowlevel<GPOSSubtable> for MarkBasePos {
    fn from_lowlevel(st: GPOSSubtable, _max_glyph_id: GlyphID) -> Self {
        match st {
            GPOSSubtable::GPOS4_1(markbase1) => {
                let mark_glyphs = coverage_or_nah(markbase1.markCoverage);
                let base_glyphs = coverage_or_nah(markbase1.baseCoverage);
                let mut bases: BTreeMap<GlyphID, BTreeMap<uint16, Anchor>> = BTreeMap::new();
                let mut marks: BTreeMap<GlyphID, (uint16, Anchor)> = BTreeMap::new();
                let mark_array = markbase1.markArray.link.unwrap_or_default();
                let base_array = markbase1.baseArray.link.unwrap_or_default();
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
                for (&base_glyph, base_record) in
                    base_glyphs.iter().zip(base_array.baseRecords.iter())
                {
                    let mut anchor_list: BTreeMap<uint16, Anchor> = BTreeMap::new();
                    for (class, base_anchor) in base_record
                        .baseAnchors
                        .iter()
                        .map(|x| x.link.unwrap_or_default())
                        .enumerate()
                    {
                        anchor_list.insert(class as u16, base_anchor);
                    }
                    bases.insert(base_glyph, anchor_list);
                }
                MarkBasePos { marks, bases }
            }
            _ => panic!(),
        }
    }
}
impl ToLowlevel<GPOSSubtable> for MarkBasePos {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GPOSSubtable {
        let mut markClassCount = 0;
        let markArray = Offset16::to(MarkArray {
            markRecords: self
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

        let base_records = self
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

        GPOSSubtable::GPOS4_1(MarkBasePosFormat1 {
            posFormat: 1,
            markCoverage: Offset16::to(Coverage {
                glyphs: self.marks.keys().copied().collect(),
            }),
            baseCoverage: Offset16::to(Coverage {
                glyphs: self.bases.keys().copied().collect(),
            }),
            baseArray: Offset16::to(BaseArray {
                baseRecords: base_records,
            }),
            markArray,
            markClassCount,
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
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
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
        let expected_tah_marks: BTreeMap<uint16, Anchor> =
            btreemap!(0 => Anchor::new(830,1600), 1 => Anchor::new(830,-83));
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::MarkToBase(vec![MarkBasePos {
                marks: btreemap!(819 => (0, Anchor::new(346,-98)), 831 => (1, Anchor::new(261, 88))),
                bases: btreemap!(400 => expected_tah_marks),
            }]),
        }]);
        assert_can_roundtrip(binary_gpos, &expected);
    }
}
