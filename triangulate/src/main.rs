//! Interpolate an instance UFO in a designspace
use clap::Parser;
use designspace::Designspace;
use norad::{Contour, Glyph};
use otmath::{ot_round, support_scalar, Location, VariationModel};
use rayon::prelude::*;
use regex::Regex;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Interpolate an instance UFO in a designspace
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Increase logging
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Output UFO
    #[clap(short, long)]
    output: Option<String>,

    /// Input designspace
    input: String,

    /// List of space separated locations. A location consists of the tag of a variation axis, followed by '=' and a number
    loc_args: Vec<String>,
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

    let unnormalized_target_location = parse_locargs(&args.loc_args);
    let ds_file = File::open(&args.input).expect("Couldn't open designspace file");
    let ds: Designspace =
        designspace::from_reader(ds_file).expect("Couldn't read designspace file");
    // Ensure locations are sensible
    let mut ok = true;
    for axis in &ds.axes.axis {
        let tag: &str = &axis.tag;
        if let Some(&location) = unnormalized_target_location.get(tag) {
            if location < axis.minimum as f32 {
                println!(
                    "Location {} is less than minimum {} on axis {}",
                    location, axis.minimum, axis.tag
                );
            }
            if location > axis.maximum as f32 {
                println!(
                    "Location {} is more than maximum {} on axis {}",
                    location, axis.maximum, axis.tag
                );
            }
        } else {
            println!("Tag {} needs a location", axis.tag);
            ok = false;
        }
    }
    if !ok {
        std::process::exit(1);
    }
    let target_location = ds
        .axes
        .axis
        .iter()
        .map(|ax| {
            let this_axis_loc = unnormalized_target_location.get(&ax.tag).unwrap();
            (
                ax.tag.to_string(),
                ax.normalize_userspace_value(*this_axis_loc),
            )
        })
        .collect();

    let mut source_locations: Vec<BTreeMap<&str, f32>> = Vec::new();
    for source in &ds.sources.source {
        let this_loc: BTreeMap<&str, f32> = ds
            .axes
            .axis
            .iter()
            .map(|x| x.tag.as_str())
            .zip(ds.location_to_tuple(&source.location))
            .collect();

        source_locations.push(this_loc);
    }
    let source_ufos: Vec<norad::Font> = ds
        .sources
        .source
        .par_iter()
        .map(|s| s.ufo(Path::new(&args.input)).expect("Couldn't load UFO"))
        .collect();

    let mut output_ufo = ds
        .default_master()
        .expect("Can't find default master")
        .ufo(Path::new(&args.input))
        .expect("Couldn't load UFO");
    log::info!("Source locations: {:?}", source_locations);
    log::info!("Target location: {:?}", target_location);
    let vm = ds.variation_model();

    for g in output_ufo.default_layer_mut().iter_mut() {
        let glyph_name = &g.name;
        let others: Vec<Option<&Arc<Glyph>>> = source_ufos
            .iter()
            .map(|u| u.default_layer().get_glyph(glyph_name))
            .collect();
        interpolate_contours(g, &others, &vm, &target_location);
        // XXX anchors
        // XXX advance widths
    }
    if let Some(p) = args.output {
        output_ufo.save(p).expect("Couldn't save UFO");
    }
}

fn str_to_fixed_to_float(s: &str) -> f32 {
    ot_round(str::parse::<f32>(s).unwrap()) as f32
}

fn parse_locargs(locargs: &[String]) -> BTreeMap<String, f32> {
    let mut res = BTreeMap::new();
    let matcher = Regex::new(r"^(\w{1,4})=([\d\.]+)$").unwrap();
    for limit_string in locargs {
        let captures = matcher
            .captures(limit_string)
            .expect("Couldn't parse location format");
        let tag = captures.get(1).unwrap().as_str();
        let location: f32 = str_to_fixed_to_float(captures.get(2).unwrap().as_str());
        res.insert(tag.to_string(), location);
    }
    res
}

trait QuickGetSet {
    fn get_contour_numbers(&self) -> ndarray::Array1<f32>;
    fn add_contour_numbers(&mut self, numbers: ndarray::Array1<f32>);
}

impl QuickGetSet for Glyph {
    fn get_contour_numbers(&self) -> ndarray::Array1<f32> {
        let mut v = vec![];
        for contour in &self.contours {
            for p in &contour.points {
                v.push(p.x as f32);
                v.push(p.y as f32);
            }
        }
        let len = v.len();
        ndarray::Array1::from_shape_vec(len, v).unwrap()
    }

    fn add_contour_numbers(&mut self, numbers: ndarray::Array1<f32>) {
        let v: Vec<f32> = numbers.to_vec();
        let mut i = 0;
        for contour in self.contours.iter_mut() {
            for p in contour.points.iter_mut() {
                p.x += (*v.get(i).expect("Not enough coordinates")) as f64;
                i += 1;
                p.y += (*v.get(i).expect("Not enough coordinates")) as f64;
                i += 1;
            }
        }
    }
}

fn interpolate_contours(
    output: &mut Glyph,
    masters: &[Option<&Arc<Glyph>>],
    model: &VariationModel<String>,
    location: &Location<String>,
) {
    let default_numbers: ndarray::Array1<f32> = output.get_contour_numbers();
    let contours: Vec<Option<ndarray::Array1<f32>>> = masters
        .iter()
        .map(|m| {
            m.and_then(|g| {
                let nums: ndarray::Array1<f32> = g.get_contour_numbers();
                if nums.shape() == default_numbers.shape() {
                    Some(g.get_contour_numbers() - default_numbers.clone())
                } else {
                    log::warn!("Incompatible masters in {}", g.name);
                    None
                }
            })
        })
        .collect();
    let deltas_and_supports = model.get_deltas_and_supports(&contours);
    let (deltas, support_scalars): (Vec<ndarray::Array1<f32>>, Vec<f32>) = deltas_and_supports
        .into_iter()
        .map(|(x, y)| (x, support_scalar(location, &y)))
        .unzip();

    let interpolated = model.interpolate_from_deltas_and_scalars(&deltas, &support_scalars);
    output.add_contour_numbers(interpolated.expect("Couldn't interpolate"));
}
