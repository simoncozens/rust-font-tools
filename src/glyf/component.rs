use bitflags::bitflags;
use kurbo::Affine;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field};
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct ComponentFlags: u16 {
        const ARG_1_AND_2_ARE_WORDS = 0x0001;
        const ARGS_ARE_XY_VALUES = 0x0002;
        const ROUND_XY_TO_GRID = 0x0004;
        const WE_HAVE_A_SCALE = 0x0008;
        const MORE_COMPONENTS = 0x0020;
        const WE_HAVE_AN_X_AND_Y_SCALE = 0x0040;
        const WE_HAVE_A_TWO_BY_TWO = 0x0080;
        const WE_HAVE_INSTRUCTIONS = 0x0100;
        const USE_MY_METRICS = 0x0200;
        const OVERLAP_COMPOUND = 0x0400;
        const SCALED_COMPONENT_OFFSET = 0x0800;
        const UNSCALED_COMPONENT_OFFSET = 0x1000;
    }
}

#[derive(Debug, PartialEq)]
enum ComponentScalingMode {
    ScaledOffset,
    UnscaledOffset,
    Default,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Component {
    pub glyphIndex: uint16,
    pub transformation: Affine,
    pub matchPoints: Option<(uint16, uint16)>,
    pub flags: ComponentFlags,
}

impl Component {
    pub fn recompute_flags(&self, more: bool, instructions: bool) -> ComponentFlags {
        let mut flags = self.flags
            & (ComponentFlags::ROUND_XY_TO_GRID
                | ComponentFlags::USE_MY_METRICS
                | ComponentFlags::SCALED_COMPONENT_OFFSET
                | ComponentFlags::UNSCALED_COMPONENT_OFFSET
                | ComponentFlags::OVERLAP_COMPOUND);
        if more {
            flags |= ComponentFlags::MORE_COMPONENTS;
        } else if instructions {
            flags |= ComponentFlags::WE_HAVE_INSTRUCTIONS;
        }
        let [x_scale, scale01, scale10, scale_y, translateX, translateY] =
            self.transformation.as_coeffs();
        if self.matchPoints.is_some() {
            let (x, y) = self.matchPoints.unwrap();
            if !(x <= 255 && y <= 255) {
                flags |= ComponentFlags::ARG_1_AND_2_ARE_WORDS;
            }
        } else {
            flags |= ComponentFlags::ARGS_ARE_XY_VALUES;
            let (x, y) = (translateX, translateY);
            if !((-128.0..=127.0).contains(&x) && (-128.0..=127.0).contains(&y)) {
                flags |= ComponentFlags::ARG_1_AND_2_ARE_WORDS;
            }
        }
        if scale01 != 0.0 || scale10 != 0.0 {
            flags |= ComponentFlags::WE_HAVE_A_TWO_BY_TWO;
        } else if (x_scale - scale_y).abs() > f64::EPSILON {
            flags |= ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE;
        } else if (x_scale - 1.0).abs() > f64::EPSILON {
            flags |= ComponentFlags::WE_HAVE_A_SCALE;
        }
        flags
    }
}

deserialize_visitor!(
    Component,
    ComponentVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let flags = read_field!(seq, ComponentFlags, "a component flag field");
        let glyphIndex = read_field!(seq, uint16, "a component glyph index");
        let mut matchPoints: Option<(uint16, uint16)> = None;
        let mut xOffset: i16 = 0;
        let mut yOffset: i16 = 0;
        if !flags.contains(ComponentFlags::ARGS_ARE_XY_VALUES) {
            // unsigned point values
            if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                let p1 = read_field!(seq, u16, "a component point value");
                let p2 = read_field!(seq, u16, "a component point value");
                matchPoints = Some((p1, p2));
            } else {
                let p1 = read_field!(seq, u8, "a component point value");
                let p2 = read_field!(seq, u8, "a component point value");
                matchPoints = Some((p1.into(), p2.into()));
            }
            if flags.contains(
                ComponentFlags::WE_HAVE_A_SCALE
                    | ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE
                    | ComponentFlags::WE_HAVE_A_TWO_BY_TWO,
            ) {
                return Err(serde::de::Error::custom("Simon is a lazy programmer"));
            }
        } else {
            // signed xy values
            if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                xOffset = read_field!(seq, i16, "a component point value");
                yOffset = read_field!(seq, i16, "a component point value");
            } else {
                xOffset = read_field!(seq, i8, "a component point value").into();
                yOffset = read_field!(seq, i8, "a component point value").into();
            }
        }
        let mut x_scale = 1.0_f64;
        let mut scale01 = 0.0_f64;
        let mut scale10 = 0.0_f64;
        let mut y_scale = 1.0_f64;
        if flags.contains(ComponentFlags::WE_HAVE_A_SCALE) {
            x_scale = F2DOT14::unpack(read_field!(seq, i16, "a scale")).into();
            y_scale = x_scale;
        } else if flags.contains(ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE) {
            x_scale = F2DOT14::unpack(read_field!(seq, i16, "an X scale")).into();
            y_scale = F2DOT14::unpack(read_field!(seq, i16, "a Y scale")).into();
        } else if flags.contains(ComponentFlags::WE_HAVE_A_TWO_BY_TWO) {
            x_scale = F2DOT14::unpack(read_field!(seq, i16, "a 2x2 component")).into();
            scale01 = F2DOT14::unpack(read_field!(seq, i16, "a 2x2 component")).into();
            scale10 = F2DOT14::unpack(read_field!(seq, i16, "a 2x2 component")).into();
            y_scale = F2DOT14::unpack(read_field!(seq, i16, "a 2x2 component")).into();
        }
        let transformation = Affine::new([
            x_scale,
            scale01,
            scale10,
            y_scale,
            xOffset.into(),
            yOffset.into(),
        ]);

        Ok(Component {
            glyphIndex,
            transformation,
            matchPoints,
            flags,
        })
    }
);
