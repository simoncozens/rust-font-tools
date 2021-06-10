//! Command line utilities for manipulating OpenType files
//!
//! This crate contains a number of utilities for manipulating OpenType files:
//!
//! The utilities are designed in the "Unix pipe" philosophy: if you provide
//! one file name, it is understood as the input font; otherwise, the input
//! font is read from stdin. If you provide a second file name, it is understood
//! as the output font; otherwise the font is written to stdout.
//!
//!  * `fontcrunch` - A Rust port of https://github.com/googlefonts/fontcrunch
//!  * `ttf-add-minimal-dsig` - Adds a minimal DSIG table if one is not present
//!  * `ttf-fix-checksum` - Ensures TTF files have correct checksum
//!  * `ttf-fix-non-hinted` - Adds a `gasp` and `prep` table which is set to smooth for all sizes
//!  * `ttf-flatten-components` - Flattens components
//!  * `ttf-optimize-gvar` - Optimizes the gvar table by omitting points which can be inferred
//!  * `ttf-remove-overlap` - Removes overlap from TTF files
//!  * `ttf-rename-glyphs` - Renames glyphs to production names

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
