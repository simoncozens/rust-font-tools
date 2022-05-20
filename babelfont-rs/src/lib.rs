#[macro_use]
extern crate shrinkwraprs;

pub mod convertors;
mod error;
mod utils;

mod anchor;
mod axis;
mod common;
mod font;
mod glyph;
mod guide;
mod i18ndictionary;
mod instance;
mod layer;
mod master;
pub mod names;
mod shape;

pub use crate::anchor::Anchor;
pub use crate::axis::Axis;
pub use crate::common::{Location, Node, NodeType, OTScalar, Position};
pub use crate::error::BabelfontError;
pub use crate::font::Font;
pub use crate::glyph::{Glyph, GlyphCategory, GlyphList};
pub use crate::guide::Guide;
pub use crate::instance::Instance;
pub use crate::layer::Layer;
pub use crate::master::Master;
pub use crate::shape::{Component, Path, PathDirection, Shape};
use std::path::PathBuf;

pub fn load(filename: &str) -> Result<Font, BabelfontError> {
    let pb = PathBuf::from(filename);
    if filename.ends_with(".designspace") {
        crate::convertors::designspace::load(pb)
    } else if filename.ends_with(".vfj") {
        crate::convertors::fontlab::load(pb)
    } else if filename.ends_with(".ufo") {
        crate::convertors::ufo::load(pb)
    } else if filename.ends_with(".glyphs") {
        crate::convertors::glyphs3::load(pb)
    } else {
        Err(BabelfontError::UnknownFileType { path: pb })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
