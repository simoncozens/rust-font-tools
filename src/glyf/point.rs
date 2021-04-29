use kurbo::Affine;
use otspec::types::*;

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
