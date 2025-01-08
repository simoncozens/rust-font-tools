use std::collections::BTreeMap;
use std::fmt;

use openstep_plist::{Dictionary, Plist};
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{serde_as, OneOrMany};

use crate::common::{
    bool_true, is_default, is_false, is_true, scale_unit, Color, CustomParameter, Feature,
    FeatureClass, FeaturePrefix, GuideAlignment, Kerning, NodeType,
};

pub(crate) fn version_two() -> i32 {
    2
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Glyphs3 {
    /// The build number of the app
    #[serde(
        rename = ".appVersion",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub app_version: String,
    /// Set to 3 for version 3. If that key is missing assume version 2.
    #[serde(rename = ".formatVersion", default = "version_two")]
    pub format_version: i32,
    /// List of strings used in the edit window
    #[serde(rename = "DisplayStrings", skip_serializing_if = "is_default", default)]
    pub display_strings: Vec<String>,
    /// The interpolation axes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub axes: Vec<Axis>,
    /// OpenType classes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<FeatureClass>,
    /// Font-wide custom parameters
    #[serde(default, rename = "customParameters")]
    pub custom_parameters: Vec<CustomParameter>,
    /// Font creation date. Format `2014-01-29 14:14:38 +0000`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
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
    /// Instances
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<Instance>,
    #[serde(
        rename = "keepAlternatesTogether",
        default,
        skip_serializing_if = "is_default"
    )]
    pub keep_alternates_together: bool,
    /// Three-level dict containing a float as value.
    #[serde(rename = "kerningLTR", default, skip_serializing_if = "is_default")]
    pub kerning: Kerning,
    #[serde(rename = "kerningRTL", default, skip_serializing_if = "is_default")]
    pub kerning_rtl: Kerning,
    #[serde(
        rename = "kerningVertical",
        default,
        skip_serializing_if = "is_default"
    )]
    pub kerning_vertical: Kerning,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<Metric>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numbers: Vec<Number>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<Property>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stems: Vec<Stem>,
    #[serde(rename = "unitsPerEm")]
    pub units_per_em: i32,
    #[serde(rename = "userData", default, skip_serializing_if = "is_default")]
    pub user_data: Dictionary,
    #[serde(default, rename = "versionMajor")]
    pub version_major: i32,
    #[serde(default, rename = "versionMinor")]
    pub version_minor: i32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Number {
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Metric {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub metric_type: Option<MetricType>,
}

#[derive(Serialize, Debug, Clone)]
pub enum MetricType {
    #[serde(rename = "ascender")]
    Ascender,
    #[serde(rename = "cap height")]
    CapHeight,
    #[serde(rename = "slant height")]
    SlantHeight,
    #[serde(rename = "x-height")]
    XHeight,
    #[serde(rename = "midHeight")]
    MidHeight,
    #[serde(rename = "topHeight")]
    TopHeight,
    #[serde(rename = "bodyHeight")]
    BodyHeight,
    #[serde(rename = "descender")]
    Descender,
    #[serde(rename = "baseline")]
    Baseline,
    #[serde(rename = "italic angle")]
    ItalicAngle,
}

impl<'de> Deserialize<'de> for MetricType {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let variant = String::deserialize(de)?;
        Ok(match variant.as_str() {
            "ascender" => MetricType::Ascender,
            "cap height" => MetricType::CapHeight,
            "slant height" => MetricType::SlantHeight,
            "x-height" => MetricType::XHeight,
            "midHeight" => MetricType::MidHeight,
            "topHeight" => MetricType::TopHeight,
            "bodyHeight" => MetricType::BodyHeight,
            "descender" => MetricType::Descender,
            "baseline" => MetricType::Baseline,
            "italic angle" => MetricType::ItalicAngle,
            _ => {
                return Err(serde::de::Error::custom(format!(
                    "unknown metric type: {}",
                    variant
                )))
            }
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Settings {
    #[serde(rename = "disablesAutomaticAlignment", default)]
    pub disables_automatic_alignment: bool,
    #[serde(rename = "disablesNiceNames", default)]
    pub disables_nice_names: bool,
    #[serde(rename = "gridLength", skip_serializing_if = "Option::is_none")]
    pub grid_length: Option<i32>,
    #[serde(rename = "gridSubDivision", skip_serializing_if = "Option::is_none")]
    pub grid_sub_division: Option<i32>,
    #[serde(rename = "keyboardIncrement", skip_serializing_if = "Option::is_none")]
    pub keyboard_increment: Option<f32>,
    #[serde(
        rename = "keyboardIncrementBig",
        skip_serializing_if = "Option::is_none"
    )]
    pub keyboard_increment_big: Option<f32>,
    #[serde(
        rename = "keyboardIncrementHuge",
        skip_serializing_if = "Option::is_none"
    )]
    pub keyboard_increment_huge: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Axis {
    /// If the axis should be visible in the UI.
    #[serde(default, skip_serializing_if = "is_default")]
    pub hidden: bool,
    /// The name of the axis (e.g. `Weight``)
    pub name: String,
    /// The axis tag (e.g. `wght`)
    pub tag: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Master {
    /// A list of float values storing the axis coordinate for each axis
    ///
    /// Axis settings are stored in the Font object.
    #[serde(rename = "axesValues", default)]
    pub axes_values: Vec<f32>,
    /// Master-wide custom parameters
    #[serde(rename = "customParameters", default)]
    pub custom_parameters: Vec<CustomParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guides: Vec<Guide>,
    /// Stores the selected master icon
    #[serde(rename = "iconName", default)]
    pub icon_name: String,
    /// A unique id that connects the layers (associated ID) with the master
    pub id: String,
    /// The metrics values
    ///
    /// Metrics settings are stored in the font object.
    #[serde(
        rename = "metricValues",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub metric_values: Vec<MetricValue>,
    /// The name of the master
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// A list of floats, number settings are stored in the font object.
    #[serde(
        rename = "numberValues",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub number_values: Vec<f32>,
    /// Properties
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<Property>,
    /// The stem values
    #[serde(rename = "stemValues", default, skip_serializing_if = "Vec::is_empty")]
    pub stem_values: Vec<f32>,
    #[serde(rename = "userData", default, skip_serializing_if = "is_default")]
    pub user_data: Dictionary,
    #[serde(default = "bool_true", skip_serializing_if = "is_true")]
    pub visible: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct MetricValue {
    #[serde(default)]
    pub over: f32,
    #[serde(default)]
    pub pos: f32,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Glyph {
    /// The 'case' of the glyph when manually set.
    ///
    /// Possible values: "noCase", "upper", "lower", "smallCaps", "other".
    /// This could be used to specify 'height' of default numbers (lining vs old style)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub case: String,
    /// Manually set category
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// The color of the glyph in the interface
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// The writing direction when manually set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    /// Export
    #[serde(default = "bool_true", skip_serializing_if = "is_true")]
    pub export: bool,
    /// The glyph name
    #[serde(rename = "glyphname")]
    pub name: String,
    ///  Bottom kerning group
    #[serde(rename = "kernBottom", skip_serializing_if = "Option::is_none")]
    pub kern_bottom: Option<String>,
    /// Left kerning group
    #[serde(rename = "kernLeft", skip_serializing_if = "Option::is_none")]
    pub kern_left: Option<String>,
    /// Right kerning group
    #[serde(rename = "kernRight", skip_serializing_if = "Option::is_none")]
    pub kern_right: Option<String>,
    /// Top kerning group
    #[serde(rename = "kernTop", skip_serializing_if = "Option::is_none")]
    pub kern_top: Option<String>,
    /// Format 2014-01-29 14:14:38 +0000
    #[serde(rename = "lastChange", skip_serializing_if = "Option::is_none")]
    pub last_change: Option<String>,
    pub layers: Vec<Layer>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub locked: bool,
    #[serde(default, skip_serializing_if = "is_default", rename = "metricBottom")]
    pub metric_bottom: Option<String>,
    #[serde(default, skip_serializing_if = "is_default", rename = "metricLeft")]
    pub metric_left: Option<String>,
    #[serde(default, skip_serializing_if = "is_default", rename = "metricRight")]
    pub metric_right: Option<String>,
    #[serde(default, skip_serializing_if = "is_default", rename = "metricTop")]
    pub metric_top: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "is_default",
        rename = "metricVertWidth"
    )]
    pub metric_vert_width: Option<String>,
    #[serde(default, skip_serializing_if = "is_default", rename = "metricWidth")]
    pub metric_width: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub note: String,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "partsSettings"
    )]
    pub smart_component_settings: Vec<SmartComponentSetting>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub production: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub script: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub subcategory: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicode: Option<Vec<u32>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SmartComponentSetting {
    #[serde(default, rename = "bottomValue")]
    bottom_value: i32,
    #[serde(default, rename = "topValue")]
    top_value: i32,
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    #[serde(default)]
    pub attr: Dictionary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Box<Layer>>,
    #[serde(
        rename = "backgroundImage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub background_image: Option<BackgroundImage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guides: Vec<Guide>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<Dictionary>, // This thing's an absolute minefield
    /// The unique id of the layer
    #[serde(rename = "layerId", default, skip_serializing_if = "String::is_empty")]
    // Not required for background layers
    pub layer_id: String,
    /// Bottom metric key
    #[serde(rename = "metricBottom", default, skip_serializing_if = "is_default")]
    pub metric_bottom: Option<String>,
    /// Left metric key
    #[serde(rename = "metricLeft", default, skip_serializing_if = "is_default")]
    pub metric_left: Option<String>,
    /// Right metric key
    #[serde(rename = "metricRight", default, skip_serializing_if = "is_default")]
    pub metric_right: Option<String>,
    /// Top metric key
    #[serde(rename = "metricTop", default, skip_serializing_if = "is_default")]
    pub metric_top: Option<String>,
    /// Vertical width metric key
    #[serde(
        rename = "metricVertWidth",
        default,
        skip_serializing_if = "is_default"
    )]
    pub metric_vert_width: Option<String>,
    /// Horizontal width metric key
    #[serde(rename = "metricWidth", default, skip_serializing_if = "is_default")]
    pub metric_width: Option<String>,
    /// The name of the layer.
    ///
    /// Only stored for non-master layers (this is changed in 2.3, before the master names where stored)
    #[serde(default, skip_serializing_if = "is_default")]
    pub name: Option<String>,
    /// Smart component part selection
    #[serde(default, skip_serializing_if = "is_default")]
    pub part_selection: BTreeMap<String, u8>,
    /// Shapes
    ///
    /// Can be paths or components
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<Shape>,
    /// User data
    #[serde(rename = "userData", default, skip_serializing_if = "is_default")]
    pub user_data: Dictionary,
    /// Offset from default (ascender)
    #[serde(
        rename = "vertOrigin",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub vert_origin: Option<f32>,
    /// Vertical width
    ///
    /// Only stored if other than the default (ascender+descender)
    #[serde(rename = "vertWidth", default, skip_serializing_if = "Option::is_none")]
    pub vert_width: Option<f32>,
    /// The visibility setting in the layer panel (the eye symbol).
    #[serde(default = "bool_true", skip_serializing_if = "is_true")]
    pub visible: bool,
    /// Layer width
    #[serde(default)]
    pub width: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Anchor {
    pub name: String,
    #[serde(default)]
    pub pos: (f32, f32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackgroundImage {
    /// The angle
    #[serde(default)]
    pub angle: f32,
    /// The image path
    #[serde(rename = "imagePath")]
    pub image_path: String,
    #[serde(default)]
    pub locked: bool,
    /// The image scale
    #[serde(default = "scale_unit")]
    pub scale: (f32, f32),
    /// The origin
    #[serde(default)]
    pub pos: (f32, f32),
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
    #[serde(default = "scale_unit")]
    pub scale: (f32, f32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Shape {
    Component(Component),
    Path(Path),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Path {
    #[serde(default, skip_serializing_if = "is_default")]
    pub attr: Dictionary,
    // Because we are using an untagged enum, types need to match precisely
    #[serde(default, deserialize_with = "int_to_bool")]
    pub closed: bool,
    pub nodes: Vec<Node>,
}

fn int_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: i8 = Deserialize::deserialize(deserializer)?;
    Ok(s == 1)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub x: f32,
    pub y: f32,
    pub node_type: NodeType,
    pub user_data: Option<Dictionary>,
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&self.x)?;
        seq.serialize_element(&self.y)?;
        seq.serialize_element(&self.node_type)?;
        if let Some(user_data) = &self.user_data {
            seq.serialize_element(user_data)?;
        }
        seq.end()
    }
}

struct SimpleNodeVisitor;
impl<'de> Visitor<'de> for SimpleNodeVisitor {
    type Value = Node;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a tuple of 3 or 4 elements")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let x = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let y = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        let node_type = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
        let user_data = seq.next_element()?;
        Ok(Node {
            x,
            y,
            node_type,
            user_data,
        })
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(SimpleNodeVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Component {
    #[serde(default)]
    /// Controls the automatic alignment of this component.
    ///
    /// -1 disables alignment, 1 forces it for glyph that are usually not aligned.
    pub alignment: i8,
    /// Should be indicated if connected to an anchor, especially if more than one possibility is available, e.g. in ligatures
    #[serde(default)]
    pub anchor: Option<String>,
    /// A completely undocumented thing.
    #[serde(default, rename = "anchorTo")]
    pub anchor_to: Option<String>,
    #[serde(default)]
    pub angle: f32,
    #[serde(default)]
    pub attr: Dictionary,
    #[serde(default = "bool_true")]
    pub locked: bool,
    /// If left, center or right aligned
    #[serde(default)]
    pub orientation: i8,
    /// Smart component location
    #[serde(rename = "piece", default)]
    pub smart_component_location: BTreeMap<String, f32>,
    /// The position
    #[serde(default, rename = "pos")]
    pub position: (f32, f32),
    /// The name of the linked glyph
    #[serde(rename = "ref")]
    pub component_glyph: String,
    #[serde(default = "scale_unit")]
    pub scale: (f32, f32),
    #[serde(default, rename = "userData")]
    pub user_data: Dictionary,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Instance {
    /// A list of float values storing the axis coordinate for each axis
    ///
    /// Axis settings are stored in the font object.
    #[serde(default, rename = "axesValues", skip_serializing_if = "is_default")]
    pub axes_values: Vec<f32>,
    #[serde(
        default,
        rename = "customParameters",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub custom_parameters: Vec<CustomParameter>,
    #[serde(default = "bool_true", skip_serializing_if = "is_true")]
    pub exports: bool,
    /// Keys are master IDs, values are the factors for that master.
    #[serde(
        default,
        rename = "instanceInterpolations",
        skip_serializing_if = "BTreeMap::is_empty"
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<Property>,
    #[serde(default, rename = "userData", skip_serializing_if = "is_default")]
    pub user_data: Dictionary,
    #[serde(default, rename = "weightClass")]
    pub weight_class: Option<Plist>, // String or integer
    #[serde(default, rename = "widthClass")]
    pub width_class: Option<Plist>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Property {
    SingularProperty {
        key: SingularPropertyKey,
        value: String,
    },
    LocalizedProperty {
        key: LocalizedPropertyKey,
        values: Vec<LocalizedValue>,
    },
}

impl Property {
    pub(crate) fn singular(key: SingularPropertyKey, value: String) -> Self {
        Property::SingularProperty { key, value }
    }
    pub(crate) fn localized_with_default(key: LocalizedPropertyKey, value: String) -> Self {
        Property::LocalizedProperty {
            key,
            values: vec![LocalizedValue {
                language: "dflt".to_string(),
                value,
            }],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LocalizedPropertyKey {
    #[serde(rename = "familyNames")]
    FamilyNames,
    #[serde(rename = "copyrights")]
    Copyrights,
    #[serde(rename = "designers")]
    Designers,
    #[serde(rename = "manufacturers")]
    Manufacturers,
    #[serde(rename = "licenses")]
    Licenses,
    #[serde(rename = "trademarks")]
    Trademarks,
    #[serde(rename = "descriptions")]
    Descriptions,
    #[serde(rename = "sampleTexts")]
    SampleTexts,
    #[serde(rename = "compatibleFullNames")]
    CompatibleFullNames,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SingularPropertyKey {
    #[serde(rename = "designerURL")]
    DesignerUrl,
    #[serde(rename = "manufacturerURL")]
    ManufacturerUrl,
    #[serde(rename = "licenseURL")]
    LicenseUrl,
    #[serde(rename = "postscriptFullName")]
    PostscriptFullName,
    #[serde(rename = "postscriptFontName")]
    PostscriptFontName,
    #[serde(rename = "WWSFamilyName")]
    WwsFamilyName,
    #[serde(rename = "versionString")]
    VersionString,
    #[serde(rename = "vendorID")]
    VendorID,
    #[serde(rename = "uniqueID")]
    UniqueID,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalizedValue {
    language: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stem {
    name: String,
    #[serde(default)]
    pub horizontal: bool,
}
