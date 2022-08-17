//! A fonticulously fast variable font builder
mod basictables;
mod buildbasic;
mod fontinfo;
mod glyph;
mod kerning;
mod notdef;
mod utils;

use buildbasic::build_font;
use clap::Parser;

/// A fonticulusly fast font builder
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Only convert the given glyphs (for testing only, always includes .notdef).
    #[clap(short, long)]
    subset: Option<String>,

    /// Don't make a variable font, make a static font for each master
    #[clap(long)]
    masters: bool,

    /// Increase logging
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    input: String,
    output: Option<String>,
}

// use rayon::prelude::*;
use std::collections::HashSet;
use std::io;

/*
    OK, here is the basic plan:

    1) This function handles command line stuff, uses babelfont-rs to load
       the source file(s) into memory, and calls into buildbasic::build_font.
    2) The build_font function in buildbasic.rs coordinates the build.
    3) basictables.rs creates the non-glyph, non-layout, non-variable metadata tables
       (that is: head, hhea, maxp, OS/2, hmtx, cmap, glyf, name, post, loca).
    3a) fontinfo.rs works out what some of the stuff in those tables should be.
    4) glyph.rs handles Babelfont->OT glyph conversion, creating the glyf and gvar
       table entries for each glyph.
    5) babelfont-rs creates the variable metadata tables (fvar,avar).
    6) We come back here and save the files at the end.
*/

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

    // If we are only handling a subset of the glyphs (usually for debugging purposes),
    // split that into a set here. Always include the ".notdef" glyph, because we might
    // dynamically add it.
    let mut subset: Option<HashSet<&str>> = args.subset.as_ref().map(|x| x.split(',').collect());
    if let Some(subset) = &mut subset {
        subset.insert(".notdef");
    }

    let mut in_font = babelfont::load(&args.input).expect("Couldn't load font");

    // --masters means we produce a TTF for each master and don't do interpolation
    if args.masters {
        create_ttf_per_master(&mut in_font, subset.as_ref());
    } else {
        create_variable_font(&mut in_font, subset.as_ref(), &args.output);
    }
}

fn create_ttf_per_master(in_font: &mut babelfont::Font, subset: Option<&HashSet<&str>>) {
    let family_name = in_font
        .names
        .family_name
        .get_default()
        .unwrap_or_else(|| "New Font".to_string());
    let master_names: Vec<String> = in_font
        .masters
        .iter()
        .enumerate()
        .map(|(ix, master)| {
            let master_name = master
                .name
                .get_default()
                .unwrap_or_else(|| format!("Master{}", ix));
            if master_name == "Unnamed master" {
                format!("Master{}", ix)
            } else {
                master_name
            }
        })
        .collect();
    for (ix, master_name) in master_names.iter().enumerate() {
        let mut out_font = build_font(in_font, subset, Some(ix));
        log::info!("Building {}", master_name);
        out_font
            .save(format!("{}-{}.ttf", family_name, master_name))
            .expect("Could not write font");
    }
}

fn create_variable_font(
    in_font: &mut babelfont::Font,
    subset: Option<&HashSet<&str>>,
    output: &Option<String>,
) {
    let mut out_font;
    if in_font.masters.len() > 1 {
        out_font = build_font(in_font, subset, None);
        // Ask babelfont to make fvar/avar
        in_font
            .add_variation_tables(&mut out_font)
            .expect("Couldn't add variation tables");
    } else {
        out_font = build_font(in_font, subset, Some(0));
    }

    match output {
        Some(filename) => out_font.save(filename).expect("Could not write font"),
        None => out_font.write(io::stdout()).expect("Could not write font"),
    }
}
