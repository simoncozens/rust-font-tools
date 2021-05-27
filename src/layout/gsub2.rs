use crate::layout::coverage::Coverage;
use crate::GSUB::ToBytes;
use otspec::types::*;
use otspec::{deserialize_visitor, read_remainder};
use otspec_macros::tables;
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

tables!(
  MultipleSubstFormat1 {
    uint16 substFormat
    uint16  coverageOffset
    Counted(uint16)  sequenceOffsets
  }
  Sequence {
    Counted(uint16) substituteGlyphIDs
  }
);

#[derive(Debug, PartialEq, Clone)]
/// A multiple substitution (one-to-many) subtable.
pub struct MultipleSubst {
    /// The mapping of input glyph IDs to sequence of replacement glyph IDs.
    pub mapping: BTreeMap<uint16, Vec<uint16>>,
}

impl ToBytes for MultipleSubst {
    fn to_bytes(&self) -> Vec<u8> {
        otspec::ser::to_bytes(self).unwrap()
    }
}

deserialize_visitor!(
    MultipleSubst,
    MultipleSubstDeserializer,
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<MultipleSubst, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let remainder = read_remainder!(seq, "a multiple substitution table");
        let mut mapping = BTreeMap::new();
        let sub: MultipleSubstFormat1 = otspec::de::from_bytes(&remainder).unwrap();
        let coverage: Coverage =
            otspec::de::from_bytes(&remainder[sub.coverageOffset as usize..]).unwrap();
        for (input, seq_offset) in coverage.glyphs.iter().zip(sub.sequenceOffsets.iter()) {
            let sequence: Sequence =
                otspec::de::from_bytes(&remainder[*seq_offset as usize..]).unwrap();
            mapping.insert(*input, sequence.substituteGlyphIDs);
        }
        Ok(MultipleSubst { mapping })
    }
);

impl Serialize for MultipleSubst {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&1_u16)?;

        let coverage = Coverage {
            glyphs: self.mapping.keys().copied().collect(),
        };
        let sequence_count = self.mapping.len() as uint16;
        let mut sequences: BTreeMap<Vec<uint16>, uint16> = BTreeMap::new();
        let mut offsets: Vec<uint16> = vec![];
        let mut seq_offset = 6 + sequence_count * 2;
        let serialized_cov = otspec::ser::to_bytes(&coverage).unwrap();
        seq.serialize_element(&seq_offset)?;
        seq_offset += serialized_cov.len() as uint16;

        let mut sequences_ser: Vec<u8> = vec![];
        for right in self.mapping.values() {
            if sequences.contains_key(right) {
                offsets.push(*sequences.get(right).unwrap());
            } else {
                let sequence = Sequence {
                    substituteGlyphIDs: right.to_vec(),
                };
                let serialized = otspec::ser::to_bytes(&sequence).unwrap();
                sequences.insert(right.to_vec(), seq_offset);
                offsets.push(seq_offset);
                seq_offset += serialized.len() as u16;
                sequences_ser.extend(serialized);
            }
        }
        seq.serialize_element(&sequence_count)?;
        seq.serialize_element(&offsets)?;
        seq.serialize_element(&coverage)?;
        seq.serialize_element(&sequences_ser)?;
        seq.end()
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
    fn test_mult_subst_ser() {
        let subst = MultipleSubst {
            mapping: btreemap!(77 => vec![71,77], 74 => vec![71,74]),
        };
        let serialized = otspec::ser::to_bytes(&subst).unwrap();
        assert_eq!(
            serialized,
            vec![
                0x00, 0x01, 0x00, 0x0A, 0x00, 0x02, 0x00, 0x12, 0x00, 0x18, 0x00, 0x01, 0x00, 0x02,
                0x00, 0x4A, 0x00, 0x4D, 0x00, 0x02, 0x00, 0x47, 0x00, 0x4A, 0x00, 0x02, 0x00, 0x47,
                0x00, 0x4D
            ]
        );
    }
}
