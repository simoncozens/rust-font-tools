use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::tables;
use std::collections::BTreeMap;

tables!(
  MultipleSubstFormat1 {
    [offset_base]
    uint16 substFormat
    Offset16(Coverage) coverage
    CountedOffset16(Sequence)  sequences
  }
  Sequence {
    Counted(uint16) substituteGlyphIDs
  }
);

#[derive(Debug, PartialEq, Clone, Default)]
/// A multiple substitution (one-to-many) subtable.
pub struct MultipleSubst {
    /// The mapping of input glyph IDs to sequence of replacement glyph IDs.
    pub mapping: BTreeMap<GlyphID, Vec<GlyphID>>,
}

impl Deserialize for MultipleSubst {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let msf1: MultipleSubstFormat1 = c.de()?;
        let mut mapping = BTreeMap::new();
        for (input, sequence) in msf1
            .coverage
            .link
            .unwrap()
            .glyphs
            .iter()
            .zip(msf1.sequences.v.iter())
        {
            mapping.insert(
                *input,
                sequence.link.as_ref().unwrap().substituteGlyphIDs.clone(),
            );
        }
        Ok(MultipleSubst { mapping })
    }
}

impl From<&MultipleSubst> for MultipleSubstFormat1 {
    fn from(lookup: &MultipleSubst) -> Self {
        let coverage = Offset16::to(Coverage {
            glyphs: lookup.mapping.keys().copied().collect(),
        });
        let mut sequences = vec![];
        for right in lookup.mapping.values() {
            sequences.push(Offset16::to(Sequence {
                substituteGlyphIDs: right.to_vec(),
            }));
        }
        MultipleSubstFormat1 {
            substFormat: 1,
            coverage,
            sequences: sequences.into(),
        }
    }
}
impl Serialize for MultipleSubst {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let i: MultipleSubstFormat1 = self.into();
        i.to_bytes(data)
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
                /* 00 */ 0x00, 0x01, // fmt 1
                /* 02 */ 0x00, 0x0A, // Offset 10 to coverage
                /* 04 */ 0x00, 0x02, // count of sequences
                /* 06 */ 0x00, 0x12, // Offset to seq 1
                /* 08 */ 0x00, 0x18, // Offset to seq 2
                /* 0A */ 0x00, 0x01, // Coverage format 1
                /* 0C */ 0x00, 0x02, // Coverage: Count of glyph ids
                /* 0E */ 0x00, 0x4A, // First glyph ID
                /* 10 */ 0x00, 0x4D, // Second glyph ID
                /* 12 */ 0x00, 0x02, // First seq: Count of gids
                /* 14 */ 0x00, 0x47, // First seq: GID1
                /* 16 */ 0x00, 0x4A, // First seq: GID2
                /* 18 */ 0x00, 0x02, // Second seq: count of gids
                /* 1A */ 0x00, 0x47, // Second seq: GID1
                /* 1C */ 0x00, 0x4D, // Second seq: GID2
            ]
        );
    }

    #[test]
    fn test_mult_subst_de() {
        let subst = MultipleSubst {
            mapping: btreemap!(77 => vec![71,77], 74 => vec![71,74]),
        };
        let binary_subst = vec![
            0x00, 0x01, 0x00, 0x0A, 0x00, 0x02, 0x00, 0x12, 0x00, 0x18, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x4A, 0x00, 0x4D, 0x00, 0x02, 0x00, 0x47, 0x00, 0x4A, 0x00, 0x02, 0x00, 0x47,
            0x00, 0x4D,
        ];

        let deserialized: MultipleSubst = otspec::de::from_bytes(&binary_subst).unwrap();
        assert_eq!(deserialized, subst);
    }
}
