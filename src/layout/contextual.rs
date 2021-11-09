use crate::format_switching_lookup;
use otspec::layout::classdef::ClassDef;
use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::{tables, Serialize};
use std::collections::BTreeSet;

tables!(
    SequenceLookupRecord {
        uint16 sequenceIndex
        uint16 lookupIndex
    }
    SequenceContextFormat1 {
        [offset_base]
        uint16 format
        Offset16(Coverage) coverage
        CountedOffset16(SequenceRuleSet) seqRuleSets
    }
    SequenceRuleSet {
        [offset_base]
        CountedOffset16(SequenceRule) sequenceRules
    }
    SequenceContextFormat2 {
        [offset_base]
        uint16 format
        Offset16(Coverage) coverage
        Offset16(ClassDef) classDef
        // In theory, this should be a ClassSequenceRuleset. But a
        // ClassSequenceRuleset is just a counted array of ClassSequenceRule
        // offsets, and the layout of ClassSequenceRule is identical to
        // that of SequenceRule
        CountedOffset16(SequenceRuleSet) classSeqRuleSets
    }
    ChainedSequenceContextFormat1 {
        [offset_base]
        uint16 format
        Offset16(Coverage) coverage
        CountedOffset16(ChainedSequenceRuleSet) chainedSeqRuleSets
    }
    ChainedSequenceRuleSet {
        CountedOffset16(ChainedSequenceRule) chainedSequenceRules
    }
    ChainedSequenceContextFormat2 {
        uint16 format
        Offset16(Coverage) coverage
        Offset16(ClassDef) backtrackClassDef
        Offset16(ClassDef) inputClassDef
        Offset16(ClassDef) lookaheadClassDef
        // See above - should be a ChainedClassSequenceRuleset, but they're the same
        CountedOffset16(ChainedSequenceRuleSet) chainedClassSeqRuleSets
    }
    ChainedSequenceContextFormat3 {
        uint16 format
        CountedOffset16(Coverage) backtrackCoverages
        CountedOffset16(Coverage) inputCoverages
        CountedOffset16(Coverage) lookaheadCoverages
        Counted(SequenceLookupRecord) sequenceLookupRecords
    }
);

// Needs to be handled manually because of awkward layout and [glyphcount-1] array
#[allow(missing_docs, non_snake_case)]
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct SequenceRule {
    pub glyphCount: uint16,
    pub seqLookupCount: uint16,
    pub inputSequence: Vec<uint16>,
    pub seqLookupRecords: Vec<SequenceLookupRecord>,
}

impl Deserialize for SequenceRule {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let glyphCount: uint16 = c.de()?;
        let seqLookupCount: uint16 = c.de()?;
        let inputSequence: Vec<uint16> = c.de_counted(glyphCount as usize - 1)?;
        let seqLookupRecords: Vec<SequenceLookupRecord> = c.de_counted(seqLookupCount as usize)?;
        Ok(SequenceRule {
            glyphCount,
            seqLookupCount,
            inputSequence,
            seqLookupRecords,
        })
    }
}

// Needs to be handled manually because of awkward layout
#[allow(missing_docs, non_snake_case)]
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct SequenceContextFormat3 {
    pub format: uint16,
    pub glyphCount: uint16,
    pub seqLookupCount: uint16,
    pub coverages: Vec<Offset16<Coverage>>,
    pub seqLookupRecords: Vec<SequenceLookupRecord>,
}

impl Deserialize for SequenceContextFormat3 {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let format: uint16 = c.de()?;
        let glyphCount: uint16 = c.de()?;
        let seqLookupCount: uint16 = c.de()?;
        let coverages: Vec<Offset16<Coverage>> = c.de_counted(glyphCount as usize)?;
        let seqLookupRecords: Vec<SequenceLookupRecord> = c.de_counted(seqLookupCount as usize)?;
        Ok(SequenceContextFormat3 {
            format,
            glyphCount,
            seqLookupCount,
            coverages,
            seqLookupRecords,
        })
    }
}

// Needs to be handled manually because of awkward layout and [glyphcount-1] array
#[allow(missing_docs, non_snake_case)]
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ChainedSequenceRule {
    #[otspec(with = "Counted")]
    pub backtrackSequence: Vec<uint16>,
    pub inputGlyphCount: uint16,
    pub inputSequence: Vec<uint16>,
    #[otspec(with = "Counted")]
    pub lookaheadSequence: Vec<uint16>,
    #[otspec(with = "Counted")]
    pub seqLookupRecords: Vec<SequenceLookupRecord>,
}

impl Deserialize for ChainedSequenceRule {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError>
    where
        Self: std::marker::Sized,
    {
        let backtrackGlyphCount: uint16 = c.de()?;
        let backtrackSequence: Vec<uint16> = c.de_counted(backtrackGlyphCount as usize)?;
        let inputGlyphCount: uint16 = c.de()?;
        let inputSequence: Vec<uint16> = c.de_counted(inputGlyphCount as usize - 1)?;
        let lookaheadGlyphCount: uint16 = c.de()?;
        let lookaheadSequence: Vec<uint16> = c.de_counted(lookaheadGlyphCount as usize)?;
        let seqLookupRecordCount: uint16 = c.de()?;
        let seqLookupRecords: Vec<SequenceLookupRecord> =
            c.de_counted(seqLookupRecordCount as usize)?;
        Ok(ChainedSequenceRule {
            backtrackSequence,
            inputGlyphCount,
            lookaheadSequence,
            inputSequence,
            seqLookupRecords,
        })
    }
}

format_switching_lookup!(SequenceContext {
    Format1,
    Format2,
    Format3
});

/// A helpful alias which makes the type a bit more self-documenting
pub type LookupID = uint16;
/// A sequence position in a sequence context rule
pub type Slot = BTreeSet<GlyphID>;
/// A sequence context rule: each "slot" matches zero or more glyph IDs and may dispatch to zero or more lookups
pub type SequenceContextRule = Vec<(Slot, Vec<LookupID>)>;

/* This struct is the user-facing representation of sequence context. */
#[derive(Debug, PartialEq, Clone, Default)]
/// A contextual substitution/positioning table (GSUB5/GPOS7).
pub struct SequenceContext {
    /// A set of sequence context rules
    pub rules: Vec<SequenceContextRule>,
}

fn coverage_or_nah(off: Offset16<Coverage>) -> Vec<GlyphID> {
    off.link
        .map(|x| x.glyphs)
        .iter()
        .flatten()
        .copied()
        .collect()
}
fn coverage_to_slot(off: Offset16<Coverage>) -> Slot {
    off.link
        .map(|x| x.glyphs)
        .iter()
        .flatten()
        .copied()
        .collect()
}
fn single_glyph_slot(gid: GlyphID) -> Slot {
    std::iter::once(gid).collect()
}

fn collate_lookup_records(
    slots: Vec<Slot>,
    lookup_records: &[SequenceLookupRecord],
) -> SequenceContextRule {
    let mut rule: SequenceContextRule = vec![];
    for (i, g) in slots.into_iter().enumerate() {
        let lookups = lookup_records
            .iter()
            .filter(|x| x.sequenceIndex as usize == i)
            .map(|x| x.lookupIndex)
            .collect();
        rule.push((g, lookups));
    }
    rule
}
/* On serialization, move to the outgoing representation by choosing the best format */
impl From<&SequenceContext> for SequenceContextInternal {
    fn from(val: &SequenceContext) -> Self {
        unimplemented!()
    }
}

impl Deserialize for SequenceContext {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let fmt = c.peek(2)?;
        let mut rules: Vec<SequenceContextRule> = vec![];
        match fmt {
            [0x00, 0x01] => {
                let sub: SequenceContextFormat1 = c.de()?;
                for (first_glyph, ruleset) in coverage_or_nah(sub.coverage)
                    .iter()
                    .zip(sub.seqRuleSets.v.iter())
                {
                    if let Some(ruleset) = &ruleset.link {
                        for rule in &ruleset.sequenceRules.v {
                            if let Some(rule) = &rule.link {
                                let mut input_glyphs: Vec<Slot> =
                                    vec![single_glyph_slot(*first_glyph)];
                                let later_glyphs: Vec<Slot> = rule
                                    .inputSequence
                                    .iter()
                                    .map(|gid| single_glyph_slot(*gid))
                                    .collect();
                                input_glyphs.extend(later_glyphs);

                                rules.push(collate_lookup_records(
                                    input_glyphs,
                                    &rule.seqLookupRecords,
                                ));
                            }
                        }
                    }
                }
            }
            [0x00, 0x02] => {
                let sub: SequenceContextFormat2 = c.de()?;
                let classdef = sub.classDef.link.unwrap_or_default();
                for (first_class, ruleset) in sub.classSeqRuleSets.v.iter().enumerate() {
                    if let Some(ruleset) = &ruleset.link {
                        for rule in ruleset.sequenceRules.v.iter() {
                            if let Some(rule) = &rule.link {
                                let mut slots = vec![classdef.get_glyphs(first_class as u16)];
                                slots.extend(
                                    rule.inputSequence
                                        .iter()
                                        .map(|&class| classdef.get_glyphs(class)),
                                );
                                rules.push(collate_lookup_records(slots, &rule.seqLookupRecords));
                            }
                        }
                    }
                }
            }
            [0x00, 0x03] => {
                let sub: SequenceContextFormat3 = c.de()?;
                let slots: Vec<Slot> = sub.coverages.into_iter().map(coverage_to_slot).collect();
                rules.push(collate_lookup_records(slots, &sub.seqLookupRecords));
            }
            _ => panic!("Bad sequence context format {:?}", fmt),
        }
        Ok(SequenceContext { rules })
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
/// A chained contextual rule, with backtrack and lookahead
pub struct ChainedSequenceContextRule {
    backtrack: Vec<Slot>,
    lookahead: Vec<Slot>,
    input: SequenceContextRule,
}

/* This struct is the user-facing representation of chained sequence context. */
#[derive(Debug, PartialEq, Clone, Default)]
/// A chained contextual substitution/positioning table (GSUB6/GPOS8).
pub struct ChainedSequenceContext {
    /// A set of sequence context rules
    pub rules: Vec<ChainedSequenceContextRule>,
}

format_switching_lookup!(ChainedSequenceContext {
    Format1,
    Format2,
    Format3
});

/* On serialization, move to the outgoing representation by choosing the best format */
impl From<&ChainedSequenceContext> for ChainedSequenceContextInternal {
    fn from(val: &ChainedSequenceContext) -> Self {
        unimplemented!()
    }
}

impl Deserialize for ChainedSequenceContext {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let fmt = c.peek(2)?;
        let mut rules: Vec<ChainedSequenceContextRule> = vec![];
        match fmt {
            [0x00, 0x01] => {
                let sub: ChainedSequenceContextFormat1 = c.de()?;
                for (first_glyph, ruleset) in coverage_or_nah(sub.coverage)
                    .iter()
                    .zip(sub.chainedSeqRuleSets.v.iter())
                {
                    if let Some(ruleset) = &ruleset.link {
                        for rule in &ruleset.chainedSequenceRules.v {
                            if let Some(rule) = &rule.link {
                                let mut input_glyphs: Vec<Slot> =
                                    vec![single_glyph_slot(*first_glyph)];
                                let later_glyphs: Vec<Slot> = rule
                                    .inputSequence
                                    .iter()
                                    .map(|gid| single_glyph_slot(*gid))
                                    .collect();
                                input_glyphs.extend(later_glyphs);
                                rules.push(ChainedSequenceContextRule {
                                    backtrack: rule
                                        .backtrackSequence
                                        .iter()
                                        .map(|gid| single_glyph_slot(*gid))
                                        .collect(),
                                    lookahead: rule
                                        .lookaheadSequence
                                        .iter()
                                        .map(|gid| single_glyph_slot(*gid))
                                        .collect(),
                                    input: collate_lookup_records(
                                        input_glyphs,
                                        &rule.seqLookupRecords,
                                    ),
                                });
                            }
                        }
                    }
                }
            }
            [0x00, 0x02] => {
                let sub: ChainedSequenceContextFormat2 = c.de()?;
                let classdef = sub.inputClassDef.link.unwrap_or_default();
                let backtrack_classdef = sub.backtrackClassDef.link.unwrap_or_default();
                let lookahead_classdef = sub.lookaheadClassDef.link.unwrap_or_default();
                for (first_class, ruleset) in sub.chainedClassSeqRuleSets.v.iter().enumerate() {
                    if let Some(ruleset) = &ruleset.link {
                        for rule in ruleset.chainedSequenceRules.v.iter() {
                            if let Some(rule) = &rule.link {
                                let mut slots = vec![classdef.get_glyphs(first_class as u16)];
                                slots.extend(
                                    rule.inputSequence
                                        .iter()
                                        .map(|&class| classdef.get_glyphs(class)),
                                );
                                rules.push(ChainedSequenceContextRule {
                                    backtrack: rule
                                        .backtrackSequence
                                        .iter()
                                        .map(|&class| backtrack_classdef.get_glyphs(class))
                                        .collect(),
                                    lookahead: rule
                                        .lookaheadSequence
                                        .iter()
                                        .map(|&class| lookahead_classdef.get_glyphs(class))
                                        .collect(),
                                    input: collate_lookup_records(slots, &rule.seqLookupRecords),
                                });
                            }
                        }
                    }
                }
            }
            [0x00, 0x03] => {
                let sub: ChainedSequenceContextFormat3 = c.de()?;
                let slots: Vec<Slot> = sub
                    .inputCoverages
                    .v
                    .into_iter()
                    .map(coverage_to_slot)
                    .collect();
                rules.push(ChainedSequenceContextRule {
                    backtrack: sub
                        .backtrackCoverages
                        .v
                        .into_iter()
                        .map(coverage_to_slot)
                        .collect(),
                    lookahead: sub
                        .lookaheadCoverages
                        .v
                        .into_iter()
                        .map(coverage_to_slot)
                        .collect(),
                    input: collate_lookup_records(slots, &sub.sequenceLookupRecords),
                });
            }
            _ => panic!("Bad sequence context format {:?}", fmt),
        }
        Ok(ChainedSequenceContext { rules })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use otspec::btreeset;
    use std::iter::FromIterator;

    #[test]
    fn test_gsub_format_1() {
        /*
        feature test {
            sub a' lookup one lookup two b' c' lookup three;
            sub a' lookup two;
            sub c' lookup three;
        } test;
        */
        let binary_lookup = vec![
            0x00, 0x01, 0x00, 0x0A, 0x00, 0x02, 0x00, 0x12, 0x00, 0x34, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x42, 0x00, 0x44, 0x00, 0x02, 0x00, 0x06, 0x00, 0x1A, 0x00, 0x03, 0x00, 0x03,
            0x00, 0x43, 0x00, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x04,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02,
        ];
        let sequence: SequenceContext = otspec::de::from_bytes(&binary_lookup).unwrap();
        let rule_one: SequenceContextRule = vec![
            (btreeset!(66), vec![0, 1]),
            (btreeset!(67), vec![]),
            (btreeset!(68), vec![2]),
        ];
        let rule_two: SequenceContextRule = vec![(btreeset!(66), vec![1])];
        let rule_three: SequenceContextRule = vec![(btreeset!(68), vec![2])];
        assert_eq!(
            sequence,
            SequenceContext {
                rules: vec![rule_one, rule_two, rule_three]
            }
        );
    }

    #[test]
    fn test_gsub_format_2() {
        /*
        feature test {
            sub [d e f]' lookup two [a b c]' lookup one;
            sub [d e f]' lookup two [d e f]' lookup two;
            sub [a b c]' lookup one [a b c]' lookup one;
        } test;
        */
        let binary_lookup = vec![
            0x00, 0x02, 0x00, 0x0E, 0x00, 0x18, 0x00, 0x03, 0x00, 0x28, 0x00, 0x2A, 0x00, 0x4C,
            0x00, 0x02, 0x00, 0x01, 0x00, 0x42, 0x00, 0x47, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02,
            0x00, 0x42, 0x00, 0x44, 0x00, 0x02, 0x00, 0x45, 0x00, 0x47, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x06, 0x00, 0x14, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x04, 0x00, 0x02, 0x00, 0x02,
            0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        ];
        let sequence: SequenceContext = otspec::de::from_bytes(&binary_lookup).unwrap();
        let rule_one: SequenceContextRule = vec![
            (btreeset!(69, 70, 71), vec![1]),
            (btreeset!(66, 67, 68), vec![0]),
        ];
        let rule_two: SequenceContextRule = vec![
            (btreeset!(69, 70, 71), vec![1]),
            (btreeset!(69, 70, 71), vec![1]),
        ];
        let rule_three: SequenceContextRule = vec![
            (btreeset!(66, 67, 68), vec![0]),
            (btreeset!(66, 67, 68), vec![0]),
        ];
        assert_eq!(
            sequence,
            SequenceContext {
                rules: vec![rule_one, rule_two, rule_three]
            }
        );
    }

    #[test]
    fn test_gsub_format_3() {
        /*
        feature test {
               sub [a d]' lookup one b' lookup two;
        }test;
        */
        let binary_lookup = vec![
            0x00, 0x03, 0x00, 0x02, 0x00, 0x02, 0x00, 0x12, 0x00, 0x1A, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x42, 0x00, 0x45, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x43,
        ];
        let sequence: SequenceContext = otspec::de::from_bytes(&binary_lookup).unwrap();
        let rule_one: SequenceContextRule =
            vec![(btreeset!(66, 69), vec![0]), (btreeset!(67), vec![1])];
        assert_eq!(
            sequence,
            SequenceContext {
                rules: vec![rule_one]
            }
        );
    }
}
