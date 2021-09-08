/// Structures for handling components within a composite glyph
use bitflags::bitflags;
use kurbo::Affine;
use otspec::types::*;
use otspec::{DeserializationError, Deserialize, Deserializer, ReaderContext};
use otspec_macros::{Deserialize, Serialize};

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

/*
#[derive(Debug, PartialEq)]
enum ComponentScalingMode {
    ScaledOffset,
    UnscaledOffset,
    Default,
}
*/

/// A high-level representation of a component within a glyph
#[derive(Debug, PartialEq, Clone)]
pub struct Component {
    /// The glyph ID that this component references.
    pub glyph_index: uint16,
    /// An affine transformation applied to the component's contours.
    pub transformation: Affine,
    /// Alternate, and rarely used, method of positioning components using contour point numbers.
    pub match_points: Option<(uint16, uint16)>,
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
        let [x_scale, scale01, scale10, scale_y, translate_x, translate_y] =
            self.transformation.as_coeffs();
        if self.match_points.is_some() {
            let (x, y) = self.match_points.unwrap();
            if !(x <= 255 && y <= 255) {
                flags |= ComponentFlags::ARG_1_AND_2_ARE_WORDS;
            }
        } else {
            flags |= ComponentFlags::ARGS_ARE_XY_VALUES;
            let (x, y) = (translate_x, translate_y);
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

fn read_f64_from_f2dot14(c: &mut ReaderContext) -> Result<f64, DeserializationError> {
    let x: F2DOT14 = c.de()?;
    let x_f32: f32 = x.into();
    Ok(x_f32 as f64)
}

impl Deserialize for Component {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let flags: ComponentFlags = c.de()?;
        let glyph_index: uint16 = c.de()?;
        let mut match_points: Option<(uint16, uint16)> = None;
        let mut x_offset: i16 = 0;
        let mut y_offset: i16 = 0;
        if !flags.contains(ComponentFlags::ARGS_ARE_XY_VALUES) {
            // unsigned point values
            if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                let p1: u16 = c.de()?;
                let p2: u16 = c.de()?;
                match_points = Some((p1, p2));
            } else {
                let p1: u8 = c.de()?;
                let p2: u8 = c.de()?;
                match_points = Some((p1.into(), p2.into()));
            }
            if flags.contains(
                ComponentFlags::WE_HAVE_A_SCALE
                    | ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE
                    | ComponentFlags::WE_HAVE_A_TWO_BY_TWO,
            ) {
                return Err(DeserializationError(
                    "Simon is a lazy programmer".to_string(),
                ));
            }
        } else {
            // signed xy values
            if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                x_offset = c.de()?;
                y_offset = c.de()?;
            } else {
                let x_off_i8: i8 = c.de()?;
                let y_off_i8: i8 = c.de()?;
                x_offset = x_off_i8.into();
                y_offset = y_off_i8.into();
            }
        }
        let mut x_scale = 1.0_f64;
        let mut scale01 = 0.0_f64;
        let mut scale10 = 0.0_f64;
        let mut y_scale = 1.0_f64;
        if flags.contains(ComponentFlags::WE_HAVE_A_SCALE) {
            x_scale = read_f64_from_f2dot14(c)?;
            y_scale = x_scale;
        } else if flags.contains(ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE) {
            x_scale = read_f64_from_f2dot14(c)?;
            y_scale = read_f64_from_f2dot14(c)?;
        } else if flags.contains(ComponentFlags::WE_HAVE_A_TWO_BY_TWO) {
            x_scale = read_f64_from_f2dot14(c)?;
            scale01 = read_f64_from_f2dot14(c)?;
            scale10 = read_f64_from_f2dot14(c)?;
            y_scale = read_f64_from_f2dot14(c)?;
        }
        let transformation = Affine::new([
            x_scale,
            scale01,
            scale10,
            y_scale,
            x_offset.into(),
            y_offset.into(),
        ]);

        Ok(Component {
            glyph_index,
            transformation,
            match_points,
            flags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font;

    #[test]
    fn test_glyf_component() {
        let binary_font = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x00, 0x40, 0x00, 0x02, 0x00, 0x20, 0x63, 0x6d,
            0x61, 0x70, 0x00, 0x0c, 0x06, 0xc2, 0x00, 0x00, 0x01, 0x84, 0x00, 0x00, 0x00, 0x34,
            0x67, 0x6c, 0x79, 0x66, 0xc5, 0x94, 0x78, 0xca, 0x00, 0x00, 0x00, 0x6c, 0x00, 0x00,
            0x00, 0x94, 0x68, 0x65, 0x61, 0x64, 0x28, 0xd0, 0x20, 0xfc, 0x00, 0x00, 0x01, 0x28,
            0x00, 0x00, 0x00, 0x36, 0x68, 0x68, 0x65, 0x61, 0x12, 0x54, 0x0f, 0x44, 0x00, 0x00,
            0x01, 0x60, 0x00, 0x00, 0x00, 0x24, 0x6c, 0x6f, 0x63, 0x61, 0x00, 0x4a, 0x00, 0x08,
            0x00, 0x00, 0x01, 0x20, 0x00, 0x00, 0x00, 0x06, 0x6d, 0x61, 0x78, 0x70, 0x00, 0x29,
            0x03, 0x0d, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x20, 0xff, 0xff, 0x00, 0x32,
            0xfe, 0x45, 0x02, 0xd2, 0x01, 0xd6, 0x02, 0x06, 0x00, 0x01, 0x00, 0xfe, 0x00, 0x01,
            0x00, 0x32, 0xfe, 0x47, 0x02, 0xd2, 0x01, 0xd8, 0x00, 0x2c, 0x00, 0x00, 0x01, 0x22,
            0x26, 0x26, 0x35, 0x34, 0x36, 0x37, 0x17, 0x06, 0x06, 0x15, 0x14, 0x16, 0x33, 0x32,
            0x3e, 0x03, 0x35, 0x34, 0x26, 0x27, 0x06, 0x06, 0x23, 0x22, 0x26, 0x35, 0x37, 0x36,
            0x36, 0x33, 0x32, 0x1e, 0x02, 0x15, 0x14, 0x06, 0x07, 0x0e, 0x02, 0x01, 0x19, 0x4c,
            0x67, 0x34, 0x3f, 0x2e, 0x1c, 0x20, 0x34, 0x6e, 0x67, 0x4e, 0x78, 0x56, 0x37, 0x1a,
            0x11, 0x14, 0x10, 0x2b, 0x11, 0x19, 0x2f, 0x36, 0x0b, 0x1a, 0x0a, 0x27, 0x31, 0x1b,
            0x0a, 0x3b, 0x37, 0x23, 0x63, 0x7a, 0xfe, 0x47, 0x4a, 0x7f, 0x4f, 0x62, 0xcd, 0x5e,
            0x0d, 0x4d, 0x98, 0x56, 0x5b, 0x6a, 0x36, 0x5a, 0x6e, 0x74, 0x33, 0x22, 0x40, 0x1b,
            0x12, 0x0f, 0x20, 0x2e, 0x99, 0x09, 0x08, 0x33, 0x52, 0x5e, 0x2b, 0x77, 0xe0, 0x5b,
            0x3b, 0x5f, 0x37, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x02, 0x34, 0x00, 0x13,
            0x00, 0xd7, 0x00, 0x0e, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x4a,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x39, 0xbf, 0xc4, 0x4c,
            0x5f, 0x0f, 0x3c, 0xf5, 0x00, 0x03, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x00, 0xdd, 0x29,
            0x99, 0xed, 0x00, 0x00, 0x00, 0x00, 0xdd, 0x5e, 0x45, 0xd1, 0xfe, 0xf1, 0xfa, 0xcb,
            0x10, 0x40, 0x05, 0x90, 0x00, 0x00, 0x00, 0x06, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x03, 0x20, 0xfc, 0xe0, 0x00, 0x00, 0x10, 0x96,
            0xfe, 0xf1, 0xfd, 0xb9, 0x10, 0x40, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x14, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x14, 0x00, 0x03, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x14, 0x00, 0x04, 0x00, 0x20, 0x00, 0x00, 0x00, 0x04, 0x00, 0x04, 0x00, 0x01,
            0x00, 0x00, 0x06, 0x6f, 0xff, 0xff, 0x00, 0x00, 0x06, 0x6f, 0xff, 0xff, 0xf9, 0x92,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut deserialized: font::Font = otspec::de::from_bytes(&binary_font).unwrap();
        deserialized.fully_deserialize();
        let glyf = deserialized
            .get_table(b"glyf")
            .unwrap()
            .unwrap()
            .glyf_unchecked();
        assert_eq!(
            glyf.glyphs[0].components[0].transformation,
            Affine::new([
            1.0,
            0.0,
            0.0,
            1.0,
            0.0,
            -2.0, // Not 254
        ])
        );
    }
}
