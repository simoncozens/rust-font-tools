use crate::layout::common::{coverage_or_nah, FromLowlevel, ToLowlevel};
use otspec::layout::coverage::Coverage;
use otspec::layout::gpos1::{SinglePosFormat1, SinglePosFormat2};
use otspec::layout::valuerecord::{coerce_to_same_format, ValueRecord};
use otspec::tables::GPOS::GPOSSubtable;
use otspec::types::*;
use otspec::utils::is_all_the_same;
use std::collections::BTreeMap;

/* This struct is the user-facing representation of single-pos. A mapping of
GID -> valuerecord is a friendly way to represent what's going on. */
#[derive(Debug, PartialEq, Clone, Default)]
/// A single positioning subtable.
pub struct SinglePos {
    /// The mapping of input glyph IDs to value records.
    pub mapping: BTreeMap<GlyphID, ValueRecord>,
}
impl FromLowlevel<GPOSSubtable> for SinglePos {
    fn from_lowlevel(st: GPOSSubtable, _max_glyph_id: GlyphID) -> Self {
        let mut singlepos = SinglePos::default();
        match st {
            GPOSSubtable::GPOS1_1(singlepos1) => {
                for glyph in coverage_or_nah(singlepos1.coverage) {
                    singlepos
                        .mapping
                        .insert(glyph, singlepos1.valueRecord.clone());
                }
            }
            GPOSSubtable::GPOS1_2(singlepos2) => {
                for (glyph, rv) in coverage_or_nah(singlepos2.coverage)
                    .iter()
                    .zip(singlepos2.valueRecords)
                {
                    singlepos.mapping.insert(*glyph, rv);
                }
            }
            _ => panic!(),
        }
        singlepos
    }
}

/* On serialization, move to the outgoing representation by choosing the best format */
impl ToLowlevel<GPOSSubtable> for &SinglePos {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GPOSSubtable {
        let mut mapping = self.mapping.clone();
        for (_, val) in mapping.iter_mut() {
            (*val).simplify()
        }
        let coverage = Coverage {
            glyphs: mapping.keys().copied().collect(),
        };
        if is_all_the_same(mapping.values()) {
            let vr = mapping.values().next().unwrap();
            GPOSSubtable::GPOS1_1(SinglePosFormat1 {
                posFormat: 1,
                coverage: Offset16::to(coverage),
                valueFormat: vr.flags(),
                valueRecord: vr.clone(),
            })
        } else {
            let vrs: Vec<ValueRecord> = mapping.values().cloned().collect();
            let vrs = coerce_to_same_format(vrs);
            GPOSSubtable::GPOS1_2(SinglePosFormat2 {
                posFormat: 2,
                coverage: Offset16::to(coverage),
                valueFormat: vrs[0].flags(),
                valueRecords: vrs,
            })
        }
    }
}
