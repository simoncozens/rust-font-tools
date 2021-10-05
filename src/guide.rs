use crate::common::Color;
use crate::common::Position;

#[derive(Debug)]
pub struct Guide {
    pub pos: Position,
    pub name: Option<String>,
    pub color: Option<Color>,
    // lib
}

impl Guide {
    pub fn new() -> Self {
        Guide {
            pos: Position::zero(),
            name: None,
            color: None,
        }
    }
}

impl From<&norad::Guideline> for Guide {
    fn from(g: &norad::Guideline) -> Self {
        let mut out = Guide::new();
        out.name = g.name.clone();
        out.color = g.color.as_ref().map(|x| x.into());
        match g.line {
            norad::Line::Angle { x, y, degrees } => {
                out.pos = Position {
                    x: x as i32,
                    y: y as i32,
                    angle: degrees as f32,
                }
            }
            norad::Line::Horizontal(y) => {
                out.pos = Position {
                    x: 0,
                    y: y as i32,
                    angle: 0.0,
                }
            }
            norad::Line::Vertical(x) => {
                out.pos = Position {
                    y: 0,
                    x: x as i32,
                    angle: 90.0,
                }
            }
        };
        out
    }
}
