use crate::common::{Node, NodeType};
use crate::BabelfontError;

#[derive(Debug, Clone)]
pub struct Component {
    pub reference: String,
    pub transform: kurbo::Affine,
}

#[derive(Debug, Clone, Default)]
pub struct Path {
    pub nodes: Vec<Node>,
    pub closed: bool,
}

impl Path {
    /// Converts the `Path` to a [`kurbo::BezPath`].
    // Stolen completely from norad
    pub fn to_kurbo(&self) -> Result<kurbo::BezPath, BabelfontError> {
        let mut path = kurbo::BezPath::new();
        let mut offs = std::collections::VecDeque::new();
        let rotate = if self.closed {
            self.nodes
                .iter()
                .rev()
                .position(|pt| pt.nodetype != NodeType::OffCurve)
                .map(|idx| self.nodes.len() - 1 - idx)
                .unwrap_or(0)
        } else {
            0
        };
        let mut nodes = self
            .nodes
            .iter()
            .cycle()
            .skip(rotate)
            .take(self.nodes.len());
        // We do this because all kurbo paths (even closed ones)
        // must start with a move_to (otherwise get_segs doesn't work)
        if let Some(start) = nodes.next() {
            path.move_to(start.to_kurbo());
        }
        for pt in nodes {
            let kurbo_point = pt.to_kurbo();
            match pt.nodetype {
                NodeType::Move => path.move_to(kurbo_point),
                NodeType::Line => path.line_to(kurbo_point),
                NodeType::OffCurve => offs.push_back(kurbo_point),
                NodeType::Curve => {
                    match offs.make_contiguous() {
                        [] => return Err(BabelfontError::BadPath),
                        [p1] => path.quad_to(*p1, kurbo_point),
                        [p1, p2] => path.curve_to(*p1, *p2, kurbo_point),
                        _ => return Err(BabelfontError::BadPath),
                    };
                    offs.clear();
                }
                NodeType::QCurve => {
                    while let Some(pt) = offs.pop_front() {
                        if let Some(next) = offs.front() {
                            let implied_point = pt.midpoint(*next);
                            path.quad_to(pt, implied_point);
                        } else {
                            path.quad_to(pt, kurbo_point);
                        }
                    }
                    offs.clear();
                }
            }
        }
        if self.closed {
            path.close_path()
        }
        Ok(path)
    }
}

#[derive(Debug, Clone)]
pub enum Shape {
    ComponentShape(Component),
    PathShape(Path),
}
