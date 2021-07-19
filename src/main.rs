//! A fonticulously fast variable font builder
mod basictables;
mod buildbasic;
mod fontinfo;
mod glyph;
mod kerning;
mod utils;

use buildbasic::{build_font, build_static_master};
use clap::{App, Arg};
use designspace::Designspace;
// use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::path::PathBuf;

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "warn"),
    );
    let matches = App::new("fonticulous")
        .about("A variable font builder")
        .arg(
            Arg::with_name("subset")
                .help("Only convert the given glyphs (for testing only)")
                .required(false)
                .takes_value(true)
                .long("subset"),
        )
        .arg(
            Arg::with_name("masters")
                .help("Don't make a variable font, make a static font for each master")
                .required(false)
                .takes_value(false)
                .long("masters"),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Sets the output file to use")
                .required(false),
        )
        .get_matches();
    let filename = matches.value_of("INPUT").unwrap();
    let subset = matches.value_of("subset").map(|x| {
        x.split(',')
            .map(|y| y.to_string())
            .collect::<HashSet<String>>()
    });
    let no_interpolate = matches.is_present("masters");
    let in_font = if filename.ends_with(".designspace") {
        babelfont::convertors::designspace::load(PathBuf::from(filename))
            .expect("Couldn't load source")
    } else if filename.ends_with(".ufo") {
        // let ufo = norad::Font::load(filename).expect("Can't load UFO file");
        // font = build_font(ufo, subset);
        unimplemented!();
    } else if filename.ends_with(".glyphs") {
        babelfont::convertors::glyphs3::load(PathBuf::from(filename)).expect("Couldn't load source")
    } else {
        panic!("Unknown file type {:?}", filename);
    };

    if no_interpolate {
        let family_name = in_font
            .names
            .family_name
            .default()
            .unwrap_or("New Font".to_string());
        for (ix, master) in in_font.masters.iter().enumerate() {
            let mut out_font = build_static_master(&in_font, &subset, ix);
            let master_name = master
                .name
                .default()
                .unwrap_or_else(|| format!("Master{}", ix));
            log::info!("Building {}", master_name);
            let mut outfile = File::create(format!("{}-{}.ttf", family_name, master_name))
                .expect("Could not open file for writing");
            out_font.save(&mut outfile);
        }
    } else {
        let mut font = build_font(&in_font, &subset);
        if in_font.masters.len() > 1 {
            in_font
                .add_variation_tables(&mut font)
                .expect("Couldn't add variation tables")
        }

        if matches.is_present("OUTPUT") {
            let mut outfile = File::create(matches.value_of("OUTPUT").unwrap())
                .expect("Could not open file for writing");
            font.save(&mut outfile);
        } else {
            font.save(&mut io::stdout());
        };
    }
}

fn default_master_not_found_error(ds: Designspace) -> ! {
    let default_location = ds.default_location();
    let location_to_string = |location: Vec<i32>| {
        ds.axis_order()
            .iter()
            .zip(location.iter())
            .map(|(tag, value)| format!("{:}={:}", std::str::from_utf8(tag).unwrap(), value))
            .collect::<Vec<String>>()
            .join(", ")
    };
    log::error!(
        "Could not find default master [{:}]",
        location_to_string(default_location)
    );
    eprintln!("Master locations were: ");
    for source in &ds.sources.source {
        eprintln!(
            "{:} = {:}",
            source.filename,
            location_to_string(ds.source_location(source))
        );
    }
    std::process::exit(1);
}
