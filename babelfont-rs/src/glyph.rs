use std::ops::{Deref, DerefMut};

use crate::common::{Direction, FormatSpecific};
use crate::layer::Layer;

#[derive(Debug, Clone)]
pub struct GlyphList(pub Vec<Glyph>);
impl GlyphList {
    pub fn get(&self, g: &str) -> Option<&Glyph> {
        self.0.iter().find(|&glyph| glyph.name == g)
    }
    pub fn get_mut(&mut self, g: &str) -> Option<&mut Glyph> {
        self.0.iter_mut().find(|glyph| glyph.name == g)
    }

    pub fn get_by_index(&self, id: usize) -> Option<&Glyph> {
        self.0.get(id)
    }
    pub fn get_by_index_mut(&mut self, id: usize) -> Option<&mut Glyph> {
        self.0.get_mut(id)
    }
}

impl Deref for GlyphList {
    type Target = Vec<Glyph>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for GlyphList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
pub enum GlyphCategory {
    Base,
    Mark,
    Unknown,
    Ligature,
}

#[derive(Debug, Clone)]
pub struct Glyph {
    pub name: String,
    pub production_name: Option<String>,
    pub category: GlyphCategory,
    pub codepoints: Vec<u32>,
    pub layers: Vec<Layer>,
    pub exported: bool,
    pub direction: Option<Direction>,
    pub formatspecific: FormatSpecific,
}

impl Glyph {
    pub fn get_layer(&self, id: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id.as_deref() == Some(id))
    }
    pub fn get_layer_mut(&mut self, id: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id.as_deref() == Some(id))
    }
}
