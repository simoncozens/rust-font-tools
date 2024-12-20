use serde_json::{Map, Value};

pub type FormatSpecific = Map<String, Value>;

#[derive(Debug, Copy, Clone, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

impl Position {
    pub fn zero() -> Position {
        Position {
            x: 0.0,
            y: 0.0,
            angle: 0.0,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct Color {
    r: i32,
    g: i32,
    b: i32,
    a: i32,
}

#[cfg(feature = "ufo")]
impl From<&norad::Color> for Color {
    fn from(c: &norad::Color) -> Self {
        let (red, green, blue, alpha) = c.channels();
        Color {
            r: (red * 255.0) as i32,
            g: (green * 255.0) as i32,
            b: (blue * 255.0) as i32,
            a: (alpha * 255.0) as i32,
        }
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

impl OTScalar {
    pub fn as_bitfield(&self) -> Option<Vec<u8>> {
        if let OTScalar::BitField(u) = self {
            Some(u.to_vec())
        } else {
            None
        }
    }
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
            OTScalar::Signed(u) => u,
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

#[derive(Debug, Clone)]
pub struct OTValue {
    pub table: String,
    pub field: String,
    pub value: OTScalar,
}

#[derive(Debug, Clone)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    TopToBottom,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NodeType {
    Move,
    Line,
    OffCurve,
    Curve,
    QCurve,
}

#[cfg(feature = "ufo")]
impl From<&norad::PointType> for NodeType {
    fn from(p: &norad::PointType) -> Self {
        match p {
            norad::PointType::Move => NodeType::Move,
            norad::PointType::Line => NodeType::Line,
            norad::PointType::OffCurve => NodeType::OffCurve,
            norad::PointType::QCurve => NodeType::QCurve,
            _ => NodeType::Curve,
        }
    }
}

#[derive(Debug, Clone)]
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

#[cfg(feature = "ufo")]
impl From<&norad::ContourPoint> for Node {
    fn from(p: &norad::ContourPoint) -> Self {
        Node {
            x: p.x as f32,
            y: p.y as f32,
            nodetype: (&p.typ).into(),
        }
    }
}
