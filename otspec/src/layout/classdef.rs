use crate::types::*;
use crate::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};
use std::collections::BTreeSet;

use otspec_macros::tables;
use std::collections::BTreeMap;

tables!(
    ClassDefFormat1 {
        uint16  startGlyphID
        Counted(uint16) classValueArray
    }
    ClassDefFormat2 {
        Counted(ClassRangeRecord) classRangeRecords
    }
    ClassRangeRecord {
        uint16  startGlyphID
        uint16  endGlyphID
        uint16  class
    }
);

// XXX This is also still too clever

#[derive(Debug, PartialEq, Clone, Default)]
/// A class definition table.
///
/// Class definitions, used to define glyph contexts in the GSUB and GPOS tables,
/// map a glyph ID to a glyph class integer.
pub struct ClassDef {
    /// Stores the mapping between glyph IDs and classes.
    pub classes: BTreeMap<GlyphID, uint16>,
}

impl ClassDef {
    /// Get a set of glyph IDs corresponding to the given class
    pub fn get_glyphs(&self, class_id: uint16, max_glyph_id: uint16) -> BTreeSet<GlyphID> {
        if class_id == 0 {
            let mut glyphs: BTreeSet<GlyphID> = (0..=max_glyph_id).collect();
            glyphs.retain(|g| !self.classes.contains_key(g));
            return glyphs;
        }
        // "Doing linear scans over an associative array is like trying to
        // club someone to death with a loaded Uzi." - Larry Wall
        self.classes
            .iter()
            .filter(|(_, &v)| v == class_id)
            .map(|(&k, _)| k)
            .collect()
    }
}

impl Deserialize for ClassDef {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let format: uint16 = c.de()?;
        let mut classes = BTreeMap::new();
        if format == 1 {
            let cdf1: ClassDefFormat1 = c.de()?;
            for (ix, class) in cdf1.classValueArray.iter().enumerate() {
                classes.insert(cdf1.startGlyphID + ix as uint16, *class);
            }
        } else {
            let cdf2: ClassDefFormat2 = c.de()?;
            for rr in cdf2.classRangeRecords {
                for g in rr.startGlyphID..(rr.endGlyphID + 1) {
                    classes.insert(g, rr.class);
                }
            }
        }
        Ok(ClassDef { classes })
    }
}

fn consecutive_slices(data: &[(uint16, uint16)]) -> Vec<&[(uint16, uint16)]> {
    let mut slice_start = 0;
    let mut result = Vec::new();
    for i in 1..data.len() {
        if data[i - 1].0 + 1 != data[i].0 || data[i - 1].1 != data[i].1 {
            result.push(&data[slice_start..i]);
            slice_start = i;
        }
    }
    if !data.is_empty() {
        result.push(&data[slice_start..]);
    }
    result
}

impl Serialize for ClassDef {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let pairs: Vec<(u16, u16)> = self.classes.iter().map(|(k, v)| (*k, *v)).collect();
        let as_consecutive = consecutive_slices(&pairs);
        if self.classes.is_empty() {
            data.put(2_u16)?;
            data.put(ClassDefFormat2 {
                classRangeRecords: vec![],
            })?;
            return Ok(());
        }
        let first_gid = pairs[0].0;
        let last_gid = pairs.last().unwrap().0;
        if as_consecutive.len() as u16 * 3 > (2 + last_gid - first_gid) {
            data.put(1_u16)?;
            data.put(ClassDefFormat1 {
                startGlyphID: first_gid,
                classValueArray: (first_gid..last_gid + 1)
                    .map(|gid| self.classes.get(&gid).map_or(0, |class| *class))
                    .collect(),
            })?;
        } else {
            data.put(2_u16)?;
            data.put(as_consecutive.len() as uint16)?;
            for slice in as_consecutive {
                data.put(ClassRangeRecord {
                    startGlyphID: slice.first().unwrap().0,
                    endGlyphID: slice.last().unwrap().0,
                    class: slice.first().unwrap().1,
                })?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use otspec::btreemap;
    use std::iter::FromIterator;

    #[test]
    fn test_format2_deser() {
        let expected = ClassDef {
            classes: btreemap!(
            24 => 1, 25 => 1, 26 => 1, 27 => 1, 28 => 1, 29 => 2, 30 => 1,
            31 => 5, 32 => 5, 33 => 1, 34 => 3, 35 => 1, 36 => 1, 70 => 1,
            71 => 2, 72 => 2, 73 => 1, 74 => 1, 75 => 1, 76 => 2, 77 => 5,
            78 => 3, 79 => 3, 80 => 1, 81 => 1, 82 => 2, 83 => 1, 84 => 2),
        };
        let binary_classdef = vec![
            0x00, 0x02, 0x00, 0x11, 0x00, 0x18, 0x00, 0x1c, 0x00, 0x01, 0x00, 0x1d, 0x00, 0x1d,
            0x00, 0x02, 0x00, 0x1e, 0x00, 0x1e, 0x00, 0x01, 0x00, 0x1f, 0x00, 0x20, 0x00, 0x05,
            0x00, 0x21, 0x00, 0x21, 0x00, 0x01, 0x00, 0x22, 0x00, 0x22, 0x00, 0x03, 0x00, 0x23,
            0x00, 0x24, 0x00, 0x01, 0x00, 0x46, 0x00, 0x46, 0x00, 0x01, 0x00, 0x47, 0x00, 0x48,
            0x00, 0x02, 0x00, 0x49, 0x00, 0x4b, 0x00, 0x01, 0x00, 0x4c, 0x00, 0x4c, 0x00, 0x02,
            0x00, 0x4d, 0x00, 0x4d, 0x00, 0x05, 0x00, 0x4e, 0x00, 0x4f, 0x00, 0x03, 0x00, 0x50,
            0x00, 0x51, 0x00, 0x01, 0x00, 0x52, 0x00, 0x52, 0x00, 0x02, 0x00, 0x53, 0x00, 0x53,
            0x00, 0x01, 0x00, 0x54, 0x00, 0x54, 0x00, 0x02,
        ];
        let deserialized: ClassDef = crate::de::from_bytes(&binary_classdef).unwrap();
        assert_eq!(deserialized, expected);
        let serialized = crate::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_classdef);
    }

    #[test]
    fn test_format1_deser() {
        let expected = ClassDef {
            classes: btreemap!(
 1 => 1,
 2 => 2,
 3 => 0,
 4 => 1,
 5 => 2,
 6 => 0,
 7 => 1,
 8 => 2,
 9 => 0,
 10 => 1,
 11 => 2,
 12 => 0,
 13 => 1,
 14 => 2),
        };
        let binary_classdef = vec![
            0x00, 0x01, 0x00, 0x01, 0x00, 0x0e, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x02,
        ];
        let deserialized: ClassDef = crate::de::from_bytes(&binary_classdef).unwrap();
        assert_eq!(deserialized, expected);
        let serialized = crate::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_classdef);
    }
}
