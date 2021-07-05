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
