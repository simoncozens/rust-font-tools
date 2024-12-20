#![deny(clippy::unwrap_used, clippy::expect_used)]

mod anchor;
mod axis;
mod common;
pub mod convertors;
mod error;
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
pub use crate::common::{Node, NodeType, OTScalar, Position};
pub use crate::error::BabelfontError;
pub use crate::font::Font;
pub use crate::glyph::{Glyph, GlyphCategory, GlyphList};
pub use crate::guide::Guide;
pub use crate::instance::Instance;
pub use crate::layer::Layer;
pub use crate::master::Master;
pub use crate::shape::{Component, Path, Shape};
pub use fontdrasil::coords::{
    DesignCoord, DesignLocation, NormalizedCoord, NormalizedLocation, UserCoord, UserLocation,
};
use std::path::PathBuf;

pub fn load(filename: &str) -> Result<Font, BabelfontError> {
    let pb = PathBuf::from(filename);
    if filename.ends_with(".designspace") {
        #[cfg(feature = "ufo")]
        return crate::convertors::designspace::load(pb);
        #[cfg(not(feature = "ufo"))]
        Err(BabelfontError::UnknownFileType { path: pb })
    } else if filename.ends_with(".vfj") {
        #[cfg(feature = "fontlab")]
        return crate::convertors::fontlab::load(pb);
        #[cfg(not(feature = "fontlab"))]
        Err(BabelfontError::UnknownFileType { path: pb })
    } else if filename.ends_with(".ufo") {
        #[cfg(feature = "ufo")]
        return crate::convertors::ufo::load(pb);

        #[cfg(not(feature = "ufo"))]
        Err(BabelfontError::UnknownFileType { path: pb })
    } else if filename.ends_with(".glyphs") {
        #[cfg(feature = "glyphs")]
        return crate::convertors::glyphs3::load(pb);
        #[cfg(not(feature = "glyphs"))]
        Err(BabelfontError::UnknownFileType { path: pb })
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
