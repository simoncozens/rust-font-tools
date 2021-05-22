use crate::glyf::component::{Component, ComponentFlags};
use crate::glyf::point::Point;
use bitflags::bitflags;
use itertools::izip;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_field_counted};
use otspec_macros::tables;
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

tables!(
    GlyphCore {
        int16	xMin
        int16	yMin
        int16	xMax
        int16	yMax
    }
);

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

#[derive(Debug, PartialEq, Clone)]
#[allow(non_snake_case)]
/// A higher-level representation of a TrueType outline glyph.
pub struct Glyph {
    /// The minimum X coordinate of points (including transformed component points) within this glyph
    pub xMin: int16,
    /// The maximum X coordinate of points (including transformed component points) within this glyph
    pub xMax: int16,
    /// The minimum Y coordinate of points (including transformed component points) within this glyph
    pub yMin: int16,
    /// The maximum Y coordinate of points (including transformed component points) within this glyph
    pub yMax: int16,
    /// A list of contours, each contour represented as a list of `Point` objects.
    pub contours: Vec<Vec<Point>>,
    /// Truetype instructions (binary)
    pub instructions: Vec<u8>,
    /// A vector of components
    pub components: Vec<Component>,
    /// A flag used in the low-level glyph representation to determine if this
    /// glyph has overlaps. This *appears* to be unused in OpenType implementations.
    pub overlap: bool,
}

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
            for flag in &flags {
                if flag.contains(SimpleGlyphFlags::X_SHORT_VECTOR) {
                    let coord = read_field!(seq, u8, "an X coordinate") as i16;
                    if flag.contains(SimpleGlyphFlags::X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR) {
                        last_x += coord;
                    } else {
                        last_x -= coord;
                    }
                    x_coords.push(last_x);
                    // println!("Read short X coordinate {:?}", coord);
                    // println!("X is now {:?}", last_x);
                } else if flag.contains(SimpleGlyphFlags::X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR) {
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
            for flag in &flags {
                if flag.contains(SimpleGlyphFlags::Y_SHORT_VECTOR) {
                    let coord = read_field!(seq, u8, "a Y coordinate") as i16;
                    if flag.contains(SimpleGlyphFlags::Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR) {
                        last_y += coord;
                    } else {
                        last_y -= coord;
                    }
                    // println!("Read short Y coordinate {:?}", coord);
                    // println!("Y is now {:?}", last_y);
                    y_coords.push(last_y);
                } else if flag.contains(SimpleGlyphFlags::Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR) {
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
                if flag.contains(SimpleGlyphFlags::OVERLAP_SIMPLE) {
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
    fn _compile_deltas_greedy(&self) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
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
            let ix = comp.glyph_index;
            match glyphs.get(ix as usize) {
                None => {
                    log::error!("Component not found for ID={:?}", ix);
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
                        log::warn!("Found nested components while decomposing");
                    }
                }
            }
        }
        if !new_contours.is_empty() {
            newglyph.contours = new_contours;
        }
        newglyph
    }

    pub fn gvar_coords_and_ends(&self) -> (Vec<(int16, int16)>, Vec<usize>) {
        let mut ends: Vec<usize> = self
            .contours
            .iter()
            .map(|c| c.len())
            .scan(0, |acc, x| {
                *acc += x;
                Some(*acc - 1)
            })
            .collect();

        let mut coords: Vec<(i16, i16)> = self
            .contours
            .iter()
            .flatten()
            .map(|pt| (pt.x, pt.y))
            .collect();
        for comp in &self.components {
            let [_, _, _, _, translate_x, translate_y] = comp.transformation.as_coeffs();
            coords.push((translate_x as i16, translate_y as i16));
            ends.push(ends.iter().max().unwrap_or(&0) + 1);
        }

        // Phantom points
        let left_side_x = 0; // XXX WRONG
        let right_side_x = 0;
        let top_side_y = 0;
        let bottom_side_y = 0;
        coords.push((left_side_x, 0));
        ends.push(ends.iter().max().unwrap_or(&0) + 1);
        coords.push((right_side_x, 0));
        ends.push(ends.iter().max().unwrap() + 1);
        coords.push((0, top_side_y));
        ends.push(ends.iter().max().unwrap() + 1);
        coords.push((0, bottom_side_y));
        ends.push(ends.iter().max().unwrap() + 1);
        (coords, ends)
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
                seq.serialize_element::<uint16>(&comp.glyph_index)?;
                let [x_scale, scale01, scale10, scale_y, translate_x, translate_y] =
                    comp.transformation.as_coeffs();
                if flags.contains(ComponentFlags::ARGS_ARE_XY_VALUES) {
                    if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                        seq.serialize_element::<i16>(&(translate_x.round() as i16))?;
                        seq.serialize_element::<i16>(&(translate_y as i16))?;
                    } else {
                        seq.serialize_element::<i8>(&(translate_x.round() as i8))?;
                        seq.serialize_element::<i8>(&(translate_y as i8))?;
                    }
                } else {
                    let (x, y) = comp.match_points.unwrap();
                    if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                        seq.serialize_element::<i16>(&(x as i16))?;
                        seq.serialize_element::<i16>(&(y as i16))?;
                    } else {
                        seq.serialize_element::<i8>(&(x as i8))?;
                        seq.serialize_element::<i8>(&(y as i8))?;
                    }
                }
                if flags.contains(ComponentFlags::WE_HAVE_A_TWO_BY_TWO) {
                    F2DOT14::serialize_element(&(x_scale as f32), &mut seq)?;
                    F2DOT14::serialize_element(&(scale01 as f32), &mut seq)?;
                    F2DOT14::serialize_element(&(scale10 as f32), &mut seq)?;
                    F2DOT14::serialize_element(&(scale_y as f32), &mut seq)?;
                } else if flags.contains(ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE) {
                    F2DOT14::serialize_element(&(x_scale as f32), &mut seq)?;
                    F2DOT14::serialize_element(&(scale_y as f32), &mut seq)?;
                } else if flags.contains(ComponentFlags::WE_HAVE_A_SCALE) {
                    F2DOT14::serialize_element(&(x_scale as f32), &mut seq)?;
                }
                if flags.contains(ComponentFlags::WE_HAVE_INSTRUCTIONS) {
                    seq.serialize_element::<uint16>(&(self.instructions.len() as u16))?;
                    seq.serialize_element::<Vec<u8>>(&self.instructions)?;
                }
            }
        } else {
            let end_pts_of_contour = self.end_points();
            seq.serialize_element::<Vec<uint16>>(&end_pts_of_contour)?;
            if !self.instructions.is_empty() {
                seq.serialize_element::<uint16>(&(self.instructions.len() as u16))?;
                seq.serialize_element::<Vec<u8>>(&self.instructions)?;
            } else {
                seq.serialize_element::<uint16>(&0)?;
            }
            let (compressed_flags, compressed_xs, compressed_ys) = self._compile_deltas_greedy();
            seq.serialize_element::<Vec<u8>>(&compressed_flags)?;
            seq.serialize_element::<Vec<u8>>(&compressed_xs)?;
            seq.serialize_element::<Vec<u8>>(&compressed_ys)?;
        }
        seq.end()
    }
}
