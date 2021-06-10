use fonttools::utils::int_list_to_num;
use norad::fontinfo::StyleMapStyle;

pub fn ascender(info: &norad::FontInfo) -> i16 {
    let upm = info.units_per_em.map_or(1000.0, |f| f.get()) as f64;
    info.ascender
        .map_or((upm * 0.80) as i16, |f| f.get() as i16)
}
pub fn descender(info: &norad::FontInfo) -> i16 {
    let upm = info.units_per_em.map_or(1000.0, |f| f.get()) as f64;
    info.descender
        .map_or((-upm * 0.20) as i16, |f| f.get() as i16)
}
pub fn hhea_ascender(info: &norad::FontInfo) -> i16 {
    info.open_type_hhea_ascender
        .map_or_else(|| ascender(info), |x| x as i16)
}
pub fn hhea_descender(info: &norad::FontInfo) -> i16 {
    info.open_type_hhea_descender
        .map_or_else(|| descender(info), |x| x as i16)
}
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
    match info.style_map_style_name {
        Some(StyleMapStyle::BoldItalic) => "bold italic",
        Some(StyleMapStyle::Bold) => "bold",
        Some(StyleMapStyle::Italic) => "italic",
        Some(StyleMapStyle::Regular) => "regular",
        None => {
            let preferred_style_name = preferred_subfamily_name(&info);
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
pub fn postscript_underline_thickness(info: &norad::FontInfo) -> i16 {
    let upm = info.units_per_em.map_or(1000.0, |f| f.get()) as f64;
    info.postscript_underline_thickness
        .map_or_else(|| upm * 0.05, |f| f.get()) as i16
}
pub fn get_panose(_info: &norad::FontInfo) -> fonttools::os2::Panose {
    // Struct not public, unfortunately.
    fonttools::os2::Panose {
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
pub fn get_selection(info: &norad::FontInfo) -> u16 {
    let mut selection = info.open_type_os2_selection.clone().unwrap_or_default();
    let style_map = style_map_style_name(info);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_calc_code_page_ranges() {
        let unicodes: HashSet<u32> = (0x20..0xFFFF).collect();
        let ranges = calc_code_page_ranges(&unicodes);
        assert_eq!(ranges.iter().count(), 32);
    }
}
