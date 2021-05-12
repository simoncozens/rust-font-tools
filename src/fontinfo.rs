use crate::utils::int_list_to_num;
use norad::fontinfo::StyleMapStyle;
use std::collections::HashSet;

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
    let mut selection = info.open_type_os2_selection.clone().unwrap_or(vec![]);
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

/// implementation based on ufo2ft:
/// https://github.com/googlefonts/ufo2ft/blob/main/lib/ufo2ft/util.py#l307
pub fn calc_code_page_ranges(unicodes: &HashSet<u32>) -> Vec<u8> {
    let mut code_page_ranges = HashSet::new();

    let ascii_range: HashSet<u32> = (0x20..0x7E).collect();
    let has_ascii = ascii_range.is_subset(&unicodes);
    let has_lineart = unicodes.contains(&0x2524); // contains '┤'

    // Don't loop through each char in the ufo implementation. Let's just
    // check if each char exists in the unicode hashset. Drops runtime from
    // O(n) to O(1)
    let unicodes_contains = | char | {
        unicodes.contains(&(char as u32))
    };
    if unicodes_contains('Þ') && has_ascii {
        code_page_ranges.insert(0); // Latin 1
    }
    if unicodes_contains('Ľ') && has_ascii {
        code_page_ranges.insert(1); // Latin 2
    }
    if unicodes_contains('Б') {
        code_page_ranges.insert(2); // Cyrillic
        if unicodes_contains('Ѕ') && has_lineart {
            code_page_ranges.insert(57); // IBM Cyrillic
        }
        if unicodes_contains('╜') && has_lineart {
            code_page_ranges.insert(49); // MS-DOS Russian
        }
    }
    if unicodes_contains('Ά') {
        code_page_ranges.insert(3); // Greek
        if unicodes_contains('½') && has_lineart {
            code_page_ranges.insert(48); // IBM Greek
        }
        if unicodes_contains('√') && has_lineart {
            code_page_ranges.insert(60); // Greek, former 437 G
        }
    }
    if unicodes_contains('İ') && has_ascii {
        code_page_ranges.insert(4);  //  Turkish
        if has_lineart {
            code_page_ranges.insert(56);  //  IBM turkish
        }
    }
    if unicodes_contains('א') {
        code_page_ranges.insert(5);  //  Hebrew
        if has_lineart && unicodes_contains('√') {
            code_page_ranges.insert(53);  //  Hebrew
        }
    }
    if unicodes_contains('ر') {
        code_page_ranges.insert(6);  //  Arabic
        if unicodes_contains('√') {
            code_page_ranges.insert(51);  //  Arabic
        }
        if has_lineart {
            code_page_ranges.insert(61);  //  Arabic; ASMO 708
        }
    }
    if unicodes_contains('ŗ') && has_ascii {
        code_page_ranges.insert(7);  //  Windows Baltic
        if has_lineart {
            code_page_ranges.insert(59);  //  MS-DOS Baltic
        }
    }
    if unicodes_contains('₫') && has_ascii {
        code_page_ranges.insert(8);  //  Vietnamese
    }
    if unicodes_contains('ๅ') {
        code_page_ranges.insert(16);  //  Thai
    }
    if unicodes_contains('エ') {
        code_page_ranges.insert(17);  //  JIS/Japan
    }
    if unicodes_contains('ㄅ') {
        code_page_ranges.insert(18);  //  Chinese: Simplified chars
    }
    if unicodes_contains('ㄱ') {
        code_page_ranges.insert(19);  //  Korean wansung
    }
    if unicodes_contains('央') {
        code_page_ranges.insert(20);  //  Chinese: Traditional chars
    }
    if unicodes_contains('곴') {
        code_page_ranges.insert(21);  //  Korean Johab
    }
    if unicodes_contains('♥') && has_ascii {
        code_page_ranges.insert(30);  //  OEM Character Set
    //  TODO: Symbol bit has a special meaning (check the spec), we need
    //  to confirm if this is wanted by default.
    //  elif chr(0xF000) <= char <= chr(0xF0FF):
    //     code_page_ranges.insert(31)          //  Symbol Character Set
    }
    if unicodes_contains('þ') && has_ascii && has_lineart {
        code_page_ranges.insert(54);  //  MS-DOS Icelandic
    }
    if unicodes_contains('╚') && has_ascii {
        code_page_ranges.insert(62);  //  WE/Latin 1
        code_page_ranges.insert(63);  //  US
    }
    if has_ascii && has_lineart && unicodes_contains('√') {
        if unicodes_contains('Å') {
            code_page_ranges.insert(50);  //  MS-DOS Nordic
        }
        if unicodes_contains('é') {
            code_page_ranges.insert(52);  //  MS-DOS Canadian French
        }
        if unicodes_contains('õ') {
            code_page_ranges.insert(55);  //  MS-DOS Portuguese
        }
    }
    if has_ascii && unicodes_contains('‰') && unicodes_contains('∑') {
        code_page_ranges.insert(29); // Macintosh Character Set (US Roman)
    }
    // when no codepage ranges can be enabled, fall back to enabling bit 0
    // (Latin 1) so that the font works in MS Word:
    // https://github.com/googlei18n/fontmake/issues/468
    if code_page_ranges.is_empty() {
        code_page_ranges.insert(0);
    }
    code_page_ranges.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_calc_code_page_ranges() {
        let unicodes: HashSet<u32> = (0x20..0xFFFF).collect();
        let ranges = calc_code_page_ranges(&unicodes);
        println!("{:?}", ranges);
    }
}