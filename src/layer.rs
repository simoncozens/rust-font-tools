use crate::anchor::Anchor;
use crate::common::Color;
use crate::common::Location;
use crate::guide::Guide;
use crate::shape::Shape;
use crate::Component;
use crate::Path;

#[derive(Debug)]
pub struct Layer {
    pub width: i32,
    pub name: Option<String>,
    pub id: Option<String>,
    pub guides: Vec<Guide>,
    pub shapes: Vec<Shape>,
    pub anchors: Vec<Anchor>,
    pub color: Option<Color>,
    pub layer_index: Option<i32>,
    pub is_background: bool,
    pub background_layer_id: Option<String>,
    pub location: Option<Location>,
}

impl Layer {
    pub fn new(width: i32) -> Layer {
        Layer {
            width,
            name: None,
            id: None,
            guides: vec![],
            shapes: vec![],
            anchors: vec![],
            color: None,
            layer_index: None,
            is_background: false,
            background_layer_id: None,
            location: None,
        }
    }

    pub fn components(&self) -> Vec<&Component> {
        self.shapes
            .iter()
            .map(|x| {
                if let Shape::ComponentShape(c) = x {
                    Some(c)
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    pub fn paths(&self) -> Vec<&Path> {
        self.shapes
            .iter()
            .map(|x| {
                if let Shape::PathShape(p) = x {
                    Some(p)
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
}
