//! Check a Designspace file for interpolatability and other issues
mod designspace;
mod interpolatability;

extern crate designspace as designspacelib;

use crate::designspace::check_designspace;
use crate::interpolatability::check_interpolatability;
use clap::Parser;
use serde::Serialize;
use std::fs::File;

/// Check a Designspace file for interpolatability and other issues
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Increase logging
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Output as JSON
    #[clap(short, long)]
    json: bool,

    /// Don't do Designspace validation
    #[clap(long)]
    no_validation: bool,

    /// Don't do interpolatability checking
    #[clap(long)]
    no_interpolatability: bool,

    /// Input designspace
    designspace: String,
}

#[derive(Serialize)]
struct Problem {
    area: String,
    glyph: Option<String>,
    location: Option<String>,
    master: Option<String>,
    description: String,
}

impl Problem {
    fn as_string(&self) -> String {
        let mut s: String = String::new();
        if let Some(g) = &self.glyph {
            s += format!("in glyph {}, ", g).as_str();
        }
        if let Some(l) = &self.location {
            s += format!("at {}, ", l).as_str();
        }
        if let Some(m) = &self.master {
            s += format!("for master {}, ", m).as_str();
        }
        s += &self.description;
        s[0..1].to_uppercase() + &s[1..]
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
    let ds_file = File::open(&args.designspace).expect("Couldn't open designspace file");
    let ds = designspacelib::from_reader(ds_file).expect("Couldn't read designspace file");
    let mut problems: Vec<Problem> = vec![];
    if !args.no_validation {
        problems.extend(check_designspace(&ds));
    }
    if !args.no_interpolatability {
        problems.extend(check_interpolatability(&ds, &args));
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&problems).expect("Couldn't serialize")
        );
    } else {
        problems.sort_by(|a, b| a.area.cmp(&b.area));
        let mut current_area: Option<&String> = None;
        for p in problems.iter() {
            if Some(&p.area) != current_area {
                println!("\n# {}\n", p.area);
                current_area = Some(&p.area);
            }
            println!("{}", p.as_string())
        }
    }
}
