use crate::utils::is_all_same;
use fonttools::glyf;
use fonttools::gvar::DeltaSet;
use fonttools::gvar::GlyphVariationData;
use fonttools::otvar::{NormalizedLocation, VariationModel};
use kurbo::{PathEl, PathSeg};
use otspec::types::Tuple;
use std::collections::BTreeMap;

pub fn glifs_to_glyph(
    default_master: usize,
    mapping: &BTreeMap<String, u16>,
    glifs: &[Option<&std::sync::Arc<norad::Glyph>>],
    model: Option<&VariationModel>,
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
    let glif = glifs[default_master].expect("No glif in default master!");
    if glif.components.is_empty() && glif.contours.is_empty() {
        return (glyph, None);
    }

    /* Do components */
    for component in &glif.components {
        if let Some(glyf_component) = norad_component_to_glyf_component(component, mapping) {
            glyph.components.push(glyf_component);
        }
    }

    /* Do outlines */

    let mut widths: Vec<Option<f32>> = vec![];
    let mut contours: Vec<Option<Vec<Vec<glyf::Point>>>> = vec![];

    /* Base case */
    if model.is_none() {
        for contour in glif.contours.iter() {
            glyph
                .contours
                .push(norad_contour_to_glyf_contour(contour, 1.0));
        }
        return (glyph, None);
    }

    for o in glifs {
        if o.is_some() {
            contours.push(Some(vec![]));
            widths.push(Some(o.unwrap().width));
        } else {
            widths.push(None);
            contours.push(None);
        }
    }

    for (index, _) in glif.contours.iter().enumerate() {
        for o in glifs {
            if o.is_some() && index > o.unwrap().contours.len() {
                // Let's assume we've done some interpolatability checks before this point
                panic!("Incompatible contour in glyph {:?}", o);
            }
        }
        // A vector of masters: each master is either sparse (None) or has a
        // matching contour for this contour. (Some(&norad::Contour))
        let all_contours: Vec<Option<&norad::Contour>> = glifs
            .iter()
            .map(|x| x.map(|y| &y.contours[index]))
            .collect();
        // Same, but with glyf Contour objects.
        let all_glyf_contours = norad_contours_to_glyf_contour(all_contours);
        // Now we put them into their respective master
        for master_id in 0..glifs.len() {
            if contours[master_id].is_some() {
                contours[master_id]
                    .as_mut()
                    .unwrap()
                    .push(all_glyf_contours[master_id].as_ref().unwrap().clone());
            }
        }
    }
    if !contours.is_empty() && !contours[default_master].as_ref().unwrap().is_empty() {
        if !glyph.components.is_empty() {
            log::warn!(
                "Can't create gvar deltas for mixed glyph {:}",
                glif.name.to_string()
            );
            return (glyph, None);
        }

        let deltas = compute_deltas(&contours, widths, model.unwrap());
        glyph.contours = contours[default_master].as_ref().unwrap().clone();
        return (glyph, Some(deltas));
    }

    (glyph, None)
}

fn compute_deltas(
    contours: &[Option<Vec<Vec<glyf::Point>>>],
    widths: Vec<Option<f32>>,
    model: &VariationModel,
) -> GlyphVariationData {
    let mut deltasets: Vec<DeltaSet> = vec![];
    let mut all_coords = vec![];
    for (ix, master) in contours.iter().enumerate() {
        if let Some(master) = master {
            let width = widths[ix].unwrap();
            let (mut master_x_coords, mut master_y_coords): (Vec<f32>, Vec<f32>) = master
                .iter()
                .flatten()
                .map(|pt| (pt.x as f32, pt.y as f32))
                .unzip();
            master_x_coords.extend(vec![0_f32, width as f32, 0.0, 0.0]);
            let len = master_x_coords.len();
            master_y_coords.extend(vec![0.0, 0.0, 0.0, 0.0]);
            master_x_coords.extend(master_y_coords);
            all_coords.push(Some(
                ndarray::Array2::from_shape_vec((2, len), master_x_coords)
                    .unwrap()
                    .reversed_axes(),
            ));
        } else {
            all_coords.push(None);
        }
    }
    let deltas_and_supports = model.get_deltas_and_supports(&all_coords);
    for (delta, support) in deltas_and_supports.iter() {
        if support.is_empty() {
            continue;
        }
        let coords = delta
            .mapv(|x| x as i16)
            .outer_iter()
            .map(|x| (x[0], x[1]))
            .collect::<Vec<(i16, i16)>>();
        let tuples: Vec<&(f32, f32, f32)> = model
            .axis_order
            .iter()
            .map(|ax| support.get(ax).unwrap_or(&(0.0, 0.0, 0.0)))
            .collect();
        let peak = tuples.iter().map(|x| x.1).collect();
        let start = tuples.iter().map(|x| x.0).collect();
        let end = tuples.iter().map(|x| x.2).collect();
        deltasets.push(DeltaSet {
            deltas: coords,
            peak,
            start,
            end,
        })
    }
    GlyphVariationData { deltasets }
}

fn norad_contours_to_glyf_contour(
    contours: Vec<Option<&norad::Contour>>,
) -> Vec<Option<Vec<glyf::Point>>> {
    let mut error = 1.0;
    while error < 100.0 {
        // This is dirty
        let glyf_contours: Vec<Option<Vec<glyf::Point>>> = contours
            .iter()
            .map(|x| x.map(|y| norad_contour_to_glyf_contour(y, error)))
            .collect();
        let lengths: Vec<usize> = glyf_contours.iter().flatten().map(|x| x.len()).collect();
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
