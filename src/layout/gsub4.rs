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
        let component_glyph_ids: Vec<uint16> =
            read_field_counted!(seq, comp_count - 1, "component glyph IDs");
        Ok(Ligature {
            ligatureGlyph: lig_glyph,
            componentGlyphIDs: component_glyph_ids,
        })
    }
);

// impl Serialize for LigatureSubst {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut seq = serializer.serialize_seq(None)?;
//         seq.serialize_element(&1_u16)?;

//         let coverage = Coverage {
//             glyphs: self.mapping.keys().map(|s| *(s.first().unwrap())).collect(),
//         };
//         let sequence_count = self.mapping.len() as uint16;
//         let mut sequences: BTreeMap<Vec<uint16>, uint16> = BTreeMap::new();
//         let mut offsets: Vec<uint16> = vec![];
//         let mut seq_offset = 6 + sequence_count * 2;
//         let serialized_cov = otspec::ser::to_bytes(&coverage).unwrap();
//         seq.serialize_element(&seq_offset)?;
//         seq_offset += serialized_cov.len() as uint16;

//         let mut sequences_ser: Vec<u8> = vec![];
//         for right in self.mapping.values() {
//             if sequences.contains_key(right) {
//                 offsets.push(*sequences.get(right).unwrap());
//             } else {
//                 let sequence = Sequence {
//                     substituteGlyphIDs: right.to_vec(),
//                 };
//                 let serialized = otspec::ser::to_bytes(&sequence).unwrap();
//                 sequences.insert(right.to_vec(), seq_offset);
//                 offsets.push(seq_offset);
//                 seq_offset += serialized.len() as u16;
//                 sequences_ser.extend(serialized);
//             }
//         }
//         seq.serialize_element(&sequence_count)?;
//         seq.serialize_element(&offsets)?;
//         seq.serialize_element(&coverage)?;
//         seq.serialize_element(&sequences_ser)?;
//         seq.end()
//     }
// }
