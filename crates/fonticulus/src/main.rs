//! A fonticulously fast variable font builder
mod basictables;
mod buildbasic;
mod fontinfo;
mod glyph;
mod utils;

use buildbasic::{build_font, build_fonts};
use clap::{App, Arg};
use designspace::Designspace;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::io;

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
    let mut font;

    if filename.ends_with(".designspace") {
        let ds = designspace::from_file(filename).expect("Couldn't parse designspace");
        let default_location = ds.default_designspace_location();
        let dm_index = ds
            .sources
            .source
            .iter()
            .position(|s| ds.source_location(s) == default_location);
        if dm_index.is_none() {
            default_master_not_found_error(ds);
        }
        let masters: Vec<norad::Font> = ds
            .sources
            .source
            .par_iter()
            .map(|s| s.ufo().expect("Couldn't open master file"))
            .collect();
        font = build_fonts(dm_index.unwrap(), masters, ds.variation_model(), subset);
        ds.add_to_font(&mut font)
            .expect("Couldn't add variation tables");
    } else if filename.ends_with(".ufo") {
        let ufo = norad::Font::load(filename).expect("Can't load UFO file");
        font = build_font(ufo, subset);
    } else {
        panic!("Unknown file type {:?}", filename);
    }

    if matches.is_present("OUTPUT") {
        let mut outfile = File::create(matches.value_of("OUTPUT").unwrap())
            .expect("Could not open file for writing");
        font.save(&mut outfile);
    } else {
        font.save(&mut io::stdout());
    };
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
