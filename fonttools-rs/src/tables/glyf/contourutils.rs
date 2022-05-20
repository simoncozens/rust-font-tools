use super::Point;
use kurbo::{PathEl, PathSeg};

/// Adds explicit oncurve points to a contour
pub fn insert_explicit_oncurves(contour: &mut Vec<Point>) {
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

/// Removes implied oncurve points from a contour
pub fn remove_implied_oncurves(contour: &mut Vec<Point>) {
    let mut i: usize = 0;
    while i < contour.len() {
        let next_ix = (i + 1) % contour.len();
        let prev_ix = if i == 0 { contour.len() - 1 } else { i - 1 };
        let this = contour[i];
        let next = contour[next_ix];
        let prev = contour[prev_ix];
        if !this.on_curve
            || prev.on_curve
            || next.on_curve
            || this.x != (prev.x + next.x) / 2
            || this.y != (prev.y + next.y) / 2
        {
            i += 1;
            continue;
        }
        contour.remove(i);
    }
}

/// Construct a vector of points from a `kurbo::BezPath` object
///
/// Cubic paths will be converted to quadratic paths using the given error tolerance.
pub fn kurbo_contour_to_glyf_contour(kurbo_path: &kurbo::BezPath, error: f32) -> Vec<Point> {
    let mut points: Vec<Point> = vec![];
    if let PathEl::MoveTo(pt) = kurbo_path.elements()[0] {
        points.push(Point {
            x: pt.x as i16,
            y: pt.y as i16,
            on_curve: true,
        });
    }
    for seg in kurbo_path.segments() {
        match seg {
            PathSeg::Line(l) => points.push(Point {
                x: l.p1.x as i16,
                y: l.p1.y as i16,
                on_curve: true,
            }),
            PathSeg::Quad(q) => points.extend(vec![
                Point {
                    x: q.p1.x as i16,
                    y: q.p1.y as i16,
                    on_curve: false,
                },
                Point {
                    x: q.p2.x as i16,
                    y: q.p2.y as i16,
                    on_curve: true,
                },
            ]),
            PathSeg::Cubic(c) => {
                for (_, _, q) in c.to_quads(error.into()) {
                    points.extend(vec![
                        Point {
                            x: q.p1.x as i16,
                            y: q.p1.y as i16,
                            on_curve: false,
                        },
                        Point {
                            x: q.p2.x as i16,
                            y: q.p2.y as i16,
                            on_curve: true,
                        },
                    ]);
                }
            }
        }
    }

    // Reverse it
    points.reverse();
    points
}

/// Returns a kurbo BezPath object representing this glyf contour
pub fn glyf_contour_to_kurbo_contour(contour: &[Point]) -> kurbo::BezPath {
    let mut path = kurbo::BezPath::new();
    let mut contour = contour.to_vec();
    insert_explicit_oncurves(&mut contour);
    path.move_to((contour[0].x as f64, contour[0].y as f64));
    let mut segment: Vec<&Point> = vec![];
    for pt in &contour[1..] {
        segment.push(pt);
        if pt.on_curve {
            match segment.len() {
                1 => {
                    path.line_to((segment[0].x as f64, segment[0].y as f64));
                }
                2 => {
                    path.quad_to(
                        (segment[0].x as f64, segment[0].y as f64),
                        (segment[1].x as f64, segment[1].y as f64),
                    );
                }
                _ => {}
            };
            segment = vec![];
        }
    }
    if !segment.is_empty() {
        path.quad_to(
            (segment[0].x as f64, segment[0].y as f64),
            (contour[0].x as f64, contour[0].y as f64),
        );
    } else if contour[0].on_curve && contour.last().unwrap().on_curve {
        path.line_to((contour[0].x as f64, contour[0].y as f64))
    }
    path.close_path();
    path
}
