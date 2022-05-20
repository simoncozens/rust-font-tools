use kurbo::Affine;
use otspec::types::*;

/// Represents a point inside a glyf::Contour
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Point {
    /// x-coordinate
    pub x: int16,
    /// y-coordinate
    pub y: int16,
    /// Is this an on-curve point?
    pub on_curve: bool,
}

impl Point {
    /// Transforms the point using the given affine transformation
    ///
    /// When supplied with a kurbo::Affine object, returns a new
    /// point with the transformation applied.
    pub fn transform(&self, t: Affine) -> Point {
        let kurbo_point = t * kurbo::Point::new(self.x as f64, self.y as f64);
        Point {
            x: kurbo_point.x as i16,
            y: kurbo_point.y as i16,
            on_curve: self.on_curve,
        }
    }
}
