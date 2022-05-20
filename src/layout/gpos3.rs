use crate::layout::common::{coverage_or_nah, FromLowlevel, ToLowlevel};
use otspec::layout::anchor::Anchor;
use otspec::layout::coverage::Coverage;
use otspec::layout::gpos3::{CursivePosFormat1, EntryExitRecord};
use otspec::tables::GPOS::GPOSSubtable;
use otspec::types::*;

use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Clone, Default)]
/// A cursive positioning subtable.
pub struct CursivePos {
    /// The mapping of glyph IDs to entry and exit anchor records.
    pub mapping: BTreeMap<GlyphID, (Option<Anchor>, Option<Anchor>)>,
}

impl FromLowlevel<GPOSSubtable> for CursivePos {
    fn from_lowlevel(st: GPOSSubtable, _max_glyph_id: GlyphID) -> Self {
        let mut mapping = BTreeMap::new();
        match st {
            GPOSSubtable::GPOS3_1(cursivepos1) => {
                for (input, anchors) in coverage_or_nah(cursivepos1.coverage)
                    .iter()
                    .zip(cursivepos1.entryExitRecord.iter())
                {
                    let entry = anchors.entryAnchor.link;
                    let exit = anchors.exitAnchor.link;
                    mapping.insert(*input, (entry, exit));
                }
            }
            _ => panic!(),
        }
        CursivePos { mapping }
    }
}

impl ToLowlevel<GPOSSubtable> for CursivePos {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GPOSSubtable {
        let coverage = Offset16::to(Coverage {
            glyphs: self.mapping.keys().copied().collect(),
        });
        let mut anchors = vec![];
        for right in self.mapping.values() {
            let entry_exit = EntryExitRecord {
                entryAnchor: right.0.map_or_else(Offset16::to_nothing, Offset16::to),
                exitAnchor: right.1.map_or_else(Offset16::to_nothing, Offset16::to),
            };
            anchors.push(entry_exit);
        }
        GPOSSubtable::GPOS3_1(CursivePosFormat1 {
            posFormat: 1,
            coverage,
            entryExitRecord: anchors,
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
    fn some_curs_de() {
        /*
        feature test {
            pos cursive A <anchor 100 200> <anchor NULL>;
            pos cursive B <anchor NULL> <anchor NULL>;
            pos cursive C <anchor NULL> <anchor -300 -400>;
            pos cursive D <anchor 1 2> <anchor 3 4>;
        } test;
        */
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x16, 0x00, 0x04, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x26, 0x00, 0x2c, 0x00, 0x32, 0x00, 0x02, 0x00, 0x01, 0x00, 0x22,
            0x00, 0x25, 0x00, 0x00, 0x00, 0x01, 0x00, 0x64, 0x00, 0xc8, 0x00, 0x01, 0xfe, 0xd4,
            0xfe, 0x70, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x03, 0x00, 0x04,
        ];
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::Cursive(vec![CursivePos {
                mapping: btreemap!(
                    34 => (Some(Anchor { xCoordinate: 100, yCoordinate: 200, anchorPoint: None }), None),
                    35 => (None, None),
                    36 => (None, Some(Anchor { xCoordinate: -300, yCoordinate: -400, anchorPoint: None })),
                    37 => (Some(Anchor { xCoordinate: 1, yCoordinate: 2, anchorPoint: None }),
                           Some(Anchor { xCoordinate: 3, yCoordinate: 4, anchorPoint: None }))
                ),
            }]),
        }]);
        assert_can_roundtrip(binary_gpos, &expected);
    }
}
