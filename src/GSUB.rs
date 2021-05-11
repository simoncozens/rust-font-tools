use crate::layout::common::*;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_remainder};
use otspec_macros::tables;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Deserializer;
use serde::Serializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

tables!(
  gsubcore {
    uint16  majorVersion            // Major version of the GSUB table
    uint16  minorVersion            // Minor version of the GSUB table
    uint16  scriptListOffset        // Offset to ScriptList table, from beginning of GSUB table
    uint16  featureListOffset       // Offset to FeatureList table, from beginning of GSUB table
    uint16  lookupListOffset        // Offset to LookupList table, from beginning of GSUB table
  }
);

#[derive(Debug, PartialEq)]
pub enum Substitution {
    Single,
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
    pub lookups: Vec<Substitution>,
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
        Ok(GSUB {
            lookups: vec![],
            features,
        })
    }
);

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
            lookups: vec![],
            features: hashmap!(*b"liga" => (vec![0],None)),
        };
        let deserialized: GSUB = otspec::de::from_bytes(&binary_gsub).unwrap();
        assert_eq!(deserialized, expected);
    }
}
