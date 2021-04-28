#![allow(non_camel_case_types, non_snake_case)]
use bitflags::bitflags;
use itertools::izip;
use kurbo::Affine;
use otspec::de::CountedDeserializer;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_field_counted, read_remainder};
use otspec_macros::tables;
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};

use std::fmt;

tables!(
    GlyphCore {
        int16	xMin
        int16	yMin
        int16	xMax
        int16	yMax
    }
);

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Point {
    pub x: int16,
    pub y: int16,
    pub on_curve: bool,
}

impl Point {
    pub fn transform(&self, t: Affine) -> Point {
        let kurbo_point = t * kurbo::Point::new(self.x as f64, self.y as f64);
        Point {
            x: kurbo_point.x as i16,
            y: kurbo_point.y as i16,
            on_curve: self.on_curve,
        }
    }
}

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
pub struct Component {
    pub glyphIndex: uint16,
    pub transformation: Affine,
    pub matchPoints: Option<(uint16, uint16)>,
    pub flags: ComponentFlags,
}

impl Component {
    fn recompute_flags(&self, more: bool, instructions: bool) -> ComponentFlags {
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
        let [scaleX, shearX, shearY, scaleY, translateX, translateY] =
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
        if shearX != 0.0 || shearY != 0.0 {
            flags |= ComponentFlags::WE_HAVE_A_TWO_BY_TWO;
        } else if (scaleX - scaleY).abs() > f64::EPSILON {
            flags |= ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE;
        } else if (scaleX - 1.0).abs() > f64::EPSILON {
            flags |= ComponentFlags::WE_HAVE_A_SCALE;
        }
        flags
    }
}

#[derive(Debug, PartialEq)]
pub struct Glyph {
    pub xMin: int16,
    pub xMax: int16,
    pub yMin: int16,
    pub yMax: int16,
    pub contours: Vec<Vec<Point>>,
    pub instructions: Vec<u8>,
    pub components: Vec<Component>,
    pub overlap: bool,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct glyf {
    pub glyphs: Vec<Glyph>,
}

pub struct GlyfDeserializer {
    locaOffsets: Vec<Option<u32>>,
}

impl<'de> DeserializeSeed<'de> for GlyfDeserializer {
    type Value = glyf;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct GlyfDeserializerVisitor {
            locaOffsets: Vec<Option<u32>>,
        }

        impl<'de> Visitor<'de> for GlyfDeserializerVisitor {
            type Value = glyf;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a glyf table")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<glyf, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut res = glyf { glyphs: Vec::new() };
                let remainder = read_remainder!(seq, "a glyph table");
                for item in self.locaOffsets {
                    match item {
                        None => res.glyphs.push(Glyph {
                            contours: vec![],
                            components: vec![],
                            overlap: false,
                            xMax: 0,
                            xMin: 0,
                            yMax: 0,
                            yMin: 0,
                            instructions: vec![],
                        }),
                        Some(item) => {
                            let binary_glyf = &remainder[(item as usize)..];
                            // println!("Reading glyf at item {:?}", item);
                            // println!("Reading binary glyf {:?}", binary_glyf);
                            let glyph: Glyph =
                                otspec::de::from_bytes(binary_glyf).map_err(|e| {
                                    serde::de::Error::custom(format!("Expecting a glyph: {:?}", e))
                                })?;
                            res.glyphs.push(glyph)
                        }
                    }
                }
                Ok(res)
            }
        }

        deserializer.deserialize_seq(GlyfDeserializerVisitor {
            locaOffsets: self.locaOffsets,
        })
    }
}

impl Serialize for glyf {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let seq = serializer.serialize_seq(None)?;
        // u32 or u16?
        seq.end()
    }
}

pub fn from_bytes(s: &[u8], locaOffsets: Vec<Option<u32>>) -> otspec::error::Result<glyf> {
    let mut deserializer = otspec::de::Deserializer::from_bytes(s);
    let cs: GlyfDeserializer = GlyfDeserializer { locaOffsets };
    cs.deserialize(&mut deserializer)
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
        let mut trA = 1.0_f64;
        let mut trB = 0.0_f64;
        let mut trC = 1.0_f64;
        let mut trD = 0.0_f64;
        if flags.contains(ComponentFlags::WE_HAVE_A_SCALE) {
            trA = F2DOT14::unpack(read_field!(seq, i16, "a scale")).into();
            trC = trA;
        } else if flags.contains(ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE) {
            trA = F2DOT14::unpack(read_field!(seq, i16, "an X scale")).into();
            trC = F2DOT14::unpack(read_field!(seq, i16, "a Y scale")).into();
        } else if flags.contains(ComponentFlags::WE_HAVE_A_TWO_BY_TWO) {
            trA = F2DOT14::unpack(read_field!(seq, i16, "a 2x2 component")).into();
            trB = F2DOT14::unpack(read_field!(seq, i16, "a 2x2 component")).into();
            trC = F2DOT14::unpack(read_field!(seq, i16, "a 2x2 component")).into();
            trD = F2DOT14::unpack(read_field!(seq, i16, "a 2x2 component")).into();
        }
        // "Note that this convention is transposed from PostScript and Direct2D"
        let transformation = Affine::new([trA, trB, trD, trC, xOffset.into(), yOffset.into()]);

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
        let mut components: Vec<Component> = vec![];
        let mut instructions: Vec<u8> = vec![];
        let mut contours: Vec<Vec<Point>> = Vec::with_capacity(if num_contours < 1 {
            0
        } else {
            num_contours as usize
        });
        let mut overlap = false;
        let mut has_instructions = false;
        if num_contours < 1 {
            loop {
                let comp = read_field!(seq, Component, "component");
                let has_more = comp.flags.contains(ComponentFlags::MORE_COMPONENTS);
                if comp.flags.contains(ComponentFlags::OVERLAP_COMPOUND) {
                    overlap = true;
                }
                if comp.flags.contains(ComponentFlags::WE_HAVE_INSTRUCTIONS) {
                    has_instructions = true;
                }
                components.push(comp);
                if !has_more {
                    break;
                }
            }
            if has_instructions {
                let instructions_count = read_field!(seq, i16, "a count of instruction bytes");
                if instructions_count > 0 {
                    instructions = read_field_counted!(seq, instructions_count, "instructions");
                }
            }
        } else {
            // println!("Reading {:?} contours", num_contours);
            let mut end_pts_of_contour: Vec<usize> = (0..num_contours as usize)
                .filter_map(|_| seq.next_element::<uint16>().unwrap())
                .map(|x| (1 + x) as usize)
                .collect();
            let instructions_count = read_field!(seq, i16, "a count of instruction bytes");
            if instructions_count > 0 {
                instructions = read_field_counted!(seq, instructions_count, "instructions");
            }
            // println!("Instructions: {:?}", instructions);
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
                        i += 1;
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
            end_pts_of_contour.insert(0, 0);
            for window in end_pts_of_contour.windows(2) {
                // println!("Window: {:?}", window);
                contours.push(points[window[0]..window[1]].to_vec());
            }
            // println!("Contours: {:?}", contours_vec);
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

impl Glyph {
    pub fn has_components(&self) -> bool {
        !self.components.is_empty()
    }
    pub fn is_empty(&self) -> bool {
        self.components.is_empty() && self.contours.is_empty()
    }
    pub fn bounds_rect(&self) -> kurbo::Rect {
        kurbo::Rect::new(
            self.xMin.into(),
            self.yMin.into(),
            self.xMax.into(),
            self.yMax.into(),
        )
    }
    pub fn set_bounds_rect(&mut self, r: kurbo::Rect) {
        self.xMin = r.min_x() as i16;
        self.xMax = r.max_x() as i16;
        self.yMin = r.min_y() as i16;
        self.yMax = r.max_y() as i16;
    }

    pub fn recalc_bounds(&mut self) {
        if self.has_components() {
            // self.components
            //    .iter()
            //    .map({
            //        |comp| {
            //            glyphs[comp.glyphIndex as usize]
            //                .as_ref()
            //                .map(|component_glyph| {
            //                    comp.transformation
            //                        .transform_rect_bbox(component_glyph.bounds_rect())
            //                })
            //        }
            //    })
            //    .flatten()
            //    .reduce(|a, b| a.union(b))
            //    .unwrap();
            return;
        }
        let (x_pts, y_pts): (Vec<i16>, Vec<i16>) = self
            .contours
            .iter()
            .flatten()
            .map(|pt| (pt.x, pt.y))
            .unzip();
        self.xMin = *x_pts.iter().min().unwrap_or(&0);
        self.xMax = *x_pts.iter().max().unwrap_or(&0);
        self.yMin = *y_pts.iter().min().unwrap_or(&0);
        self.yMax = *y_pts.iter().max().unwrap_or(&0);
    }
    fn end_points(&self) -> Vec<u16> {
        assert!(!self.has_components());
        let mut count: i16 = -1;
        let mut end_points = Vec::new();
        for contour in &self.contours {
            count += contour.len() as i16;
            end_points.push(count as u16);
        }
        end_points
    }
    pub fn insert_explicit_oncurves(&mut self) {
        if self.contours.is_empty() {
            return;
        }
        for contour in self.contours.iter_mut() {
            for i in (0..contour.len() - 1).rev() {
                if !contour[i].on_curve && !contour[i + 1].on_curve {
                    contour.insert(
                        i + 1,
                        Point {
                            on_curve: true,
                            x: (contour[i].x + contour[i + 1].x) / 2,
                            y: (contour[i].y + contour[i + 1].y) / 2,
                        },
                    )
                }
            }
        }
    }
    fn _compileDeltasGreedy(&self) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        assert!(!self.has_components());
        let mut last_x = 0;
        let mut last_y = 0;
        let mut compressed_flags: Vec<u8> = vec![];
        let mut compressed_xs: Vec<u8> = vec![];
        let mut compressed_ys: Vec<u8> = vec![];
        for point in self.contours.iter().flatten() {
            let mut x = point.x - last_x;
            let mut y = point.y - last_y;
            let mut flag = if point.on_curve {
                SimpleGlyphFlags::ON_CURVE_POINT
            } else {
                SimpleGlyphFlags::empty()
            };
            if x == 0 {
                flag |= SimpleGlyphFlags::X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR
            } else if -255 <= x && x <= 255 {
                flag |= SimpleGlyphFlags::X_SHORT_VECTOR;
                if x > 0 {
                    flag |= SimpleGlyphFlags::X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR
                } else {
                    x = -x;
                }
                compressed_xs.push(x as u8);
            } else {
                compressed_xs.extend(&i16::to_be_bytes(x as i16));
            }
            if y == 0 {
                flag |= SimpleGlyphFlags::Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR
            } else if -255 <= y && y <= 255 {
                flag |= SimpleGlyphFlags::Y_SHORT_VECTOR;
                if y > 0 {
                    flag |= SimpleGlyphFlags::Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR
                } else {
                    y = -y;
                }
                compressed_ys.push(y as u8);
            } else {
                compressed_ys.extend(&i16::to_be_bytes(y as i16));
            }
            /* Not gonna do repeating flags today */
            compressed_flags.push(flag.bits());

            last_x = point.x;
            last_y = point.y;
        }
        (compressed_flags, compressed_xs, compressed_ys)
    }

    pub fn decompose(&self, glyphs: &[Glyph]) -> Glyph {
        let mut newglyph = Glyph {
            xMin: 0,
            xMax: 0,
            yMin: 0,
            yMax: 0,
            instructions: vec![],
            overlap: self.overlap,
            contours: vec![],
            components: vec![],
        };
        let mut new_contours = vec![];
        new_contours.extend(self.contours.clone());
        for comp in &self.components {
            let ix = comp.glyphIndex;
            match glyphs.get(ix as usize) {
                None => {
                    println!("Component not found for ID={:?}", ix);
                }
                Some(other_glyph) => {
                    for c in &other_glyph.contours {
                        new_contours.push(
                            c.iter()
                                .map(|pt| pt.transform(comp.transformation))
                                .collect(),
                        );
                    }
                    if other_glyph.has_components() {
                        println!("Found nested components while decomposing");
                    }
                }
            }
        }
        if !new_contours.is_empty() {
            newglyph.contours = new_contours;
            newglyph.recalc_bounds();
        }
        newglyph
    }
}

impl Serialize for Glyph {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        if self.is_empty() {
            return seq.end();
        }
        seq.serialize_element::<i16>(
            &(if self.has_components() {
                -1
            } else {
                self.contours.len() as i16
            }),
        )?;
        // recalc bounds?
        seq.serialize_element::<GlyphCore>(&GlyphCore {
            xMin: self.xMin,
            xMax: self.xMax,
            yMin: self.yMin,
            yMax: self.yMax,
        })?;
        if self.has_components() {
            for (i, comp) in self.components.iter().enumerate() {
                let flags = comp
                    .recompute_flags(i < self.components.len() - 1, !self.instructions.is_empty());
                seq.serialize_element::<uint16>(&flags.bits())?;
                seq.serialize_element::<uint16>(&comp.glyphIndex)?;
                let [scaleX, shearX, shearY, scaleY, translateX, translateY] =
                    comp.transformation.as_coeffs();
                if flags.contains(ComponentFlags::ARGS_ARE_XY_VALUES) {
                    if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                        seq.serialize_element::<i16>(&(translateX.round() as i16))?;
                        seq.serialize_element::<i16>(&(translateY as i16))?;
                    } else {
                        seq.serialize_element::<i8>(&(translateX.round() as i8))?;
                        seq.serialize_element::<i8>(&(translateY as i8))?;
                    }
                } else {
                    let (x, y) = comp.matchPoints.unwrap();
                    if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                        seq.serialize_element::<i16>(&(x as i16))?;
                        seq.serialize_element::<i16>(&(y as i16))?;
                    } else {
                        seq.serialize_element::<i8>(&(x as i8))?;
                        seq.serialize_element::<i8>(&(y as i8))?;
                    }
                }
                if flags.contains(ComponentFlags::WE_HAVE_A_TWO_BY_TWO) {
                    F2DOT14::serialize_element(&(scaleX as f32), &mut seq)?;
                    F2DOT14::serialize_element(&(shearY as f32), &mut seq)?;
                    F2DOT14::serialize_element(&(shearX as f32), &mut seq)?;
                    F2DOT14::serialize_element(&(scaleY as f32), &mut seq)?;
                } else if flags.contains(ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE) {
                    F2DOT14::serialize_element(&(scaleX as f32), &mut seq)?;
                    F2DOT14::serialize_element(&(scaleY as f32), &mut seq)?;
                } else if flags.contains(ComponentFlags::WE_HAVE_A_SCALE) {
                    F2DOT14::serialize_element(&(scaleX as f32), &mut seq)?;
                }
                if flags.contains(ComponentFlags::WE_HAVE_INSTRUCTIONS) {
                    seq.serialize_element::<uint16>(&(self.instructions.len() as u16))?;
                    seq.serialize_element::<Vec<u8>>(&self.instructions)?;
                }
            }
        } else {
            let end_pts_of_contour = self.end_points();
            seq.serialize_element::<Vec<uint16>>(&end_pts_of_contour)?;
            if self.instructions.len() > 0 {
                seq.serialize_element::<uint16>(&(self.instructions.len() as u16))?;
                seq.serialize_element::<Vec<u8>>(&self.instructions)?;
            } else {
                seq.serialize_element::<uint16>(&0)?;
            }
            let (compressed_flags, compressed_xs, compressed_ys) = self._compileDeltasGreedy();
            seq.serialize_element::<Vec<u8>>(&compressed_flags)?;
            seq.serialize_element::<Vec<u8>>(&compressed_xs)?;
            seq.serialize_element::<Vec<u8>>(&compressed_ys)?;
        }
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::font;
    use crate::glyf;
    use crate::glyf::ComponentFlags;
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
            0x25, 0x36, 0x36, 0x25,
        ];
        let deserialized = otspec::de::from_bytes::<glyf::Glyph>(&binary_glyf).unwrap();
        #[rustfmt::skip]
        let glyph = glyf::Glyph {
            xMin: 20, xMax: 567, yMin: 0, yMax: 290,
            contours: vec![
                vec![
                    Point {x: 20, y: 0, on_curve: true, },
                    Point {x: 220, y: 0, on_curve: true, },
                    Point {x: 100, y: 200, on_curve: true, },
                ],
                vec![
                    Point {x: 386, y: 237, on_curve: false, },
                    Point {x: 440, y: 290, on_curve: false, },
                    Point {x: 477, y: 290, on_curve: true, },
                    Point {x: 514, y: 290, on_curve: false, },
                    Point {x: 567, y: 237, on_curve: false, },
                    Point {x: 567, y: 200, on_curve: true, },
                    Point {x: 567, y: 163, on_curve: false, },
                    Point {x: 514, y: 109, on_curve: false, },
                    Point {x: 477, y: 109, on_curve: true, },
                    Point {x: 440, y: 109, on_curve: false, },
                    Point {x: 386, y: 163, on_curve: false, },
                    Point {x: 386, y: 200, on_curve: true, },
                ],
            ],
            instructions: vec![],
            components: vec![],
            overlap: false,
        };
        assert_eq!(deserialized, glyph);
        let serialized = otspec::ser::to_bytes(&glyph).unwrap();
        // println!("Got:      {:02x?}", serialized);
        // println!("Expected: {:02x?}", binary_glyf);
        assert_eq!(serialized, binary_glyf);
    }

    #[test]
    fn test_glyf_de() {
        let binary_font = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x80, 0x00, 0x03, 0x00, 0x20, 0x4f, 0x53,
            0x2f, 0x32, 0x47, 0x36, 0x45, 0x90, 0x00, 0x00, 0x01, 0x28, 0x00, 0x00, 0x00, 0x60,
            0x63, 0x6d, 0x61, 0x70, 0x01, 0x5c, 0x04, 0x51, 0x00, 0x00, 0x01, 0xa8, 0x00, 0x00,
            0x00, 0x64, 0x67, 0x6c, 0x79, 0x66, 0x01, 0x73, 0xbf, 0xf8, 0x00, 0x00, 0x02, 0x20,
            0x00, 0x00, 0x02, 0x1e, 0x68, 0x65, 0x61, 0x64, 0x1a, 0x46, 0x65, 0x4f, 0x00, 0x00,
            0x00, 0xac, 0x00, 0x00, 0x00, 0x36, 0x68, 0x68, 0x65, 0x61, 0x05, 0x85, 0x01, 0xc2,
            0x00, 0x00, 0x00, 0xe4, 0x00, 0x00, 0x00, 0x24, 0x68, 0x6d, 0x74, 0x78, 0x10, 0xf6,
            0xff, 0xda, 0x00, 0x00, 0x01, 0x88, 0x00, 0x00, 0x00, 0x20, 0x6c, 0x6f, 0x63, 0x61,
            0x02, 0x55, 0x01, 0xd6, 0x00, 0x00, 0x02, 0x0c, 0x00, 0x00, 0x00, 0x12, 0x6d, 0x61,
            0x78, 0x70, 0x00, 0x12, 0x00, 0x47, 0x00, 0x00, 0x01, 0x08, 0x00, 0x00, 0x00, 0x20,
            0x6e, 0x61, 0x6d, 0x65, 0xff, 0x72, 0x0d, 0x88, 0x00, 0x00, 0x04, 0x40, 0x00, 0x00,
            0x00, 0xb4, 0x70, 0x6f, 0x73, 0x74, 0x16, 0xf9, 0xc6, 0xb7, 0x00, 0x00, 0x04, 0xf4,
            0x00, 0x00, 0x00, 0x48, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x7e, 0x62,
            0x06, 0x11, 0x5f, 0x0f, 0x3c, 0xf5, 0x00, 0x03, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x00,
            0xdc, 0x27, 0x59, 0x19, 0x00, 0x00, 0x00, 0x00, 0xdc, 0xa5, 0xc8, 0x08, 0xff, 0x73,
            0xff, 0xb4, 0x02, 0xef, 0x03, 0x93, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x03, 0x20, 0xff, 0x38, 0x00, 0x00,
            0x02, 0xf4, 0xff, 0x73, 0xff, 0x8d, 0x02, 0xef, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x08, 0x00, 0x34, 0x00, 0x03, 0x00, 0x10, 0x00, 0x04, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x01, 0x00, 0x03, 0x02, 0x1f, 0x01, 0x90, 0x00, 0x05, 0x00, 0x04, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x3f, 0x3f, 0x3f, 0x3f, 0x00, 0x00, 0x00, 0x20, 0x03, 0x01,
            0x03, 0x20, 0xff, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x01, 0xf4, 0x02, 0xbc, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00,
            0x02, 0xf4, 0x00, 0x05, 0x02, 0xf4, 0x00, 0x05, 0x02, 0x98, 0x00, 0x1e, 0x02, 0xf4,
            0x00, 0x05, 0x00, 0xc8, 0x00, 0x00, 0x02, 0x58, 0x00, 0x1d, 0x02, 0x58, 0x00, 0x1d,
            0x00, 0x0a, 0xff, 0x73, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
            0x00, 0x14, 0x00, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x14, 0x00, 0x04, 0x00, 0x50,
            0x00, 0x00, 0x00, 0x10, 0x00, 0x10, 0x00, 0x03, 0x00, 0x00, 0x00, 0x20, 0x00, 0x24,
            0x00, 0x41, 0x00, 0x4f, 0x00, 0x56, 0x00, 0xc1, 0x03, 0x01, 0xff, 0xff, 0x00, 0x00,
            0x00, 0x20, 0x00, 0x24, 0x00, 0x41, 0x00, 0x4f, 0x00, 0x56, 0x00, 0xc1, 0x03, 0x01,
            0xff, 0xff, 0xff, 0xe4, 0xff, 0xe1, 0xff, 0xbf, 0xff, 0xb3, 0xff, 0xad, 0xff, 0x40,
            0xfd, 0x06, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1e, 0x00, 0x2a, 0x00, 0x51,
            0x00, 0x66, 0x00, 0x66, 0x00, 0xb6, 0x01, 0x01, 0x01, 0x0f, 0x00, 0x00, 0x00, 0x03,
            0x00, 0x05, 0x00, 0x00, 0x02, 0xef, 0x02, 0xbc, 0x00, 0x03, 0x00, 0x07, 0x00, 0x0b,
            0x00, 0x00, 0x01, 0x01, 0x33, 0x01, 0x23, 0x01, 0x33, 0x01, 0x13, 0x35, 0x21, 0x15,
            0x01, 0x43, 0x01, 0x3e, 0x6e, 0xfe, 0xc2, 0x6e, 0xfe, 0xc2, 0x6e, 0x01, 0x3e, 0x86,
            0xfe, 0x61, 0x02, 0xbc, 0xfd, 0x44, 0x02, 0xbc, 0xfd, 0x44, 0x02, 0xbc, 0xfe, 0x10,
            0x50, 0x50, 0xff, 0xff, 0x00, 0x05, 0x00, 0x00, 0x02, 0xef, 0x03, 0x93, 0x00, 0x26,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x07, 0x01, 0x92, 0x00, 0x82, 0x00, 0x02,
            0x00, 0x1e, 0xff, 0xf6, 0x02, 0x7a, 0x02, 0xc6, 0x00, 0x0b, 0x00, 0x17, 0x00, 0x00,
            0x01, 0x14, 0x06, 0x23, 0x22, 0x26, 0x35, 0x34, 0x36, 0x33, 0x32, 0x16, 0x05, 0x14,
            0x16, 0x33, 0x32, 0x36, 0x35, 0x34, 0x26, 0x23, 0x22, 0x06, 0x02, 0x7a, 0x96, 0x98,
            0x97, 0x97, 0x97, 0x97, 0x98, 0x96, 0xfd, 0xfe, 0x6a, 0x6a, 0x6a, 0x6a, 0x6a, 0x6a,
            0x6a, 0x6a, 0x01, 0x5e, 0xb5, 0xb3, 0xb3, 0xb5, 0xb5, 0xb3, 0xb3, 0xb5, 0x8a, 0x8a,
            0x8a, 0x8a, 0x8a, 0x8a, 0x8a, 0x00, 0x00, 0x02, 0x00, 0x05, 0x00, 0x00, 0x02, 0xef,
            0x02, 0xbc, 0x00, 0x03, 0x00, 0x07, 0x00, 0x00, 0x21, 0x23, 0x01, 0x33, 0x01, 0x23,
            0x01, 0x33, 0x01, 0xb1, 0x6e, 0x01, 0x3e, 0x6e, 0xfe, 0xc2, 0x6e, 0xfe, 0xc2, 0x6e,
            0x02, 0xbc, 0xfd, 0x44, 0x02, 0xbc, 0x00, 0x03, 0x00, 0x1d, 0xff, 0xbc, 0x02, 0x44,
            0x02, 0xf7, 0x00, 0x23, 0x00, 0x2b, 0x00, 0x33, 0x00, 0x00, 0x01, 0x35, 0x33, 0x15,
            0x16, 0x16, 0x17, 0x07, 0x26, 0x26, 0x27, 0x15, 0x16, 0x16, 0x15, 0x14, 0x06, 0x07,
            0x15, 0x23, 0x35, 0x26, 0x26, 0x27, 0x37, 0x16, 0x16, 0x17, 0x35, 0x27, 0x26, 0x26,
            0x35, 0x34, 0x36, 0x36, 0x17, 0x06, 0x06, 0x15, 0x14, 0x16, 0x16, 0x17, 0x17, 0x15,
            0x36, 0x36, 0x35, 0x34, 0x26, 0x26, 0x01, 0x08, 0x5a, 0x3d, 0x68, 0x2b, 0x35, 0x24,
            0x4b, 0x2c, 0x71, 0x71, 0x79, 0x69, 0x5a, 0x48, 0x78, 0x2b, 0x34, 0x2a, 0x54, 0x39,
            0x0f, 0x65, 0x69, 0x38, 0x64, 0x41, 0x3d, 0x42, 0x17, 0x36, 0x2f, 0x5d, 0x45, 0x3f,
            0x18, 0x39, 0x02, 0xcb, 0x2c, 0x2c, 0x06, 0x2f, 0x2a, 0x48, 0x24, 0x25, 0x06, 0xfe,
            0x16, 0x58, 0x4f, 0x52, 0x6e, 0x0a, 0x32, 0x32, 0x06, 0x2e, 0x28, 0x48, 0x24, 0x25,
            0x04, 0xe8, 0x03, 0x13, 0x61, 0x52, 0x39, 0x5b, 0x39, 0x50, 0x09, 0x41, 0x33, 0x20,
            0x2a, 0x1a, 0x0a, 0x6b, 0xd8, 0x07, 0x3b, 0x30, 0x1c, 0x27, 0x19, 0x00, 0x00, 0x01,
            0x00, 0x1d, 0xff, 0xb4, 0x02, 0x44, 0x02, 0xf7, 0x00, 0x32, 0x00, 0x00, 0x01, 0x35,
            0x33, 0x15, 0x16, 0x16, 0x17, 0x07, 0x26, 0x26, 0x23, 0x22, 0x06, 0x15, 0x14, 0x16,
            0x16, 0x17, 0x17, 0x16, 0x16, 0x15, 0x14, 0x06, 0x06, 0x07, 0x15, 0x23, 0x35, 0x26,
            0x26, 0x27, 0x37, 0x1e, 0x02, 0x33, 0x32, 0x36, 0x35, 0x34, 0x26, 0x26, 0x27, 0x27,
            0x26, 0x26, 0x35, 0x34, 0x36, 0x36, 0x01, 0x08, 0x5a, 0x3d, 0x68, 0x2b, 0x35, 0x2d,
            0x5e, 0x3e, 0x52, 0x59, 0x17, 0x36, 0x2f, 0x63, 0x6d, 0x6f, 0x37, 0x65, 0x46, 0x5a,
            0x48, 0x78, 0x2b, 0x34, 0x22, 0x41, 0x4f, 0x33, 0x5d, 0x53, 0x19, 0x3c, 0x35, 0x63,
            0x65, 0x69, 0x38, 0x64, 0x02, 0xcb, 0x2c, 0x2c, 0x06, 0x2f, 0x2a, 0x48, 0x2c, 0x26,
            0x44, 0x3c, 0x20, 0x2a, 0x1a, 0x0a, 0x14, 0x16, 0x57, 0x4f, 0x36, 0x57, 0x36, 0x07,
            0x3a, 0x3a, 0x06, 0x2e, 0x28, 0x48, 0x1c, 0x23, 0x10, 0x3d, 0x37, 0x1d, 0x27, 0x1a,
            0x09, 0x12, 0x13, 0x61, 0x52, 0x39, 0x5b, 0x39, 0x00, 0x01, 0xff, 0x73, 0x02, 0x76,
            0x00, 0x7d, 0x03, 0x11, 0x00, 0x03, 0x00, 0x00, 0x13, 0x07, 0x07, 0x37, 0x7d, 0xf3,
            0x17, 0xf3, 0x03, 0x11, 0x45, 0x56, 0x45, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x66, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x0f, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x06, 0x00, 0x0f, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x01, 0x00, 0x05, 0x00, 0x15, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
            0x00, 0x01, 0x00, 0x1e, 0x00, 0x1a, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09, 0x00, 0x10,
            0x00, 0x1e, 0x00, 0x1a, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09, 0x01, 0x00, 0x00, 0x0c,
            0x00, 0x38, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09, 0x01, 0x01, 0x00, 0x0a, 0x00, 0x44,
            0x53, 0x69, 0x6d, 0x70, 0x6c, 0x65, 0x20, 0x54, 0x77, 0x6f, 0x20, 0x41, 0x78, 0x69,
            0x73, 0x57, 0x65, 0x69, 0x67, 0x68, 0x74, 0x53, 0x6c, 0x61, 0x6e, 0x74, 0x00, 0x53,
            0x00, 0x69, 0x00, 0x6d, 0x00, 0x70, 0x00, 0x6c, 0x00, 0x65, 0x00, 0x20, 0x00, 0x54,
            0x00, 0x77, 0x00, 0x6f, 0x00, 0x20, 0x00, 0x41, 0x00, 0x78, 0x00, 0x69, 0x00, 0x73,
            0x00, 0x57, 0x00, 0x65, 0x00, 0x69, 0x00, 0x67, 0x00, 0x68, 0x00, 0x74, 0x00, 0x53,
            0x00, 0x6c, 0x00, 0x61, 0x00, 0x6e, 0x00, 0x74, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x24, 0x00, 0xc9, 0x00, 0x32, 0x00, 0x39, 0x00, 0x03, 0x00, 0x07, 0x01, 0x02,
            0x01, 0x03, 0x0b, 0x64, 0x6f, 0x6c, 0x6c, 0x61, 0x72, 0x2e, 0x62, 0x6f, 0x6c, 0x64,
            0x09, 0x61, 0x63, 0x75, 0x74, 0x65, 0x63, 0x6f, 0x6d, 0x62,
        ];
        let mut deserialized: font::Font = otspec::de::from_bytes(&binary_font).unwrap();
        deserialized.fully_deserialize();
        let glyf = deserialized
            .get_table(b"glyf")
            .unwrap()
            .unwrap()
            .glyf_unchecked();
        /*
        <TTGlyph name="A" xMin="5" yMin="0" xMax="751" yMax="700">
          <contour>
            <pt x="323" y="700" on="1"/>
            <pt x="641" y="0" on="1"/>
            <pt x="751" y="0" on="1"/>
            <pt x="433" y="700" on="1"/>
          </contour>
          <contour>
            <pt x="323" y="700" on="1"/>
            <pt x="5" y="0" on="1"/>
            <pt x="115" y="0" on="1"/>
            <pt x="433" y="700" on="1"/>
          </contour>
          <contour>
            <pt x="567" y="204" on="1"/>
            <pt x="567" y="284" on="1"/>
            <pt x="152" y="284" on="1"/>
            <pt x="152" y="204" on="1"/>
          </contour>
          <instructions/>
        </TTGlyph>
        */
        let A = &glyf.glyphs[0];
        #[rustfmt::skip]
        assert_eq!(A, &glyf::Glyph {
            xMin:5, yMin:0, xMax: 751, yMax:700,
            contours: vec![
                vec![
                    Point { x:323, y:700, on_curve: true },
                    Point { x:641, y:0, on_curve: true },
                    Point { x:751, y:0, on_curve: true },
                    Point { x:433, y:700, on_curve: true },
                ],
                vec![
                    Point { x:323, y:700, on_curve: true },
                    Point { x:5, y:0, on_curve: true },
                    Point { x:115, y:0, on_curve: true },
                    Point { x:433, y:700, on_curve: true },
                ],
                vec![
                    Point { x:567, y:204, on_curve: true },
                    Point { x:567, y:284, on_curve: true },
                    Point { x:152, y:284, on_curve: true },
                    Point { x:152, y:204, on_curve: true },
                ],
            ],
            components: vec![],
            instructions: vec![],
            overlap: false // There is, though.
        });

        /*
        <TTGlyph name="Aacute" xMin="5" yMin="0" xMax="751" yMax="915">
          <component glyphName="A" x="0" y="0" flags="0x4"/>
          <component glyphName="acutecomb" x="402" y="130" flags="0x4"/>
        </TTGlyph>
        */
        let aacute = &glyf.glyphs[1];
        assert_eq!(
            aacute.components[0],
            glyf::Component {
                glyphIndex: 0,
                transformation: kurbo::Affine::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
                matchPoints: None,
                flags: ComponentFlags::ROUND_XY_TO_GRID
                    | ComponentFlags::ARGS_ARE_XY_VALUES
                    | ComponentFlags::MORE_COMPONENTS /* ttx hides these */
            }
        );

        #[rustfmt::skip]
        assert_eq!(
            aacute.components[1],
            glyf::Component {
                glyphIndex: 7,
                transformation: kurbo::Affine::new([1.0, 0.0, 0.0, 1.0, 402.0, 130.0]),
                matchPoints: None,
                flags: glyf::ComponentFlags::ROUND_XY_TO_GRID
                    | ComponentFlags::ARGS_ARE_XY_VALUES
                    | ComponentFlags::ARG_1_AND_2_ARE_WORDS
            }
        );

        let component1_bytes = otspec::ser::to_bytes(&aacute).unwrap();
        let rede: glyf::Glyph = otspec::de::from_bytes(&component1_bytes).unwrap();
        assert_eq!(
            component1_bytes,
            vec![
                0xff, 0xff, 0x00, 0x05, 0x00, 0x00, 0x02, 0xef, 0x03, 0x93, 0x00, 0x26, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x07, 0x00, 0x07, 0x01, 0x92, 0x00, 0x82
            ]
        );

        assert!(glyf.glyphs[4].is_empty());
        let dollarbold = &glyf.glyphs[6];
        assert_eq!(dollarbold.xMin, 29);
        assert_eq!(dollarbold.yMin, -76);
        assert_eq!(dollarbold.xMax, 580);
        assert_eq!(dollarbold.yMax, 759);
        let firstpoint = dollarbold.contours[0][0];
        assert_eq!(
            firstpoint,
            Point {
                x: 264,
                y: 715,
                on_curve: true
            }
        );
    }

    #[test]
    fn test_insert_implicit_oncurves() {
        #[rustfmt::skip]
        let mut glyph = glyf::Glyph {
            xMin: 30, xMax: 634, yMin: -10, yMax: 710,
            components: vec![],
            instructions: vec![],
            overlap: false,
            contours: vec![
                vec![
                    Point {x: 634, y: 650, on_curve: true, },
                    Point {x: 634, y: 160, on_curve: false, },
                    Point {x: 484, y: -10, on_curve: false, },
                    Point {x: 332, y: -10, on_curve: true, },
                    Point {x: 181, y: -10, on_curve: false, },
                    Point {x: 30,  y: 169, on_curve: false, },
                    Point {x: 30,  y: 350, on_curve: true, },
                    Point {x: 30,  y: 531, on_curve: false, },
                    Point {x: 181, y: 710, on_curve: false, },
                    Point {x: 332, y: 710, on_curve: true, },
                ]
            ]
        };
        glyph.insert_explicit_oncurves();
        #[rustfmt::skip]
        assert_eq!(
            glyph.contours[0],
            vec![
                Point { x: 634, y: 650, on_curve: true },
                Point { x: 634, y: 160, on_curve: false },
                Point { x: 559, y: 75, on_curve: true },
                Point { x: 484, y: -10, on_curve: false },
                Point { x: 332, y: -10, on_curve: true },
                Point { x: 181, y: -10, on_curve: false },
                Point { x: 105, y: 79, on_curve: true },
                Point { x: 30, y: 169, on_curve: false },
                Point { x: 30, y: 350, on_curve: true },
                Point { x: 30, y: 531, on_curve: false },
                Point { x: 105, y: 620, on_curve: true },
                Point { x: 181, y: 710, on_curve: false },
                Point { x: 332, y: 710, on_curve: true }]
        );
    }
}
