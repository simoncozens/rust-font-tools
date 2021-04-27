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

/* FontInfo things for ufo2ttf */
pub mod font_info_data {
    pub fn preferred_family_name(info: &norad::FontInfo) -> String {
        info.open_type_name_preferred_family_name
            .as_ref()
            .or_else(|| info.family_name.as_ref())
            .map_or("New Font".to_string(), |x| x.to_string())
    }

    pub fn preferred_subfamily_name(info: &norad::FontInfo) -> String {
        info.open_type_name_preferred_subfamily_name
            .as_ref()
            .or_else(|| info.style_name.as_ref())
            .map_or("Regular".to_string(), |x| x.to_string())
    }

    pub fn style_map_family_name(info: &norad::FontInfo) -> String {
        if let Some(smfn) = &info.style_map_family_name {
            return smfn.to_string();
        }

        let style_name = info
            .style_name
            .as_ref()
            .or_else(|| info.open_type_name_preferred_subfamily_name.as_ref());
        let family_name = preferred_family_name(&info);
        if style_name.is_none() {
            return family_name;
        }
        let lower = style_name.unwrap().to_lowercase();
        match &lower[..] {
            "regular" => family_name,
            "bold" => family_name,
            "italic" => family_name,
            "bold italic" => family_name,
            _ => {
                let mut res = String::new();
                res.push_str(&family_name);
                if !lower.is_empty() {
                    res.push_str(&" ".to_string());
                    res.push_str(style_name.unwrap());
                }
                res
            }
        }
    }

    pub fn style_map_style_name(info: &norad::FontInfo) -> String {
        match info
            .style_map_style_name
            .as_ref()
            .map_or(1, |x| x.clone() as u16) // Tricks we have to pull to use private fields
        {
            2 => "bold",
            3 => "italic",
            4 => "bold italic",
            _ => "regular",
        }
        .to_string()
    }

    pub fn postscript_font_name(info: &norad::FontInfo) -> String {
        format!(
            "{0}-{1}",
            preferred_family_name(info),
            preferred_subfamily_name(info)
        )
        // XXX check postscript characters here
    }
    pub fn name_version(info: &norad::FontInfo) -> String {
        info.open_type_name_version.as_ref().map_or_else(
            {
                || {
                    format!(
                        "Version {0}.{1:03}",
                        info.version_major.unwrap_or(0),
                        info.version_minor.unwrap_or(0)
                    )
                }
            },
            |x| x.clone(),
        )
    }
    pub fn unique_id(info: &norad::FontInfo) -> String {
        info.open_type_name_unique_id.as_ref().map_or_else(
            || {
                format!(
                    "{0};{1};{2}",
                    name_version(info),
                    info.open_type_os2_vendor_id.as_ref().map_or("NONE", |x| x),
                    postscript_font_name(info)
                )
            },
            |x| x.clone(),
        )
    }
}
