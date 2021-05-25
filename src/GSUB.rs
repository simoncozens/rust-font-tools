use crate::layout::common::*;
use crate::layout::coverage::Coverage;
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
use std::collections::BTreeMap;
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
  MultipleSubstFormat1 {
    uint16 substFormat
    uint16  coverageOffset
    Counted(uint16)  sequenceOffsets
  }
  Sequence {
    Counted(uint16) substituteGlyphIDs
  }
);

/// A general substitution lookup rule, of whatever type
#[derive(Debug, PartialEq)]
pub struct SubstLookup {
    /// Lookup flags
    pub flags: LookupFlags,
    /// The mark filtering set index in the `GDEF` table.
    pub mark_filtering_set: Option<uint16>,
    /// The concrete substitution rule.
    pub substitution: Substitution,
}

#[derive(Debug, PartialEq)]
/// A single substitution subtable.
pub struct SingleSubst {
    /// The mapping of input glyph IDs to replacement glyph IDs.
    pub mapping: BTreeMap<uint16, uint16>,
}

#[derive(Debug, PartialEq)]
/// A multiple substitution (one-to-many) subtable.
pub struct MultipleSubst {
    /// The mapping of input glyph IDs to sequence of replacement glyph IDs.
    pub mapping: BTreeMap<uint16, Vec<uint16>>,
}

#[derive(Debug, PartialEq)]
/// A alternate substitution (`sub ... from ...`) subtable.
pub struct AlternateSubst {
    /// The mapping of input glyph IDs to array of possible glyph IDs.
    pub mapping: BTreeMap<uint16, Vec<uint16>>,
}

/// A container which represents a generic substitution rule
///
/// Each rule is expressed as a vector of subtables.
#[derive(Debug, PartialEq)]
pub enum Substitution {
    /// Contains a single substitution rule.
    Single(Vec<SingleSubst>),
    /// Contains a multiple substitution rule.
    Multiple(Vec<MultipleSubst>),
    /// Contains an alternate substitution rule.
    Alternate(Vec<AlternateSubst>),
    /// Contains a ligature substitution rule.
    Ligature,
    /// Contains a contextual substitution rule.
    Contextual,
    /// Contains a chained contextual substitution rule.
    ChainedContextual,
    /// Contains an extension subtable.
    Extension,
    /// Contains a reverse chaining single substitution rule.
    ReverseChaining,
}

#[derive(Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
/// The Glyph Substitution table
pub struct GSUB {
    /// A list of substitution lookups
    pub lookups: Vec<SubstLookup>,
    /// A mapping between script tags and `Script` tables.
    pub scripts: ScriptList,
    /// The association between feature tags and the list of indices into the
    /// lookup table used to process this feature, together with any feature parameters.
    pub features: Vec<(Tag, Vec<usize>, Option<FeatureParams>)>,
}

deserialize_visitor!(
    GSUB,
    GSUBVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let core = read_field!(seq, gsubcore, "a GSUB table header");
        let mut header_size = 10;
        if core.minorVersion == 1 {
            let _featureVariationsOffset =
                read_field!(seq, uint16, "A feature variations table offset");
            header_size += 2;
        }
        let remainder = read_remainder!(seq, "a GSUB table");

        // Script list
        let beginning_of_scriptlist = core.scriptListOffset as usize - header_size;
        let scripts: ScriptList =
            otspec::de::from_bytes(&remainder[beginning_of_scriptlist..]).unwrap();

        // Feature list
        let mut features = vec![];
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
            features.push((tag, indices, None));
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

        Ok(GSUB {
            lookups,
            scripts,
            features,
        })
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
        let subtable_offsets = lookup
            .subtableOffsets
            .iter()
            .map(|x| *x as usize - header_size);

        let substitution = match lookup.lookupType {
            1 => Substitution::Single(
                subtable_offsets
                    .map(|off| {
                        let subtable_bin = &remainder[off..];
                        otspec::de::from_bytes::<SingleSubst>(subtable_bin).unwrap()
                    })
                    .collect(),
            ),
            2 => Substitution::Multiple(
                subtable_offsets
                    .map(|off| {
                        let subtable_bin = &remainder[off..];
                        otspec::de::from_bytes::<MultipleSubst>(subtable_bin).unwrap()
                    })
                    .collect(),
            ),
            3 => Substitution::Alternate(
                subtable_offsets
                    .map(|off| {
                        let subtable_bin = &remainder[off..];
                        otspec::de::from_bytes::<AlternateSubst>(subtable_bin).unwrap()
                    })
                    .collect(),
            ),
            _ => unimplemented!(),
        };

        Ok(SubstLookup {
            substitution,
            flags: lookup.lookupFlag,
            mark_filtering_set,
        })
    }
);

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

deserialize_visitor!(
    MultipleSubst,
    MultipleSubstDeserializer,
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<MultipleSubst, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let remainder = read_remainder!(seq, "a multiple substitution table");
        let mut mapping = BTreeMap::new();
        let sub: MultipleSubstFormat1 = otspec::de::from_bytes(&remainder).unwrap();
        let coverage: Coverage =
            otspec::de::from_bytes(&remainder[sub.coverageOffset as usize..]).unwrap();
        for (input, seq_offset) in coverage.glyphs.iter().zip(sub.sequenceOffsets.iter()) {
            let sequence: Sequence =
                otspec::de::from_bytes(&remainder[*seq_offset as usize..]).unwrap();
            mapping.insert(*input, sequence.substituteGlyphIDs);
        }
        Ok(MultipleSubst { mapping })
    }
);

impl Serialize for MultipleSubst {
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;
    use std::iter::FromIterator;

    macro_rules! hashmap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::HashMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }

    macro_rules! btreemap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }
    #[test]
    fn test_simple_gsub_de() {
        /* languagesystem DFLT dflt;
           lookup ssf1 { sub a by b; sub c by d; } ssf1;
           lookup ssf2 { sub A by a; sub B by a; sub C by a; } ssf2;
           lookup mult { sub i by f i; sub l by f l; } mult;
           lookup aalt {sub a from [b c d]; } aalt;

           feature sing { lookup ssf1; lookup ssf2; } sing;
           feature mult { lookup mult; } mult;
           feature alte { lookup aalt; } alte;
        */
        let binary_gsub = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x22, 0x00, 0x4a, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x03,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x61, 0x6c, 0x74, 0x65, 0x00, 0x14,
            0x6d, 0x75, 0x6c, 0x74, 0x00, 0x1a, 0x73, 0x69, 0x6e, 0x67, 0x00, 0x20, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x0a, 0x00, 0x12, 0x00, 0x1a, 0x00, 0x22,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x1e, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x22, 0x00, 0x03, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x30, 0x00, 0x01, 0x00, 0x38, 0x00, 0x01, 0x00, 0x02, 0x00, 0x3a,
            0x00, 0x03, 0x00, 0x42, 0x00, 0x42, 0x00, 0x42, 0x00, 0x01, 0x00, 0x38, 0x00, 0x02,
            0x00, 0x0a, 0x00, 0x10, 0x00, 0x02, 0x00, 0x47, 0x00, 0x4a, 0x00, 0x02, 0x00, 0x47,
            0x00, 0x4d, 0x00, 0x01, 0x00, 0x2a, 0x00, 0x01, 0x00, 0x08, 0x00, 0x03, 0x00, 0x43,
            0x00, 0x44, 0x00, 0x45, 0x00, 0x01, 0x00, 0x02, 0x00, 0x42, 0x00, 0x44, 0x00, 0x02,
            0x00, 0x01, 0x00, 0x22, 0x00, 0x24, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x4a,
            0x00, 0x4d, 0x00, 0x01, 0x00, 0x01, 0x00, 0x42,
        ];
        let expected = GSUB {
            lookups: vec![
                SubstLookup {
                    flags: LookupFlags::empty(),
                    mark_filtering_set: None,
                    substitution: Substitution::Single(vec![SingleSubst {
                        mapping: btreemap!(66 => 67, 68 => 69),
                    }]),
                },
                SubstLookup {
                    flags: LookupFlags::empty(),
                    mark_filtering_set: None,
                    substitution: Substitution::Single(vec![SingleSubst {
                        mapping: btreemap!(34 => 66, 35 => 66, 36  => 66),
                    }]),
                },
                SubstLookup {
                    flags: LookupFlags::empty(),
                    mark_filtering_set: None,
                    substitution: Substitution::Multiple(vec![MultipleSubst {
                        mapping: btreemap!(77 => vec![71,77], 74 => vec![71,74]),
                    }]),
                },
                SubstLookup {
                    flags: LookupFlags::empty(),
                    mark_filtering_set: None,
                    substitution: Substitution::Alternate(vec![AlternateSubst {
                        mapping: btreemap!(66 => vec![67,68,69]),
                    }]),
                },
            ],
            scripts: ScriptList {
                scripts: hashmap!(*b"DFLT" => Script {
                    default_language_system: Some(
                        LanguageSystem {
                            required_feature: None,
                            feature_indices: vec![
                                0,
                                1,
                                2,
                           ],
                        },
                    ),
                    language_systems: HashMap::new()
                }),
            },
            features: vec![
                (*b"alte", vec![3], None),
                (*b"mult", vec![2], None),
                (*b"sing", vec![0, 1], None),
            ],
        };
        let deserialized: GSUB = otspec::de::from_bytes(&binary_gsub).unwrap();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_single_subst_1_ser() {
        let subst = SingleSubst {
            mapping: btreemap!(66 => 67, 68 => 69),
        };
        let serialized = otspec::ser::to_bytes(&subst).unwrap();
        assert_eq!(
            serialized,
            vec![0x00, 0x01, 0x00, 0x06, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 66, 0x00, 68]
        );
    }

    #[test]
    fn test_single_subst_2_ser() {
        let subst = SingleSubst {
            mapping: btreemap!(34 => 66, 35 => 66, 36  => 66),
        };
        let serialized = otspec::ser::to_bytes(&subst).unwrap();
        assert_eq!(
            serialized,
            vec![
                0x00, 0x02, 0x00, 0x0C, 0x00, 0x03, 0x00, 0x42, 0x00, 0x42, 0x00, 0x42, 0x00, 0x01,
                0x00, 0x03, 0x00, 0x22, 0x00, 0x23, 0x00, 0x24
            ]
        );
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
                0x00, 0x01, 0x00, 0x0A, 0x00, 0x02, 0x00, 0x12, 0x00, 0x18, 0x00, 0x01, 0x00, 0x02,
                0x00, 0x4A, 0x00, 0x4D, 0x00, 0x02, 0x00, 0x47, 0x00, 0x4A, 0x00, 0x02, 0x00, 0x47,
                0x00, 0x4D
            ]
        );
    }
}
