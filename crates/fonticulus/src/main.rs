mod basictables;
mod buildbasic;
mod fontinfo;
mod glyph;
mod utils;

use buildbasic::{build_font, build_fonts};
use clap::{App, Arg};
use fonttools::otvar::NormalizedLocation;
use std::fs::File;
use std::io;

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "warn"),
    );
    let matches = App::new("fonticulous")
        .about("A variable font builder")
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
    let mut font;

    if filename.ends_with(".designspace") {
        let ds = designspace::from_file(filename).expect("Couldn't parse designspace");
        let dm = ds.default_master().expect("Couldn't find default master");
        let mut dm_ufo = None;
        let mut other_masters: Vec<(NormalizedLocation, &norad::Layer)> = vec![];
        let all_sources: Vec<(&designspace::Source, norad::Font)> = ds
            .sources
            .source
            .iter()
            .map(|s| (s, s.ufo().expect("Couldn't open master file")))
            .collect();
        for (source, ufo) in &all_sources {
            if source.filename == dm.filename {
                dm_ufo = Some(ufo);
                continue;
            }
            other_masters.push((
                ds.normalize_location(ds.source_location(&source)),
                ufo.default_layer(),
            ));
        }

        font = build_fonts(dm_ufo.unwrap(), other_masters);

        ds.add_to_font(&mut font)
            .expect("Couldn't add variation tables");
    } else if filename.ends_with(".ufo") {
        let ufo = norad::Font::load(filename).expect("Can't load UFO file");
        font = build_font(ufo);
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
