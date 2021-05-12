use crate::layout::common::*;
use crate::layout::coverage::Coverage;
use otspec::error::Error;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_remainder, stateful_deserializer};
use otspec_macros::tables;
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Deserializer;
use serde::Serializer;
use serde::{Deserialize, Serialize};
use std::array::TryFromSliceError;
use std::collections::HashMap;
use std::convert::TryInto;

tables!(
  gsubcore {
    uint16  majorVersion            // Major version of the GSUB table
    uint16  minorVersion            // Minor version of the GSUB table
    uint16  scriptListOffset        // Offset to ScriptList table, from beginning of GSUB table
    uint16  featureListOffset       // Offset to FeatureList table, from beginning of GSUB table
    uint16  lookupListOffset        // Offset to LookupList table, from beginning of GSUB table
  }
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
pub struct SubstLookup {
    pub flags: LookupFlags,
    pub mark_filtering_set: Option<uint16>,
    pub substitution: Substitution,
}

#[derive(Debug, PartialEq)]
pub struct SingleSubst {
    pub mapping: HashMap<uint16, uint16>,
}

#[derive(Debug, PartialEq)]
pub enum Substitution {
    Single(SingleSubst),
    Multiple,
    Alternate,
    Ligature,
    Contextual,
    ChainedContextual,
    Extension,
    ReverseChaining,
}

#[derive(Debug, PartialEq)]
pub struct GSUB {
    pub lookups: Vec<SubstLookup>,
    pub features: HashMap<Tag, (Vec<usize>, Option<FeatureParams>)>,
}

deserialize_visitor!(
    GSUB,
    GSUBVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let core = read_field!(seq, gsubcore, "a GSUB table header");
        let mut header_size = 10;
        if core.minorVersion == 1 {
            let featureVariationsOffset =
                read_field!(seq, uint16, "A feature variations table offset");
            header_size += 2;
        }
        let remainder = read_remainder!(seq, "a GSUB table");

        // Feature list
        let mut features = HashMap::new();
        let beginning_of_featurelist = core.featureListOffset as usize - header_size;
        let featurelist: FeatureList =
            otspec::de::from_bytes(&remainder[beginning_of_featurelist..]).unwrap();
        for f in featurelist.featureRecords {
            let tag = f.featureTag;
            let offset = f.featureOffset as usize;
            let feature_table: FeatureTable =
                otspec::de::from_bytes(&remainder[beginning_of_featurelist + offset..]).unwrap();
            let indices = feature_table
                .lookupListIndices
                .iter()
                .map(|x| *x as usize)
                .collect();
            if feature_table.featureParamsOffset != 0 {
                unimplemented!()
            }
            features.insert(tag, (indices, None));
        }

        // Lookup list
        let beginning_of_lookuplist = core.lookupListOffset as usize - header_size;
        let lookuplist: LookupList =
            otspec::de::from_bytes(&remainder[beginning_of_lookuplist..]).unwrap();
        let mut lookups: Vec<SubstLookup> = vec![];
        for offset in lookuplist.lookupOffsets {
            let beginning_of_lookup_table = beginning_of_lookuplist + (offset as usize);
            let lookup: SubstLookup =
                otspec::de::from_bytes(&remainder[beginning_of_lookup_table..]).unwrap();
            lookups.push(lookup);
        }

        Ok(GSUB { lookups, features })
    }
);

fn peek_format(d: &[u8], off: usize) -> Result<uint16, TryFromSliceError> {
    Ok(u16::from_be_bytes(d[off..off + 2].try_into()?))
}

deserialize_visitor!(
    SubstLookup,
    SubstLookupVisitor,
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<SubstLookup, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let lookup = read_field!(seq, Lookup, "A lookup table");
        let mut header_size = 6 + lookup.subtableOffsets.len() * 2;
        let mark_filtering_set = if lookup
            .lookupFlag
            .contains(LookupFlags::USE_MARK_FILTERING_SET)
        {
            header_size += 2;
            Some(read_field!(seq, uint16, "Mark filtering set"))
        } else {
            None
        };
        let remainder = read_remainder!(seq, "a substitution lookup");
        let subtable_offsets: Vec<usize> = lookup
            .subtableOffsets
            .iter()
            .map(|x| *x as usize - header_size)
            .collect();

        let mut de = otspec::de::Deserializer::from_bytes(&remainder);
        let cs = match lookup.lookupType {
            1 => SingleSubstDeserializer { subtable_offsets },
            _ => unimplemented!(),
        };

        let substitution = cs.deserialize(&mut de).unwrap();

        Ok(SubstLookup {
            substitution,
            flags: lookup.lookupFlag,
            mark_filtering_set,
        })
    }
);

stateful_deserializer!(
Substitution,
SingleSubstDeserializer,
{
    subtable_offsets: Vec<usize>
},
fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Substitution, A::Error>
where
    A: SeqAccess<'de>,
{
    let mut mapping = HashMap::new();
    let remainder = read_remainder!(seq, "a single substitution table");
    for off in self.subtable_offsets {
        match peek_format(&remainder, off) {
            Ok(1) => {
                let sub: SingleSubstFormat1 =
                    otspec::de::from_bytes(&remainder[off + 2..]).unwrap();
                let coverage: Coverage =
                    otspec::de::from_bytes(&remainder[off + sub.coverageOffset as usize..])
                        .unwrap();
                for gid in &coverage.glyphs {
                    mapping.insert(*gid, (*gid as i16 + sub.deltaGlyphID) as u16);
                }
            }
            Ok(2) => {
                let sub: SingleSubstFormat2 =
                    otspec::de::from_bytes(&remainder[off + 2..]).unwrap();
                let coverage: Coverage =
                    otspec::de::from_bytes(&remainder[off + sub.coverageOffset as usize..])
                        .unwrap();
                for (gid, newgid) in coverage.glyphs.iter().zip(sub.substituteGlyphIDs.iter()) {
                    mapping.insert(*gid, *newgid);
                }
            }
            _ => panic!("Better error handling needed"),
        }
    }
    Ok(Substitution::Single(SingleSubst { mapping }))
});

#[cfg(test)]
mod tests {
    use super::*;

    use std::iter::FromIterator;

    macro_rules! hashmap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::HashMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }

    #[test]
    fn test_simple_gsub_de() {
        /* languagesystem DFLT dflt;
          feature liga { sub a by b; } liga;
        */
        let binary_gsub = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x6c, 0x69, 0x67, 0x61, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x06, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x42,
        ];
        let expected = GSUB {
            lookups: vec![SubstLookup {
                flags: LookupFlags::empty(),
                mark_filtering_set: None,
                substitution: Substitution::Single(SingleSubst {
                    mapping: hashmap!(66 => 67),
                }),
            }],
            features: hashmap!(*b"liga" => (vec![0],None)),
        };
        let deserialized: GSUB = otspec::de::from_bytes(&binary_gsub).unwrap();
        assert_eq!(deserialized, expected);
    }
}
