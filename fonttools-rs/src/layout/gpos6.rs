use crate::layout::common::{coverage_or_nah, FromLowlevel, ToLowlevel};
use otspec::layout::anchor::Anchor;
use otspec::layout::common::{MarkArray, MarkRecord};
use otspec::layout::coverage::Coverage;
use otspec::layout::gpos6::{Mark2Array, Mark2Record, MarkMarkPosFormat1};
use otspec::tables::GPOS::GPOSSubtable;
use otspec::types::*;

use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
/// A mark-to-base subtable.
pub struct MarkMarkPos {
    /// Base marks to be attached in this subtable
    /// Each base mark glyph is mapped to a mapping of (anchor class, anchor position)
    pub base_marks: BTreeMap<GlyphID, BTreeMap<uint16, Anchor>>,
    /// Combining marks to be attached in this subtable
    /// Each combining mark glyph is associated with an anchor class and anchor position
    pub combining_marks: BTreeMap<GlyphID, (uint16, Anchor)>,
}

impl FromLowlevel<GPOSSubtable> for MarkMarkPos {
    fn from_lowlevel(st: GPOSSubtable, _max_glyph_id: GlyphID) -> Self {
        match st {
            GPOSSubtable::GPOS6_1(markmark) => {
                let combining_mark_glyphs = coverage_or_nah(markmark.mark1Coverage);
                let base_mark_glyphs = coverage_or_nah(markmark.mark2Coverage);
                let mut base_marks: BTreeMap<GlyphID, BTreeMap<uint16, Anchor>> = BTreeMap::new();
                let mut combining_marks: BTreeMap<GlyphID, (uint16, Anchor)> = BTreeMap::new();
                let base_mark_array = markmark.mark2Array.link.unwrap_or_default();
                let combining_mark_array = markmark.mark1Array.link.unwrap_or_default();
                for (&combining_mark_glyph, combining_mark_record) in combining_mark_glyphs
                    .iter()
                    .zip(combining_mark_array.markRecords.iter())
                {
                    combining_marks.insert(
                        combining_mark_glyph,
                        (
                            combining_mark_record.markClass,
                            combining_mark_record.markAnchor.link.unwrap_or_default(),
                        ),
                    );
                }
                for (&base_glyph, base_mark_record) in base_mark_glyphs
                    .iter()
                    .zip(base_mark_array.mark2Records.iter())
                {
                    let mut anchor_list: BTreeMap<uint16, Anchor> = BTreeMap::new();
                    for (class, base_anchor) in base_mark_record
                        .mark2Anchors
                        .iter()
                        .map(|x| x.link)
                        .enumerate()
                    {
                        if let Some(anchor) = base_anchor {
                            anchor_list.insert(class as u16, anchor);
                        }
                    }
                    base_marks.insert(base_glyph, anchor_list);
                }
                MarkMarkPos {
                    base_marks,
                    combining_marks,
                }
            }
            _ => panic!(),
        }
    }
}
impl ToLowlevel<GPOSSubtable> for MarkMarkPos {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GPOSSubtable {
        let mut mark_class_count = 0;
        let mark1_array = Offset16::to(MarkArray {
            markRecords: self
                .combining_marks
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

        let base_records = self
            .base_marks
            .values()
            .map(|base| Mark2Record {
                mark2Anchors: (0..mark_class_count)
                    .map(|i| {
                        base.get(&i)
                            .copied()
                            .map(Offset16::to)
                            .unwrap_or_else(Offset16::to_nothing)
                    })
                    .collect(),
            })
            .collect();

        GPOSSubtable::GPOS6_1(MarkMarkPosFormat1 {
            posFormat: 1,
            mark1Coverage: Offset16::to(Coverage {
                glyphs: self.combining_marks.keys().copied().collect(),
            }),
            mark2Coverage: Offset16::to(Coverage {
                glyphs: self.base_marks.keys().copied().collect(),
            }),
            mark2Array: Offset16::to(Mark2Array {
                mark2Records: base_records,
            }),
            mark1Array: mark1_array,
            markClassCount: mark_class_count,
        })
    }
}
