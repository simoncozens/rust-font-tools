use crate::common::Direction;
use crate::layer::Layer;

#[derive(Debug, Shrinkwrap)]
#[shrinkwrap(mutable)]
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

#[derive(Debug)]
pub enum GlyphCategory {
    Base,
    Mark,
    Unknown,
    Ligature,
}

#[derive(Debug)]
pub struct Glyph {
    pub name: String,
    pub production_name: Option<String>,
    pub category: GlyphCategory,
    pub codepoints: Vec<usize>,
    pub layers: Vec<Layer>,
    pub exported: bool,
    pub direction: Option<Direction>,
}

impl Glyph {
    pub fn get_layer(&self, id: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id.as_deref() == Some(id))
    }
    pub fn get_layer_mut(&mut self, id: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id.as_deref() == Some(id))
    }
}
