use crate::utils::is_all_same;
use fonttools::glyf;
use fonttools::gvar::DeltaSet;
use fonttools::gvar::GlyphVariationData;
use fonttools::otvar::NormalizedLocation;
use kurbo::{PathEl, PathSeg};
use otspec::types::Tuple;
use std::collections::BTreeMap;

pub fn glifs_to_glyph(
    glif: &norad::Glyph,
    mapping: &BTreeMap<String, u16>,
    variations: Vec<(&NormalizedLocation, &std::sync::Arc<norad::Glyph>)>,
) -> (glyf::Glyph, Option<GlyphVariationData>) {
    let mut glyph = glyf::Glyph {
        xMin: 0,
        xMax: 0,
        yMin: 0,
        yMax: 0,
        contours: vec![],
        instructions: vec![],
        components: vec![],
        overlap: false,
    };
    if glif.components.is_empty() && glif.contours.is_empty() {
        return (glyph, None);
    }

    /* Do components */
    let mut components: Vec<glyf::Component> = vec![];
    for component in &glif.components {
        if let Some(glyf_component) = norad_component_to_glyf_component(component, mapping) {
            components.push(glyf_component);
        }
    }
    if !components.is_empty() {
        glyph.components = components;
    }

    /* Do outlines */

    let mut locations: Vec<&NormalizedLocation> = vec![];
    let mut other_glifs: Vec<&std::sync::Arc<norad::Glyph>> = vec![];
    let mut contours: Vec<Vec<glyf::Point>> = vec![];
    let mut widths: Vec<f32> = vec![];
    let mut other_contours: Vec<Vec<Vec<glyf::Point>>> = vec![];
    for (l, o) in &variations {
        locations.push(l);
        other_glifs.push(o);
        other_contours.push(vec![]);
        widths.push(o.width);
    }

    for (index, contour) in glif.contours.iter().enumerate() {
        for o in &other_glifs {
            if index > o.contours.len() {
                // Let's assume we've done some interpolatability checks before this point
                panic!("Incompatible contour in glyph {:?}", o);
            }
        }
        let mut all_contours: Vec<&norad::Contour> = vec![contour];
        all_contours.extend::<Vec<&norad::Contour>>(
            other_glifs.iter().map(|x| &x.contours[index]).collect(),
        );
        let all_glyf_contours = norad_contours_to_glyf_contour(all_contours);
        contours.push(all_glyf_contours[0].clone());
        for master_id in 0..variations.len() {
            other_contours[master_id].push(all_glyf_contours[1 + master_id].clone());
        }
    }
    if !contours.is_empty() {
        let deltas = compute_deltas(&contours, other_contours, glif.width, widths, locations);
        glyph.contours = contours;
        return (glyph, Some(deltas));
    }

    (glyph, None)
}

fn compute_deltas(
    base: &Vec<Vec<glyf::Point>>,
    others: Vec<Vec<Vec<glyf::Point>>>,
    base_width: f32,
    other_widths: Vec<f32>,
    locations: Vec<&NormalizedLocation>,
) -> GlyphVariationData {
    let mut deltasets: Vec<DeltaSet> = vec![];
    let (mut base_x_coords, mut base_y_coords): (Vec<i16>, Vec<i16>) =
        base.iter().flatten().map(|pt| (pt.x, pt.y)).unzip();
    // Sure, this is bogus, don't @ me.
    base_x_coords.extend(vec![0, base_width as i16, 0, 0]);
    base_y_coords.extend(vec![0, 0, 0, 0]);
    for (ix, master) in others.iter().enumerate() {
        let location = locations[ix];
        let width = other_widths[ix];
        let (mut master_x_coords, mut master_y_coords): (Vec<i16>, Vec<i16>) =
            master.iter().flatten().map(|pt| (pt.x, pt.y)).unzip();
        // Putting width in here should work! But it doesn't!
        master_x_coords.extend(vec![0, base_width as i16, 0, 0]);
        master_y_coords.extend(vec![0, 0, 0, 0]);
        let x_delta: Vec<i16> = base_x_coords
            .iter()
            .zip(master_x_coords.iter())
            .map(|(a, b)| b - a)
            .collect();
        let y_delta: Vec<i16> = base_y_coords
            .iter()
            .zip(master_y_coords.iter())
            .map(|(a, b)| b - a)
            .collect();
        let deltas: Vec<(i16, i16)> = x_delta
            .iter()
            .zip(y_delta.iter())
            .map(|(a, b)| (*a, *b))
            .collect();
        let peak = (*location).0.clone();
        // This is also terrible
        let bad_start = peak
            .iter()
            .map(|l| if *l > 0.0 { 0_f32 } else { -1_f32 })
            .collect();
        let bad_end = peak
            .iter()
            .map(|l| if *l > 0.0 { 1_f32 } else { 0_f32 })
            .collect();
        deltasets.push(DeltaSet {
            deltas,
            peak,
            start: bad_start,
            end: bad_end,
        })
    }
    GlyphVariationData { deltasets }
}

fn norad_contours_to_glyf_contour(contours: Vec<&norad::Contour>) -> Vec<Vec<glyf::Point>> {
    let mut error = 1.0;
    while error < 100.0 {
        // This is dirty
        let glyf_contours: Vec<Vec<glyf::Point>> = contours
            .iter()
            .map(|x| norad_contour_to_glyf_contour(x, error))
            .collect();
        let lengths: Vec<usize> = glyf_contours.iter().map(|x| x.len()).collect();
        if is_all_same(&lengths) {
            return glyf_contours;
        }
        error += 1.0;
    }
    panic!("Couldn't compatibly interpolate contours");
}

fn norad_contour_to_glyf_contour(contour: &norad::Contour, error: f32) -> Vec<glyf::Point> {
    let kurbo_path = contour.to_kurbo().expect("Bad contour construction");
    let mut points: Vec<glyf::Point> = vec![];
    if let PathEl::MoveTo(pt) = kurbo_path.elements()[0] {
        points.push(glyf::Point {
            x: pt.x as i16,
            y: pt.y as i16,
            on_curve: true,
        });
    }
    for seg in kurbo_path.segments() {
        match seg {
            PathSeg::Line(l) => points.push(glyf::Point {
                x: l.p1.x as i16,
                y: l.p1.y as i16,
                on_curve: true,
            }),
            PathSeg::Quad(q) => points.extend(vec![
                glyf::Point {
                    x: q.p1.x as i16,
                    y: q.p1.y as i16,
                    on_curve: false,
                },
                glyf::Point {
                    x: q.p2.x as i16,
                    y: q.p2.y as i16,
                    on_curve: true,
                },
            ]),
            PathSeg::Cubic(c) => {
                for (_, _, q) in c.to_quads(error.into()) {
                    points.extend(vec![
                        glyf::Point {
                            x: q.p1.x as i16,
                            y: q.p1.y as i16,
                            on_curve: false,
                        },
                        glyf::Point {
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

fn norad_component_to_glyf_component(
    component: &norad::Component,
    mapping: &BTreeMap<String, u16>,
) -> Option<glyf::Component> {
    let maybe_id = mapping.get(&component.base.to_string());

    if maybe_id.is_none() {
        log::warn!("Couldn't find component for {:?}", component.base);
        return None;
    }

    Some(glyf::Component {
        glyphIndex: *maybe_id.unwrap(),
        matchPoints: None,
        flags: glyf::ComponentFlags::empty(),
        transformation: component.transform.into(),
    })
}
