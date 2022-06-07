use crate::{Args, Problem};
use designspace::{Designspace, Source};
use norad::{Anchor, Component, Glyph, PointType};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

pub(crate) fn check_interpolatability(
    ds: &Designspace,
    args: &Args,
) -> impl Iterator<Item = Problem> {
    let default_master = ds.default_master();
    if default_master.is_none() {
        let mut problems: Vec<Problem> = vec![];
        if args.no_validation {
            problems.push(Problem {
                area: "designspace".to_string(),
                glyph: None,
                location: None,
                master: None,
                description: "couldn't find default master".to_string(),
            });
        }
        return problems.into_iter();
    }
    let mut problems: Vec<Problem> = vec![];
    let default_master = default_master.unwrap();
    let default_ufo = default_master
        .ufo(Path::new(&args.designspace))
        .expect("Couldn't load default UFO");
    let other_sources: Vec<&Source> = ds
        .sources
        .source
        .iter()
        .filter(|source| source.filename != default_master.filename)
        .collect();
    let other_source_names: Vec<String> = other_sources
        .iter()
        .map(|s| s.name.as_ref().unwrap_or(&s.filename))
        .cloned()
        .collect();
    let other_ufos: Vec<norad::Font> = other_sources
        .par_iter()
        .map(|s| {
            s.ufo(Path::new(&args.designspace))
                .expect("Couldn't load UFO")
        })
        .collect();

    for g in default_ufo.default_layer().iter() {
        let glyph_name = &g.name;
        let others: Vec<&Arc<Glyph>> = other_ufos
            .iter()
            .map(|u| u.default_layer().get_glyph(glyph_name))
            .flatten()
            .collect();
        problems.extend(check_glyph(g, &others, &other_source_names));
    }

    problems.into_iter()
}

fn check_glyph(
    g: &norad::Glyph,
    others: &[&Arc<Glyph>],
    others_names: &[String],
) -> impl Iterator<Item = Problem> {
    let mut problems: Vec<Problem> = vec![];
    let glyph_name = Some((&g.name).to_string());
    // Contours
    let path_count = g.contours.len();
    for (other_glyph, master) in others.iter().zip(others_names.iter()) {
        let other_path_count = other_glyph.contours.len();
        let master_name = Some(master.to_string());
        if other_path_count != path_count {
            problems.push(Problem {
                area: "contours".to_string(),
                glyph: glyph_name.clone(),
                location: None,
                master: master_name,
                description: format!(
                    "path count should be {}, found {}",
                    path_count, other_path_count
                ),
            });
            continue;
        }
        for contour_ix in 0..path_count {
            problems.extend(check_contour(
                contour_ix,
                &master_name,
                &glyph_name,
                &g.contours[contour_ix],
                &other_glyph.contours[contour_ix],
            ))
        }
        problems.extend(check_anchors(
            &master_name,
            &glyph_name,
            &g.anchors,
            &other_glyph.anchors,
        ));
        problems.extend(check_components(
            &master_name,
            &glyph_name,
            &g.components,
            &other_glyph.components,
        ));
    }
    problems.into_iter()
}

fn check_contour(
    contour_ix: usize,
    master_name: &Option<String>,
    glyph_name: &Option<String>,
    contour: &norad::Contour,
    other: &norad::Contour,
) -> impl Iterator<Item = Problem> {
    let mut problems: Vec<Problem> = vec![];
    if contour.points.len() != other.points.len() {
        problems.push(Problem {
            area: "contours".to_string(),
            glyph: glyph_name.clone(),
            location: Some(format!("contour {}", contour_ix + 1)),
            master: master_name.clone(),
            description: format!(
                "point count should be {}, found {}",
                contour.points.len(),
                other.points.len()
            ),
        });
        return problems.into_iter();
    }
    for (ix, (left, right)) in contour.points.iter().zip(other.points.iter()).enumerate() {
        if left.typ != right.typ {
            problems.push(Problem {
                area: "contours".to_string(),
                glyph: glyph_name.clone(),
                location: Some(format!("contour {}, point {}", contour_ix + 1, ix + 1)),
                master: master_name.clone(),
                description: format!("point type should be {}, found {}", left.typ, right.typ),
            });
        }
    }

    // Check for wrong contour starting point, adapted from
    // https://github.com/fonttools/fonttools/pull/2571/files

    // Build list of isomorphisms for the other guy, build real point list for me
    let point_list: Vec<(f64, f64)> = contour.points.iter().map(|p| (p.x, p.y)).collect();
    let mut other_point_list: Vec<(f64, f64)> = other.points.iter().map(|p| (p.x, p.y)).collect();
    let my_types: Vec<PointType> = contour.points.iter().map(|p| p.typ.clone()).collect();
    let mut other_types: Vec<PointType> = other.points.iter().map(|p| p.typ.clone()).collect();
    let mut other_possible_configurations: Vec<Vec<(f64, f64)>> = vec![];
    for _ in 0..other_point_list.len() {
        if !my_types.iter().zip(other_types.iter()).all(|(a, b)| a == b) {
            continue;
        }
        other_possible_configurations.push(other_point_list.clone());
        other_point_list.rotate_left(1);
        other_types.rotate_left(1);
    }
    other_point_list.reverse();
    other_types.reverse();
    for _ in 0..other_point_list.len() {
        if !my_types.iter().zip(other_types.iter()).all(|(a, b)| a == b) {
            continue;
        }
        other_possible_configurations.push(other_point_list.clone());
        other_point_list.rotate_left(1);
        other_types.rotate_left(1);
    }

    let costs: Vec<f64> = other_possible_configurations
        .iter()
        .map(|configuration| {
            configuration
                .iter()
                .zip(point_list.iter())
                .map(|(pt1, pt2)| {
                    (pt1.0 - pt2.0) * (pt1.0 - pt2.0) + (pt1.1 - pt2.1) * (pt1.1 - pt2.1)
                })
                .sum()
        })
        .collect();
    // println!("Alternative positions of {:?}:", glyph_name);
    // println!("{:?}", other_possible_configurations);
    // println!("Costs: {:?}", costs);
    let mincost = costs.iter().copied().fold(f64::NAN, f64::min);
    if mincost < costs.first().unwrap_or(&0.0) * 0.95 {
        problems.push(Problem {
            area: "contours".to_string(),
            glyph: glyph_name.clone(),
            location: Some(format!("contour {}", contour_ix + 1)),
            master: master_name.clone(),
            description: "wrong start point".to_string(),
        });
    }
    problems.into_iter()
}

fn check_anchors(
    master_name: &Option<String>,
    glyph_name: &Option<String>,
    our_anchors: &[Anchor],
    their_anchors: &[Anchor],
) -> impl Iterator<Item = Problem> {
    let mut problems: Vec<Problem> = vec![];
    let our_set: HashSet<&String> = our_anchors
        .iter()
        .map(|a| a.name.as_ref())
        .flatten()
        .collect();
    let their_set: HashSet<&String> = their_anchors
        .iter()
        .map(|a| a.name.as_ref())
        .flatten()
        .collect();
    for missing in our_set.difference(&their_set) {
        problems.push(Problem {
            area: "anchors".to_string(),
            glyph: glyph_name.clone(),
            location: None,
            master: master_name.clone(),
            description: format!("anchor {} missing", missing),
        })
    }
    for extra in their_set.difference(&our_set) {
        problems.push(Problem {
            area: "anchors".to_string(),
            glyph: glyph_name.clone(),
            location: None,
            master: master_name.clone(),
            description: format!("anchor {} in master but not in default", extra),
        })
    }
    problems.into_iter()
}

fn check_components(
    master_name: &Option<String>,
    glyph_name: &Option<String>,
    our_components: &[Component],
    their_components: &[Component],
) -> impl Iterator<Item = Problem> {
    let mut problems: Vec<Problem> = vec![];
    if our_components.len() != their_components.len() {
        problems.push(Problem {
            area: "components".to_string(),
            glyph: glyph_name.clone(),
            location: None,
            master: master_name.clone(),
            description: format!(
                "component count should be {}, found {}",
                our_components.len(),
                their_components.len()
            ),
        });
        return problems.into_iter();
    }
    for (ix, (ours, theirs)) in our_components
        .iter()
        .zip(their_components.iter())
        .enumerate()
    {
        if ours.base != theirs.base {
            problems.push(Problem {
                area: "components".to_string(),
                glyph: glyph_name.clone(),
                location: Some(format!("component {}", ix + 1)),
                master: master_name.clone(),
                description: format!(
                    "component base should be {}, found {}",
                    ours.base, theirs.base
                ),
            });
        }
    }
    problems.into_iter()
}
