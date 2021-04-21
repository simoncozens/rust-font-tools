#![allow(non_camel_case_types, non_snake_case)]

use bitflags::bitflags;
use itertools::izip;
use kurbo::Affine;
use otspec::de::CountedDeserializer;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_field_counted};
use otspec_macros::tables;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};

tables!(
    GlyphCore {
        int16	xMin
        int16	yMin
        int16	xMax
        int16	yMax
    }
);

#[derive(Debug, PartialEq, Copy, Clone)]
struct Point {
    x: int16,
    y: int16,
    on_curve: bool,
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    struct ComponentFlags: u16 {
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

bitflags! {
    #[derive(Serialize, Deserialize)]
    struct SimpleGlyphFlags: u8 {
        const ON_CURVE_POINT = 0x01;
        const X_SHORT_VECTOR = 0x02;
        const Y_SHORT_VECTOR = 0x04;
        const REPEAT_FLAG = 0x08;
        const X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR = 0x10;
        const Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR = 0x20;
        const OVERLAP_SIMPLE = 0x40;
        const RESERVED = 0x80;
    }
}

#[derive(Debug, PartialEq)]
enum ComponentScalingMode {
    ScaledOffset,
    UnscaledOffset,
    Default,
}

#[derive(Debug, PartialEq)]
struct Component {
    glyphIndex: uint16,
    transformation: Affine,
    matchPoints: Option<(uint16, uint16)>,
    flags: ComponentFlags,
}

#[derive(Debug, PartialEq)]
struct Glyph {
    xMin: int16,
    xMax: int16,
    yMin: int16,
    yMax: int16,
    contours: Option<Vec<Vec<Point>>>,
    instructions: Option<Vec<u8>>,
    components: Option<Vec<Component>>,
    overlap: bool,
}

#[derive(Debug, PartialEq, Deserialize)]
struct glyf {
    glyphs: Vec<Glyph>,
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
        if flags.contains(ComponentFlags::ARGS_ARE_XY_VALUES) {
            // unsigned point values
            if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                let p1 = read_field!(seq, u8, "a component point value");
                let p2 = read_field!(seq, u8, "a component point value");
                matchPoints = Some((p1.into(), p2.into()));
            } else {
                let p1 = read_field!(seq, u16, "a component point value");
                let p2 = read_field!(seq, u16, "a component point value");
                matchPoints = Some((p1, p2));
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
                xOffset = read_field!(seq, i8, "a component point value").into();
                yOffset = read_field!(seq, i8, "a component point value").into();
            } else {
                xOffset = read_field!(seq, i16, "a component point value");
                yOffset = read_field!(seq, i16, "a component point value");
            }
        }
        let mut trA = 1.0_f64;
        let mut trB = 0.0_f64;
        let mut trC = 1.0_f64;
        let mut trD = 0.0_f64;
        if flags.contains(ComponentFlags::WE_HAVE_A_SCALE) {
            let scale = read_field!(seq, i16, "a scale");
            trA = ((scale as f32) / 16384.0).into();
            trC = trA;
        } else if flags.contains(ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE) {
            let scaleX_i16 = read_field!(seq, i16, "an X scale");
            let scaleY_i16 = read_field!(seq, i16, "a Y scale");
            trA = ((scaleX_i16 as f32) / 16384.0).into();
            trC = ((scaleY_i16 as f32) / 16384.0).into();
        } else if flags.contains(ComponentFlags::WE_HAVE_A_TWO_BY_TWO) {
            let trA_i16 = read_field!(seq, i16, "a 2x2 component");
            let trB_i16 = read_field!(seq, i16, "a 2x2 component");
            let trC_i16 = read_field!(seq, i16, "a 2x2 component");
            let trD_i16 = read_field!(seq, i16, "a 2x2 component");
            trA = ((trA_i16 as f32) / 16384.0).into();
            trB = ((trB_i16 as f32) / 16384.0).into();
            trC = ((trC_i16 as f32) / 16384.0).into();
            trD = ((trD_i16 as f32) / 16384.0).into();
        }
        let transformation = Affine::new([trA, trB, trC, trD, xOffset.into(), yOffset.into()]);

        Ok(Component {
            glyphIndex,
            transformation,
            matchPoints,
            flags,
        })
    }
);

deserialize_visitor!(
    Glyph,
    GlyphVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        // println!("Reading a glyph");
        let maybe_num_contours = seq.next_element::<i16>()?;
        let num_contours = maybe_num_contours.unwrap();
        // println!("Num contours: {:?}", num_contours);
        let core = read_field!(seq, GlyphCore, "a glyph header");
        let mut components = None;
        let mut instructions: Option<Vec<u8>> = None;
        let mut contours: Option<Vec<Vec<Point>>> = None;
        let mut overlap = false;
        let mut has_instructions = false;
        if num_contours < 1 {
            let mut components_vec: Vec<Component> = Vec::new();
            loop {
                let comp = read_field!(seq, Component, "component");
                let has_more = comp.flags.contains(ComponentFlags::MORE_COMPONENTS);
                if comp.flags.contains(ComponentFlags::OVERLAP_COMPOUND) {
                    overlap = true;
                }
                if comp.flags.contains(ComponentFlags::WE_HAVE_INSTRUCTIONS) {
                    has_instructions = true;
                }
                components_vec.push(comp);
                if !has_more {
                    break;
                }
            }
            components = Some(components_vec);
            if has_instructions {
                let instructions_count = read_field!(seq, i16, "a count of instruction bytes");
                if instructions_count > 0 {
                    instructions =
                        Some(read_field_counted!(seq, instructions_count, "instructions"));
                }
            }
        } else {
            // println!("Reading {:?} contours", num_contours);
            let mut end_pts_of_contour: Vec<usize> = (0..num_contours as usize)
                .filter_map(|_| seq.next_element::<uint16>().unwrap())
                .map(|x| (1 + x) as usize)
                .collect();
            // println!("End points of contours: {:?}", end_pts_of_contour);
            let instructions_count = read_field!(seq, i16, "a count of instruction bytes");
            if instructions_count > 0 {
                instructions = Some(read_field_counted!(seq, instructions_count, "instructions"));
            }
            // println!("Instructions: {:?}", instruction_length);
            let num_points = *(end_pts_of_contour
                .last()
                .ok_or_else(|| serde::de::Error::custom("No points?"))?)
                as usize;
            let mut i = 0;
            // println!("Number of points: {:?}", num_points);
            let mut flags: Vec<SimpleGlyphFlags> = Vec::with_capacity(num_points);
            while i < num_points {
                let flag = read_field!(seq, SimpleGlyphFlags, "a glyph flag");
                flags.push(flag);
                if flag.contains(SimpleGlyphFlags::REPEAT_FLAG) {
                    let mut repeat_count = read_field!(seq, u8, "a flag repeat count");
                    // println!("Repeated flag! {:?}", repeat_count);
                    while repeat_count > 0 {
                        flags.push(flag);
                        repeat_count -= 1;
                    }
                }
                i += 1;
            }
            let mut x_coords: Vec<int16> = Vec::with_capacity(num_points);
            let mut y_coords: Vec<int16> = Vec::with_capacity(num_points);
            let mut last_x = 0_i16;
            let mut last_y = 0_i16;
            for i in 0..num_points {
                if flags[i].contains(SimpleGlyphFlags::X_SHORT_VECTOR) {
                    let coord = read_field!(seq, u8, "an X coordinate") as i16;
                    if flags[i].contains(SimpleGlyphFlags::X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR) {
                        last_x += coord;
                    } else {
                        last_x -= coord;
                    }
                    x_coords.push(last_x);
                    // println!("Read short X coordinate {:?}", coord);
                    // println!("X is now {:?}", last_x);
                } else if flags[i].contains(SimpleGlyphFlags::X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR)
                {
                    x_coords.push(last_x);
                    // println!("Elided X coordinate");
                    // println!("X is still {:?}", last_x);
                } else {
                    let coord = read_field!(seq, i16, "an X coordinate");
                    // println!("Read long X coordinate {:?}", coord);
                    last_x += coord;
                    // println!("X is now {:?}", last_x);
                    x_coords.push(last_x);
                }
            }
            for i in 0..num_points {
                if flags[i].contains(SimpleGlyphFlags::Y_SHORT_VECTOR) {
                    let coord = read_field!(seq, u8, "a Y coordinate") as i16;
                    if flags[i].contains(SimpleGlyphFlags::Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR) {
                        last_y += coord;
                    } else {
                        last_y -= coord;
                    }
                    // println!("Read short Y coordinate {:?}", coord);
                    // println!("Y is now {:?}", last_y);
                    y_coords.push(last_y);
                } else if flags[i].contains(SimpleGlyphFlags::Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR)
                {
                    y_coords.push(last_y);
                    // println!("Elided Y coordinate");
                    // println!("Y is still {:?}", last_y);
                } else {
                    let coord = read_field!(seq, i16, "a Y coordinate");
                    last_y += coord;
                    // println!("Read long Y coordinate {:?}", coord);
                    // println!("Y is now {:?}", last_y);
                    y_coords.push(last_y);
                }
                if flags[i].contains(SimpleGlyphFlags::OVERLAP_SIMPLE) {
                    overlap = true;
                }
            }
            // Divvy x/y coords into contours
            let points: Vec<Point> = izip!(&x_coords, &y_coords, &flags)
                .map(|(x, y, flag)| Point {
                    x: *x,
                    y: *y,
                    on_curve: flag.contains(SimpleGlyphFlags::ON_CURVE_POINT),
                })
                .collect();
            let mut contours_vec = Vec::with_capacity(num_contours as usize);
            end_pts_of_contour.insert(0, 0);
            for window in end_pts_of_contour.windows(2) {
                // println!("Window: {:?}", window);
                contours_vec.push(points[window[0]..window[1]].to_vec());
            }
            // println!("Contours: {:?}", contours_vec);

            contours = Some(contours_vec);
        }
        Ok(Glyph {
            contours,
            components,
            instructions,
            overlap,
            xMax: core.xMax,
            yMax: core.yMax,
            xMin: core.xMin,
            yMin: core.yMin,
        })
    }
);

#[cfg(test)]
mod tests {
    use crate::glyf;
    use crate::glyf::Point;

    #[test]
    fn glyf_de() {
        let binary_glyf = vec![
            0x00, 0x02, // Two contours
            0x00, 0x14, // xMin
            0x00, 0x00, // yMin
            0x02, 0x37, // xMax
            0x01, 0x22, // yMax
            0x00, 0x02, // end of first
            0x00, 0x0e, // end of second
            0x00, 0x00, // No instructions
            /* First contour flags */
            0x33, // pt0: Oncurve. X is short. Y is repeated.
            0x33, // pt1: Oncurve. X is short. Y is repeated.
            0x27, // pt2: Oncurve. X is short + negative. Y is short + positive.
            /* Second contour flags */
            0x24, // pt3: Offcurve. Y is short + positive
            0x36, // pt4:
            0x33, // pt5: Oncurve. X is short. Y is repeated.
            0x32, // pt6: Offcurve. X is short. Y is repeated.
            0x16, // pt7:
            0x15, // pt8:
            0x14, // pt9:
            0x06, // pt10: On curve, x and y short
            0x23, // pt11:
            0x22, // pt12:
            0x26, // pt13:
            0x35, // pt14:
            /* Point 0 */
            0x14, // X = 20
            /* Point 1 */
            0xc8, // X += 200
            /* Point 2 */
            0x78, // X -= 120
            0x01, // ???
            0x1e, 0x36, 0x25, 0x25, 0x35, 0x35, 0x25, 0x25, 0x36, 0xc8, 0x25, 0x35, 0x35, 0x25,
            0x25, 0x36, 0x36, 0x25, 0x00,
        ];
        let deserialized = otspec::de::from_bytes::<glyf::Glyph>(&binary_glyf).unwrap();
        let glyph = glyf::Glyph {
            xMin: 20,
            xMax: 567,
            yMin: 0,
            yMax: 290,
            contours: Some(vec![
                vec![
                    Point {
                        x: 20,
                        y: 0,
                        on_curve: true,
                    },
                    Point {
                        x: 220,
                        y: 0,
                        on_curve: true,
                    },
                    Point {
                        x: 100,
                        y: 200,
                        on_curve: true,
                    },
                ],
                vec![
                    Point {
                        x: 386,
                        y: 237,
                        on_curve: false,
                    },
                    Point {
                        x: 440,
                        y: 290,
                        on_curve: false,
                    },
                    Point {
                        x: 477,
                        y: 290,
                        on_curve: true,
                    },
                    Point {
                        x: 514,
                        y: 290,
                        on_curve: false,
                    },
                    Point {
                        x: 567,
                        y: 237,
                        on_curve: false,
                    },
                    Point {
                        x: 567,
                        y: 200,
                        on_curve: true,
                    },
                    Point {
                        x: 567,
                        y: 163,
                        on_curve: false,
                    },
                    Point {
                        x: 514,
                        y: 109,
                        on_curve: false,
                    },
                    Point {
                        x: 477,
                        y: 109,
                        on_curve: true,
                    },
                    Point {
                        x: 440,
                        y: 109,
                        on_curve: false,
                    },
                    Point {
                        x: 386,
                        y: 163,
                        on_curve: false,
                    },
                    Point {
                        x: 386,
                        y: 200,
                        on_curve: true,
                    },
                ],
            ]),
            instructions: None,
            components: None,
            overlap: false,
        };
        assert_eq!(deserialized, glyph);
    }
    // #[test]
    // fn post_serde_glyf2() {
    //     let binary_glyf = vec![
    //         0x00, 0x02, 0x00, 0x32, 0xff, 0x38, 0x01, 0xc2, 0x03, 0x20, 0x00, 0x03, 0x00, 0x07,
    //         0x00, 0x00, 0x17, 0x11, 0x21, 0x11, 0x25, 0x21, 0x11, 0x21, 0x32, 0x01, 0x90, 0xfe,
    //         0xa2, 0x01, 0x2c, 0xfe, 0xd4, 0xc8, 0x03, 0xe8, 0xfc, 0x18, 0x32, 0x03, 0x84, 0x00,
    //         0x00, 0x02, 0x00, 0x05, 0x00, 0x00, 0x02, 0xef, 0x02, 0xbc, 0x00, 0x07, 0x00, 0x0a,
    //         0x00, 0x00, 0x33, 0x01, 0x33, 0x01, 0x23, 0x27, 0x21, 0x07, 0x13, 0x21, 0x03, 0x05,
    //         0x01, 0x3e, 0x6e, 0x01, 0x3e, 0x6e, 0x5d, 0xfe, 0xac, 0x5d, 0x81, 0x01, 0x0c, 0x86,
    //         0x02, 0xbc, 0xfd, 0x44, 0xcc, 0xcc, 0x01, 0x1c, 0x01, 0x27, 0xff, 0xff, 0x00, 0x05,
    //         0x00, 0x00, 0x02, 0xef, 0x03, 0x93, 0x02, 0x26, 0x00, 0x01, 0x00, 0x00, 0x00, 0x07,
    //         0x00, 0x08, 0x01, 0x92, 0x00, 0x82, 0x00, 0x02, 0x00, 0x1e, 0xff, 0xf6, 0x02, 0x7a,
    //         0x02, 0xc6, 0x00, 0x07, 0x00, 0x0f, 0x00, 0x00, 0x05, 0x20, 0x11, 0x10, 0x21, 0x20,
    //         0x11, 0x10, 0x25, 0x32, 0x11, 0x10, 0x23, 0x22, 0x11, 0x10, 0x01, 0x4c, 0xfe, 0xd2,
    //         0x01, 0x2e, 0x01, 0x2e, 0xfe, 0xd2, 0xd4, 0xd4, 0xd4, 0x0a, 0x01, 0x68, 0x01, 0x68,
    //         0xfe, 0x98, 0xfe, 0x98, 0x54, 0x01, 0x14, 0x01, 0x14, 0xfe, 0xec, 0xfe, 0xec, 0x00,
    //         0x00, 0x01, 0x00, 0x05, 0x00, 0x00, 0x02, 0xef, 0x02, 0xbc, 0x00, 0x06, 0x00, 0x00,
    //         0x21, 0x01, 0x33, 0x01, 0x01, 0x33, 0x01, 0x01, 0x43, 0xfe, 0xc2, 0x6e, 0x01, 0x07,
    //         0x01, 0x07, 0x6e, 0xfe, 0xc2, 0x02, 0xbc, 0xfd, 0xbd, 0x02, 0x43, 0xfd, 0x44, 0x00,
    //         0x00, 0x03, 0x00, 0x1d, 0xff, 0xbc, 0x02, 0x44, 0x02, 0xf7, 0x00, 0x21, 0x00, 0x28,
    //         0x00, 0x2f, 0x00, 0x00, 0x01, 0x33, 0x15, 0x16, 0x16, 0x17, 0x07, 0x26, 0x27, 0x15,
    //         0x16, 0x16, 0x15, 0x14, 0x06, 0x07, 0x15, 0x23, 0x35, 0x26, 0x27, 0x37, 0x16, 0x16,
    //         0x17, 0x35, 0x27, 0x26, 0x26, 0x35, 0x34, 0x36, 0x36, 0x37, 0x07, 0x14, 0x16, 0x17,
    //         0x37, 0x06, 0x06, 0x13, 0x36, 0x36, 0x35, 0x34, 0x26, 0x27, 0x01, 0x08, 0x5a, 0x3d,
    //         0x68, 0x2b, 0x35, 0x45, 0x56, 0x71, 0x71, 0x79, 0x69, 0x5a, 0x93, 0x58, 0x34, 0x2a,
    //         0x54, 0x39, 0x0f, 0x65, 0x69, 0x38, 0x64, 0x41, 0x7f, 0x36, 0x46, 0x03, 0x3d, 0x42,
    //         0xd9, 0x45, 0x3f, 0x38, 0x4c, 0x02, 0xf7, 0x2c, 0x06, 0x2f, 0x2a, 0x48, 0x44, 0x0b,
    //         0xfe, 0x16, 0x58, 0x4f, 0x52, 0x6e, 0x0a, 0x32, 0x32, 0x0b, 0x51, 0x48, 0x24, 0x25,
    //         0x04, 0xe8, 0x03, 0x13, 0x61, 0x52, 0x39, 0x5b, 0x39, 0x08, 0xd5, 0x30, 0x30, 0x0e,
    //         0xeb, 0x09, 0x41, 0xfe, 0x1c, 0x07, 0x3b, 0x30, 0x2b, 0x2d, 0x0e, 0x00, 0x00, 0x01,
    //         0x00, 0x1d, 0xff, 0xb4, 0x02, 0x44, 0x02, 0xf7, 0x00, 0x2d, 0x00, 0x00, 0x01, 0x33,
    //         0x15, 0x16, 0x16, 0x17, 0x07, 0x26, 0x26, 0x23, 0x22, 0x06, 0x15, 0x14, 0x16, 0x17,
    //         0x17, 0x16, 0x16, 0x15, 0x14, 0x06, 0x07, 0x15, 0x23, 0x35, 0x26, 0x27, 0x37, 0x16,
    //         0x16, 0x33, 0x32, 0x36, 0x35, 0x34, 0x26, 0x27, 0x27, 0x26, 0x26, 0x35, 0x34, 0x36,
    //         0x36, 0x37, 0x01, 0x08, 0x5a, 0x3d, 0x68, 0x2b, 0x35, 0x2d, 0x5e, 0x3e, 0x52, 0x59,
    //         0x36, 0x46, 0x63, 0x6d, 0x6f, 0x79, 0x69, 0x5a, 0x93, 0x58, 0x34, 0x32, 0x66, 0x4d,
    //         0x5d, 0x53, 0x3a, 0x50, 0x63, 0x65, 0x69, 0x38, 0x64, 0x41, 0x02, 0xf7, 0x2c, 0x06,
    //         0x2f, 0x2a, 0x48, 0x2c, 0x26, 0x44, 0x3c, 0x30, 0x30, 0x0e, 0x14, 0x16, 0x57, 0x4f,
    //         0x52, 0x6e, 0x0a, 0x3a, 0x3a, 0x0b, 0x51, 0x48, 0x2b, 0x24, 0x3d, 0x37, 0x2c, 0x2d,
    //         0x0e, 0x12, 0x13, 0x61, 0x52, 0x39, 0x5b, 0x39, 0x08, 0x00, 0x00, 0x01, 0xff, 0x73,
    //         0x02, 0x76, 0x00, 0x7d, 0x03, 0x11, 0x00, 0x03, 0x00, 0x00, 0x03, 0x37, 0x07, 0x07,
    //         0x76, 0xf3, 0x17, 0xf3, 0x02, 0xcc, 0x45, 0x56, 0x45, 0x00, 0x00, 0x01, 0x00, 0x1d,
    //         0xff, 0xb4, 0x02, 0x44, 0x02, 0xf7, 0x00, 0x2d, 0x00, 0x00, 0x01, 0x33, 0x15, 0x16,
    //         0x16, 0x17, 0x07, 0x26, 0x26, 0x23, 0x22, 0x06, 0x15, 0x14, 0x16, 0x17, 0x17, 0x16,
    //         0x16, 0x15, 0x14, 0x06, 0x07, 0x15, 0x23, 0x35, 0x26, 0x27, 0x37, 0x16, 0x16, 0x33,
    //         0x32, 0x36, 0x35, 0x34, 0x26, 0x27, 0x27, 0x26, 0x26, 0x35, 0x34, 0x36, 0x36, 0x37,
    //         0x01, 0x08, 0x5a, 0x3d, 0x68, 0x2b, 0x35, 0x2d, 0x5e, 0x3e, 0x52, 0x59, 0x36, 0x46,
    //         0x63, 0x6d, 0x6f, 0x79, 0x69, 0x5a, 0x93, 0x58, 0x34, 0x32, 0x66, 0x4d, 0x5d, 0x53,
    //         0x3a, 0x50, 0x63, 0x65, 0x69, 0x38, 0x64, 0x41, 0x02, 0xf7, 0x2c, 0x06, 0x2f, 0x2a,
    //         0x48, 0x2c, 0x26, 0x44, 0x3c, 0x30, 0x30, 0x0e, 0x14, 0x16, 0x57, 0x4f, 0x52, 0x6e,
    //         0x0a, 0x3a, 0x3a, 0x0b, 0x51, 0x48, 0x2b, 0x24, 0x3d, 0x37, 0x2c, 0x2d, 0x0e, 0x12,
    //         0x13, 0x61, 0x52, 0x39, 0x5b, 0x39, 0x08, 0x00,
    //     ];
    //     let deserialized: glyf::glyf = otspec::de::from_bytes(&binary_glyf).unwrap();
    //     // assert_eq!(deserialized.version, U16F16::from_num(2.0));
    // }
}
