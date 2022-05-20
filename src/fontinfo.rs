use babelfont::names::StyleMapStyle;
use babelfont::OTScalar;
use fonttools::tables::os2;
use otspec::utils::int_list_to_num;

pub fn ascender(input: &babelfont::Font) -> i16 {
    let upm = input.upm as f32;
    input
        .default_metric("ascender")
        .map_or((upm * 0.80) as i16, |f| f as i16)
}
pub fn typo_linegap(input: &babelfont::Font) -> i16 {
    let upm = input.upm as f32;
    input
        .ot_value("OS2", "sTypoLineGap", true)
        .map(i32::from)
        .unwrap_or((upm * 1.2) as i32 + (-ascender(input) + descender(input)) as i32) as i16
}
pub fn descender(input: &babelfont::Font) -> i16 {
    let upm = input.upm as f32;
    input
        .default_metric("descender")
        .map_or((-upm * 0.20) as i16, |f| f as i16)
}
pub fn hhea_ascender(input: &babelfont::Font) -> i16 {
    input
        .ot_value("hhea", "ascent", true)
        .map_or_else(|| ascender(input), i16::from)
        + typo_linegap(input)
}
pub fn hhea_descender(input: &babelfont::Font) -> i16 {
    input
        .ot_value("hhea", "descent", true)
        .map_or_else(|| descender(input), i16::from)
}
pub fn preferred_family_name(input: &babelfont::Font) -> String {
    input
        .names
        .family_name
        .default()
        .unwrap_or_else(|| "New Font".to_string())
}

pub fn preferred_subfamily_name(input: &babelfont::Font) -> String {
    input
        .names
        .typographic_subfamily
        .default()
        .unwrap_or_else(|| "Regular".to_string())
}

pub fn style_map_family_name(input: &babelfont::Font) -> String {
    if let Some(smfn) = &input.names.style_map_family_name.default() {
        return smfn.to_string();
    }

    let style_name = input.names.typographic_subfamily.default();
    let family_name = input
        .names
        .family_name
        .default()
        .unwrap_or_else(|| "New Font".to_string());
    if style_name.is_none() {
        return family_name;
    }
    let lower = style_name.as_ref().unwrap().to_lowercase();
    match &lower[..] {
        "regular" => family_name,
        "bold" => family_name,
        "italic" => family_name,
        "bold italic" => family_name,
        _ => {
            let mut res = String::new();
            res.push_str(&family_name);
            if !lower.is_empty() {
                res.push(' ');
                res.push_str(&style_name.unwrap());
            }
            res
        }
    }
}

pub fn style_map_style_name(input: &babelfont::Font) -> String {
    match input.names.style_map_style_name {
        Some(StyleMapStyle::BoldItalic) => "bold italic",
        Some(StyleMapStyle::Bold) => "bold",
        Some(StyleMapStyle::Italic) => "italic",
        Some(StyleMapStyle::Regular) => "regular",
        None => {
            let preferred_style_name = preferred_subfamily_name(input);
            match preferred_style_name.to_lowercase().as_str() {
                "bold italic" => "bold italic",
                "bold" => "bold",
                "italic" => "italic",
                _ => "regular",
            }
        }
    }
    .to_string()
}

pub fn postscript_font_name(input: &babelfont::Font) -> String {
    format!(
        "{0}-{1}",
        preferred_family_name(input),
        preferred_subfamily_name(input)
    )
    .replace(" ", "")
    // XXX check postscript characters here
}
pub fn name_version(input: &babelfont::Font) -> String {
    input.names.version.default().as_ref().map_or_else(
        || format!("{0}.{1:03}", input.version.0, input.version.1),
        |x| x.clone(),
    )
}
pub fn unique_id(input: &babelfont::Font) -> String {
    input.names.unique_id.default().as_ref().map_or_else(
        || {
            format!(
                "{0};{1};{2}",
                name_version(input),
                input
                    .ot_value("OS2", "achVendID", true)
                    .map_or("NONE".to_string(), String::from),
                postscript_font_name(input)
            )
        },
        |x| x.clone(),
    )
}

pub fn postscript_underline_thickness(input: &babelfont::Font) -> i16 {
    let upm = input.upm as f32;
    input
        .ot_value("post", "underlineThickness", true)
        .map_or_else(|| upm * 0.05, f32::from) as i16
}

pub fn get_panose(_input: &babelfont::Font) -> os2::Panose {
    // XXX
    os2::Panose {
        panose0: 0,
        panose1: 0,
        panose2: 0,
        panose3: 0,
        panose4: 0,
        panose5: 0,
        panose6: 0,
        panose7: 0,
        panose8: 0,
        panose9: 0,
    }
}
pub fn get_selection(input: &babelfont::Font) -> u16 {
    let mut selection =
        if let Some(OTScalar::BitField(s)) = input.ot_value("OS2", "fsSelection", true) {
            s
        } else {
            vec![]
        };
    let style_map = style_map_style_name(input);
    match style_map.as_str() {
        "regular" => selection.push(6),
        "bold" => selection.push(5),
        "italic" => selection.push(0),
        "bold italic" => {
            selection.push(0);
            selection.push(5);
        }
        _ => {}
    };
    int_list_to_num(&selection) as u16
}

pub fn caret_slope_rise(input: &babelfont::Font) -> i16 {
    let italic_angle = input.default_metric("italic angle").unwrap_or(0) as i32;
    if italic_angle == 0 {
        return 1;
    }
    if let Some(slope_run) = input.ot_value("hhea", "caretSlopeRun", true).map(i16::from) {
        if slope_run > 0 {
            (slope_run as f64 / f64::to_radians(-italic_angle as f64).tan()) as i16
        } else {
            1000
        }
    } else {
        1000
    }
}

pub fn caret_slope_run(input: &babelfont::Font) -> i16 {
    let italic_angle = input.default_metric("italic angle").unwrap_or(0) as i32;
    if italic_angle != 0 {
        let slope_rise = caret_slope_rise(input);
        (slope_rise as f64 * f64::to_radians(-italic_angle as f64).tan()) as i16
    } else {
        0
    }
}

fn bitfield_to_flags(bits: &[u8]) -> u16 {
    let mut out = 0;
    for b in bits {
        out += 1 << b;
    }
    out
}

pub fn head_flags(input: &babelfont::Font) -> u16 {
    let flags: Vec<u8> = input
        .ot_value("head", "flags", true)
        .and_then(|x| x.as_bitfield())
        .unwrap_or_else(|| vec![0, 1]);
    bitfield_to_flags(&flags)
}

pub fn head_mac_style(input: &babelfont::Font) -> u16 {
    match style_map_style_name(input).as_str() {
        "bold" => 1,
        "bold italic" => 3,
        "italic" => 2,
        _ => 0,
    }
}

pub fn os2_fstype(input: &babelfont::Font) -> u16 {
    let flags: Vec<u8> = input
        .ot_value("OS/2", "fsType", true)
        .and_then(|x| x.as_bitfield())
        .unwrap_or_else(|| vec![2]);
    bitfield_to_flags(&flags)
}
