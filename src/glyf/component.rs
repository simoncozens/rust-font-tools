/// Structures for handling components within a composite glyph
use bitflags::bitflags;
use kurbo::Affine;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field};
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

bitflags! {
    /// Flags used when serializing/deserializing the component.
    ///
    /// These are computed automatically, so you don't need to worry about them.
    #[derive(Serialize, Deserialize)]
    pub struct ComponentFlags: u16 {
        ///  If this is set, the arguments are 16-bit (uint16 or int16); otherwise, they are bytes (uint8 or int8).
        const ARG_1_AND_2_ARE_WORDS = 0x0001;
        ///  If this is set, the arguments are signed xy values; otherwise, they are unsigned point numbers.
        const ARGS_ARE_XY_VALUES = 0x0002;
        /// For the xy values if the preceding is true.
        ///
        /// (Whatever that means.)
        const ROUND_XY_TO_GRID = 0x0004;
        /// The transform matrix is a simple linear scale.
        const WE_HAVE_A_SCALE = 0x0008;
        /// Indicates at least one more component after this one.
        const MORE_COMPONENTS = 0x0020;
        /// The transform matrix is a scaling transform with independent X and Y scales.
        const WE_HAVE_AN_X_AND_Y_SCALE = 0x0040;
        /// The transform matrix is a full two-by-two matrix with scaleXY and scaleYX values.
        const WE_HAVE_A_TWO_BY_TWO = 0x0080;
        /// TrueType instructions follow this component.
        const WE_HAVE_INSTRUCTIONS = 0x0100;
        /// The metrics of the composite glyph should be the same as the metrics of this component.
        const USE_MY_METRICS = 0x0200;
        /// The contours of the components overlap.
        ///
        /// Generally unused, but if used, should be set on the first component of a glyph.
        const OVERLAP_COMPOUND = 0x0400;
        /// The component's offset should be scaled.
        const SCALED_COMPONENT_OFFSET = 0x0800;
        /// The component's offset should not be scaled.
        const UNSCALED_COMPONENT_OFFSET = 0x1000;
    }
}

#[derive(Debug, PartialEq)]
enum ComponentScalingMode {
    ScaledOffset,
    UnscaledOffset,
    Default,
}

/// A high-level representation of a component within a glyph
#[derive(Debug, PartialEq, Clone)]
pub struct Component {
    /// The glyph ID that this component references.
    pub glyphIndex: uint16,
    /// An affine transformation applied to the component's contours.
    pub transformation: Affine,
    /// Alternate, and rarely used, method of positioning components using contour point numbers.
    pub matchPoints: Option<(uint16, uint16)>,
    /// Flags.
    /// Most of these are calculated automatically on serialization. Those which can be
    /// meaningfully manually set are `ROUND_XY_TO_GRID`, `USE_MY_METRICS`,
    /// `SCALED_COMPONENT_OFFSET`, `UNSCALED_COMPONENT_OFFSET` and `OVERLAP_COMPOUND`.
    pub flags: ComponentFlags,
}

impl Component {
    /// Recompute the flags prior to serialization. `more` should be true if this
    /// is not the final component in a glyph; `instructions` should be true if
    /// there are TrueType instructions in the glyph. This is called automatically
    /// on serialization; you don't have to do it yourself.
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
