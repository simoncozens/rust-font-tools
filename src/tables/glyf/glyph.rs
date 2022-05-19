use super::component::{Component, ComponentFlags};
use super::contourutils;
use super::point::Point;
use bitflags::bitflags;
use itertools::izip;
use otspec::types::*;
use otspec::Serializer;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::{tables, Deserialize, Serialize};
use std::cmp::max;
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

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct CompositeMaxpValues {
    pub num_points: u16,
    pub num_contours: u16,
    pub max_depth: u16,
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

impl Deserialize for Glyph {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        // println!("Reading a glyph");
        let num_contours: i16 = c.de()?;
        // println!("Num contours: {:?}", num_contours);
        let core: GlyphCore = c.de()?;
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
                let comp: Component = c.de()?;
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
                let instructions_count: i16 = c.de()?;
                if instructions_count > 0 {
                    instructions = c.de_counted(instructions_count as usize)?;
                }
            }
        } else {
            // println!("Reading {:?} contours", num_contours);
            let mut end_pts_of_contour: Vec<usize> = (0..num_contours as usize)
                .map(|_| {
                    let x: Result<uint16, DeserializationError> = c.de();
                    1 + (x.unwrap() as usize)
                })
                .collect();
            let instructions_count: i16 = c.de()?;
            if instructions_count > 0 {
                instructions = c.de_counted(instructions_count as usize)?;
            }
            // println!("Instructions: {:?}", instructions);
            let num_points = *(end_pts_of_contour
                .last()
                .ok_or_else(|| DeserializationError("No points?".to_string()))?)
                as usize;
            let mut i = 0;
            // println!("Number of points: {:?}", num_points);
            let mut flags: Vec<SimpleGlyphFlags> = Vec::with_capacity(num_points);
            while i < num_points {
                let flag: SimpleGlyphFlags = c.de()?;
                flags.push(flag);
                if flag.contains(SimpleGlyphFlags::REPEAT_FLAG) {
                    let mut repeat_count: u8 = c.de()?;
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
                    let coord: u8 = c.de()?;
                    if flag.contains(SimpleGlyphFlags::X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR) {
                        last_x += coord as i16;
                    } else {
                        last_x -= coord as i16;
                    }
                    // println!("Read short X coordinate {:?}", coord);
                    // println!("X is now {:?}", last_x);
                } else if flag.contains(SimpleGlyphFlags::X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR) {
                    // println!("Elided X coordinate");
                    // println!("X is still {:?}", last_x);
                } else {
                    let coord: i16 = c.de()?;
                    // println!("Read long X coordinate {:?}", coord);
                    last_x += coord;
                    // println!("X is now {:?}", last_x);
                }
                x_coords.push(last_x);
            }
            for flag in &flags {
                if flag.contains(SimpleGlyphFlags::Y_SHORT_VECTOR) {
                    let coord: u8 = c.de()?;
                    if flag.contains(SimpleGlyphFlags::Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR) {
                        last_y += coord as i16;
                    } else {
                        last_y -= coord as i16;
                    }
                    // println!("Read short Y coordinate {:?}", coord);
                    // println!("Y is now {:?}", last_y);
                } else if flag.contains(SimpleGlyphFlags::Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR) {
                    // println!("Elided Y coordinate");
                    // println!("Y is still {:?}", last_y);
                } else {
                    let coord: i16 = c.de()?;
                    last_y += coord;
                    // println!("Read long Y coordinate {:?}", coord);
                    // println!("Y is now {:?}", last_y);
                }
                y_coords.push(last_y);
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
}

impl Glyph {
    /// Returns true if this glyph has any components
    pub fn has_components(&self) -> bool {
        !self.components.is_empty()
    }

    /// Returns true if this glyph has neither components nor contours
    pub fn is_empty(&self) -> bool {
        self.components.is_empty() && self.contours.is_empty()
    }

    /// Returns a bounding box rectangle for this glyph as a `kurbo::Rect`.
    pub fn bounds_rect(&self) -> kurbo::Rect {
        kurbo::Rect::new(
            self.xMin.into(),
            self.yMin.into(),
            self.xMax.into(),
            self.yMax.into(),
        )
    }
    /// Sets the bounding box rectangle for this glyph from a `kurbo::Rect`.
    pub fn set_bounds_rect(&mut self, r: kurbo::Rect) {
        self.xMin = r.min_x() as i16;
        self.xMax = r.max_x() as i16;
        self.yMin = r.min_y() as i16;
        self.yMax = r.max_y() as i16;
    }

    /// Assuming that the contour list has been expanded into a flat list of
    /// points, returns an array of indices representing the final points of
    /// each contour.
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
    /// Inserts explicit on-curve points.
    ///
    /// As a space-saving optimization, TrueType outlines may omit on-curve
    /// points if they lay directly at the midpoint of the two surrounding
    /// off-curve points. This function reinserts the implict on-curve points
    /// to allow for simpler processing of the glyph contours.
    pub fn insert_explicit_oncurves(&mut self) {
        if self.contours.is_empty() {
            return;
        }
        for contour in self.contours.iter_mut() {
            contourutils::insert_explicit_oncurves(contour);
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

    /// Decomposes components in this glyph (but not recursively)
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

    /// Produces a tuple made up of a list of X/Y coordinates and a list
    /// of ends-of-contour indices, suitable for use when constructing a
    /// `gvar` table.
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
            ends.push(ends.iter().max().map(|x| x + 1).unwrap_or(0));
        }

        // Phantom points
        let left_side_x = 0; // XXX WRONG
        let right_side_x = 0;
        let top_side_y = 0;
        let bottom_side_y = 0;
        coords.push((left_side_x, 0));
        ends.push(ends.iter().max().map(|x| x + 1).unwrap_or(0));
        coords.push((right_side_x, 0));
        ends.push(ends.iter().max().unwrap() + 1);
        coords.push((0, top_side_y));
        ends.push(ends.iter().max().unwrap() + 1);
        coords.push((0, bottom_side_y));
        ends.push(ends.iter().max().unwrap() + 1);
        assert_eq!(
            *ends.last().unwrap(),
            coords.len() - 1,
            "Coords: {:?}\nEnds: {:?}\nGlyf: {:#?}",
            coords,
            ends,
            self
        );
        (coords, ends)
    }

    /// Number of points in this glyph (without counting components)
    pub fn num_points(&self) -> usize {
        self.contours.iter().map(|x| x.len()).sum()
    }

    /// Number of contours in this glyph (without counting components)
    pub fn num_contours(&self) -> usize {
        self.contours.len()
    }

    /// Get information about composite depth and contour points
    /// suitable for feeding to a maxp table
    pub fn composite_maxp_values(&self, glyphs: &[Glyph]) -> Option<CompositeMaxpValues> {
        self._composite_maxp_values(glyphs, 1)
    }
    fn _composite_maxp_values(&self, glyphs: &[Glyph], depth: u16) -> Option<CompositeMaxpValues> {
        if !self.has_components() {
            return None;
        }
        let mut info = CompositeMaxpValues {
            num_points: 0,
            num_contours: 0,
            max_depth: depth,
        };
        for base_glyph in self
            .components
            .iter()
            .map(|c| glyphs.get(c.glyph_index as usize))
            .flatten()
        {
            if !base_glyph.has_components() {
                info.num_points += base_glyph.num_points() as u16;
                info.num_contours += base_glyph.num_contours() as u16;
            } else if let Some(other_info) = base_glyph._composite_maxp_values(glyphs, depth + 1) {
                info.num_points += other_info.num_points;
                info.num_contours += other_info.num_contours;
                info.max_depth = max(info.max_depth, other_info.max_depth);
            }
        }
        Some(info)
    }
}

impl Serialize for Glyph {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        if self.is_empty() {
            return Ok(());
        }
        data.put(if self.has_components() {
            -1
        } else {
            self.contours.len() as i16
        })?;
        // recalc bounds?
        data.put(GlyphCore {
            xMin: self.xMin,
            xMax: self.xMax,
            yMin: self.yMin,
            yMax: self.yMax,
        })?;
        if self.has_components() {
            for (i, comp) in self.components.iter().enumerate() {
                let flags = comp
                    .recompute_flags(i < self.components.len() - 1, !self.instructions.is_empty());
                data.put(flags.bits())?;
                data.put(comp.glyph_index)?;
                let [x_scale, scale01, scale10, scale_y, translate_x, translate_y] =
                    comp.transformation.as_coeffs();
                if flags.contains(ComponentFlags::ARGS_ARE_XY_VALUES) {
                    if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                        data.put(translate_x.round() as i16)?;
                        data.put(translate_y as i16)?;
                    } else {
                        data.put(translate_x.round() as i8)?;
                        data.put(translate_y as i8)?;
                    }
                } else {
                    let (x, y) = comp.match_points.unwrap();
                    if flags.contains(ComponentFlags::ARG_1_AND_2_ARE_WORDS) {
                        data.put(x as i16)?;
                        data.put(y as i16)?;
                    } else {
                        data.put(x as i8)?;
                        data.put(y as i8)?;
                    }
                }
                if flags.contains(ComponentFlags::WE_HAVE_A_TWO_BY_TWO) {
                    data.put(F2DOT14(x_scale as f32))?;
                    data.put(F2DOT14(scale01 as f32))?;
                    data.put(F2DOT14(scale10 as f32))?;
                    data.put(F2DOT14(scale_y as f32))?;
                } else if flags.contains(ComponentFlags::WE_HAVE_AN_X_AND_Y_SCALE) {
                    data.put(F2DOT14(x_scale as f32))?;
                    data.put(F2DOT14(scale_y as f32))?;
                } else if flags.contains(ComponentFlags::WE_HAVE_A_SCALE) {
                    data.put(F2DOT14(x_scale as f32))?;
                }
                if flags.contains(ComponentFlags::WE_HAVE_INSTRUCTIONS) {
                    data.put(self.instructions.len() as u16)?;
                    data.put(self.instructions.clone())?;
                }
            }
        } else {
            let end_pts_of_contour = self.end_points();
            data.put(end_pts_of_contour)?;
            if !self.instructions.is_empty() {
                data.put(self.instructions.len() as u16)?;
                data.put(self.instructions.clone())?;
            } else {
                data.put(0_u16)?;
            }
            let (compressed_flags, compressed_xs, compressed_ys) = self._compile_deltas_greedy();
            data.put(compressed_flags)?;
            data.put(compressed_xs)?;
            data.put(compressed_ys)?;
        }
        Ok(())
    }
}
