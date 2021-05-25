use crate::layout::coverage::Coverage;
use crate::layout::gsub2::{MultipleSubstFormat1, Sequence};
use otspec::types::*;
use otspec::{deserialize_visitor, read_remainder};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
/// A alternate substitution (`sub ... from ...`) subtable.
pub struct AlternateSubst {
    /// The mapping of input glyph IDs to array of possible glyph IDs.
    pub mapping: BTreeMap<uint16, Vec<uint16>>,
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
