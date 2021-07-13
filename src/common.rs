use std::collections::HashMap;

use chrono::Offset;

pub type Tag = [u8; 4];

#[derive(Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub angle: f32,
}

impl Position {
    pub fn zero() -> Position {
        Position {
            x: 0,
            y: 0,
            angle: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct Color {
    r: i32,
    g: i32,
    b: i32,
    a: i32,
}

impl From<&norad::Color> for Color {
    fn from(c: &norad::Color) -> Self {
        Color {
            r: (c.red * 255.0) as i32,
            g: (c.green * 255.0) as i32,
            b: (c.blue * 255.0) as i32,
            a: (c.alpha * 255.0) as i32,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Location(pub HashMap<String, f32>);
impl Location {
    pub fn new() -> Self {
        Location(HashMap::new())
    }
}

#[derive(Debug, Clone)]
pub enum OTScalar {
    StringType(String),
    Bool(bool),
    Unsigned(u32),
    Signed(i32),
    Float(f32),
    BitField(Vec<u8>),
}

impl From<OTScalar> for f32 {
    fn from(p: OTScalar) -> f32 {
        match p {
            OTScalar::Unsigned(u) => u as f32,
            OTScalar::Signed(u) => u as f32,
            OTScalar::Float(f) => f,
            _ => 0.0,
        }
    }
}

impl From<OTScalar> for i16 {
    fn from(p: OTScalar) -> i16 {
        match p {
            OTScalar::Unsigned(u) => u as i16,
            OTScalar::Signed(u) => u as i16,
            OTScalar::Float(f) => f as i16,
            _ => 0,
        }
    }
}

impl From<OTScalar> for u16 {
    fn from(p: OTScalar) -> u16 {
        match p {
            OTScalar::Unsigned(u) => u as u16,
            OTScalar::Signed(u) => u as u16,
            OTScalar::Float(f) => f as u16,
            _ => 0,
        }
    }
}
impl From<OTScalar> for i32 {
    fn from(p: OTScalar) -> i32 {
        match p {
            OTScalar::Unsigned(u) => u as i32,
            OTScalar::Signed(u) => u as i32,
            OTScalar::Float(f) => f as i32,
            _ => 0,
        }
    }
}

impl From<OTScalar> for bool {
    fn from(p: OTScalar) -> bool {
        match p {
            OTScalar::Bool(b) => b,
            _ => false,
        }
    }
}

impl From<OTScalar> for String {
    fn from(p: OTScalar) -> String {
        match p {
            OTScalar::StringType(s) => s,
            OTScalar::Unsigned(p) => format!("{}", p),
            OTScalar::Signed(p) => format!("{}", p),
            OTScalar::Bool(p) => format!("{}", p),
            OTScalar::Float(p) => format!("{}", p),
            OTScalar::BitField(p) => format!("{:?}", p),
        }
    }
}

#[derive(Debug)]
pub struct OTValue {
    pub table: String,
    pub field: String,
    pub value: OTScalar,
}

#[derive(Debug)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    TopToBottom,
}

#[derive(Debug, PartialEq)]
pub enum NodeType {
    Move,
    Line,
    OffCurve,
    Curve,
}

impl From<&norad::PointType> for NodeType {
    fn from(p: &norad::PointType) -> Self {
        match p {
            norad::PointType::Move => NodeType::Move,
            norad::PointType::Line => NodeType::Line,
            norad::PointType::OffCurve => NodeType::OffCurve,
            _ => NodeType::Curve,
        }
    }
}

#[derive(Debug)]
pub struct Node {
    pub x: f32,
    pub y: f32,
    pub nodetype: NodeType,
    // userData: XXX
}

impl Node {
    pub fn to_kurbo(&self) -> kurbo::Point {
        kurbo::Point::new(self.x as f64, self.y as f64)
    }
}

impl From<&norad::ContourPoint> for Node {
    fn from(p: &norad::ContourPoint) -> Self {
        Node {
            x: p.x,
            y: p.y,
            nodetype: (&p.typ).into(),
        }
    }
}
