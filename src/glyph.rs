use crate::utils::is_all_same;
use fonttools::glyf;
use fonttools::glyf::contourutils::{kurbo_contour_to_glyf_contour, remove_implied_oncurves};
use fonttools::gvar::DeltaSet;
use fonttools::gvar::GlyphVariationData;
use fonttools::otvar::VariationModel;
use kurbo::{BezPath, PathEl, PathSeg};
use std::collections::BTreeMap;

type GlyphContour = Vec<Vec<glyf::Point>>;

pub fn glifs_to_glyph(
    default_master: usize,
    mapping: &BTreeMap<String, u16>,
    glifs: &[Option<&babelfont::Layer>],
    model: Option<&VariationModel>,
    glif_name: &str,
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
    if glif.components().is_empty() && glif.paths().is_empty() {
        return (glyph, None);
    }

    /* Do components */
    for component in &glif.components() {
        if let Some(glyf_component) = babelfont_component_to_glyf_component(component, mapping) {
            glyph.components.push(glyf_component);
        }
    }

    /* Do outlines */

    let mut widths: Vec<Option<i32>> = vec![];
    let mut contours: Vec<Option<GlyphContour>> = vec![];

    /* Base case */
    if model.is_none() {
        for contour in glif.paths().iter() {
            let glyph_contour = babelfont_contours_to_glyf_contours(vec![contour], 0, glif_name)
                .first()
                .unwrap()
                .clone();
            glyph.contours.push(glyph_contour);
        }
        return (glyph, None);
    }

    let indexes_of_nonsparse_masters: Vec<usize> =
        (0..glifs.len()).filter(|x| glifs[*x].is_some()).collect();

    for o in glifs {
        widths.push(o.and_then(|x| Some(x.width)));
        contours.push(o.and_then(|_| Some(vec![])));
    }

    for (index, _) in glif.paths().iter().enumerate() {
        for o in glifs {
            if o.is_some() && index >= o.unwrap().paths().len() {
                log::error!("Incompatible contour count in glyph {:}", glif_name);
                return (glyph, None);
            }
        }
        let all_contours: Vec<&babelfont::Path> = glifs
            .iter()
            .filter(|g| g.is_some())
            .map(|x| x.unwrap().paths()[index])
            .collect();
        let all_glyf_contours =
            babelfont_contours_to_glyf_contours(all_contours, default_master, glif_name);
        // Now we put them into their respective master
        for (finished_contour, master_id) in all_glyf_contours
            .iter()
            .zip(indexes_of_nonsparse_masters.iter())
        {
            assert!(contours[*master_id].is_some());
            contours[*master_id]
                .as_mut()
                .unwrap()
                .push(finished_contour.clone());
        }
    }
    if glifs.len() > 1
        && !contours.is_empty()
        && !contours[default_master].as_ref().unwrap().is_empty()
    {
        if !glyph.components.is_empty() {
            log::warn!("Can't create gvar deltas for mixed glyph {:}", glif_name);
            return (glyph, None);
        }
        let lengths: Vec<usize> = contours
            .iter()
            .filter(|x| x.is_some())
            .map(|g| g.as_ref().unwrap().iter().flatten().count())
            .collect();
        if !is_all_same(&lengths) {
            log::warn!("Incompatible glyph: {:}, lengths: {:?}", glif_name, lengths);
            glyph.contours = contours[default_master].as_ref().unwrap().clone();
            return (glyph, None);
        }
        let deltas = compute_deltas(&contours, widths, model.unwrap());
        glyph.contours = contours[default_master].as_ref().unwrap().clone();
        return (glyph, Some(deltas));
    }

    (glyph, None)
}

fn compute_deltas(
    contours: &[Option<GlyphContour>],
    widths: Vec<Option<i32>>,
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

fn babelfont_contours_to_glyf_contours(
    paths: Vec<&babelfont::Path>,
    default_master: usize,
    glif_name: &str,
) -> Vec<Vec<glyf::Point>> {
    // let's first get them all to kurbo elements
    let kurbo_paths: Vec<BezPath> = paths
        .iter()
        .map(|x| x.to_kurbo().expect("Bad contour construction"))
        .collect();
    let mut returned_contours: Vec<kurbo::BezPath> = paths.iter().map(|_| BezPath::new()).collect();
    let default_elements: &[PathEl] = kurbo_paths[default_master].elements();

    for (el_ix, el) in default_elements.iter().enumerate() {
        match el {
            PathEl::CurveTo(_, _, _) => {
                let all_curves: Vec<PathSeg> = kurbo_paths
                    .iter()
                    .map(|x| x.get_seg(el_ix).unwrap())
                    .collect();
                let all_quadratics = cubics_to_quadratics(all_curves, glif_name);
                for (c_ix, contour) in returned_contours.iter_mut().enumerate() {
                    for quad in &all_quadratics[c_ix] {
                        contour.push(*quad);
                    }
                }
            }
            _ => {
                for (c_ix, contour) in returned_contours.iter_mut().enumerate() {
                    contour.push(kurbo_paths[c_ix].elements()[el_ix]);
                }
            }
        }
    }

    returned_contours
        .iter()
        .map(|x| {
            let mut c = kurbo_contour_to_glyf_contour(x, 0.5);
            // remove_implied_oncurves(&mut c);
            c
        })
        .collect()
}

fn cubics_to_quadratics(cubics: Vec<PathSeg>, glif_name: &str) -> Vec<Vec<PathEl>> {
    let mut error = 0.05;
    let mut warned = false;
    while error < 50.0 {
        let mut quads: Vec<Vec<kurbo::PathEl>> = vec![];
        for pathseg in &cubics {
            if let PathSeg::Cubic(cubic) = pathseg {
                quads.push(
                    cubic
                        .to_quads(error)
                        .map(|(_, _, x)| PathEl::QuadTo(x.p1, x.p2))
                        .collect(),
                )
            } else {
                log::error!("Incompatible contours in glyph {:?}", glif_name);
                return itertools::repeat_n(vec![], cubics.len()).collect();
            }
        }

        let lengths: Vec<usize> = quads.iter().map(|x| x.len()).collect();
        if is_all_same(&lengths) {
            return quads;
        }
        error *= 1.5; // Exponential backoff
        if error > 20.0 && !warned {
            log::warn!(
                "{:} is proving difficult to interpolate - consider redesigning?",
                glif_name
            );
            warned = true;
        }
    }
    panic!("Couldn't compatibly interpolate contours");
}

fn babelfont_component_to_glyf_component(
    component: &babelfont::Component,
    mapping: &BTreeMap<String, u16>,
) -> Option<glyf::Component> {
    let maybe_id = mapping.get(&component.reference.to_string());

    if maybe_id.is_none() {
        log::warn!("Couldn't find component for {:?}", component.reference);
        return None;
    }

    Some(glyf::Component {
        glyph_index: *maybe_id.unwrap(),
        match_points: None,
        flags: glyf::ComponentFlags::empty(),
        transformation: component.transform.into(),
    })
}
