use clap::{App, Arg};
use fonttools::otvar::instancer::{
    instantiate_variable_font, AxisRange, UserAxisLimit, UserAxisLimits,
};
use fonttools::tag;
use fonttools::types::*;
use fonttools_cli::open_font;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;

fn main() {
    let matches = App::new("ttf-instantiate")
        .about("Partially or fully instantiate variable fonts")
        .arg(Arg::from_usage("-d, --drop-names"))
        .arg(Arg::from_usage("-o, --output=[FILE]  Output instance TTF file"))
        .arg(Arg::from_usage("--no-overlap-flag    Dont set OVERLAP_SIMPLE/OVERLAP_COMPOUND glyf flags"))
        .arg(Arg::from_usage("--update-name-table  Update the instantiated fonts `name` table."))
        .arg(Arg::with_name("verbose").short("v").multiple(true).required(false).help("Run more verbosely"))

        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true),
        ).arg(
            Arg::with_name("loc-args")
                .help("List of space separated locations. A location consists of the tag of a variation axis, followed by '=' and one of number, number:number or the literal string 'drop'. E.g.: wdth=100 or wght=75.0:125.0 or wght=drop")
                 .multiple(true)
                .required(true),
        )

        .get_matches();

    if matches.is_present("verbose") {
        simple_logger::init_with_level(log::Level::Debug).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Warn).unwrap();
    }

    let locargs: Vec<&str> = matches.values_of("loc-args").unwrap().collect();
    let locarg_len = locargs.len();
    let limits = parse_locargs(locargs);
    if limits.len() != locarg_len {
        println!("Multiple limits for same axis");
        return;
    }

    let mut infont = open_font(&matches);
    if !infont.tables.contains(&tag!("fvar")) {
        println!("This isn't a variable font");
        return;
    }

    log::debug!("Axis limits = {:?}", limits);
    if instantiate_variable_font(&mut infont, limits) {
        if let Some(out_fn) = matches.value_of("output") {
            log::info!("Saving on {}", out_fn);
            infont.save(out_fn)
        } else {
            let input_filename = matches.value_of("INPUT").unwrap();
            let out_fn = Path::new(input_filename)
                .with_extension("")
                .with_extension("partial.ttf");
            log::info!("Saving on {}", out_fn.to_str().unwrap());
            infont.save(out_fn)
        }
        .unwrap();
    }
}

fn str_to_fixed_to_float(s: &str) -> f32 {
    Fixed::round(str::parse::<f32>(s).unwrap())
}

fn parse_locargs(locargs: Vec<&str>) -> UserAxisLimits {
    let mut res = BTreeMap::new();
    let matcher = Regex::new(r"^(\w{1,4})=(?:(drop)|(?:([^:]+)(?:[:](.+))?))$").unwrap();
    for limit_string in locargs {
        let captures = matcher
            .captures(limit_string)
            .expect("Couldn't parse location format");
        let btag = Tag::from_raw(captures.get(1).unwrap().as_str()).unwrap();
        let lower: Option<f32> = if captures.get(2).is_some() {
            None
        } else {
            Some(str_to_fixed_to_float(captures.get(3).unwrap().as_str()))
        };
        let mut upper = lower;
        if let Some(ustr) = captures.get(4) {
            upper = Some(str_to_fixed_to_float(ustr.as_str()));
        }
        if upper != lower {
            res.insert(
                btag,
                UserAxisLimit::Partial(AxisRange::new(lower.unwrap(), upper.unwrap())),
            );
        } else if let Some(l) = lower {
            res.insert(btag, UserAxisLimit::Full(l));
        } else {
            res.insert(btag, UserAxisLimit::Drop);
        }
    }
    UserAxisLimits(res)
}
