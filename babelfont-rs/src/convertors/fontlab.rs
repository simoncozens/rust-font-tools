use crate::{
    Anchor, Axis, BabelfontError, Component, Font, Glyph, GlyphCategory, Layer, Location, Master,
    Node, NodeType, Path, PathDirection, Shape,
};
use kurbo::Affine;
use lazy_static::lazy_static;
use otmath::ot_round;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::path::PathBuf;

fn to_point(s: String) -> Result<(i32, i32), BabelfontError> {
    let mut i = s.split(' ');
    let x_str = i.next().expect("Couldn't read X coordinate");
    let x = x_str.parse::<f32>().map_err(|_| BabelfontError::General {
        msg: format!("Couldn't parse X coordinate {:}", x_str),
    })?;
    let y_str = i.next().expect("Couldn't read Y coordinate");
    let y = y_str.parse::<f32>().map_err(|_| BabelfontError::General {
        msg: format!("Couldn't parse Y coordinate {:}", y_str),
    })?;
    Ok((ot_round(x), ot_round(y)))
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct FontlabComponent {
    glyphName: String,
}

impl Into<Shape> for FontlabComponent {
    fn into(self) -> Shape {
        Shape::ComponentShape(Component {
            reference: self.glyphName,
            transform: Affine::IDENTITY,
        })
    }
}
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct FontlabContour {
    nodes: Vec<String>,
}

fn nodestring_to_nodes(s: String) -> Vec<Node> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(-?[\d\.]+) (-?[\d\.]+)( s)?").unwrap();
    }
    let count = s.split("  ").count();
    s.split("  ")
        .enumerate()
        .map(|(ix, n)| {
            if let Some(mat) = RE.captures(n) {
                let nodetype = if count == 1 {
                    NodeType::Line
                } else if (count == 3 && ix == 2) || (count == 2 && ix == 1) {
                    NodeType::Curve
                } else {
                    NodeType::OffCurve
                };
                Some(Node {
                    x: mat[1].parse().unwrap(),
                    y: mat[2].parse().unwrap(),
                    nodetype,
                })
            } else {
                None
            }
        })
        .flatten()
        .collect()
}
impl From<FontlabContour> for Shape {
    fn from(val: FontlabContour) -> Self {
        Shape::PathShape(Path {
            nodes: val
                .nodes
                .into_iter()
                .map(nodestring_to_nodes)
                .flatten()
                .collect(),
            closed: true,
            direction: PathDirection::Clockwise,
        })
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct FontlabPath {
    contours: Vec<FontlabContour>,
}

impl From<FontlabPath> for Vec<Shape> {
    fn from(val: FontlabPath) -> Self {
        val.contours.into_iter().map(|x| x.into()).collect()
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum FontlabShape {
    ComponentShape { component: FontlabComponent },
    PathShape(FontlabPath),
}
impl From<FontlabShape> for Vec<Shape> {
    fn from(val: FontlabShape) -> Self {
        match val {
            FontlabShape::ComponentShape { component } => vec![component.into()],
            FontlabShape::PathShape(p) => p.into(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum FontlabTransform {
    NamedTransform(String),
    LiteralTransform(HashMap<String, f32>),
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum FontlabElement {
    TaggedShape {
        elementData: FontlabShape,
        // transform: Option<FontlabTransform>,
    },
    UntaggedShape {
        component: FontlabComponent,
    },
}

impl Into<Vec<Shape>> for FontlabElement {
    fn into(self) -> Vec<Shape> {
        match self {
            FontlabElement::TaggedShape { elementData } => elementData.into(),
            FontlabElement::UntaggedShape { component } => vec![component.into()],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FontlabAnchor {
    name: String,
    point: Option<String>,
}

impl TryInto<Option<Anchor>> for FontlabAnchor {
    fn try_into(self) -> Result<Option<Anchor>, BabelfontError> {
        if let Some(point) = self.point {
            let (x, y) = to_point(point)?;
            Ok(Some(Anchor {
                x,
                y,
                name: self.name,
            }))
        } else {
            Ok(None)
        }
    }

    type Error = BabelfontError;
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct FontlabLayer {
    advanceWidth: i32,
    name: Option<String>,
    #[serde(default)]
    anchors: Vec<FontlabAnchor>,
    #[serde(default)]
    elements: Vec<FontlabElement>,
}

impl FontlabLayer {
    fn try_into_babel(self, _font: &Font) -> Result<Layer, BabelfontError> {
        let anchors: Result<Vec<Option<Anchor>>, BabelfontError> =
            self.anchors.into_iter().map(|x| x.try_into()).collect();
        Ok(Layer {
            width: self.advanceWidth,
            name: self.name.clone(),
            id: self.name,
            guides: vec![],
            shapes: self
                .elements
                .into_iter()
                .map(|x| {
                    let v: Vec<Shape> = x.into();
                    v
                })
                .flatten()
                .collect(),
            anchors: anchors?.into_iter().flatten().collect(),
            color: None,
            layer_index: None,
            is_background: false,
            background_layer_id: None,
            location: None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FontlabGlyph {
    name: String,
    unicode: Option<String>,
    layers: Vec<FontlabLayer>,
}

impl FontlabGlyph {
    fn try_into(self, font: &Font) -> Result<Glyph, BabelfontError> {
        let codepoints = if let Some(unicode) = self.unicode {
            unicode
                .split(',')
                .map(|x| usize::from_str_radix(x, 16))
                .flatten()
                .collect()
        } else {
            vec![]
        };
        let layers: Result<Vec<Layer>, BabelfontError> = self
            .layers
            .into_iter()
            .map(|x| x.try_into_babel(font))
            .collect();

        Ok(Glyph {
            name: self.name,
            production_name: None,
            category: GlyphCategory::Unknown,
            codepoints,
            layers: layers?,
            exported: true,
            direction: None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FontlabKerning {
    // XXX
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct FontlabAxis {
    name: String,
    shortName: String,
    tag: String,
    designMinimum: f32,
    designMaximum: f32,
    minimum: Option<f32>,
    maximum: Option<f32>,
    default: Option<f32>,
    axisGraph: Option<HashMap<String, f32>>,
}

impl From<FontlabAxis> for Axis {
    fn from(val: FontlabAxis) -> Self {
        let mut ax = Axis::new(val.name, val.tag);
        ax.min = val.minimum;
        ax.max = val.maximum;
        ax.default = val.default;
        if let Some(map) = val.axisGraph {
            let mut axismap = vec![];
            for (left, right) in map.iter() {
                if let Ok(l_f32) = left.parse() {
                    axismap.push((*right, l_f32));
                }
            }
            axismap.sort_by(|l, r| l.0.partial_cmp(&r.0).unwrap());
            ax.map = Some(axismap);
        }
        ax
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FontlabInstance {
    name: String,
    tsn: String,
    sgn: String,
    location: HashMap<String, f32>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct FontlabFontInfo {
    tfn: String,
    sgn: String,
    creationDate: String,
    copyright: Option<String>,
    trademark: Option<String>,
    designer: Option<String>,
    designerURL: Option<String>,
    manufacturer: Option<String>,
    manufacturerURL: Option<String>,
    description: Option<String>,
    license: Option<String>,
    vendorID: Option<String>,
    versionMajor: Option<u16>,
    versionMinor: Option<u16>,
    version: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct FontlabMaster {
    name: String,
    tsn: String,
    sgn: String,
    ffn: String,
    psn: String,
    ascender: i32,
    descender: i32,
    xHeight: Option<i32>,
    capsHeight: Option<i32>,
    lineGap: Option<i32>,
    underlineThickness: Option<i32>,
    underlinePosition: Option<i32>,
    location: HashMap<String, f32>,
    otherData: HashMap<String, serde_json::Value>, // coward
    kerning: FontlabKerning,
}

impl FontlabMaster {
    fn into(self, axes: &HashMap<String, String>) -> Master {
        let location: Location = Location(
            self.location
                .iter()
                .map(|(short_name, val)| axes.get(short_name).map(|axis| (axis.clone(), *val)))
                .flatten()
                .collect(),
        );
        Master::new(self.name.clone(), self.name, location)
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct FontlabMasterWrapper {
    fontMaster: FontlabMaster,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct FontlabFont {
    glyphsCount: u16,
    upm: u16,
    glyphs: Vec<FontlabGlyph>,
    axes: Vec<FontlabAxis>,
    instances: Vec<FontlabInstance>,
    defaultMaster: Option<String>,
    currentMaster: Option<String>,
    masters: Vec<FontlabMasterWrapper>,
    // classes: Vec<FontlabClass>,
    // openTypeFeatures: XXX,
    // hinting: XXX,
    info: FontlabFontInfo,
}

#[derive(Serialize, Deserialize, Debug)]
struct FontlabFontWrapper {
    version: u8,
    font: FontlabFont,
}

pub fn load(path: PathBuf) -> Result<Font, BabelfontError> {
    let mut axes_short_name_to_tag: HashMap<String, String> = HashMap::new();
    log::debug!("Reading to string");
    let s = fs::read_to_string(&path).map_err(|source| BabelfontError::IO {
        path: path.clone(),
        source,
    })?;
    log::debug!("Parsing to internal structs");
    let mut font = Font::new();
    let p: FontlabFontWrapper = serde_json::from_str(&s).map_err(|e| BabelfontError::General {
        msg: format!("Couldn't parse VFJ: {:}", e),
    })?;
    let fontlab = p.font;
    // log::debug!("{:#?}", fontlab);
    for axis in fontlab.axes {
        let sn = axis.shortName.clone();
        let new_axis: Axis = axis.into();
        axes_short_name_to_tag.insert(sn, new_axis.tag.clone());
        font.axes.push(new_axis);
    }
    for master in fontlab.masters {
        font.masters
            .push(master.fontMaster.into(&axes_short_name_to_tag));
    }
    if let Some(default_master) = fontlab.defaultMaster.and_then(|name| font.master(&name)) {
        let new_loc = default_master.location.designspace_to_userspace(&font.axes);
        for axis in font.axes.iter_mut() {
            if let Some(val) = new_loc.0.get(&axis.tag) {
                axis.default = Some(*val);
            }
        }
        assert!(font.default_master_index().is_some())
    }
    for glyph in fontlab.glyphs {
        let new_glyph = glyph.try_into(&font)?;
        font.glyphs.push(new_glyph);
    }

    font.upm = fontlab.upm;
    Ok(font)
}
