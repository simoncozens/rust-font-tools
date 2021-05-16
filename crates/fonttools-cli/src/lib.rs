use clap::{App, Arg};
use fonttools::font::{self, Font};
use std::fs::File;
use std::io;

pub fn read_args(name: &str, description: &str) -> clap::ArgMatches<'static> {
    App::new(name)
        .about(description)
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(false),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Sets the output file to use")
                .required(false),
        )
        .get_matches()
}

pub fn open_font(matches: &clap::ArgMatches) -> Font {
    if matches.is_present("INPUT") {
        let filename = matches.value_of("INPUT").unwrap();
        let infile = File::open(filename).unwrap();
        font::load(infile)
    } else {
        font::load(io::stdin())
    }
    .expect("Could not parse font")
}

pub fn save_font(mut font: Font, matches: &clap::ArgMatches) {
    if matches.is_present("OUTPUT") {
        let mut outfile = File::create(matches.value_of("OUTPUT").unwrap())
            .expect("Could not open file for writing");
        font.save(&mut outfile);
    } else {
        font.save(&mut io::stdout());
    };
}
