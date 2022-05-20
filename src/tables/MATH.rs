use otspec::layout::coverage::Coverage;
use otspec::layout::device::Device;

use otspec::types::*;
use otspec::{DeserializationError, Deserialize, Deserializer, Serialize, Serializer};
use otspec_macros::{tables, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The 'MATH' OpenType tag.
pub const TAG: Tag = crate::tag!("MATH");

tables!(
    MathValueRecord [embedded] {
        FWORD value
        Offset16(Device) device
    }
    MATHinternal {
        uint16 majorVersion
        uint16  minorVersion
        Offset16(MathConstants) mathConstants
        Offset16(MathGlyphInfo) mathGlyphInfo
        Offset16(MathVariants) mathVariants
    }
    MathConstants {
        int16   scriptPercentScaleDown
        int16   scriptScriptPercentScaleDown
        UFWORD  delimitedSubFormulaMinHeight
        UFWORD  displayOperatorMinHeight
        MathValueRecord mathLeading
        MathValueRecord axisHeight
        MathValueRecord accentBaseHeight
        MathValueRecord flattenedAccentBaseHeight
        MathValueRecord subscriptShiftDown
        MathValueRecord subscriptTopMax
        MathValueRecord subscriptBaselineDropMin
        MathValueRecord superscriptShiftUp
        MathValueRecord superscriptShiftUpCramped
        MathValueRecord superscriptBottomMin
        MathValueRecord superscriptBaselineDropMax
        MathValueRecord subSuperscriptGapMin
        MathValueRecord superscriptBottomMaxWithSubscript
        MathValueRecord spaceAfterScript
        MathValueRecord upperLimitGapMin
        MathValueRecord upperLimitBaselineRiseMin
        MathValueRecord lowerLimitGapMin
        MathValueRecord lowerLimitBaselineDropMin
        MathValueRecord stackTopShiftUp
        MathValueRecord stackTopDisplayStyleShiftUp
        MathValueRecord stackBottomShiftDown
        MathValueRecord stackBottomDisplayStyleShiftDown
        MathValueRecord stackGapMin
        MathValueRecord stackDisplayStyleGapMin
        MathValueRecord stretchStackTopShiftUp
        MathValueRecord stretchStackBottomShiftDown
        MathValueRecord stretchStackGapAboveMin
        MathValueRecord stretchStackGapBelowMin
        MathValueRecord fractionNumeratorShiftUp
        MathValueRecord fractionNumeratorDisplayStyleShiftUp
        MathValueRecord fractionDenominatorShiftDown
        MathValueRecord fractionDenominatorDisplayStyleShiftDown
        MathValueRecord fractionNumeratorGapMin
        MathValueRecord fractionNumDisplayStyleGapMin
        MathValueRecord fractionRuleThickness
        MathValueRecord fractionDenominatorGapMin
        MathValueRecord fractionDenomDisplayStyleGapMin
        MathValueRecord skewedFractionHorizontalGap
        MathValueRecord skewedFractionVerticalGap
        MathValueRecord overbarVerticalGap
        MathValueRecord overbarRuleThickness
        MathValueRecord overbarExtraAscender
        MathValueRecord underbarVerticalGap
        MathValueRecord underbarRuleThickness
        MathValueRecord underbarExtraDescender
        MathValueRecord radicalVerticalGap
        MathValueRecord radicalDisplayStyleVerticalGap
        MathValueRecord radicalRuleThickness
        MathValueRecord radicalExtraAscender
        MathValueRecord radicalKernBeforeDegree
        MathValueRecord radicalKernAfterDegree
        int16   radicalDegreeBottomRaisePercent
    }
    MathGlyphInfo {
        [offset_base]
        Offset16(MathItalicsCorrectionInfo) mathItalicsCorrectionInfo
        Offset16(MathTopAccentAttachment) mathTopAccentAttachment
        Offset16(Coverage) extendedShapeCoverage
        Offset16(MathKernInfo) mathKernInfo
    }
    MathItalicsCorrectionInfo {
        Offset16(Coverage) italicsCorrectionCoverage
        Counted(MathValueRecord) italicsCorrection
    }
    MathTopAccentAttachment {
        Offset16(Coverage) topAccentCoverage
        Counted(MathValueRecord) topAccentAttachment
    }
    MathKernInfo {
        [offset_base]
        Offset16(Coverage) mathKernCoverage
        [embed]
        Counted(MathKernInfoRecord) mathKernInfoRecords
    }
    MathKernInfoRecord [embedded] {
        Offset16(MathKern) topRightMathKern
        Offset16(MathKern) topLeftMathKern
        Offset16(MathKern) bottomRightMathKern
        Offset16(MathKern) bottomLeftMathKern
    }
    MathGlyphConstruction {
        [offset_base]
        Offset16(GlyphAssembly) glyphAssembly
        Counted(MathGlyphVariantRecord) mathGlyphVariantRecord
    }
    MathGlyphVariantRecord {
        uint16  variantGlyph
        UFWORD  advanceMeasurement
    }
    GlyphAssembly {
        MathValueRecord italicsCorrection
        Counted(GlyphPartRecord) partRecords
    }
    GlyphPartRecord {
        uint16  glyphID
        UFWORD  startConnectorLength
        UFWORD  endConnectorLength
        UFWORD  fullAdvance
        uint16  partFlags
    }
);

impl MathValueRecord {
    /// Create a new MATH value record, with no device table
    pub fn new(value: FWORD) -> Self {
        Self {
            value,
            device: Offset16::to_nothing(),
        }
    }
}
// Needs to be handled manually because of awkward layout
#[allow(missing_docs, non_snake_case)]
#[derive(Debug, Clone, PartialEq)]
pub struct MathVariants {
    pub minConnectorOverlap: UFWORD,
    pub vertGlyphCoverage: Offset16<Coverage>,
    pub horizGlyphCoverage: Offset16<Coverage>,
    pub vertGlyphCount: uint16,
    pub horizGlyphCount: uint16,
    pub vertGlyphConstruction: Vec<Offset16<MathGlyphConstruction>>,
    pub horizGlyphConstruction: Vec<Offset16<MathGlyphConstruction>>,
}

impl Deserialize for MathVariants {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut otspec::ReaderContext) -> Result<Self, otspec::DeserializationError> {
        c.push();
        let minConnectorOverlap: UFWORD = c.de()?;
        let vertGlyphCoverage: Offset16<Coverage> = c.de()?;
        let horizGlyphCoverage: Offset16<Coverage> = c.de()?;
        let vertGlyphCount: uint16 = c.de()?;
        let horizGlyphCount: uint16 = c.de()?;
        let vertGlyphConstruction: Vec<Offset16<MathGlyphConstruction>> =
            c.de_counted(vertGlyphCount.into())?;
        let horizGlyphConstruction: Vec<Offset16<MathGlyphConstruction>> =
            c.de_counted(horizGlyphCount.into())?;
        c.pop();
        Ok(MathVariants {
            minConnectorOverlap,
            vertGlyphCoverage,
            horizGlyphCoverage,
            vertGlyphCount,
            horizGlyphCount,
            vertGlyphConstruction,
            horizGlyphConstruction,
        })
    }
}

impl Serialize for MathVariants {
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        let mut v: Vec<&dyn OffsetMarkerTrait> =
            vec![&self.vertGlyphCoverage, &self.horizGlyphCoverage];
        for m in &self.vertGlyphConstruction {
            v.push(m);
        }
        for m in &self.horizGlyphConstruction {
            v.push(m);
        }
        v
    }
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        data.put(&self.minConnectorOverlap)?;
        data.put(&self.vertGlyphCoverage)?;
        data.put(&self.horizGlyphCoverage)?;
        data.put(&self.vertGlyphCount)?;
        data.put(&self.horizGlyphCount)?;
        data.put(&self.vertGlyphConstruction)?;
        data.put(&self.horizGlyphConstruction)?;
        Ok(())
    }
}
// Needs to be handled manually because of n+1 count in kernValues...
#[allow(missing_docs, non_snake_case)]
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct MathKern {
    pub heightCount: uint16,
    pub correctionHeight: Vec<MathValueRecord>,
    pub kernValues: Vec<MathValueRecord>,
}

impl Deserialize for MathKern {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut otspec::ReaderContext) -> Result<Self, otspec::DeserializationError> {
        let heightCount: uint16 = c.de()?;
        let correctionHeight: Vec<MathValueRecord> = c.de_counted(heightCount.into())?;
        let kernValues: Vec<MathValueRecord> = c.de_counted(1 + heightCount as usize)?;
        Ok(MathKern {
            heightCount,
            correctionHeight,
            kernValues,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
/// The Mathematical Typesetting table
pub struct MATH {
    /// Global constants for the mathematical typesetter
    pub constants: MathConstants,
    /// A set of italic correction information for each glyph
    pub italic_correction: BTreeMap<GlyphID, MathValueRecord>,
    /// A set of top accent attachment correction information for each glyph
    pub top_accent_attachment: BTreeMap<GlyphID, MathValueRecord>,
    /// Which glyph IDs are considered extended shapes
    pub extended_shapes: BTreeSet<GlyphID>,
    /// Math kerning information for each shape
    pub kerning: BTreeMap<GlyphID, MathKernInfoRecord>,
    /// The number of units by which two glyphs need to overlap with each other when used to construct a larger shape
    pub min_overlap: Option<UFWORD>,
    /// Information about glyphs which grow in the vertical direction
    pub vertical_extensions: BTreeMap<GlyphID, MathGlyphConstruction>,
    /// Information about glyphs which grow in the horizontal direction
    pub horizontal_extensions: BTreeMap<GlyphID, MathGlyphConstruction>,
}

impl Deserialize for MATH {
    fn from_bytes(c: &mut otspec::ReaderContext) -> Result<Self, otspec::DeserializationError> {
        let core: MATHinternal = c.de()?;
        let mut italic_correction: BTreeMap<GlyphID, MathValueRecord> = BTreeMap::new();
        let mut top_accent_attachment: BTreeMap<GlyphID, MathValueRecord> = BTreeMap::new();
        let mut extended_shapes: BTreeSet<GlyphID> = BTreeSet::new();
        let mut kerning: BTreeMap<GlyphID, MathKernInfoRecord> = BTreeMap::new();
        let mut vertical_extensions: BTreeMap<GlyphID, MathGlyphConstruction> = BTreeMap::new();
        let mut horizontal_extensions: BTreeMap<GlyphID, MathGlyphConstruction> = BTreeMap::new();
        if let Some(glyph_info) = core.mathGlyphInfo.link {
            if let Some(italic_correction_info) = glyph_info.mathItalicsCorrectionInfo.link {
                italic_correction = italic_correction_info
                    .italicsCorrectionCoverage
                    .as_ref()
                    .map(|x| x.glyphs.clone())
                    .iter()
                    .flatten()
                    .zip(italic_correction_info.italicsCorrection.iter())
                    .map(|(g, x)| (*g, x.clone()))
                    .collect();
            }

            if let Some(top_accent_attachment_info) = glyph_info.mathTopAccentAttachment.link {
                top_accent_attachment = top_accent_attachment_info
                    .topAccentCoverage
                    .as_ref()
                    .map(|x| x.glyphs.clone())
                    .iter()
                    .flatten()
                    .zip(top_accent_attachment_info.topAccentAttachment.iter())
                    .map(|(g, x)| (*g, x.clone()))
                    .collect();
            }

            if let Some(kerning_info) = glyph_info.mathKernInfo.link {
                kerning = kerning_info
                    .mathKernCoverage
                    .as_ref()
                    .map(|x| x.glyphs.clone())
                    .iter()
                    .flatten()
                    .zip(kerning_info.mathKernInfoRecords.iter())
                    .map(|(g, x)| (*g, x.clone()))
                    .collect();
            }

            for g in glyph_info
                .extendedShapeCoverage
                .link
                .as_ref()
                .map(|x| x.glyphs.clone())
                .iter()
                .flatten()
            {
                extended_shapes.insert(*g);
            }
        }

        if let Some(variant_info) = core.mathVariants.link.as_ref() {
            vertical_extensions = variant_info
                .vertGlyphCoverage
                .as_ref()
                .map(|x| x.glyphs.clone())
                .iter()
                .flatten()
                .zip(variant_info.vertGlyphConstruction.iter())
                .map(|(g, x)| (*g, x.link.clone()))
                .filter_map(|(g, x)| x.map(|x| (g, x)))
                .collect();
            horizontal_extensions = variant_info
                .horizGlyphCoverage
                .as_ref()
                .map(|x| x.glyphs.clone())
                .iter()
                .flatten()
                .zip(variant_info.horizGlyphConstruction.iter())
                .map(|(g, x)| (*g, x.link.clone()))
                .filter_map(|(g, x)| x.map(|x| (g, x)))
                .collect();
        }

        Ok(MATH {
            constants: core
                .mathConstants
                .as_ref()
                .ok_or_else(|| DeserializationError("No MATH constants table found".to_string()))?
                .clone(),
            italic_correction,
            top_accent_attachment,
            extended_shapes,
            kerning,
            min_overlap: core.mathVariants.as_ref().map(|x| x.minConnectorOverlap),
            vertical_extensions,
            horizontal_extensions,
        })
    }
}

#[cfg(test)]
mod tests {
    use otspec::btreemap;
    use std::iter::FromIterator;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_linux_libertine() {
        let math_binary = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0xe0, 0x00, 0xe8, 0x00, 0x50, 0x00, 0x3c,
            0x05, 0xdc, 0x04, 0xe2, 0x00, 0x96, 0x00, 0x00, 0x01, 0x22, 0x00, 0x00, 0x01, 0xe0,
            0x00, 0x00, 0x02, 0x87, 0x00, 0x00, 0x00, 0xd2, 0x00, 0x00, 0x01, 0x68, 0x00, 0x00,
            0x00, 0xa0, 0x00, 0x00, 0x01, 0x77, 0x00, 0x00, 0x01, 0x36, 0x00, 0x00, 0x00, 0x78,
            0x00, 0x00, 0x00, 0xe6, 0x00, 0x00, 0x00, 0x96, 0x00, 0x00, 0x01, 0x7c, 0x00, 0x00,
            0x00, 0x2b, 0x00, 0x00, 0x00, 0x41, 0x00, 0x00, 0x00, 0xfa, 0x00, 0x00, 0x00, 0x41,
            0x00, 0x00, 0x02, 0x6c, 0x00, 0x00, 0x01, 0xd6, 0x00, 0x00, 0x02, 0xee, 0x00, 0x00,
            0x01, 0x7c, 0x00, 0x00, 0x02, 0xbc, 0x00, 0x00, 0x00, 0xc8, 0x00, 0x00, 0x01, 0x4a,
            0x00, 0x00, 0x03, 0x20, 0x00, 0x00, 0x02, 0x58, 0x00, 0x00, 0x00, 0x41, 0x00, 0x00,
            0x00, 0x41, 0x00, 0x00, 0x02, 0x58, 0x00, 0x00, 0x03, 0x20, 0x00, 0x00, 0x02, 0x26,
            0x00, 0x00, 0x02, 0xbc, 0x00, 0x00, 0x00, 0x41, 0x00, 0x00, 0x00, 0x82, 0x00, 0x00,
            0x00, 0x41, 0x00, 0x00, 0x00, 0x41, 0x00, 0x00, 0x00, 0x82, 0x00, 0x00, 0x01, 0x90,
            0x00, 0x00, 0x00, 0x41, 0x00, 0x00, 0x00, 0x78, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00,
            0x00, 0x28, 0x00, 0x00, 0x00, 0x78, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x28,
            0x00, 0x00, 0x00, 0x5a, 0x00, 0x00, 0x00, 0xaa, 0x00, 0x00, 0x00, 0x41, 0x00, 0x00,
            0x00, 0x41, 0x00, 0x00, 0x00, 0x41, 0x00, 0x00, 0xfe, 0xc0, 0x00, 0x00, 0x00, 0x21,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x9c, 0x00, 0x00,
            0x00, 0x03, 0x00, 0x00, 0x00, 0x10, 0x00, 0x14, 0x00, 0x18, 0x00, 0x0c, 0x00, 0x00,
            0x00, 0x2c, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
            0x08, 0x0a, 0x00, 0x00, 0x00, 0x88, 0x03, 0x7e, 0x00, 0x00, 0x08, 0x09, 0x03, 0x7d,
            0x03, 0x7d, 0x03, 0x7e, 0x00, 0x01, 0x08, 0x08, 0x00, 0x88, 0x00, 0x00, 0x03, 0x7e,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x08, 0x16, 0x00, 0x00, 0x00, 0x8a,
            0x01, 0xb0, 0x00, 0x00, 0x08, 0x17, 0x01, 0xaf, 0x01, 0xaf, 0x01, 0xb0, 0x00, 0x01,
            0x08, 0x15, 0x00, 0x3a, 0x00, 0x3a, 0x03, 0x7e, 0x00, 0x00, 0x08, 0x17, 0x01, 0xaf,
            0x01, 0xaf, 0x01, 0xb0, 0x00, 0x01, 0x08, 0x14, 0x00, 0x8a, 0x00, 0x00, 0x01, 0xb0,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x00, 0x00, 0xd4,
            0x03, 0x7d, 0x00, 0x00, 0x08, 0x1b, 0x03, 0x7d, 0x03, 0x7d, 0x03, 0x7d, 0x00, 0x01,
            0x07, 0xff, 0x00, 0xd4, 0x00, 0x00, 0x03, 0x7d, 0x00, 0x00, 0x00, 0x01, 0x00, 0x03,
            0x00, 0x09, 0x00, 0x5c, 0x07, 0xd3,
        ];
        let math: MATH = otspec::de::from_bytes(&math_binary).unwrap();
        let paren_left = MathGlyphConstruction {
            glyphAssembly: Offset16::to(GlyphAssembly {
                italicsCorrection: MathValueRecord::new(0),
                partRecords: vec![
                    GlyphPartRecord {
                        glyphID: 2058,
                        startConnectorLength: 0,
                        endConnectorLength: 136,
                        fullAdvance: 894,
                        partFlags: 0,
                    },
                    GlyphPartRecord {
                        glyphID: 2057,
                        startConnectorLength: 893,
                        endConnectorLength: 893,
                        fullAdvance: 894,
                        partFlags: 1,
                    },
                    GlyphPartRecord {
                        glyphID: 2056,
                        startConnectorLength: 136,
                        endConnectorLength: 0,
                        fullAdvance: 894,
                        partFlags: 0,
                    },
                ],
            }),
            mathGlyphVariantRecord: vec![],
        };
        let brace_left = MathGlyphConstruction {
            glyphAssembly: Offset16::to(GlyphAssembly {
                italicsCorrection: MathValueRecord::new(0),
                partRecords: vec![
                    GlyphPartRecord {
                        glyphID: 2070,
                        startConnectorLength: 0,
                        endConnectorLength: 138,
                        fullAdvance: 432,
                        partFlags: 0,
                    },
                    GlyphPartRecord {
                        glyphID: 2071,
                        startConnectorLength: 431,
                        endConnectorLength: 431,
                        fullAdvance: 432,
                        partFlags: 1,
                    },
                    GlyphPartRecord {
                        glyphID: 2069,
                        startConnectorLength: 58,
                        endConnectorLength: 58,
                        fullAdvance: 894,
                        partFlags: 0,
                    },
                    GlyphPartRecord {
                        glyphID: 2071,
                        startConnectorLength: 431,
                        endConnectorLength: 431,
                        fullAdvance: 432,
                        partFlags: 1,
                    },
                    GlyphPartRecord {
                        glyphID: 2068,
                        startConnectorLength: 138,
                        endConnectorLength: 0,
                        fullAdvance: 432,
                        partFlags: 0,
                    },
                ],
            }),
            mathGlyphVariantRecord: vec![],
        };

        let integral = MathGlyphConstruction {
            glyphAssembly: Offset16::to(GlyphAssembly {
                italicsCorrection: MathValueRecord::new(0),
                partRecords: vec![
                    GlyphPartRecord {
                        glyphID: 2048,
                        startConnectorLength: 0,
                        endConnectorLength: 212,
                        fullAdvance: 893,
                        partFlags: 0,
                    },
                    GlyphPartRecord {
                        glyphID: 2075,
                        startConnectorLength: 893,
                        endConnectorLength: 893,
                        fullAdvance: 893,
                        partFlags: 1,
                    },
                    GlyphPartRecord {
                        glyphID: 2047,
                        startConnectorLength: 212,
                        endConnectorLength: 0,
                        fullAdvance: 893,
                        partFlags: 0,
                    },
                ],
            }),
            mathGlyphVariantRecord: vec![],
        };

        assert_eq!(
            math,
            MATH {
                constants: MathConstants {
                    scriptPercentScaleDown: 80,
                    scriptScriptPercentScaleDown: 60,
                    delimitedSubFormulaMinHeight: 1500,
                    displayOperatorMinHeight: 1250,
                    mathLeading: MathValueRecord::new(150),
                    axisHeight: MathValueRecord::new(290),
                    accentBaseHeight: MathValueRecord::new(480),
                    flattenedAccentBaseHeight: MathValueRecord::new(647),
                    subscriptShiftDown: MathValueRecord::new(210),
                    subscriptTopMax: MathValueRecord::new(360),
                    subscriptBaselineDropMin: MathValueRecord::new(160),
                    superscriptShiftUp: MathValueRecord::new(375),
                    superscriptShiftUpCramped: MathValueRecord::new(310),
                    superscriptBottomMin: MathValueRecord::new(120),
                    superscriptBaselineDropMax: MathValueRecord::new(230),
                    subSuperscriptGapMin: MathValueRecord::new(150),
                    superscriptBottomMaxWithSubscript: MathValueRecord::new(380),
                    spaceAfterScript: MathValueRecord::new(43),
                    upperLimitGapMin: MathValueRecord::new(65),
                    upperLimitBaselineRiseMin: MathValueRecord::new(250),
                    lowerLimitGapMin: MathValueRecord::new(65),
                    lowerLimitBaselineDropMin: MathValueRecord::new(620),
                    stackTopShiftUp: MathValueRecord::new(470),
                    stackTopDisplayStyleShiftUp: MathValueRecord::new(750),
                    stackBottomShiftDown: MathValueRecord::new(380),
                    stackBottomDisplayStyleShiftDown: MathValueRecord::new(700),
                    stackGapMin: MathValueRecord::new(200),
                    stackDisplayStyleGapMin: MathValueRecord::new(330),
                    stretchStackTopShiftUp: MathValueRecord::new(800),
                    stretchStackBottomShiftDown: MathValueRecord::new(600),
                    stretchStackGapAboveMin: MathValueRecord::new(65),
                    stretchStackGapBelowMin: MathValueRecord::new(65),
                    fractionNumeratorShiftUp: MathValueRecord::new(600),
                    fractionNumeratorDisplayStyleShiftUp: MathValueRecord::new(800),
                    fractionDenominatorShiftDown: MathValueRecord::new(550),
                    fractionDenominatorDisplayStyleShiftDown: MathValueRecord::new(700),
                    fractionNumeratorGapMin: MathValueRecord::new(65),
                    fractionNumDisplayStyleGapMin: MathValueRecord::new(130),
                    fractionRuleThickness: MathValueRecord::new(65),
                    fractionDenominatorGapMin: MathValueRecord::new(65),
                    fractionDenomDisplayStyleGapMin: MathValueRecord::new(130),
                    skewedFractionHorizontalGap: MathValueRecord::new(400),
                    skewedFractionVerticalGap: MathValueRecord::new(65),
                    overbarVerticalGap: MathValueRecord::new(120),
                    overbarRuleThickness: MathValueRecord::new(40),
                    overbarExtraAscender: MathValueRecord::new(40),
                    underbarVerticalGap: MathValueRecord::new(120),
                    underbarRuleThickness: MathValueRecord::new(40),
                    underbarExtraDescender: MathValueRecord::new(40),
                    radicalVerticalGap: MathValueRecord::new(90),
                    radicalDisplayStyleVerticalGap: MathValueRecord::new(170),
                    radicalRuleThickness: MathValueRecord::new(65),
                    radicalExtraAscender: MathValueRecord::new(65),
                    radicalKernBeforeDegree: MathValueRecord::new(65),
                    radicalKernAfterDegree: MathValueRecord::new(-320),
                    radicalDegreeBottomRaisePercent: 33,
                },
                italic_correction: BTreeMap::new(),
                top_accent_attachment: BTreeMap::new(),
                extended_shapes: BTreeSet::new(),
                kerning: BTreeMap::new(),
                min_overlap: Some(100),
                vertical_extensions: btreemap!(
                    9 => paren_left,
                    92 => brace_left,
                    2003 => integral
                ),
                horizontal_extensions: BTreeMap::new(),
            },
        )
    }
}
