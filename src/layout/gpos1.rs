use crate::layout::coverage::Coverage;
use crate::layout::valuerecord::{ValueRecord, ValueRecords};
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
  SinglePosFormat1 {
    [offset_base]
    uint16 posFormat
    Offset16(Coverage) coverage // Offset to Coverage table, from beginning of positioning subtable
    ValueRecord valueRecord  // Positioning value (includes type)
  }
  SinglePosFormat2 {
    [offset_base]
    uint16 posFormat
    Offset16(Coverage)  coverage  // Offset to Coverage table, from beginning of positioning subtable
    ValueRecords valueRecords // Positioning values (includes type)
  }
);

#[derive(Debug, Clone, PartialEq)]
pub enum SinglePosInternal {
    Format1(SinglePosFormat1),
    Format2(SinglePosFormat2),
}

impl Serialize for SinglePosInternal {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        match self {
            SinglePosInternal::Format1(s) => s.to_bytes(data),
            SinglePosInternal::Format2(s) => s.to_bytes(data),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// A single positioning subtable.
pub struct SinglePos {
    /// The mapping of input glyph IDs to value records.
    pub mapping: BTreeMap<uint16, ValueRecord>,
}

impl SinglePos {
    fn best_format(&self) -> uint16 {
        let vals: Vec<ValueRecord> = self.mapping.values().copied().collect();
        if vals.windows(2).all(|w| w[0] == w[1]) {
            1
        } else {
            2
        }
    }
}

impl Deserialize for SinglePos {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let mut mapping = BTreeMap::new();
        let fmt = c.peek(2)?;
        match fmt {
            [0x00, 0x01] => {
                let pos: SinglePosFormat1 = c.de()?;
                for glyph_id in &pos.coverage.as_ref().unwrap().glyphs {
                    mapping.insert(*glyph_id, pos.valueRecord);
                }
            }
            [0x00, 0x02] => {
                let pos: SinglePosFormat2 = c.de()?;
                for (glyph_id, vr) in pos
                    .coverage
                    .as_ref()
                    .unwrap()
                    .glyphs
                    .iter()
                    .zip(pos.valueRecords.0.iter())
                {
                    mapping.insert(*glyph_id, *vr);
                }
            }
            _ => panic!("Bad single pos format {:?}", fmt),
        }
        Ok(SinglePos { mapping })
    }
}

impl From<&SinglePos> for SinglePosInternal {
    fn from(val: &SinglePos) -> Self {
        let coverage = Coverage {
            glyphs: val.mapping.keys().copied().collect(),
        };
        let format = val.best_format();
        if format == 1 {
            let vr = val.mapping.values().next().unwrap();
            SinglePosInternal::Format1(SinglePosFormat1 {
                posFormat: 1,
                coverage: Offset16::to(coverage),
                valueRecord: *vr,
            })
        } else {
            let vrs = val.mapping.values();
            SinglePosInternal::Format2(SinglePosFormat2 {
                posFormat: 2,
                coverage: Offset16::to(coverage),
                valueRecords: ValueRecords(vrs.copied().collect()),
            })
        }
    }
}

impl Serialize for SinglePos {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let ssi: SinglePosInternal = self.into();
        ssi.to_bytes(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use otspec_macros::Serialize;
    use std::iter::FromIterator;

    macro_rules! btreemap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }

    macro_rules! valuerecord {
        ($($k:ident => $v:expr),* $(,)?) => {{
            let mut v = ValueRecord::new();
            $( v.$k = Some($v); )*
            v
        }};
    }

    #[test]
    fn test_single_pos_1_serde() {
        let pos = SinglePos {
            mapping: btreemap!(66 => valuerecord!(xAdvance=>10)),
        };
        let binary_pos = vec![
            0x00, 0x01, 0x00, 0x08, 0x00, 0x04, 0x00, 0x0a, 0x00, 0x01, 0x00, 0x01, 0x00, 66,
        ];
        let serialized = otspec::ser::to_bytes(&pos).unwrap();
        assert_eq!(serialized, binary_pos);
        let de: SinglePos = otspec::de::from_bytes(&binary_pos).unwrap();
        assert_eq!(de, pos);
    }
}
