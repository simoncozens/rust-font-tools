use std::collections::BTreeMap;

mod curlybraceserde;
use curlybraceserde::{
    deserialize_commify, serialize_commify, CropRectReceiver, CropRectVisitor, CurlyBraceReceiver,
    CurlyBraceVisitor,
};

use openstep_plist::Dictionary;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, OneOrMany};

use crate::common::{
    bool_true, is_default, is_false, is_scale_unit, is_true, scale_unit, Color, CustomParameter,
    Feature, FeatureClass, FeaturePrefix, GuideAlignment, Kerning, NodeType,
};

fn is_one_hundred(value: &i32) -> bool {
    *value == 100
}

fn one_hundred() -> i32 {
    100
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Glyphs2 {
    /// The build number of the app
    #[serde(rename = ".AppVersion", default)]
    pub app_version: String,
    /// List of strings used in the edit window
    #[serde(
        rename = "DisplayStrings",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub display_strings: Vec<String>,
    /// OpenType classes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<FeatureClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    /// Font-wide custom parameters
    #[serde(default, rename = "customParameters")]
    pub custom_parameters: Vec<CustomParameter>,
    /// Font creation date. Format `2014-01-29 14:14:38 +0000`.
    #[serde(default)]
    pub date: String,
    /// The designer of the font
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designer: Option<String>,
    /// The designer URL
    #[serde(
        default,
        rename = "designerURL",
        skip_serializing_if = "Option::is_none"
    )]
    pub designer_url: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "is_true",
        rename = "disablesAutomaticAlignment"
    )]
    pub disables_automatic_alignment: bool,
    #[serde(default, skip_serializing_if = "is_true", rename = "disablesNiceNames")]
    pub disables_nice_names: bool,
    /// The family name of the font
    #[serde(rename = "familyName")]
    pub family_name: String,
    /// OpenType feature code before the class definitions.
    #[serde(
        default,
        rename = "featurePrefixes",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub feature_prefixes: Vec<FeaturePrefix>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<Feature>,
    /// Masters
    #[serde(rename = "fontMaster", skip_serializing_if = "Vec::is_empty", default)]
    pub masters: Vec<Master>,
    /// Glyphs
    #[serde(default)]
    pub glyphs: Vec<Glyph>,
    #[serde(rename = "gridLength", skip_serializing_if = "Option::is_none")]
    pub grid_length: Option<i32>,
    #[serde(rename = "gridSubDivision", skip_serializing_if = "Option::is_none")]
    pub grid_sub_division: Option<i32>,

    /// Instances
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<Instance>,
    #[serde(rename = "keepAlternatesTogether", default)]
    pub keep_alternates_together: bool,
    /// Three-level dict containing a float as value.
    #[serde(default, skip_serializing_if = "Kerning::is_empty")]
    pub kerning: Kerning,
    #[serde(
        rename = "vertKerning",
        default,
        skip_serializing_if = "Kerning::is_empty"
    )]
    pub kerning_vertical: Kerning,
    #[serde(rename = "keyboardIncrement", skip_serializing_if = "Option::is_none")]
    pub keyboard_increment: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "manufacturerURL"
    )]
    pub manufacturer_url: Option<String>,
    #[serde(rename = "unitsPerEm")]
    pub units_per_em: i32,
    #[serde(rename = "userData", default, skip_serializing_if = "is_default")]
    pub user_data: Dictionary,
    #[serde(rename = "versionMajor", default)]
    pub version_major: i32,
    #[serde(rename = "versionMinor", default)]
    pub version_minor: i32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Master {
    #[serde(
        rename = "alignmentZones",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub alignment_zones: Vec<AlignmentZone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ascender: Option<f32>,
    #[serde(rename = "capHeight", default, skip_serializing_if = "Option::is_none")]
    pub cap_height: Option<f32>,
    ///  All other parts of the master name that doesn’t fit into weight or width
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<String>,
    /// Axis position for the third axis.
    ///
    /// Is only present if the value is not 0.
    #[serde(default, skip_serializing_if = "is_default", rename = "customValue")]
    pub custom_value: i32,
    /// Axis position for the fourth axis.
    ///
    /// Is only present if the value is not 0.
    #[serde(default, skip_serializing_if = "is_default", rename = "customValue1")]
    pub custom_value_1: i32,
    /// Axis position for the fifth axis.
    ///
    /// Is only present if the value is not 0.
    #[serde(default, skip_serializing_if = "is_default", rename = "customValue2")]
    pub custom_value_2: i32,
    /// Axis position for the sixth axis.
    ///
    /// Is only present if the value is not 0.
    #[serde(default, skip_serializing_if = "is_default", rename = "customValue3")]
    pub custom_value_3: i32,
    /// Master-wide custom parameters
    #[serde(rename = "customParameters", default)]
    pub custom_parameters: Vec<CustomParameter>,
    /// The descender of the master
    ///
    /// Is always negative
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub descender: Option<f32>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "horizontalStems"
    )]
    pub horizontal_stems: Vec<i32>,
    /// Stores the selected master icon
    #[serde(rename = "iconName", default)]
    pub icon_name: String,
    /// A unique id that connects the layers (associated ID) with the master
    pub id: String,
    #[serde(rename = "userData", default)]
    pub user_data: Dictionary,
    #[serde(
        rename = "verticalStems",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub vertical_stems: Vec<i32>,
    #[serde(default = "bool_true", skip_serializing_if = "is_true")]
    pub visible: bool,
    #[serde(default)]
    pub weight: String,
    #[serde(
        default = "one_hundred",
        skip_serializing_if = "is_one_hundred",
        rename = "weightValue"
    )]
    pub weight_value: i32,
    #[serde(default)]
    pub width: String,
    #[serde(
        default = "one_hundred",
        skip_serializing_if = "is_one_hundred",
        rename = "widthValue"
    )]
    pub width_value: i32,
    #[serde(default, rename = "xHeight")]
    pub x_height: Option<f32>,
}

// "position and overshot in a string with curly braces (e.g. "{800, 15}")"
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlignmentZone {
    pub position: f32,
    pub overshoot: f32,
}

impl<'de> Deserialize<'de> for AlignmentZone {
    fn deserialize<D>(deserializer: D) -> Result<AlignmentZone, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(CurlyBraceVisitor::<2, AlignmentZone>::default())
    }
}
impl Serialize for AlignmentZone {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_commify(self, serializer)
    }
}

impl CurlyBraceReceiver<f32, 2> for AlignmentZone {
    fn new(values: [f32; 2]) -> Self {
        AlignmentZone {
            position: values[0],
            overshoot: values[1],
        }
    }
    fn as_parts(&self) -> [f32; 2] {
        [self.position, self.overshoot]
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Instance {
    /// Instance custom parameters
    #[serde(rename = "customParameters", default)]
    pub custom_parameters: Vec<CustomParameter>,
    /// Is only present if the value is not 0.
    #[serde(default = "bool_true", skip_serializing_if = "is_true")]
    pub exports: bool,
    /// axis position for the third axis
    #[serde(
        default,
        skip_serializing_if = "is_default",
        rename = "interpolationCustom"
    )]
    pub custom_value: f32,
    /// axis position for the fourth axis
    #[serde(
        default,
        skip_serializing_if = "is_default",
        rename = "interpolationCustom1"
    )]
    pub custom_value_1: f32,
    /// axis position for the fifth axis
    #[serde(
        default,
        skip_serializing_if = "is_default",
        rename = "interpolationCustom2"
    )]
    pub custom_value_2: f32,
    /// axis position for the sixth axis
    #[serde(
        default,
        skip_serializing_if = "is_default",
        rename = "interpolationCustom3"
    )]
    pub custom_value_3: f32,
    /// axis position for the first axis
    #[serde(
        default,
        skip_serializing_if = "is_default",
        rename = "interpolationWeight"
    )]
    pub weight_value: f32,
    /// axis position for the second axis
    #[serde(
        default,
        skip_serializing_if = "is_default",
        rename = "interpolationWidth"
    )]
    pub width_value: f32,
    /// Instance interpolations
    ///
    /// keys are master IDs, values are the factors for that master.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        rename = "instanceInterpolations"
    )]
    pub instance_interpolations: BTreeMap<String, f32>,
    /// For style linking. Always set to 1, otherwise omit the key.
    #[serde(default, rename = "isBold", skip_serializing_if = "is_false")]
    pub is_bold: bool,
    /// For style linking. Always set to 1, otherwise omit the key.
    #[serde(default, rename = "isItalic", skip_serializing_if = "is_false")]
    pub is_italic: bool,
    #[serde(default, rename = "isRegular")]
    pub link_style: Option<String>,
    /// If set, use the instanceInterpolations, otherwise calculate from axisValues.
    ///
    /// Always set to 1, otherwise omit the key.
    #[serde(
        default,
        rename = "manualInterpolation",
        skip_serializing_if = "is_false"
    )]
    pub manual_interpolation: bool,
    /// The style name
    pub name: String,
    #[serde(default, rename = "userData", skip_serializing_if = "is_default")]
    pub user_data: Dictionary,
    #[serde(
        default,
        rename = "weightClass",
        skip_serializing_if = "Option::is_none"
    )]
    pub weight_class: Option<String>,
    #[serde(
        default,
        rename = "widthClass",
        skip_serializing_if = "Option::is_none"
    )]
    pub width_class: Option<String>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Glyph {
    /// The glyph name
    #[serde(rename = "glyphname")]
    pub name: String,
    #[serde(default)]
    pub production: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
    /// Manually set category
    #[serde(default)]
    pub category: Option<String>,
    /// The color of the glyph in the interface
    #[serde(default)]
    pub color: Option<Color>,
    /// Export
    #[serde(default = "bool_true", skip_serializing_if = "is_true")]
    pub export: bool,
    /// Left kerning group
    #[serde(rename = "leftKerningGroup")]
    pub kern_left: Option<String>,
    /// Right kerning group
    #[serde(rename = "rightKerningGroup")]
    pub kern_right: Option<String>,
    /// Top kerning group
    #[serde(rename = "kernTop")]
    pub kern_top: Option<String>,
    /// Format 2014-01-29 14:14:38 +0000
    #[serde(rename = "lastChange")]
    pub last_change: Option<String>,
    pub layers: Vec<Layer>,

    #[serde(
        default,
        deserialize_with = "deserialize_comma_hexstring",
        serialize_with = "serialize_comma_hexstring"
    )]
    pub unicode: Option<Vec<u32>>,
}

fn serialize_comma_hexstring<S>(value: &Option<Vec<u32>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut seq = serializer.serialize_seq(None)?;
    if let Some(v) = value {
        for (ix, i) in v.iter().enumerate() {
            seq.serialize_element(&format!("{:04X}", i))?;
            if ix < v.len() - 1 {
                seq.serialize_element(",")?;
            }
        }
    }
    seq.end()
}

fn deserialize_comma_hexstring<'de, D>(deserializer: D) -> Result<Option<Vec<u32>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let codepoints = s.split(',');
    let mut result = Vec::new();
    for codepoint in codepoints {
        result.push(u32::from_str_radix(codepoint, 16).map_err(serde::de::Error::custom)?);
    }
    Ok(Some(result))
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Layer {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<Anchor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Dictionary>,

    /// ID of the master the layer is linked to
    ///
    /// Not present if it equals layerID, i.e. if the layer is in use as master.
    #[serde(
        rename = "associatedMasterId",
        default,
        skip_serializing_if = "is_default"
    )]
    pub associated_master_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Box<Layer>>,
    #[serde(
        rename = "backgroundImage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub background_image: Option<BackgroundImage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guidelines: Vec<Guide>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<Hint>,
    /// The unique id of the layer
    #[serde(rename = "layerId", default, skip_serializing_if = "String::is_empty")]
    pub layer_id: String,
    /// Left metric key
    #[serde(rename = "leftMetricsKey", default, skip_serializing_if = "is_default")]
    pub metric_left: Option<String>,
    /// Right metric key
    #[serde(
        rename = "rightMetricsKey",
        default,
        skip_serializing_if = "is_default"
    )]
    pub metric_right: Option<String>,
    /// Horizontal width metric key
    #[serde(
        rename = "widthMetricsKey",
        default,
        skip_serializing_if = "is_default"
    )]
    pub metric_width: Option<String>,
    /// The name of the layer.
    ///
    /// Only stored for non-master layers (this is changed in 2.3, before the master names where stored)
    #[serde(default, skip_serializing_if = "is_default")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<Path>,
    #[serde(rename = "userData", default)]
    pub user_data: Dictionary,
    /// Vertical width
    ///
    /// Only stored if other than the default (ascender+descender)
    #[serde(rename = "vertWidth", default)]
    pub vert_width: Option<f32>,
    /// Layer width
    #[serde(default)]
    pub width: f32,
    /// The visibility setting in the layer panel (the eye symbol).
    #[serde(default = "bool_true", skip_serializing_if = "is_true")]
    pub visible: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Anchor {
    /// The anchor name
    pub name: String,
    /// The anchor position
    #[serde(
        serialize_with = "serialize_commify",
        deserialize_with = "deserialize_commify"
    )]
    pub position: (f32, f32),
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct BackgroundImage {
    /// Portion of the image to show in pixels, format: {{t,l},{b,r}}
    pub crop: CropRect,
    /// The file path to the image.
    ///
    /// It is stored relative if close enough. Otherwise the full path.
    #[serde(rename = "imagePath", default)]
    pub image_path: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
    pub transform: Transform,
}

// Another nasty curly brace thing
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Transform {
    pub m11: f32,
    pub m12: f32,
    pub m21: f32,
    pub m22: f32,
    pub t_x: f32,
    pub t_y: f32,
}

impl<'de> Deserialize<'de> for Transform {
    fn deserialize<D>(deserializer: D) -> Result<Transform, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(CurlyBraceVisitor::<6, Transform>::default())
    }
}
impl Serialize for Transform {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_commify(self, serializer)
    }
}

impl CurlyBraceReceiver<f32, 6> for Transform {
    fn new(values: [f32; 6]) -> Self {
        Transform {
            m11: values[0],
            m12: values[1],
            m21: values[2],
            m22: values[3],
            t_x: values[4],
            t_y: values[5],
        }
    }
    fn as_parts(&self) -> [f32; 6] {
        [self.m11, self.m12, self.m21, self.m22, self.t_x, self.t_y]
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CropRect {
    top: i32,
    left: i32,
    bottom: i32,
    right: i32,
}
impl<'de> Deserialize<'de> for CropRect {
    fn deserialize<D>(deserializer: D) -> Result<CropRect, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(CropRectVisitor::<CropRect>::default())
    }
}

impl CropRectReceiver for CropRect {
    fn new(top: i32, left: i32, bottom: i32, right: i32) -> Self {
        CropRect {
            top,
            left,
            bottom,
            right,
        }
    }
}
impl Serialize for CropRect {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&format!(
            "{{{{{},{}}},{{{}, {}}}}}",
            self.top, self.left, self.bottom, self.right
        ))?;
        seq.end()
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Component {
    /// Should be indicated if connected to an anchor, especially if more than one possibility is available, e.g. in ligatures.
    #[serde(default, skip_serializing_if = "is_default")]
    pub anchor: Option<String>,
    #[serde(rename = "name")]
    pub component_glyph: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub transform: Transform,
    #[serde(default, skip_serializing_if = "is_default")]
    pub alignment: i8,
    #[serde(
        default,
        skip_serializing_if = "is_default",
        rename = "disableAlignment"
    )]
    pub disable_alignment: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Guide {
    #[serde(default)]
    pub alignment: GuideAlignment,
    #[serde(default)]
    pub angle: f32,
    #[serde(default)]
    pub locked: bool,
    pub pos: (f32, f32),
    #[serde(
        default = "scale_unit",
        serialize_with = "serialize_commify",
        deserialize_with = "deserialize_commify",
        skip_serializing_if = "is_scale_unit"
    )]
    pub scale: (f32, f32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hint {
    #[serde(default, skip_serializing_if = "is_default")]
    pub horizontal: bool,
    #[serde(default, rename = "type")]
    pub type_: String,
    #[serde(
        default,
        skip_serializing_if = "is_default",
        serialize_with = "serialize_commify",
        deserialize_with = "deserialize_commify"
    )]
    pub origin: (f32, f32),
    #[serde(
        default,
        skip_serializing_if = "is_default",
        serialize_with = "serialize_commify",
        deserialize_with = "deserialize_commify"
    )]
    pub target: (f32, f32),
    #[serde(
        default,
        skip_serializing_if = "is_default",
        serialize_with = "serialize_commify",
        deserialize_with = "deserialize_commify"
    )]
    pub other1: (f32, f32),
    #[serde(
        default,
        skip_serializing_if = "is_default",
        serialize_with = "serialize_commify",
        deserialize_with = "deserialize_commify"
    )]
    pub other2: (f32, f32),
    #[serde(
        default,
        skip_serializing_if = "is_default",
        serialize_with = "serialize_commify",
        deserialize_with = "deserialize_commify"
    )]
    pub scale: (f32, f32),
    #[serde(default, skip_serializing_if = "is_default")]
    pub stem: bool,
    pub options: i8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Path {
    #[serde(default, skip_serializing_if = "is_false")]
    pub closed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub x: f32,
    pub y: f32,
    pub node_type: NodeType,
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // string X Y (full node type) (SMOOTH)?
        let node_type = match self.node_type {
            NodeType::Line => "LINE",
            NodeType::Curve => "CURVE",
            NodeType::QCurve => "QCURVE",
            NodeType::OffCurve => "OFFCURVE",
            NodeType::LineSmooth => "LINE SMOOTH",
            NodeType::CurveSmooth => "CURVE SMOOTH",
            NodeType::QCurveSmooth => "QCURVE SMOOTH",
        };
        let node_str = format!("{} {} {}", self.x, self.y, node_type);
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&node_str)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Node, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(NodeVisitor)
    }
}

#[derive(Debug, Default, Clone)]
struct NodeVisitor;

impl<'de> Visitor<'de> for NodeVisitor {
    type Value = Node;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string with node data")
    }

    fn visit_str<E>(self, value: &str) -> Result<Node, E>
    where
        E: serde::de::Error,
    {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(E::custom("not enough parts"));
        }
        let x = parts[0]
            .parse::<f32>()
            .map_err(|_| E::custom("could not parse x"))?;
        let y = parts[1]
            .parse::<f32>()
            .map_err(|_| E::custom("could not parse y"))?;
        let smooth = parts.len() > 3 && parts[3] == "SMOOTH";
        let node_type = match (parts[2], smooth) {
            ("LINE", false) => NodeType::Line,
            ("CURVE", false) => NodeType::Curve,
            ("QCURVE", false) => NodeType::QCurve,
            ("OFFCURVE", false) => NodeType::OffCurve,
            ("LINE", true) => NodeType::LineSmooth,
            ("CURVE", true) => NodeType::CurveSmooth,
            ("QCURVE", true) => NodeType::QCurveSmooth,
            _ => return Err(E::custom("unknown node type")),
        };
        Ok(Node { x, y, node_type })
    }
}
