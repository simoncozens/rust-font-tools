use crate::layout::coverage::Coverage;
use crate::GSUB::peek_format;
use otspec::types::*;
use otspec::{deserialize_visitor, read_remainder};
use otspec_macros::tables;
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

tables!(
  SingleSubstFormat1 {
    uint16 coverageOffset // Offset to Coverage table, from beginning of substitution subtable
    int16 deltaGlyphID  // Add to original glyph ID to get substitute glyph ID
  }
  SingleSubstFormat2 {
    uint16  coverageOffset  // Offset to Coverage table, from beginning of substitution subtable
    Counted(uint16)  substituteGlyphIDs // Array of substitute glyph IDs — ordered by Coverage index
  }
);

#[derive(Debug, PartialEq)]
/// A single substitution subtable.
pub struct SingleSubst {
    /// The mapping of input glyph IDs to replacement glyph IDs.
    pub mapping: BTreeMap<uint16, uint16>,
}

deserialize_visitor!(
    SingleSubst,
    SingleSubstDeserializer,
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<SingleSubst, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut mapping = BTreeMap::new();
        let remainder = read_remainder!(seq, "a single substitution table");
        match peek_format(&remainder, 0) {
            Ok(1) => {
                let sub: SingleSubstFormat1 = otspec::de::from_bytes(&remainder[2..]).unwrap();
                let coverage: Coverage =
                    otspec::de::from_bytes(&remainder[sub.coverageOffset as usize..]).unwrap();
                for gid in &coverage.glyphs {
                    mapping.insert(*gid, (*gid as i16 + sub.deltaGlyphID) as u16);
                }
            }
            Ok(2) => {
                let sub: SingleSubstFormat2 = otspec::de::from_bytes(&remainder[2..]).unwrap();
                let coverage: Coverage =
                    otspec::de::from_bytes(&remainder[sub.coverageOffset as usize..]).unwrap();
                for (gid, newgid) in coverage.glyphs.iter().zip(sub.substituteGlyphIDs.iter()) {
                    mapping.insert(*gid, *newgid);
                }
            }
            _ => panic!("Better error handling needed"),
        }
        Ok(SingleSubst { mapping })
    }
);

impl Serialize for SingleSubst {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Determine format
        let mut delta = 0_u16;
        let mut map = self.mapping.iter();
        let format: u16 = if let Some((&first_left, &first_right)) = map.next() {
            delta = first_right.wrapping_sub(first_left);
            let mut format = 1;
            for (&left, &right) in map {
                if left.wrapping_add(delta) != right {
                    format = 2;
                    break;
                }
            }
            format
        } else {
            2
        };
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&format)?;
        if format == 1 {
            seq.serialize_element(&6_u16)?;
            seq.serialize_element(&delta)?;
        } else {
            let len = self.mapping.len() as u16;
            seq.serialize_element(&(6 + 2 * len))?;
            seq.serialize_element(&len)?;
            for k in self.mapping.values() {
                seq.serialize_element(k)?;
            }
        }
        seq.serialize_element(&Coverage {
            glyphs: self.mapping.keys().copied().collect(),
        })?;
        seq.end()
    }
}
