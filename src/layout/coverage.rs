use otspec::types::*;
use otspec::{deserialize_visitor, read_field};
use otspec_macros::tables;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Deserializer;
use serde::Serializer;
use serde::{Deserialize, Serialize};

tables!(
    CoverageFormat1 {
        Counted(uint16) glyphArray
    }
    CoverageFormat2 {
        Counted(RangeRecord) rangeRecords
    }
    RangeRecord {
        uint16  startGlyphID
        uint16  endGlyphID
        uint16    startCoverageIndex
    }
);

#[derive(Debug, PartialEq, Clone)]
/// A coverage table.
///
/// OpenType lookups store information about which glyphs are affected by the
/// lookup, as a way to optimize the shaper's operation.
pub struct Coverage {
    /// The glyphs (usually the first glyph in a sequence) affected by this lookup.
    pub glyphs: Vec<uint16>,
}

deserialize_visitor!(
    Coverage,
    CoverageVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let format = read_field!(seq, uint16, "a coverage table format field");
        let glyphs: Vec<uint16> = if format == 1 {
            let cf1 = read_field!(seq, CoverageFormat1, "a coverage table format 1");
            cf1.glyphArray
        } else {
            let cf1 = read_field!(seq, CoverageFormat2, "a coverage table format 1");
            cf1.rangeRecords
                .iter()
                .map(|rr| rr.startGlyphID..rr.endGlyphID + 1)
                .flatten()
                .collect()
        };
        Ok(Coverage { glyphs })
    }
);

fn consecutive_slices(data: &[uint16]) -> Vec<&[uint16]> {
    let mut slice_start = 0;
    let mut result = Vec::new();
    for i in 1..data.len() {
        if data[i - 1] + 1 != data[i] {
            result.push(&data[slice_start..i]);
            slice_start = i;
        }
    }
    if !data.is_empty() {
        result.push(&data[slice_start..]);
    }
    result
}

// TODO: delete when `is_sorted` stablizes: https://github.com/rust-lang/rust/issues/53485
// copied from stdlib
fn is_sorted<T: Ord>(slice: &[T]) -> bool {
    let mut iter = slice.iter();
    let mut prev = match iter.next() {
        Some(x) => x,
        None => return true,
    };
    for next in iter {
        if next < prev {
            return false;
        }
        prev = next;
    }
    true
}

impl Serialize for Coverage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        let as_consecutive = consecutive_slices(&self.glyphs);
        if self.glyphs.is_empty()
            || !is_sorted(&self.glyphs)
            || as_consecutive.len() * 3 >= self.glyphs.len()
        {
            seq.serialize_element::<uint16>(&1)?;
            seq.serialize_element(&CoverageFormat1 {
                glyphArray: self.glyphs.clone(),
            })?;
        } else {
            seq.serialize_element::<uint16>(&2)?;
            let mut index = 0;
            seq.serialize_element(&(as_consecutive.len() as uint16))?;
            for slice in as_consecutive {
                seq.serialize_element(&RangeRecord {
                    startGlyphID: *slice.first().unwrap(),
                    endGlyphID: *slice.last().unwrap(),
                    startCoverageIndex: index,
                })?;
                index += slice.len() as u16;
            }
        }
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cov1_deser() {
        let binary_coverage = vec![
            0x00, 0x01, 0x00, 0x06, 0x00, 0x25, 0x00, 0x64, 0x00, 0xfc, 0x01, 0x53, 0x02, 0xda,
            0x03, 0x02,
        ];
        let expected = Coverage {
            glyphs: vec![37, 100, 252, 339, 730, 770],
        };
        let deserialized: Coverage = otspec::de::from_bytes(&binary_coverage).unwrap();
        assert_eq!(deserialized, expected);
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_coverage);
    }

    #[test]
    fn test_cov2_deser() {
        let binary_coverage = vec![
            0x00, 0x02, 0x00, 0x02, 0x00, 0x05, 0x00, 0x08, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x0e,
            0x00, 0x04,
        ];
        let expected = Coverage {
            glyphs: vec![5, 6, 7, 8, 10, 11, 12, 13, 14],
        };
        let deserialized: Coverage = otspec::de::from_bytes(&binary_coverage).unwrap();
        assert_eq!(deserialized, expected);
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_coverage);
    }
}
