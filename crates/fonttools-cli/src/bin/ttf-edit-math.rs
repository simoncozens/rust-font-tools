use clap::{App, Arg};
use fonttools::font::Table;
use fonttools::types::*;
use fonttools::MATH::*;
use fonttools_cli::open_font;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io;

fn get_math_record<S>(mvr: &MathValueRecord, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i16(mvr.value)
}

#[derive(Serialize)]
#[serde(remote = "MathConstants")]
#[allow(non_snake_case)]
struct MathConstantsDef {
    scriptPercentScaleDown: int16,
    scriptScriptPercentScaleDown: int16,
    delimitedSubFormulaMinHeight: UFWORD,
    displayOperatorMinHeight: UFWORD,
    #[serde(serialize_with = "get_math_record")]
    mathLeading: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    axisHeight: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    accentBaseHeight: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    flattenedAccentBaseHeight: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    subscriptShiftDown: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    subscriptTopMax: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    subscriptBaselineDropMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    superscriptShiftUp: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    superscriptShiftUpCramped: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    superscriptBottomMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    superscriptBaselineDropMax: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    subSuperscriptGapMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    superscriptBottomMaxWithSubscript: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    spaceAfterScript: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    upperLimitGapMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    upperLimitBaselineRiseMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    lowerLimitGapMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    lowerLimitBaselineDropMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stackTopShiftUp: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stackTopDisplayStyleShiftUp: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stackBottomShiftDown: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stackBottomDisplayStyleShiftDown: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stackGapMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stackDisplayStyleGapMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stretchStackTopShiftUp: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stretchStackBottomShiftDown: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stretchStackGapAboveMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    stretchStackGapBelowMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    fractionNumeratorShiftUp: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    fractionNumeratorDisplayStyleShiftUp: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    fractionDenominatorShiftDown: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    fractionDenominatorDisplayStyleShiftDown: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    fractionNumeratorGapMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    fractionNumDisplayStyleGapMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    fractionRuleThickness: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    fractionDenominatorGapMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    fractionDenomDisplayStyleGapMin: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    skewedFractionHorizontalGap: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    skewedFractionVerticalGap: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    overbarVerticalGap: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    overbarRuleThickness: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    overbarExtraAscender: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    underbarVerticalGap: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    underbarRuleThickness: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    underbarExtraDescender: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    radicalVerticalGap: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    radicalDisplayStyleVerticalGap: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    radicalRuleThickness: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    radicalExtraAscender: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    radicalKernBeforeDegree: MathValueRecord,
    #[serde(serialize_with = "get_math_record")]
    radicalKernAfterDegree: MathValueRecord,
    radicalDegreeBottomRaisePercent: int16,
}

#[derive(Serialize)]
struct SimpleKernRecord {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub correction_height: Vec<i16>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub kern_values: Vec<i16>,
}

impl From<&MathKern> for SimpleKernRecord {
    fn from(x: &MathKern) -> Self {
        SimpleKernRecord {
            correction_height: x.correctionHeight.iter().map(|x| x.value).collect(),
            kern_values: x.kernValues.iter().map(|x| x.value).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct SimpleMathKern {
    #[serde(skip_serializing_if = "Option::is_none")]
    top_right: Option<SimpleKernRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bottom_left: Option<SimpleKernRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bottom_right: Option<SimpleKernRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_left: Option<SimpleKernRecord>,
}

impl From<&MathKernInfoRecord> for SimpleMathKern {
    fn from(x: &MathKernInfoRecord) -> Self {
        SimpleMathKern {
            top_left: x.topLeftMathKern.link.as_ref().map(|x| x.into()),
            top_right: x.topRightMathKern.link.as_ref().map(|x| x.into()),
            bottom_left: x.bottomLeftMathKern.link.as_ref().map(|x| x.into()),
            bottom_right: x.bottomRightMathKern.link.as_ref().map(|x| x.into()),
        }
    }
}

impl SimpleMathKern {
    fn all_none(&self) -> bool {
        self.bottom_left.is_none()
            && self.bottom_right.is_none()
            && self.top_right.is_none()
            && self.top_left.is_none()
    }
}

fn get_mgc_map<S>(
    mgc: &BTreeMap<String, MathGlyphConstruction>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i16(1)
}

fn get_mvr_map<S>(mgc: &BTreeMap<String, MathValueRecord>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i16(1)
}

#[derive(Serialize)]
pub struct MathEvenEasier {
    #[serde(with = "MathConstantsDef")]
    pub constants: MathConstants,
    pub italic_correction: BTreeMap<String, int16>,
    pub top_accent_attachment: BTreeMap<String, int16>,
    pub extended_shapes: BTreeSet<String>,
    pub kerning: BTreeMap<String, SimpleMathKern>,
    pub min_overlap: Option<UFWORD>,
    #[serde(serialize_with = "get_mgc_map")]
    pub vertical_extensions: BTreeMap<String, MathGlyphConstruction>,
    #[serde(serialize_with = "get_mgc_map")]
    pub horizontal_extensions: BTreeMap<String, MathGlyphConstruction>,
}

fn simplify(math: &MATH, glyph_names: Vec<String>) -> MathEvenEasier {
    MathEvenEasier {
        constants: math.constants.clone(),
        min_overlap: math.min_overlap,
        extended_shapes: math
            .extended_shapes
            .iter()
            .map(|gid| {
                glyph_names
                    .get(*gid as usize)
                    .cloned()
                    .unwrap_or_else(|| format!("\\{:}", gid))
            })
            .collect(),
        italic_correction: math
            .italic_correction
            .iter()
            .map(|(gid, corr)| {
                (
                    glyph_names
                        .get(*gid as usize)
                        .cloned()
                        .unwrap_or_else(|| format!("\\{:}", gid)),
                    corr.value,
                )
            })
            .collect(),
        top_accent_attachment: math
            .top_accent_attachment
            .iter()
            .map(|(gid, corr)| {
                (
                    glyph_names
                        .get(*gid as usize)
                        .cloned()
                        .unwrap_or_else(|| format!("\\{:}", gid)),
                    corr.value,
                )
            })
            .collect(),
        kerning: math
            .kerning
            .iter()
            .map(|(gid, corr)| {
                let simple: SimpleMathKern = corr.into();
                (
                    glyph_names
                        .get(*gid as usize)
                        .cloned()
                        .unwrap_or_else(|| format!("\\{:}", gid)),
                    simple,
                )
            })
            .collect(),
        horizontal_extensions: BTreeMap::new(),
        vertical_extensions: BTreeMap::new(),
    }
}

fn main() {
    env_logger::init();
    let matches = App::new("ttf-edit-math")
        .about("Dumps and loads the MATH table")
        .arg(
            Arg::with_name("mode")
                .possible_values(&["dump", "load"])
                .required(true),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Font file to open")
                .required(true),
        )
        .get_matches();
    let mut infont = open_font(&matches);

    let glyph_names = if let Table::Post(post) = infont
        .get_table(tag!("post"))
        .expect("Error reading post table")
        .expect("No post table found")
    {
        post.glyphnames.clone().unwrap_or_else(std::vec::Vec::new)
    } else {
        vec![]
    };

    if matches.value_of("mode").unwrap() == "dump" {
        let math = infont
            .get_table(tag!("MATH"))
            .expect("No math table")
            .expect("Couldn't parse MATH table")
            .MATH_unchecked();
        let simplified: MathEvenEasier = simplify(math, glyph_names);
        serde_json::to_writer_pretty(io::stdout(), &simplified).expect("Oops");
    }
    // save_font(infont, &matches);
}
