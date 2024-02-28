//! Interpolate an instance UFO in a designspace
use clap::Parser;
use norad::designspace::DesignSpaceDocument;
use norad::Glyph;
use otmath::{ot_round, VariationModel};
use rayon::prelude::*;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod fontinfo;
mod glyph;
mod instance;
mod interpolate;
mod kerning;
mod noradextensions;

use crate::fontinfo::interpolate_fontinfo;
use crate::glyph::interpolate_glyph;
use crate::instance::{
    filename_for, find_instance_by_location, find_instance_by_name, instance_to_location,
};
use crate::kerning::interpolate_kerning;
use crate::noradextensions::{BetterAxis, BetterDesignspace, BetterSource};

/// Interpolate an instance UFO in a designspace
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Increase logging
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Maintain zero kerns so that this UFO can be used in a merge
    #[clap(long)]
    will_merge: bool,

    #[clap(short, long, conflicts_with = "output")]
    instance: Option<String>,

    /// Output directory for instance UFOs
    #[clap(long, default_value = "instance_ufo", conflicts_with = "output")]
    output_directory: String,

    /// Output UFO
    #[clap(short, long)]
    output: Option<String>,

    /// Input designspace
    input: String,

    /// List of space separated locations. A location consists of the tag of a variation axis, followed by '=' and a number
    #[clap(conflicts_with = "instance")]
    location: Vec<String>,
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

    let ds: DesignSpaceDocument =
        DesignSpaceDocument::load(&args.input).expect("Couldn't read designspace file");
    let mut source_locations: Vec<BTreeMap<&str, f32>> = Vec::new();
    for source in &ds.sources {
        let this_loc: BTreeMap<&str, f32> = ds
            .axes
            .iter()
            .map(|x| x.tag.as_str())
            .zip(ds.location_to_tuple(&source.location))
            .collect();

        source_locations.push(this_loc);
    }
    let source_ufos: Vec<norad::Font> = ds
        .sources
        .par_iter()
        .map(|s| s.ufo(Path::new(&args.input)).expect("Couldn't load UFO"))
        .collect();
    let default_master = ds.default_master().expect("Can't find default master");
    let mut output_ufo = default_master
        .ufo(Path::new(&args.input))
        .expect("Couldn't load UFO");
    log::debug!("Source locations: {:?}", source_locations);

    let unnormalized_target_location = if let Some(instancename) = args.instance.as_deref() {
        let instance = find_instance_by_name(&ds, instancename).expect("Couldn't find instance");
        instance_to_location(&ds, instance)
    } else {
        parse_locargs(&args.location)
    };
    log::info!("Target location: {:?}", unnormalized_target_location);

    ensure_locations_are_sensible(&ds, &unnormalized_target_location);
    let target_location = ds
        .axes
        .iter()
        .map(|ax| {
            let this_axis_loc = unnormalized_target_location.get(&ax.tag).unwrap();
            (
                ax.tag.to_string(),
                ax.normalize_userspace_value(*this_axis_loc),
            )
        })
        .collect();

    log::debug!("Normalized target location: {:?}", target_location);
    let vm = ds.variation_model();

    interpolate_ufo(&mut output_ufo, source_ufos, vm, target_location, &args);

    let mut output_name = PathBuf::new();
    if args.instance.is_some() {
        output_name.push(args.output_directory.clone());
        if !output_name.exists() {
            std::fs::create_dir(&output_name).expect("Couldn't create output directory");
        }
    }

    if let Some(p) = args.output {
        output_name.push(&p);
    } else {
        output_name.push(make_a_name(unnormalized_target_location, &ds, &args));
    };
    log::info!(
        "Saving to {}",
        output_name
            .as_os_str()
            .to_str()
            .unwrap_or("Unrepresentable")
    );
    output_ufo.save(output_name).expect("Couldn't save UFO");
}

fn ensure_locations_are_sensible(
    ds: &DesignSpaceDocument,
    unnormalized_target_location: &BTreeMap<String, f32>,
) {
    // Ensure locations are sensible
    let mut ok = true;
    for axis in &ds.axes {
        let tag: &str = &axis.tag;
        if let Some(&location) = unnormalized_target_location.get(tag) {
            if axis.minimum.is_some() && Some(location) < axis.minimum {
                log::warn!(
                    "Location {} is less than minimum {} on axis {}",
                    location,
                    axis.minimum.unwrap(),
                    axis.tag
                );
            }
            if axis.maximum.is_some() && Some(location) > axis.maximum {
                log::warn!(
                    "Location {} is more than maximum {} on axis {}",
                    location,
                    axis.maximum.unwrap(),
                    axis.tag
                );
            }
        } else {
            log::error!("Tag {} needs a location", axis.tag);
            ok = false;
        }
    }
    if !ok {
        std::process::exit(1);
    }
}

fn make_a_name(
    unnormalized_target_location: BTreeMap<String, f32>,
    ds: &DesignSpaceDocument,
    args: &Args,
) -> String {
    if let Some(instance) = find_instance_by_location(ds, &unnormalized_target_location) {
        if let Some(name) = filename_for(instance) {
            return name;
        }
    }
    let location_str: Vec<String> = unnormalized_target_location
        .iter()
        .map(|(tag, val)| format!("{}{}", tag, val))
        .collect();
    let joined = location_str.join("-");
    args.input
        .replace(".designspace", &format!("-{}.ufo", &joined))
}

fn interpolate_ufo(
    output_ufo: &mut norad::Font,
    source_ufos: Vec<norad::Font>,
    vm: VariationModel<String>,
    target_location: BTreeMap<String, f32>,
    args: &Args,
) {
    for g in output_ufo.default_layer_mut().iter_mut() {
        let glyph_name = &g.name();
        let others: Vec<Option<&Glyph>> = source_ufos
            .iter()
            .map(|u| u.default_layer().get_glyph(glyph_name))
            .collect();
        interpolate_glyph(g, &others, &vm, &target_location);
    }

    interpolate_kerning(
        output_ufo,
        &source_ufos,
        &vm,
        &target_location,
        args.will_merge,
    );
    let fontinfos: Vec<Option<&norad::FontInfo>> =
        source_ufos.iter().map(|x| Some(&x.font_info)).collect();

    interpolate_fontinfo(&mut output_ufo.font_info, &fontinfos, &vm, &target_location);
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
