use crate::anchor::Anchor;
use crate::common::{Color, Location};
use crate::guide::Guide;
use crate::shape::Shape;
use crate::{Component, Path};

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

    pub fn components(&self) -> impl DoubleEndedIterator<Item = &Component> {
        self.shapes.iter().filter_map(|x| {
            if let Shape::ComponentShape(c) = x {
                Some(c)
            } else {
                None
            }
        })
    }

    pub fn components_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut Component> {
        self.shapes.iter_mut().filter_map(|x| {
            if let Shape::ComponentShape(c) = x {
                Some(c)
            } else {
                None
            }
        })
    }

    pub fn paths(&self) -> impl DoubleEndedIterator<Item = &Path> {
        self.shapes.iter().filter_map(|x| {
            if let Shape::PathShape(p) = x {
                Some(p)
            } else {
                None
            }
        })
    }

    pub fn clear_components(&mut self) {
        self.shapes.retain(|sh| matches!(sh, Shape::PathShape(_)));
    }

    pub fn push_component(&mut self, c: Component) {
        self.shapes.push(Shape::ComponentShape(c))
    }

    pub fn push_path(&mut self, p: Path) {
        self.shapes.push(Shape::PathShape(p))
    }

    pub fn has_components(&self) -> bool {
        self.shapes
            .iter()
            .any(|sh| matches!(sh, Shape::ComponentShape(_)))
    }

    pub fn has_paths(&self) -> bool {
        self.shapes
            .iter()
            .any(|sh| matches!(sh, Shape::PathShape(_)))
    }
}
