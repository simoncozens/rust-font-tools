#[macro_use]
extern crate shrinkwraprs;

pub mod convertors;
mod error;

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
pub use crate::common::{Location, Position};
pub use crate::common::{Node, NodeType, OTScalar};
pub use crate::error::BabelfontError;
pub use crate::font::Font;
pub use crate::glyph::{Glyph, GlyphList};
pub use crate::guide::Guide;
pub use crate::layer::Layer;
pub use crate::master::Master;
pub use crate::shape::{Component, Path, Shape};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
