use crate::layout::coverage::Coverage;

use crate::layout::gsub2::MultipleSubstFormat1;
use crate::layout::gsub2::Sequence;
use otspec::types::*;
use otspec::DeserializationError;
use otspec::Deserialize;
use otspec::Deserializer;
use otspec::ReaderContext;
use otspec::SerializationError;
use otspec::Serialize;

use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Clone)]
/// A alternate substitution (`sub ... from ...`) subtable.
pub struct AlternateSubst {
    /// The mapping of input glyph IDs to array of possible glyph IDs.
    pub mapping: BTreeMap<uint16, Vec<uint16>>,
}

// This is very naughty. AltSubst is the same layout as MultipleSubst, so we
// just pretend it is one.
impl From<&AlternateSubst> for MultipleSubstFormat1 {
    fn from(lookup: &AlternateSubst) -> Self {
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

impl Deserialize for AlternateSubst {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let msf1: MultipleSubstFormat1 = c.de()?;
        let mut mapping = BTreeMap::new();
        for (input, sequence) in msf1
            .coverage
            .link
            .unwrap()
            .glyphs
            .iter()
            .zip(msf1.sequences.0.iter())
        {
            mapping.insert(
                *input,
                sequence.link.as_ref().unwrap().substituteGlyphIDs.clone(),
            );
        }
        Ok(AlternateSubst { mapping })
    }
}

impl Serialize for AlternateSubst {
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
