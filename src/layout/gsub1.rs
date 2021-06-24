use crate::layout::coverage::Coverage;
use otspec::types::*;
use otspec::DeserializationError;
use otspec::Deserialize;
use otspec::Deserializer;
use otspec::ReaderContext;
use otspec::SerializationError;
use otspec::Serialize;
use otspec::Serializer;

use otspec_macros::tables;
use std::collections::BTreeMap;

tables!(
  SingleSubstFormat1 {
    Offset16(Coverage) coverage // Offset to Coverage table, from beginning of substitution subtable
    int16 deltaGlyphID  // Add to original glyph ID to get substitute glyph ID
  }
  SingleSubstFormat2 {
    Offset16(Coverage)  coverage  // Offset to Coverage table, from beginning of substitution subtable
    Counted(uint16)  substituteGlyphIDs // Array of substitute glyph IDs — ordered by Coverage index
  }
);

#[derive(Debug, PartialEq, Clone)]
/// A single substitution subtable.
pub struct SingleSubst {
    /// The mapping of input glyph IDs to replacement glyph IDs.
    pub mapping: BTreeMap<uint16, uint16>,
}

// impl ToBytes for SingleSubst {
//     fn to_bytes(&self) -> Vec<u8> {
//         otspec::ser::to_bytes(self).unwrap()
//     }
// }

impl Deserialize for SingleSubst {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let mut mapping = BTreeMap::new();
        let fmt: uint16 = c.de()?;
        match fmt {
            1 => {
                let sub: SingleSubstFormat1 = c.de()?;
                for gid in &sub.coverage.as_ref().unwrap().glyphs {
                    mapping.insert(*gid, (*gid as i16 + sub.deltaGlyphID) as u16);
                }
            }
            2 => {
                let sub: SingleSubstFormat2 = c.de()?;
                for (gid, newgid) in sub
                    .coverage
                    .as_ref()
                    .unwrap()
                    .glyphs
                    .iter()
                    .zip(sub.substituteGlyphIDs.iter())
                {
                    mapping.insert(*gid, *newgid);
                }
            }
            _ => panic!("Better error handling needed"),
        }
        Ok(SingleSubst { mapping })
    }
}

impl Serialize for SingleSubst {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        // Determine format
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
        data.put(format)?;
        let coverage = Coverage {
            glyphs: self.mapping.keys().copied().collect(),
        };
        if format == 1 {
            data.put(SingleSubstFormat1 {
                coverage: Offset16::to(coverage),
                deltaGlyphID: delta,
            })?;
        } else {
            let len = self.mapping.len() as u16;
            data.put(6 + 2 * len)?;
            data.put(len)?;
            for k in self.mapping.values() {
                data.put(k)?;
            }
            data.put(coverage)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    macro_rules! btreemap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }

    #[test]
    fn test_single_subst_1_serde() {
        let subst = SingleSubst {
            mapping: btreemap!(66 => 67, 68 => 69),
        };
        let binary_subst = vec![
            0x00, 0x01, 0x00, 0x04, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 66, 0x00, 68,
        ];
        let serialized = otspec::ser::to_bytes(&subst).unwrap();
        assert_eq!(serialized, binary_subst);
        assert_eq!(
            otspec::de::from_bytes::<SingleSubst>(&binary_subst).unwrap(),
            subst
        );
    }

    #[test]
    fn test_single_subst_2_ser() {
        let subst = SingleSubst {
            mapping: btreemap!(34 => 66, 35 => 66, 36  => 66),
        };
        let binary_subst = vec![
            0x00, 0x02, 0x00, 0x0C, 0x00, 0x03, 0x00, 0x42, 0x00, 0x42, 0x00, 0x42, 0x00, 0x01,
            0x00, 0x03, 0x00, 0x22, 0x00, 0x23, 0x00, 0x24,
        ];
        let serialized = otspec::ser::to_bytes(&subst).unwrap();
        assert_eq!(serialized, binary_subst);
        assert_eq!(
            otspec::de::from_bytes::<SingleSubst>(&binary_subst).unwrap(),
            subst
        );
    }
}
