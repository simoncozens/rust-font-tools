use crate::common::OTValue;
use crate::glyph::GlyphCategory;
use crate::i18ndictionary::I18NDictionary;
use crate::OTScalar::Signed;
use crate::Shape::{ComponentShape, PathShape};
use crate::{
    Anchor, Axis, BabelfontError, Component, Font, Glyph, GlyphList, Guide, Instance, Layer,
    Master, Node, NodeType, OTScalar, Path, Position, Shape,
};
use chrono::TimeZone;
use fontdrasil::coords::{DesignCoord, DesignLocation, Location, UserCoord};
use std::collections::HashMap;
use std::fs;
use std::hash::Hash;
use std::path::PathBuf;
use std::str::FromStr;
use write_fonts::types::Tag;

pub fn load(path: PathBuf) -> Result<Font, BabelfontError> {
    log::debug!("Reading to string");
    let s = fs::read_to_string(&path).map_err(|source| BabelfontError::IO {
        path: path.clone(),
        source,
    })?;
    load_str(&s, path.clone())
}

pub fn load_str(s: &str, path: PathBuf) -> Result<Font, BabelfontError> {
    let mut font = Font::new();
    let glyphs_font =
        glyphslib::Font::load_str(s).map_err(|source| BabelfontError::PlistParse {
            source,
            path: path.clone(),
        })?;
    let glyphs_font = glyphs_font
        .as_glyphs3()
        .ok_or_else(|| BabelfontError::WrongConvertor { path })?;
    // Copy axes
    font.axes = glyphs_font
        .axes
        .iter()
        .map(|axis| Axis {
            tag: Tag::from_str(&axis.tag).unwrap_or_else(|_| Tag::from_str("????").unwrap()),
            name: axis.name.clone().into(),
            hidden: axis.hidden,
            ..Default::default()
        })
        .collect();

    // Copy masters
    font.masters = glyphs_font
        .masters
        .iter()
        .map(|master| convert_master(master, &glyphs_font, &font))
        .collect();
    // Copy glyphs
    font.glyphs = GlyphList(glyphs_font.glyphs.iter().map(Into::into).collect());

    // Copy instances
    // Copy metadata
    font.names.family_name = glyphs_font.family_name.clone().into();
    // Copy kerning
    // Interpret metrics
    // Interpret axes
    interpret_axes(&mut font);

    Ok(font)
}

fn convert_master(
    master: &glyphslib::glyphs3::Master,
    glyphs_font: &glyphslib::glyphs3::Glyphs3,
    font: &Font,
) -> Master {
    let designspace_to_location = |numbers: &[f32]| -> DesignLocation {
        numbers
            .iter()
            .zip(font.axes.iter())
            .map(|(number, axis)| (axis.tag, DesignCoord::new(*number)))
            .collect()
    };
    let mut m = Master {
        name: master.name.clone().into(),
        id: master.id.clone(),
        location: designspace_to_location(&&master.axes_values),
        guides: master.guides.iter().map(Into::into).collect(),
        metrics: HashMap::new(),
        kerning: HashMap::new(),
        custom_ot_values: vec![],
    };
    m.kerning = glyphs_font
        .kerning
        .get(&m.id)
        .map(|kerndict| {
            let mut kerns = HashMap::new();
            for (first, items) in kerndict {
                for (second, kern) in items {
                    kerns.insert((first.clone(), second.clone()), *kern as i16);
                }
            }
            kerns
        })
        .unwrap_or_default();
    m
}

impl Into<Guide> for &glyphslib::glyphs3::Guide {
    fn into(self) -> Guide {
        Guide {
            pos: Position {
                x: self.pos.0,
                y: self.pos.1,
                angle: self.angle as f32,
            },
            name: None,
            color: None,
        }
    }
}

impl Into<Glyph> for &glyphslib::glyphs3::Glyph {
    fn into(self) -> Glyph {
        Glyph {
            name: self.name.clone(),
            production_name: self.production.clone(),
            category: GlyphCategory::Unknown,
            codepoints: self.unicode.clone().unwrap_or_default(),
            layers: self.layers.iter().map(Into::into).collect(),
            exported: true,
            direction: None,
            formatspecific: Default::default(),
        }
    }
}

impl Into<Layer> for &glyphslib::glyphs3::Layer {
    fn into(self) -> Layer {
        Layer {
            id: Some(self.layer_id.clone()),
            name: self.name.clone(),
            color: None,
            shapes: self.shapes.iter().map(Into::into).collect(),
            width: self.width,
            guides: self.guides.iter().map(Into::into).collect(),
            anchors: self.anchors.iter().map(Into::into).collect(),
            layer_index: None,
            is_background: false,
            background_layer_id: None,
            location: None,
        }
    }
}

impl Into<Anchor> for &glyphslib::glyphs3::Anchor {
    fn into(self) -> Anchor {
        Anchor {
            name: self.name.clone(),
            x: self.pos.0,
            y: self.pos.1,
        }
    }
}

impl Into<Shape> for &glyphslib::glyphs3::Shape {
    fn into(self) -> Shape {
        match self {
            glyphslib::glyphs3::Shape::Component(c) => ComponentShape(c.into()),
            glyphslib::glyphs3::Shape::Path(p) => PathShape(p.into()),
        }
    }
}

impl Into<Component> for &glyphslib::glyphs3::Component {
    fn into(self) -> Component {
        let transform = kurbo::Affine::IDENTITY
            * kurbo::Affine::translate((self.position.0 as f64, self.position.1 as f64))
            * kurbo::Affine::rotate((self.angle as f64).to_radians())
            * kurbo::Affine::scale_non_uniform(self.scale.0 as f64, self.scale.1 as f64);
        Component {
            reference: self.component_glyph.clone(),
            transform,
        }
    }
}

impl Into<Path> for &glyphslib::glyphs3::Path {
    fn into(self) -> Path {
        let mut nodes = vec![];
        for node in &self.nodes {
            nodes.push(node.into());
        }
        Path {
            nodes,
            closed: self.closed,
        }
    }
}

impl Into<Node> for &glyphslib::glyphs3::Node {
    fn into(self) -> Node {
        Node {
            x: self.x,
            y: self.y,
            nodetype: match self.node_type {
                glyphslib::glyphs3::NodeType::Line => NodeType::Line,
                glyphslib::glyphs3::NodeType::OffCurve => NodeType::OffCurve,
                glyphslib::glyphs3::NodeType::Curve => NodeType::Curve,
                glyphslib::glyphs3::NodeType::QCurve => NodeType::QCurve,
                glyphslib::glyphs3::NodeType::LineSmooth => NodeType::Line,
                glyphslib::glyphs3::NodeType::CurveSmooth => NodeType::Curve,
                glyphslib::glyphs3::NodeType::QCurveSmooth => NodeType::QCurve,
            },
        }
    }
}

fn interpret_axes(font: &mut Font) {
    // This is going to look very wrong, but after much trial and error I can confirm
    // it works. First: load the axes assuming that userspace=designspace. Then
    // work out the axis mappings. Then apply the mappings to the axis locations.
    if let Some(origin) = font.masters.first() {
        // XXX *or* custom parameter Variable Font Origin
        for master in font.masters.iter() {
            for axis in font.axes.iter_mut() {
                let loc = master
                    .location
                    .get(axis.tag)
                    .unwrap_or(DesignCoord::default());
                axis.min = if axis.min.is_none() {
                    Some(UserCoord::new(loc.to_f32()))
                } else {
                    axis.min.map(|v| v.min(UserCoord::new(loc.to_f32())))
                };
                axis.max = if axis.max.is_none() {
                    Some(UserCoord::new(loc.to_f32()))
                } else {
                    axis.max.map(|v| v.max(UserCoord::new(loc.to_f32())))
                };
                if master.id == origin.id {
                    axis.default = Some(UserCoord::new(loc.to_f32()));
                }
            }
        }
        // XXX find axis mappings here

        for axis in font.axes.iter_mut() {
            axis.default = Some(
                axis.designspace_to_userspace(DesignCoord::new(
                    axis.default.map(|v| v.to_f32()).unwrap_or(0.0),
                ))
                .unwrap_or(UserCoord::default()),
            );
            axis.min = axis.min.map(|v| {
                axis.designspace_to_userspace(DesignCoord::new(v.to_f32()))
                    .unwrap_or(UserCoord::default())
            });
            axis.max = axis.max.map(|v| {
                axis.designspace_to_userspace(DesignCoord::new(v.to_f32()))
                    .unwrap_or(UserCoord::default())
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_something() {
        let _f = load("data/Nunito3.glyphs".into()).unwrap();
    }
}
