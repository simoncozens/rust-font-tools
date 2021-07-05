use crate::common::Direction;
use crate::layer::Layer;

#[derive(Debug, Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct GlyphList(pub Vec<Glyph>);
impl GlyphList {
    pub fn get(&self, g: &str) -> Option<&Glyph> {
        for glyph in self.0.iter() {
            if glyph.name == g {
                return Some(glyph);
            }
        }
        None
    }
    pub fn get_mut(&mut self, g: &str) -> Option<&Glyph> {
        for glyph in self.0.iter_mut() {
            if glyph.name == g {
                return Some(glyph);
            }
        }
        None
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
