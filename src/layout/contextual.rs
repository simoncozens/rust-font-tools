use crate::layout::classdef::ClassDef;
use crate::layout::coverage::Coverage;
use otspec::types::*;
use otspec::{DeserializationError, Deserialize, Deserializer, ReaderContext};
use otspec_macros::{tables, Serialize};

tables!(
    SequenceLookupRecord {
        uint16 sequenceIndex
        uint16 lookupIndex
    }
    SequenceContextFormat1 {
        uint16 format
        Offset16(Coverage) coverage
        CountedOffset16(SequenceRuleSet) seqRuleSets
    }
    SequenceRuleSet {
        CountedOffset16(SequenceRule) sequenceRules
    }
    SequenceContextFormat2 {
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
        CountedOffset16(Coverage) backtraceCoverages
        CountedOffset16(Coverage) inputCoverages
        CountedOffset16(Coverage) lookaheadCoverages
        CountedOffset16(SequenceLookupRecord) sequenceLookupRecords
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
    #[serde(with = "Counted")]
    pub backtrackSequence: Vec<uint16>,
    pub inputGlyphCount: uint16,
    pub inputSequence: Vec<uint16>,
    #[serde(with = "Counted")]
    pub lookaheadSequence: Vec<uint16>,
    #[serde(with = "Counted")]
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
