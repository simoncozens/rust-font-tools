use crate::layout::coverage::Coverage;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_field_counted, read_remainder};
use otspec_macros::tables;
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

tables!(
  LigatureSubstFormat1 {
    uint16 substFormat
    uint16  coverageOffset
    Counted(uint16)  ligatureSetOffsets
  }
  LigatureSet {
    Counted(uint16) ligatureOffsets
  }
);

#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, PartialEq)]
struct Ligature {
    ligatureGlyph: uint16,
    componentGlyphIDs: Vec<uint16>,
}

#[derive(Debug, PartialEq)]
/// A ligature substitution (many-to-one) subtable.
pub struct LigatureSubst {
    /// The mapping of sequences of input glyphs IDs to replacement glyph IDs.
    pub mapping: BTreeMap<Vec<uint16>, uint16>,
}

deserialize_visitor!(
    LigatureSubst,
    LigatureSubstDeserializer,
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<LigatureSubst, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let remainder = read_remainder!(seq, "a ligature substitution table");
        let mut mapping = BTreeMap::new();
        let sub: LigatureSubstFormat1 = otspec::de::from_bytes(&remainder).unwrap();
        let coverage: Coverage =
            otspec::de::from_bytes(&remainder[sub.coverageOffset as usize..]).unwrap();
        for (input, lig_set_offset) in coverage.glyphs.iter().zip(sub.ligatureSetOffsets.iter()) {
            let lig_set: LigatureSet =
                otspec::de::from_bytes(&remainder[*lig_set_offset as usize..]).unwrap();
            for lig_off in lig_set.ligatureOffsets {
                let ligature: Ligature =
                    otspec::de::from_bytes(&remainder[(lig_set_offset + lig_off) as usize..])
                        .unwrap();
                let mut input_sequence: Vec<u16> = vec![*input];
                input_sequence.extend(ligature.componentGlyphIDs);
                mapping.insert(input_sequence, ligature.ligatureGlyph);
            }
        }
        Ok(LigatureSubst { mapping })
    }
);

deserialize_visitor!(
    Ligature,
    LigatureDeserializer,
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Ligature, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let lig_glyph = read_field!(seq, uint16, "a ligature glyph");
        let comp_count = read_field!(seq, uint16, "a component count");
        if comp_count < 1 {
            return Err(serde::de::Error::custom("Overflow in ligature component"));
        }
        let component_glyph_ids: Vec<uint16> =
            read_field_counted!(seq, comp_count - 1, "component glyph IDs");
        Ok(Ligature {
            ligatureGlyph: lig_glyph,
            componentGlyphIDs: component_glyph_ids,
        })
    }
);

impl Serialize for Ligature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.ligatureGlyph)?;
        seq.serialize_element(&(self.componentGlyphIDs.len() as uint16 + 1))?;
        seq.serialize_element(&self.componentGlyphIDs)?;
        seq.end()
    }
}

impl Serialize for LigatureSubst {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&1_u16)?;

        // Split the map by covered first glyph
        let mut split_map: BTreeMap<u16, Vec<Vec<u16>>> = BTreeMap::new();
        for left in self.mapping.keys() {
            let covered = left.first().unwrap();
            split_map
                .entry(*covered)
                .or_insert_with(std::vec::Vec::new)
                .push(left.clone());
        }
        println!("Split map {:?}", split_map);

        let coverage = Coverage {
            glyphs: split_map.keys().copied().collect(),
        };
        let ligature_set_count = coverage.glyphs.len() as uint16;
        println!("Ligature set count = {:?}", ligature_set_count);
        let mut offsets: Vec<uint16> = vec![];
        let mut seq_offset = 6 + ligature_set_count * 2;
        let serialized_cov = otspec::ser::to_bytes(&coverage).unwrap();
        println!("Offset to coverage = {:?}", seq_offset);
        seq.serialize_element(&seq_offset)?;
        seq_offset += serialized_cov.len() as uint16;
        let mut output: Vec<u8> = vec![];

        for first in &coverage.glyphs {
            println!("For covered glyph {:?}", first);
            println!("Offset: {:?}", seq_offset + output.len() as u16);
            offsets.push(seq_offset + output.len() as u16);
            let mut ls = LigatureSet {
                ligatureOffsets: vec![],
            };
            let relevant_keys = split_map.get(&first).unwrap();
            let ligatures: Vec<Ligature> = relevant_keys
                .iter()
                .map(|k| Ligature {
                    ligatureGlyph: *self.mapping.get(k).unwrap(),
                    componentGlyphIDs: k[1..].to_vec(),
                })
                .collect();
            println!("  Ligatures: {:?}", ligatures);
            let mut offset = 2 + 2 * ligatures.len();
            let mut serialized_ligatures: Vec<u8> = vec![];
            for liga in ligatures {
                ls.ligatureOffsets.push(offset as u16);
                let this = otspec::ser::to_bytes(&liga).unwrap();
                offset += this.len();
                serialized_ligatures.extend(this);
            }
            println!("  Ligature set: {:?}", ls);
            output.extend(otspec::ser::to_bytes(&ls).unwrap());
            println!(
                "   Serialized ligature set {:?}",
                otspec::ser::to_bytes(&ls).unwrap()
            );
            println!("   Serialized Ligatures {:?}", serialized_ligatures);
            output.extend(serialized_ligatures);
        }
        seq.serialize_element(&ligature_set_count)?;
        seq.serialize_element(&offsets)?;
        seq.serialize_element(&coverage)?;
        seq.serialize_element(&output)?;
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
}
