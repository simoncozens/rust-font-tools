use crate::layout::common::{coverage_or_nah, FromLowlevel};
use otspec::layout::contextual::{
    ChainedSequenceContextFormat1, ChainedSequenceContextFormat2, ChainedSequenceContextFormat3,
    SequenceContextFormat1, SequenceContextFormat2, SequenceContextFormat3, SequenceLookupRecord,
};
use otspec::layout::coverage::Coverage;
use otspec::tables::GPOS::GPOSSubtable;
use otspec::tables::GSUB::GSUBSubtable;
use otspec::types::*;
use std::collections::BTreeSet;

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

pub(crate) fn coverage_to_slot(off: Offset16<Coverage>) -> Slot {
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

impl SequenceContext {
    fn from_lowlevel_format1(st: SequenceContextFormat1, _max_glyph_id: GlyphID) -> Self {
        let mut sequence_context = SequenceContext::default();
        for (first_glyph, ruleset) in coverage_or_nah(st.coverage)
            .iter()
            .zip(st.seqRuleSets.v.iter())
        {
            if let Some(ruleset) = &ruleset.link {
                for rule in &ruleset.sequenceRules.v {
                    if let Some(rule) = &rule.link {
                        let mut input_glyphs: Vec<Slot> = vec![single_glyph_slot(*first_glyph)];
                        let later_glyphs: Vec<Slot> = rule
                            .inputSequence
                            .iter()
                            .map(|gid| single_glyph_slot(*gid))
                            .collect();
                        input_glyphs.extend(later_glyphs);

                        sequence_context
                            .rules
                            .push(collate_lookup_records(input_glyphs, &rule.seqLookupRecords));
                    }
                }
            }
        }
        sequence_context
    }

    fn from_lowlevel_format2(st: SequenceContextFormat2, max_glyph_id: GlyphID) -> Self {
        let mut sequence_context = SequenceContext::default();
        let classdef = st.classDef.link.unwrap_or_default();
        for (first_class, ruleset) in st.classSeqRuleSets.v.iter().enumerate() {
            if let Some(ruleset) = &ruleset.link {
                for rule in ruleset.sequenceRules.v.iter() {
                    if let Some(rule) = &rule.link {
                        let mut slots = vec![classdef.get_glyphs(first_class as u16, max_glyph_id)];
                        slots.extend(
                            rule.inputSequence
                                .iter()
                                .map(|&class| classdef.get_glyphs(class, max_glyph_id)),
                        );
                        sequence_context
                            .rules
                            .push(collate_lookup_records(slots, &rule.seqLookupRecords));
                    }
                }
            }
        }
        sequence_context
    }
    fn from_lowlevel_format3(st: SequenceContextFormat3, _max_glyph_id: GlyphID) -> Self {
        let mut sequence_context = SequenceContext::default();
        let slots: Vec<Slot> = st.coverages.into_iter().map(coverage_to_slot).collect();
        sequence_context
            .rules
            .push(collate_lookup_records(slots, &st.seqLookupRecords));
        sequence_context
    }
}

impl FromLowlevel<GPOSSubtable> for SequenceContext {
    fn from_lowlevel(st: GPOSSubtable, max_glyph_id: GlyphID) -> Self {
        match st {
            GPOSSubtable::GPOS7_1(gpos71) => {
                SequenceContext::from_lowlevel_format1(gpos71, max_glyph_id)
            }
            GPOSSubtable::GPOS7_2(gpos72) => {
                SequenceContext::from_lowlevel_format2(gpos72, max_glyph_id)
            }
            GPOSSubtable::GPOS7_3(gpos73) => {
                SequenceContext::from_lowlevel_format3(gpos73, max_glyph_id)
            }
            _ => panic!(),
        }
    }
}
impl FromLowlevel<GSUBSubtable> for SequenceContext {
    fn from_lowlevel(st: GSUBSubtable, max_glyph_id: GlyphID) -> Self {
        match st {
            GSUBSubtable::GSUB5_1(gsub51) => {
                SequenceContext::from_lowlevel_format1(gsub51, max_glyph_id)
            }
            GSUBSubtable::GSUB5_2(gsub52) => {
                SequenceContext::from_lowlevel_format2(gsub52, max_glyph_id)
            }
            GSUBSubtable::GSUB5_3(gsub53) => {
                SequenceContext::from_lowlevel_format3(gsub53, max_glyph_id)
            }
            _ => panic!(),
        }
    }
}

impl SequenceContext {
    fn to_format3(&self) -> Vec<SequenceContextFormat3> {
        self.rules
            .iter()
            .map(|rule| {
                let mut coverages: Vec<Offset16<Coverage>> = vec![];

                let mut sequence_lookup_records: Vec<SequenceLookupRecord> = vec![];

                for (ix, (slot, lookup_ids)) in rule.iter().enumerate() {
                    coverages.push(Offset16::to(Coverage {
                        glyphs: slot.iter().copied().collect(),
                    }));
                    for lookup_id in lookup_ids {
                        sequence_lookup_records.push(SequenceLookupRecord {
                            sequenceIndex: ix as uint16,
                            lookupIndex: *lookup_id,
                        });
                    }
                }
                SequenceContextFormat3 {
                    format: 3,
                    glyphCount: rule.len() as uint16,
                    seqLookupCount: sequence_lookup_records.len() as uint16,
                    seqLookupRecords: sequence_lookup_records,
                    coverages,
                }
            })
            .collect()
    }

    pub(crate) fn to_lowlevel_subtables_gpos(&self, _max_glyph_id: GlyphID) -> Vec<GPOSSubtable> {
        self.to_format3()
            .into_iter()
            .map(GPOSSubtable::GPOS7_3)
            .collect()
    }
    pub(crate) fn to_lowlevel_subtables_gsub(&self, _max_glyph_id: GlyphID) -> Vec<GSUBSubtable> {
        self.to_format3()
            .into_iter()
            .map(GSUBSubtable::GSUB5_3)
            .collect()
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
/// A chained contextual rule, with backtrack and lookahead
pub struct ChainedSequenceContextRule {
    /// Glyphs which must appear before the input sequence
    pub backtrack: Vec<Slot>,
    /// Glyphs which must appear after the input sequence
    pub lookahead: Vec<Slot>,
    /// The input sequence
    ///
    /// This consists of one or more inputs, where each input is a set of
    /// one or more glyphs and zero or more lookups.
    pub input: SequenceContextRule,
}

/* This struct is the user-facing representation of chained sequence context. */
#[derive(Debug, PartialEq, Clone, Default)]
/// A chained contextual substitution/positioning table (GSUB6/GPOS8).
pub struct ChainedSequenceContext {
    /// A set of sequence context rules
    pub rules: Vec<ChainedSequenceContextRule>,
}

impl ChainedSequenceContext {
    fn from_lowlevel_format1(st: ChainedSequenceContextFormat1, _max_glyph_id: GlyphID) -> Self {
        let mut chained_sequence_context = ChainedSequenceContext::default();
        for (first_glyph, ruleset) in coverage_or_nah(st.coverage)
            .iter()
            .zip(st.chainedSeqRuleSets.v.iter())
        {
            if let Some(ruleset) = &ruleset.link {
                for rule in &ruleset.chainedSequenceRules.v {
                    if let Some(rule) = &rule.link {
                        let mut input_glyphs: Vec<Slot> = vec![single_glyph_slot(*first_glyph)];
                        let later_glyphs: Vec<Slot> = rule
                            .inputSequence
                            .iter()
                            .map(|gid| single_glyph_slot(*gid))
                            .collect();
                        input_glyphs.extend(later_glyphs);
                        chained_sequence_context
                            .rules
                            .push(ChainedSequenceContextRule {
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
                                input: collate_lookup_records(input_glyphs, &rule.seqLookupRecords),
                            });
                    }
                }
            }
        }
        chained_sequence_context
    }
    fn from_lowlevel_format2(st: ChainedSequenceContextFormat2, max_glyph_id: GlyphID) -> Self {
        let mut chained_sequence_context = ChainedSequenceContext::default();
        let classdef = st.inputClassDef.link.unwrap_or_default();
        let backtrack_classdef = st.backtrackClassDef.link.unwrap_or_default();
        let lookahead_classdef = st.lookaheadClassDef.link.unwrap_or_default();
        for (first_class, ruleset) in st.chainedClassSeqRuleSets.v.iter().enumerate() {
            if let Some(ruleset) = &ruleset.link {
                for rule in ruleset.chainedSequenceRules.v.iter() {
                    if let Some(rule) = &rule.link {
                        let mut slots = vec![classdef.get_glyphs(first_class as u16, max_glyph_id)];
                        slots.extend(
                            rule.inputSequence
                                .iter()
                                .map(|&class| classdef.get_glyphs(class, max_glyph_id)),
                        );
                        chained_sequence_context
                            .rules
                            .push(ChainedSequenceContextRule {
                                backtrack: rule
                                    .backtrackSequence
                                    .iter()
                                    .map(|&class| {
                                        backtrack_classdef.get_glyphs(class, max_glyph_id)
                                    })
                                    .collect(),
                                lookahead: rule
                                    .lookaheadSequence
                                    .iter()
                                    .map(|&class| {
                                        lookahead_classdef.get_glyphs(class, max_glyph_id)
                                    })
                                    .collect(),
                                input: collate_lookup_records(slots, &rule.seqLookupRecords),
                            });
                    }
                }
            }
        }
        chained_sequence_context
    }
    fn from_lowlevel_format3(st: ChainedSequenceContextFormat3, _max_glyph_id: GlyphID) -> Self {
        let mut chained_sequence_context = ChainedSequenceContext::default();
        let slots: Vec<Slot> = st
            .inputCoverages
            .v
            .into_iter()
            .map(coverage_to_slot)
            .collect();
        chained_sequence_context
            .rules
            .push(ChainedSequenceContextRule {
                backtrack: st
                    .backtrackCoverages
                    .v
                    .into_iter()
                    .map(coverage_to_slot)
                    .collect(),
                lookahead: st
                    .lookaheadCoverages
                    .v
                    .into_iter()
                    .map(coverage_to_slot)
                    .collect(),
                input: collate_lookup_records(slots, &st.seqLookupRecords),
            });
        chained_sequence_context
    }
}

impl FromLowlevel<GPOSSubtable> for ChainedSequenceContext {
    fn from_lowlevel(st: GPOSSubtable, max_glyph_id: GlyphID) -> Self {
        match st {
            GPOSSubtable::GPOS8_1(gpos81) => {
                ChainedSequenceContext::from_lowlevel_format1(gpos81, max_glyph_id)
            }
            GPOSSubtable::GPOS8_2(gpos82) => {
                ChainedSequenceContext::from_lowlevel_format2(gpos82, max_glyph_id)
            }
            GPOSSubtable::GPOS8_3(gpos83) => {
                ChainedSequenceContext::from_lowlevel_format3(gpos83, max_glyph_id)
            }
            _ => panic!(),
        }
    }
}

impl FromLowlevel<GSUBSubtable> for ChainedSequenceContext {
    fn from_lowlevel(st: GSUBSubtable, max_glyph_id: GlyphID) -> Self {
        match st {
            GSUBSubtable::GSUB6_1(gsub61) => {
                ChainedSequenceContext::from_lowlevel_format1(gsub61, max_glyph_id)
            }
            GSUBSubtable::GSUB6_2(gsub62) => {
                ChainedSequenceContext::from_lowlevel_format2(gsub62, max_glyph_id)
            }
            GSUBSubtable::GSUB6_3(gsub63) => {
                ChainedSequenceContext::from_lowlevel_format3(gsub63, max_glyph_id)
            }
            _ => panic!(),
        }
    }
}

impl ChainedSequenceContext {
    fn to_format3(&self) -> Vec<ChainedSequenceContextFormat3> {
        self.rules
            .iter()
            .map(|rule| {
                let mut coverages: Vec<Offset16<Coverage>> = vec![];
                let lookahead_coverages: Vec<Offset16<Coverage>> = rule
                    .lookahead
                    .iter()
                    .map(|slot| {
                        Offset16::to(Coverage {
                            glyphs: slot.iter().copied().collect(),
                        })
                    })
                    .collect();
                let backtrack_coverages: Vec<Offset16<Coverage>> = rule
                    .backtrack
                    .iter()
                    .map(|slot| {
                        Offset16::to(Coverage {
                            glyphs: slot.iter().copied().collect(),
                        })
                    })
                    .collect();

                let mut sequence_lookup_records: Vec<SequenceLookupRecord> = vec![];

                for (ix, (slot, lookup_ids)) in rule.input.iter().enumerate() {
                    coverages.push(Offset16::to(Coverage {
                        glyphs: slot.iter().copied().collect(),
                    }));
                    for lookup_id in lookup_ids {
                        sequence_lookup_records.push(SequenceLookupRecord {
                            sequenceIndex: ix as uint16,
                            lookupIndex: *lookup_id,
                        });
                    }
                }
                ChainedSequenceContextFormat3 {
                    format: 3,
                    inputCoverages: coverages.into(),
                    seqLookupRecords: sequence_lookup_records,
                    backtrackCoverages: backtrack_coverages.into(),
                    lookaheadCoverages: lookahead_coverages.into(),
                }
            })
            .collect()
    }

    pub(crate) fn to_lowlevel_subtables_gpos(&self, _max_glyph_id: GlyphID) -> Vec<GPOSSubtable> {
        self.to_format3()
            .into_iter()
            .map(GPOSSubtable::GPOS8_3)
            .collect()
    }
    pub(crate) fn to_lowlevel_subtables_gsub(&self, _max_glyph_id: GlyphID) -> Vec<GSUBSubtable> {
        self.to_format3()
            .into_iter()
            .map(GSUBSubtable::GSUB6_3)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::{Lookup, LookupFlags};
    use crate::tables::GPOS::tests::{assert_can_deserialize, assert_can_roundtrip, expected_gpos};
    use crate::tables::GPOS::Positioning;
    use otspec::btreeset;
    use std::iter::FromIterator;

    #[test]
    fn test_gpos_format_1() {
        /*
        feature test {
            sub a' lookup one lookup two b' c' lookup three;
            sub a' lookup two;
            sub c' lookup three;
        } test;
        */
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x07, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x0A, 0x00, 0x02, 0x00, 0x12, 0x00, 0x34, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x42, 0x00, 0x44, 0x00, 0x02, 0x00, 0x06, 0x00, 0x1A, 0x00, 0x03, 0x00, 0x03,
            0x00, 0x43, 0x00, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x04,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02,
        ];
        let rule_one: SequenceContextRule = vec![
            (btreeset!(66), vec![0, 1]),
            (btreeset!(67), vec![]),
            (btreeset!(68), vec![2]),
        ];
        let rule_two: SequenceContextRule = vec![(btreeset!(66), vec![1])];
        let rule_three: SequenceContextRule = vec![(btreeset!(68), vec![2])];
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::Contextual(vec![SequenceContext {
                rules: vec![rule_one, rule_two, rule_three],
            }]),
        }]);
        assert_can_deserialize(binary_gpos, &expected);
    }

    #[test]
    fn test_gpos_format_2() {
        /*
        feature test {
            sub [d e f]' lookup two [a b c]' lookup one;
            sub [d e f]' lookup two [d e f]' lookup two;
            sub [a b c]' lookup one [a b c]' lookup one;
        } test;
        */
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x07, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x02, 0x00, 0x0E, 0x00, 0x18, 0x00, 0x03, 0x00, 0x28, 0x00, 0x2A, 0x00, 0x4C,
            0x00, 0x02, 0x00, 0x01, 0x00, 0x42, 0x00, 0x47, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02,
            0x00, 0x42, 0x00, 0x44, 0x00, 0x02, 0x00, 0x45, 0x00, 0x47, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x06, 0x00, 0x14, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x04, 0x00, 0x02, 0x00, 0x02,
            0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        ];
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
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::Contextual(vec![SequenceContext {
                rules: vec![rule_one, rule_two, rule_three],
            }]),
        }]);
        assert_can_deserialize(binary_gpos, &expected);
    }
    #[test]
    fn test_gsub_format_3() {
        /*
        feature test {
               sub [a d]' lookup one b' lookup two;
        }test;
        */
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x07, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            /* 0 */ 0x00, 0x03, // format 3
            /* 2 */ 0x00, 0x02, // glyph count
            /* 4 */ 0x00, 0x02, // seqLookupCount
            /* 6 */ 0x00, 0x12, // coverage1 = @18
            /* 8 */ 0x00, 0x1A, // coverage2 = @26
            /* 10 */ 0x00, 0x00, 0x00, 0x00, /* 14 */ 0x00, 0x01, 0x00, 0x01,
            /* 18 */ 0x00, 0x01, 0x00, 0x02, 0x00, 0x42, 0x00, 0x45, 0x00, /* 26 */ 0x01,
            0x00, 0x01, 0x00, 0x43,
        ];
        let rule_one: SequenceContextRule =
            vec![(btreeset!(66, 69), vec![0]), (btreeset!(67), vec![1])];
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::Contextual(vec![SequenceContext {
                rules: vec![rule_one],
            }]),
        }]);
        assert_can_deserialize(binary_gpos, &expected);
    }

    #[test]
    fn test_gpos_chained_format1() {
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x01, 0x00, 0x0a, 0x00, 0x02, 0x00, 0x12, 0x00, 0x42, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x42, 0x00, 0x44, 0x00, 0x02, 0x00, 0x06, 0x00, 0x20, 0x00, 0x01, 0x00, 0x59,
            0x00, 0x03, 0x00, 0x43, 0x00, 0x44, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x5a, 0x00, 0x5b, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x04,
            0x00, 0x01, 0x00, 0x59, 0x00, 0x01, 0x00, 0x01, 0x00, 0x5a, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x02,
        ];
        let rule_one: SequenceContextRule = vec![
            (btreeset!(66), vec![0, 1]),
            (btreeset!(67), vec![]),
            (btreeset!(68), vec![2]),
        ];
        let rule_two: SequenceContextRule = vec![(btreeset!(66), vec![1])];
        let rule_three: SequenceContextRule = vec![(btreeset!(68), vec![2])];
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::ChainedContextual(vec![ChainedSequenceContext {
                rules: vec![
                    ChainedSequenceContextRule {
                        backtrack: vec![btreeset!(89)],
                        lookahead: vec![],
                        input: rule_one,
                    },
                    ChainedSequenceContextRule {
                        backtrack: vec![],
                        lookahead: vec![btreeset!(90), btreeset!(91)],
                        input: rule_two,
                    },
                    ChainedSequenceContextRule {
                        backtrack: vec![btreeset!(89)],
                        lookahead: vec![btreeset!(90)],
                        input: rule_three,
                    },
                ],
            }]),
        }]);
        assert_can_deserialize(binary_gpos, &expected);
    }

    #[test]
    fn test_gpos_chained_format2() {
        /*feature test {
            pos x [d e f]' lookup two [a b c]' lookup one;
            pos [d e f]' lookup two [d e f]' lookup two y z;
            pos x [a b c]' lookup one [a b c]' lookup one y;
        } test;*/
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x02, 0x00, 0x12, 0x00, 0x1c, 0x00, 0x24, 0x00, 0x34, 0x00, 0x03, 0x00, 0x3e,
            0x00, 0x40, 0x00, 0x70, 0x00, 0x02, 0x00, 0x01, 0x00, 0x42, 0x00, 0x47, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x59, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x42,
            0x00, 0x44, 0x00, 0x02, 0x00, 0x45, 0x00, 0x47, 0x00, 0x01, 0x00, 0x01, 0x00, 0x5a,
            0x00, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x06, 0x00, 0x1a,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x02,
            0x00, 0x02, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x04, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x01,
            0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        ];
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
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::ChainedContextual(vec![ChainedSequenceContext {
                rules: vec![
                    ChainedSequenceContextRule {
                        backtrack: vec![btreeset!(89)],
                        lookahead: vec![],
                        input: rule_one,
                    },
                    ChainedSequenceContextRule {
                        backtrack: vec![],
                        lookahead: vec![btreeset!(90), btreeset!(91)],
                        input: rule_two,
                    },
                    ChainedSequenceContextRule {
                        backtrack: vec![btreeset!(89)],
                        lookahead: vec![btreeset!(90)],
                        input: rule_three,
                    },
                ],
            }]),
        }]);
        assert_can_deserialize(binary_gpos, &expected);
    }
    #[test]
    fn test_gpos_chained_format3() {
        /*feature test {
                pos x [a d]' lookup one b' lookup two y;
          } test;
        */
        let binary_gpos = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x1e, 0x00, 0x2c, 0x00, 0x01, 0x44, 0x46,
            0x4c, 0x54, 0x00, 0x08, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x74, 0x65, 0x73, 0x74, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08,
            0x00, 0x03, 0x00, 0x01, 0x00, 0x1a, 0x00, 0x02, 0x00, 0x20, 0x00, 0x28, 0x00, 0x01,
            0x00, 0x2e, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x59, 0x00, 0x01, 0x00, 0x02, 0x00, 0x42, 0x00, 0x45, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x43, 0x00, 0x01, 0x00, 0x01, 0x00, 0x5a,
        ];
        let rule_one: SequenceContextRule =
            vec![(btreeset!(66, 69), vec![0]), (btreeset!(67), vec![1])];
        let expected = expected_gpos(vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::ChainedContextual(vec![ChainedSequenceContext {
                rules: vec![ChainedSequenceContextRule {
                    backtrack: vec![btreeset!(89)],
                    lookahead: vec![btreeset!(90)],
                    input: rule_one,
                }],
            }]),
        }]);
        assert_can_roundtrip(binary_gpos, &expected);
    }
}
