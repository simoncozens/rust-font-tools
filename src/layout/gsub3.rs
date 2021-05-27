use crate::layout::coverage::Coverage;
use crate::layout::gsub2::{MultipleSubstFormat1, Sequence};
use crate::GSUB::ToBytes;
use otspec::types::*;
use otspec::{deserialize_visitor, read_remainder};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Clone)]
/// A alternate substitution (`sub ... from ...`) subtable.
pub struct AlternateSubst {
    /// The mapping of input glyph IDs to array of possible glyph IDs.
    pub mapping: BTreeMap<uint16, Vec<uint16>>,
}

impl ToBytes for AlternateSubst {
    fn to_bytes(&self) -> Vec<u8> {
        otspec::ser::to_bytes(self).unwrap()
    }
}
deserialize_visitor!(
    AlternateSubst,
    AlternateSubstDeserializer,
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<AlternateSubst, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut mapping = BTreeMap::new();
        let remainder = read_remainder!(seq, "a multiple substitution table");
        // Slightly naughty here, repurposing the fact that mult subst and
        // alt subst have the same layout, just differ in lookupType
        let sub: MultipleSubstFormat1 = otspec::de::from_bytes(&remainder).unwrap();
        let coverage: Coverage =
            otspec::de::from_bytes(&remainder[sub.coverageOffset as usize..]).unwrap();
        for (input, seq_offset) in coverage.glyphs.iter().zip(sub.sequenceOffsets.iter()) {
            let sequence: Sequence =
                otspec::de::from_bytes(&remainder[*seq_offset as usize..]).unwrap();
            mapping.insert(*input, sequence.substituteGlyphIDs);
        }
        Ok(AlternateSubst { mapping })
    }
);

impl Serialize for AlternateSubst {
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
        let subst = AlternateSubst {
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
