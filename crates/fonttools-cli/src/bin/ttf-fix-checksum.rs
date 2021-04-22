use clap::{App, Arg};
use fonttools::font;
use std::fs::File;
use std::io;

fn main() {
    let matches = App::new("ttf-fix-checksum")
        .about("Ensures TTF files have correct checksum")
        .arg(Arg::with_name("drop-names"))
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
        .get_matches();

    let mut infont = if matches.is_present("INPUT") {
        let filename = matches.value_of("INPUT").unwrap();
        let infile = File::open(filename).unwrap();
        font::load(infile)
    } else {
        font::load(io::stdin())
    }
    .expect("Could not parse font");
    if matches.is_present("OUTPUT") {
        let mut outfile = File::create(matches.value_of("OUTPUT").unwrap())
            .expect("Could not open file for writing");
        infont.save(&mut outfile);
    } else {
        infont.save(&mut io::stdout());
    };
}
