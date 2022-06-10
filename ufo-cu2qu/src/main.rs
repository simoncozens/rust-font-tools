use clap::Parser;
use kurbo::{cubics_to_quadratic_splines, BezPath, CubicBez, PathEl, PathSeg};
///! Decompose cubic to quadratic curves in one or more UFO files
use norad::Glyph;
use norad::{Contour, ContourPoint, Font, PointType};
use rayon::prelude::*;

/// Decompose cubic to quadratic curves in one or more UFO files
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Increase logging
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Layer to decompose
    #[clap(short, long)]
    layer: Option<String>,

    /// Output UFOs
    #[clap(short, long, multiple_values = true)]
    output: Vec<String>,

    /// Input UFO
    #[clap(required = true)]
    input: Vec<String>,
}

/// Tests if all elements of an iterator have the same content
pub fn is_all_the_same<T, U>(mut iter: T) -> bool
where
    T: Iterator<Item = U>,
    U: PartialEq,
{
    if let Some(first) = iter.next() {
        for n in iter {
            if first != n {
                return false;
            }
        }
    }
    true
}

fn cu2qu(contours: &mut [&mut Contour], glyph_name: &str) {
    let kurbo_paths: Vec<BezPath> = contours
        .iter()
        .map(|x| x.to_kurbo().expect("Bad contour construction"))
        .collect();
    let default_master = 0;
    let mut quadratic_paths: Vec<Contour> = contours
        .iter()
        .map(|_| Contour::new(vec![], None, None))
        .collect();
    let default_elements: &[PathEl] = kurbo_paths[default_master].elements();
    for (el_ix, el) in default_elements.iter().enumerate() {
        match el {
            PathEl::CurveTo(_, _, _) => {
                // Convert all the cubics to quadratics in one go, across masters
                let all_curves: Vec<CubicBez> = kurbo_paths
                    .iter()
                    .filter_map(|x| match x.get_seg(el_ix).unwrap() {
                        PathSeg::Cubic(c) => Some(c),
                        _ => None,
                    })
                    .collect();
                if let Some(all_quadratics) = cubics_to_quadratic_splines(&all_curves, 1.0) {
                    if all_quadratics.len() != quadratic_paths.len() {
                        log::error!(
                            "Didn't get as many curves as we expected for {:}",
                            glyph_name
                        );
                        return;
                    }

                    for (c_ix, contour) in quadratic_paths.iter_mut().enumerate() {
                        let spline_points = all_quadratics[c_ix].points();
                        // Skip the spline start, because we already have a point for that
                        for pt in spline_points.iter().skip(1) {
                            contour.points.push(ContourPoint::new(
                                pt.x,
                                pt.y,
                                PointType::OffCurve,
                                false,
                                None,
                                None,
                                None,
                            ));
                        }
                        // Last one is on-curve
                        if let Some(last) = contour.points.last_mut() {
                            last.typ = PointType::QCurve
                        }
                    }
                } else {
                    log::warn!("Could not compatibly interpolate {:}", glyph_name);
                    return;
                }
            }
            _ => {
                for (c_ix, contour) in quadratic_paths.iter_mut().enumerate() {
                    let this_path_el = kurbo_paths[c_ix].elements()[el_ix];
                    match this_path_el {
                        // PathEl::MoveTo(pt) => contour.points.push(ContourPoint::new(
                        //     pt.x,
                        //     pt.y,
                        //     PointType::Move,
                        //     false,
                        //     None,
                        //     None,
                        //     None,
                        // )),
                        PathEl::LineTo(pt) => contour.points.push(ContourPoint::new(
                            pt.x,
                            pt.y,
                            PointType::Line,
                            false,
                            None,
                            None,
                            None,
                        )),
                        PathEl::QuadTo(pt1, pt2) => {
                            // Maybe it was already?
                            contour.points.push(ContourPoint::new(
                                pt1.x,
                                pt1.y,
                                PointType::OffCurve,
                                false,
                                None,
                                None,
                                None,
                            ));
                            contour.points.push(ContourPoint::new(
                                pt2.x,
                                pt2.y,
                                PointType::QCurve,
                                false,
                                None,
                                None,
                                None,
                            ));
                        }
                        PathEl::CurveTo(_, _, _) => {
                            log::error!("Why is there a cubic in {}? (bug)", glyph_name);
                            return;
                        }
                        PathEl::ClosePath => {}
                        PathEl::MoveTo(_) => {}
                    }
                }
            }
        }
    }
    for (ix, old) in contours.iter_mut().enumerate() {
        std::mem::swap(
            &mut old.points,
            &mut quadratic_paths.get_mut(ix).unwrap().points,
        );
    }
}

fn main() {
    // Command line handling
    let args = Args::parse();

    env_logger::init_from_env(env_logger::Env::default().filter_or(
        env_logger::DEFAULT_FILTER_ENV,
        match args.verbose {
            0 => "warn",
            1 => "info",
            _ => "debug",
        },
    ));
    if args.input.len() != args.output.len() {
        log::error!(
            "Output length {:} should equal input length {:}",
            args.output.len(),
            args.input.len()
        );
        std::process::exit(1);
    }

    let mut fonts: Vec<Font> = args
        .input
        .par_iter()
        .map(|x| Font::load(x).unwrap_or_else(|_| panic!("Couldn't open UFO file {:}", x)))
        .collect();

    let glyph_names: Vec<String> = fonts
        .iter()
        .map(|font| {
            if let Some(layername) = &args.layer {
                font.layers.get(layername).expect("Couldn't find layer")
            } else {
                font.layers.default_layer()
            }
        })
        .next()
        .expect("No input font")
        .iter()
        .map(|x| x.name.to_string())
        .collect();
    log::info!("Loaded, converting glyphs");
    for glyph in glyph_names {
        let mut glyphs: Vec<&mut Glyph> = fonts
            .iter_mut()
            .map(|font| {
                if let Some(layername) = &args.layer {
                    font.layers
                        .get_mut(layername)
                        .expect("Layer not found")
                        .get_glyph_mut(&glyph)
                } else {
                    font.layers.default_layer_mut().get_glyph_mut(&glyph)
                }
            })
            .flatten()
            .collect();
        if !is_all_the_same(glyphs.iter().map(|x| x.contours.len())) {
            log::warn!("Incompatible glyph {:}", glyph);
            continue;
        }
        for index in 0..glyphs.first().unwrap().contours.len() {
            let mut contours: Vec<&mut Contour> = glyphs
                .iter_mut()
                .map(|g| g.contours.get_mut(index).unwrap())
                .collect();
            // If there are no Cubics, ignore

            cu2qu(&mut contours, &glyph);
        }
    }
    fonts
        .into_par_iter()
        .zip(args.output)
        .for_each(|(font, file)| {
            log::info!("Saving {}", file);
            font.save(file).expect("Could not save");
        });
}
