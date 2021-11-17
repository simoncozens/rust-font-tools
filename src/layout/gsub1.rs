use crate::layout::common::{coverage_or_nah, FromLowlevel, ToLowlevel};
use otspec::layout::coverage::Coverage;
use otspec::layout::gsub1::{SingleSubstFormat1, SingleSubstFormat2};
use otspec::tables::GSUB::GSUBSubtable;
use otspec::types::*;
use std::collections::BTreeMap;

/* This struct is the user-facing representation of single-subst. A mapping of
GID -> GID is a friendly way to represent what's going on. */

#[derive(Debug, PartialEq, Clone, Default)]
/// A single substitution subtable.
pub struct SingleSubst {
    /// The mapping of input glyph IDs to replacement glyph IDs.
    pub mapping: BTreeMap<GlyphID, GlyphID>,
}

impl SingleSubst {
    fn best_format(&self) -> (uint16, i16) {
        let mut delta = 0_i16;
        let mut map = self.mapping.iter();
        let format: u16 = if let Some((&first_left, &first_right)) = map.next() {
            delta = (first_right as i16).wrapping_sub(first_left as i16);
            let mut format = 1;
            for (&left, &right) in map {
                if (left as i16).wrapping_add(delta) != (right as i16) {
                    format = 2;
                    break;
                }
            }
            format
        } else {
            2
        };
        (format, delta)
    }
}

impl FromLowlevel<GSUBSubtable> for SingleSubst {
    fn from_lowlevel(st: GSUBSubtable, _max_glyph_id: GlyphID) -> Self {
        let mut singlesubst = SingleSubst::default();
        match st {
            GSUBSubtable::GSUB1_1(singlesubst1) => {
                for glyph in coverage_or_nah(singlesubst1.coverage) {
                    singlesubst
                        .mapping
                        .insert(glyph, (glyph as i16 + singlesubst1.deltaGlyphID) as u16);
                }
            }
            GSUBSubtable::GSUB1_2(singlesubst2) => {
                for (glyph, subst) in coverage_or_nah(singlesubst2.coverage)
                    .iter()
                    .zip(singlesubst2.substituteGlyphIDs)
                {
                    singlesubst.mapping.insert(*glyph, subst);
                }
            }
            _ => panic!(),
        }
        singlesubst
    }
}

impl ToLowlevel<GSUBSubtable> for SingleSubst {
    fn to_lowlevel(&self, _max_glyph_id: GlyphID) -> GSUBSubtable {
        let (format, delta) = self.best_format();
        let coverage = Coverage {
            glyphs: self.mapping.keys().copied().collect(),
        };
        if format == 1 {
            GSUBSubtable::GSUB1_1(SingleSubstFormat1 {
                substFormat: 1,
                coverage: Offset16::to(coverage),
                deltaGlyphID: delta,
            })
        } else {
            GSUBSubtable::GSUB1_2(SingleSubstFormat2 {
                substFormat: 2,
                coverage: Offset16::to(coverage),
                substituteGlyphIDs: self.mapping.values().copied().collect(),
            })
        }
    }
}
