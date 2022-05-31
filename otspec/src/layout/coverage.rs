use crate::types::*;
use crate::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};

use otspec_macros::tables;

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

// XXX This is still clever

#[derive(Debug, PartialEq, Clone, Default)]
/// A coverage table.
///
/// OpenType lookups store information about which glyphs are affected by the
/// lookup, as a way to optimize the shaper's operation.
pub struct Coverage {
    /// The glyphs (usually the first glyph in a sequence) affected by this lookup.
    pub glyphs: Vec<uint16>,
}

impl Deserialize for Coverage {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let format: uint16 = c.de()?;
        let glyphs: Vec<uint16> = if format == 1 {
            let cf1: CoverageFormat1 = c.de()?;
            cf1.glyphArray
        } else {
            let cf2: CoverageFormat2 = c.de()?;
            cf2.rangeRecords
                .iter()
                .flat_map(|rr| rr.startGlyphID..rr.endGlyphID + 1)
                .collect()
        };
        Ok(Coverage { glyphs })
    }
}

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

impl Coverage {
    fn most_efficient_format(&self) -> u16 {
        let as_consecutive = consecutive_slices(&self.glyphs);
        if self.glyphs.is_empty()
            || !is_sorted(&self.glyphs)
            || as_consecutive.len() * 3 >= self.glyphs.len()
        {
            1
        } else {
            2
        }
    }
}

impl Serialize for Coverage {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let as_consecutive = consecutive_slices(&self.glyphs);
        if self.most_efficient_format() == 1 {
            1_u16.to_bytes(data)?;
            CoverageFormat1 {
                glyphArray: self.glyphs.clone(),
            }
            .to_bytes(data)?
        } else {
            2_u16.to_bytes(data)?;
            let mut index = 0;
            (as_consecutive.len() as uint16).to_bytes(data)?;
            for slice in as_consecutive {
                RangeRecord {
                    startGlyphID: *slice.first().unwrap(),
                    endGlyphID: *slice.last().unwrap(),
                    startCoverageIndex: index,
                }
                .to_bytes(data)?;
                index += slice.len() as u16;
            }
        }
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        2 + if self.most_efficient_format() == 1 {
            2 + 2 * self.glyphs.len()
        } else {
            let as_consecutive = consecutive_slices(&self.glyphs);
            2 + 6 * as_consecutive.len()
        }
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
