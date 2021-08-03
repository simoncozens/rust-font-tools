use crate::layout::coverage::Coverage;
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};
use otspec_macros::tables;
use std::collections::BTreeMap;

tables!(
  LigatureSubstFormat1 {
    [offset_base]
    uint16 substFormat
    Offset16(Coverage) coverage
    CountedOffset16(LigatureSet)  ligatureSet
  }
  LigatureSet {
    [offset_base]
    CountedOffset16(Ligature) ligatureOffsets
  }
);

// We can't use the magic tables here because the component count is the array
// length MINUS ONE.
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, PartialEq, Clone)]
pub struct Ligature {
    ligatureGlyph: uint16,
    componentGlyphIDs: Vec<uint16>,
}

#[derive(Debug, PartialEq, Clone, Default)]
/// A ligature substitution (many-to-one) subtable.
pub struct LigatureSubst {
    /// The mapping of sequences of input glyphs IDs to replacement glyph IDs.
    pub mapping: BTreeMap<Vec<uint16>, uint16>,
}

impl From<&LigatureSubst> for LigatureSubstFormat1 {
    fn from(ls: &LigatureSubst) -> Self {
        let mut split_map: BTreeMap<u16, Vec<Vec<u16>>> = BTreeMap::new();
        for left in ls.mapping.keys() {
            let covered = left.first().unwrap();
            split_map
                .entry(*covered)
                .or_insert_with(std::vec::Vec::new)
                .push(left.clone());
        }
        // println!("Split map {:?}", split_map);

        let coverage = Coverage {
            glyphs: split_map.keys().copied().collect(),
        };
        let mut ligature_sets: Vec<Offset16<LigatureSet>> = vec![];
        for first in &coverage.glyphs {
            // println!("For covered glyph {:?}", first);
            let relevant_keys = split_map.get(&first).unwrap();
            let ligature_offsets: Vec<Offset16<Ligature>> = relevant_keys
                .iter()
                .map(|k| {
                    Offset16::to(Ligature {
                        ligatureGlyph: *ls.mapping.get(k).unwrap(),
                        componentGlyphIDs: k[1..].to_vec(),
                    })
                })
                .collect();

            let ls = LigatureSet {
                ligatureOffsets: ligature_offsets.into(),
            };
            ligature_sets.push(Offset16::to(ls));
        }
        LigatureSubstFormat1 {
            substFormat: 1,
            coverage: Offset16::to(coverage),
            ligatureSet: VecOffset16(ligature_sets),
        }
    }
}

impl From<LigatureSubstFormat1> for LigatureSubst {
    fn from(lsf1: LigatureSubstFormat1) -> Self {
        let mut mapping = BTreeMap::new();
        for (input, lig_set) in lsf1
            .coverage
            .link
            .unwrap()
            .glyphs
            .iter()
            .zip(lsf1.ligatureSet.0.iter())
        {
            for ligature in lig_set.link.as_ref().unwrap().ligatureOffsets.0.iter() {
                let ligature = ligature.link.as_ref().unwrap();
                let mut input_sequence: Vec<u16> = vec![*input];
                input_sequence.extend(ligature.componentGlyphIDs.clone());
                mapping.insert(input_sequence, ligature.ligatureGlyph);
            }
        }
        LigatureSubst { mapping }
    }
}

impl Serialize for LigatureSubst {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let lsf1: LigatureSubstFormat1 = self.into();
        lsf1.to_bytes(data)
    }
}

impl Deserialize for LigatureSubst {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let lsf1: LigatureSubstFormat1 = c.de()?;
        Ok(lsf1.into())
    }
}

impl Serialize for Ligature {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        data.put(self.ligatureGlyph)?;
        data.put(self.componentGlyphIDs.len() as uint16 + 1)?;
        data.put(&self.componentGlyphIDs)
    }
}

impl Deserialize for Ligature {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let ligature_glyph: uint16 = c.de()?;
        let component_count: uint16 = c.de()?;
        let components: Vec<uint16> = c.de_counted(component_count as usize - 1)?;
        Ok(Ligature {
            ligatureGlyph: ligature_glyph,
            componentGlyphIDs: components,
        })
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
    fn test_ligature_ser() {
        let subst = LigatureSubst {
            mapping: btreemap!(
                vec![ 10, 20, 30] => 11,
                vec![ 10, 20, 31] => 12,
                vec![ 20, 30] => 21,
                vec![ 20, 40, 50] => 22,
            ),
        };
        let serialized = otspec::ser::to_bytes(&subst).unwrap();
        assert_eq!(
            serialized,
            vec![
                0, 1, 0, 10, 0, 2, 0, 18, 0, 40, 0, 1, 0, 2, 0, 10, 0, 20, 0, 2, 0, 6, 0, 14, 0,
                11, 0, 3, 0, 20, 0, 30, 0, 12, 0, 3, 0, 20, 0, 31, 0, 2, 0, 6, 0, 12, 0, 21, 0, 2,
                0, 30, 0, 22, 0, 3, 0, 40, 0, 50
            ]
        );
    }

    #[test]
    fn test_ligature_de() {
        let expected = LigatureSubst {
            mapping: btreemap!(
                vec![ 10, 20, 30] => 11,
                vec![ 10, 20, 31] => 12,
                vec![ 20, 30] => 21,
                vec![ 20, 40, 50] => 22,
            ),
        };
        let binary_lig = vec![
            0, 1, // subst format
            0, 10, // coverage offset
            0, 2, // ligature set count
            0, 18, // ligature set offset (0)
            0, 40, // ligature set offset (1)
            0, 1, 0, 2, 0, 10, 0, 20, 0, 2, 0, 6, 0, 14, 0, 11, 0, 3, 0, 20, 0, 30, 0, 12, 0, 3, 0,
            20, 0, 31, 0, 2, 0, 6, 0, 12, 0, 21, 0, 2, 0, 30, 0, 22, 0, 3, 0, 40, 0, 50,
        ];
        let deserialized: LigatureSubst = otspec::de::from_bytes(&binary_lig).unwrap();
        assert_eq!(deserialized, expected);
    }
}
