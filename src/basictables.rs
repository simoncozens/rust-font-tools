use crate::fontinfo::*;
use crate::utils::adjust_offset;
use fonttools::cmap;
use fonttools::font;
use fonttools::font::Font;
use fonttools::font::Table;
use fonttools::glyf;
use fonttools::head::head;
use fonttools::hhea;
use fonttools::hmtx;
use fonttools::maxp::maxp;
use fonttools::name::{name, NameRecord, NameRecordID};
use fonttools::os2::os2;
use fonttools::post::post;
use fonttools::utils::int_list_to_num;
use otspec::types::Tuple;
use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;

pub fn compile_head(info: &norad::FontInfo, glyf: &glyf::glyf) -> head {
    let mut minor = info.version_minor.unwrap_or(0);
    while minor > 999 {
        minor /= 10;
    }
    let font_revision: f32 =
        (info.version_major.unwrap_or(1) as f32 * 1000.0 + minor as f32).round() / 1000.0;

    // bounding box
    let bounds: Vec<(i16, i16, i16, i16)> = glyf
        .glyphs
        .iter()
        .map(|x| (x.xMin, x.xMax, x.yMin, x.yMax))
        .collect();
    let mut head_table = head::new(
        font_revision,
        info.units_per_em.map_or(1000, |f| f.get() as u16),
        bounds.iter().map(|x| x.0).min().unwrap_or(0), /* xmin */
        bounds.iter().map(|x| x.2).min().unwrap_or(0), /* ymin */
        bounds.iter().map(|x| x.1).max().unwrap_or(0), /* xmax */
        bounds.iter().map(|x| x.3).max().unwrap_or(0), /* ymax */
    );

    // dates (modified is set to now by default)
    if info.open_type_head_created.is_some() {
        if let Ok(date) = chrono::NaiveDateTime::parse_from_str(
            &info.open_type_head_created.as_ref().unwrap(),
            "%Y/%m/%d %H:%M:%S",
        ) {
            head_table.created = date
        } else {
            log::warn!(
                "Couldn't parse created date {:?}",
                info.open_type_head_created
            )
        }
    }

    // mac style
    if let Some(lowest_rec_ppm) = info.open_type_head_lowest_rec_ppem {
        head_table.lowestRecPPEM = lowest_rec_ppm as u16;
    }

    // misc
    if let Some(flags) = &info.open_type_head_flags {
        head_table.flags = int_list_to_num(flags) as u16;
    }
    head_table
}

pub fn compile_post(info: &norad::FontInfo, names: &[String]) -> post {
    let upm = info.units_per_em.map_or(1000.0, |f| f.get());
    post::new(
        2.0,
        info.italic_angle.map_or(0.0, |f| f.get() as f32),
        info.postscript_underline_position
            .map_or_else(|| upm * -0.075, |f| f.get()) as i16,
        postscript_underline_thickness(info),
        info.postscript_is_fixed_pitch.unwrap_or(false),
        Some(names.to_vec()),
    )
}

pub fn compile_cmap(mapping: BTreeMap<u32, u16>) -> cmap::cmap {
    cmap::cmap {
        subtables: vec![
            cmap::CmapSubtable {
                format: 4,
                platformID: 0,
                encodingID: 3,
                languageID: 0,
                mapping: mapping.clone(),
            },
            cmap::CmapSubtable {
                format: 4,
                platformID: 3,
                encodingID: 1,
                languageID: 0,
                mapping,
            },
        ],
    }
}

pub fn compile_hhea(
    info: &norad::FontInfo,
    metrics: &[hmtx::Metric],
    glyf: &glyf::glyf,
) -> hhea::hhea {
    hhea::hhea {
        majorVersion: 1,
        minorVersion: 0,
        ascender: hhea_ascender(info),
        descender: hhea_descender(info),
        lineGap: info.open_type_hhea_line_gap.unwrap_or(0) as i16,
        advanceWidthMax: metrics.iter().map(|x| x.advanceWidth).max().unwrap_or(0),
        minLeftSideBearing: metrics.iter().map(|x| x.lsb).min().unwrap_or(0),
        minRightSideBearing: metrics
            .iter()
            .map(|x| x.advanceWidth as i16)
            .zip(glyf.glyphs.iter().map(|g| g.xMax))
            .map(|t| t.0 - t.1)
            .min()
            .unwrap_or(0),
        xMaxExtent: glyf.glyphs.iter().map(|g| g.xMax).max().unwrap_or(0),
        caretSlopeRise: 1, // XXX
        caretSlopeRun: 0,  // XXX
        caretOffset: info.open_type_hhea_caret_offset.unwrap_or(0) as i16,
        reserved0: 0,
        reserved1: 0,
        reserved2: 0,
        reserved3: 0,
        metricDataFormat: 0,
        numberOfHMetrics: 0,
    }
}

pub fn compile_os2(
    info: &norad::FontInfo,
    metrics: &[hmtx::Metric],
    glyf: &glyf::glyf,
    mapping: &BTreeMap<u32, u16>,
) -> os2 {
    let upm = info.units_per_em.map_or(1000.0, |f| f.get());
    let italic_angle = info.italic_angle.map_or(0.0, |f| f.get());
    let xHeight = info.x_height.map_or(upm * 0.5, |f| f.get());
    let subscript_y_offset = info
        .open_type_os2_subscript_y_offset
        .unwrap_or((upm * 0.075).round() as i32) as i16;
    let font_ascender = ascender(info);
    let font_descender = descender(info);
    let sTypoAscender = info
        .open_type_os2_typo_ascender
        .unwrap_or(font_ascender.into()) as i16;
    let sTypoDescender = info
        .open_type_os2_typo_descender
        .unwrap_or(font_descender.into()) as i16;
    let sTypoLineGap =
        info.open_type_hhea_line_gap
            .unwrap_or((upm * 1.2) as i32 + (font_ascender - font_descender) as i32) as i16;
    let superscript_y_offset = info
        .open_type_os2_superscript_y_offset
        .unwrap_or((upm * 0.35).round() as i32) as i16;

    let subscript_x_size = info
        .open_type_os2_subscript_x_size
        .unwrap_or((upm * 0.65).round() as i32) as i16;

    let mut table = os2 {
        version: 4,
        xAvgCharWidth: (metrics.iter().map(|m| m.advanceWidth as f32).sum::<f32>()
            / metrics.iter().filter(|m| m.advanceWidth != 0).count() as f32)
            .round() as i16,
        usWeightClass: info.open_type_os2_weight_class.unwrap_or(400) as u16,
        usWidthClass: info.open_type_os2_width_class.map_or(5, |f| f as u16),
        fsType: int_list_to_num(&info.open_type_os2_type.as_ref().unwrap_or(&vec![2])) as u16,
        ySubscriptXSize: subscript_x_size,
        ySubscriptYSize: info
            .open_type_os2_subscript_y_size
            .unwrap_or((upm * 0.6).round() as i32) as i16,
        ySubscriptYOffset: subscript_y_offset,
        ySubscriptXOffset: info
            .open_type_os2_subscript_x_offset
            .unwrap_or(adjust_offset(-subscript_y_offset, italic_angle))
            as i16,

        ySuperscriptXSize: info
            .open_type_os2_superscript_x_size
            .unwrap_or((upm * 0.65).round() as i32) as i16,
        ySuperscriptYSize: info
            .open_type_os2_superscript_y_size
            .unwrap_or((upm * 0.6).round() as i32) as i16,
        ySuperscriptYOffset: superscript_y_offset,
        ySuperscriptXOffset: info
            .open_type_os2_superscript_x_offset
            .unwrap_or(adjust_offset(-superscript_y_offset, italic_angle))
            as i16,

        yStrikeoutSize: info
            .open_type_os2_strikeout_size
            .unwrap_or(postscript_underline_thickness(info).into()) as i16,
        yStrikeoutPosition: info
            .open_type_os2_strikeout_position
            .unwrap_or((xHeight * 0.22) as i32) as i16,

        sxHeight: Some(xHeight as i16),
        achVendID: info
            .open_type_os2_vendor_id
            .as_ref()
            .map_or(*b"NONE", |x| x.as_bytes().try_into().unwrap()),
        sCapHeight: Some(info.cap_height.map_or(upm * 0.7, |f| f.get()) as i16),
        sTypoAscender,
        sTypoDescender,
        sTypoLineGap,
        usWinAscent: info
            .open_type_os2_win_ascent
            .unwrap_or((font_ascender + sTypoLineGap).try_into().unwrap())
            as u16,
        usWinDescent: info
            .open_type_os2_win_descent
            .unwrap_or(font_descender.abs() as u32) as u16,
        usBreakChar: Some(32),
        usMaxContext: Some(0),
        usDefaultChar: Some(0),
        // sFamilyClass: info.open_type_os2_family_class... (not public)
        sFamilyClass: 0,
        panose: get_panose(info),
        ulCodePageRange1: Some(0),
        ulCodePageRange2: Some(0),
        ulUnicodeRange1: 0b10100001000000000000000011111111, // XXX
        ulUnicodeRange2: 0,                                  // XXX
        ulUnicodeRange3: 0,                                  // XXX
        ulUnicodeRange4: 0,                                  // XXX
        usFirstCharIndex: *mapping.keys().min().unwrap_or(&0xFFFF) as u16,
        usLastCharIndex: *mapping.keys().max().unwrap_or(&0xFFFF) as u16,
        usLowerOpticalPointSize: None,
        usUpperOpticalPointSize: None,
        fsSelection: get_selection(info),
    };
    if let Some(page_ranges) = info.open_type_os2_code_page_ranges.as_ref() {
        table.int_list_to_code_page_ranges(page_ranges);
    } else {
        table.calc_code_page_ranges(&mapping);
    }
    table
}

pub fn compile_name(info: &norad::FontInfo) -> name {
    let mut name = name { records: vec![] };
    /* Ideally...
    if let Some(records) = &info.open_type_name_records {
        for record in records {
            name.records.push(NameRecord {
                nameID: record.name_id as u16,
                platformID: record.platform_id as u16,
                encodingID: record.encoding_id as u16,
                languageID: record.language_id as u16,
                string: record.string,
            })
        }
    }
    */

    let mut records: Vec<(NameRecordID, String)> = vec![];
    if let Some(copyright) = &info.copyright {
        records.push((NameRecordID::Copyright, copyright.to_string()));
    }

    let family_name = style_map_family_name(info);
    let style_name = style_map_style_name(info);
    let pfn = preferred_family_name(info);
    let psfn = preferred_subfamily_name(info);
    records.extend(vec![
        (NameRecordID::FontFamilyName, family_name.clone()),
        (NameRecordID::FontSubfamilyName, style_name.clone()),
        (NameRecordID::UniqueID, unique_id(info)),
        (NameRecordID::FullFontName, format!("{0} {1}", pfn, psfn)),
        (NameRecordID::Version, name_version(info)),
        (NameRecordID::PostscriptName, postscript_font_name(info)),
    ]);
    for (id, field) in &[
        (NameRecordID::Trademark, &info.trademark),
        (
            NameRecordID::Manufacturer,
            &info.open_type_name_manufacturer,
        ),
        (NameRecordID::Designer, &info.open_type_name_designer),
        (NameRecordID::Description, &info.open_type_name_description),
        (
            NameRecordID::ManufacturerURL,
            &info.open_type_name_manufacturer_url,
        ),
        (NameRecordID::DesignerURL, &info.open_type_name_designer_url),
        (NameRecordID::License, &info.open_type_name_license),
        (NameRecordID::LicenseURL, &info.open_type_name_license_url),
    ] {
        if let Some(value) = field {
            records.push((*id, value.to_string()));
        }
    }

    if pfn != family_name {
        records.push((NameRecordID::PreferredFamilyName, pfn));
    }
    if psfn != style_name {
        records.push((NameRecordID::PreferredSubfamilyName, psfn));
    }

    for (id, field) in &[
        (
            NameRecordID::CompatibleFullName,
            &info.open_type_name_compatible_full_name,
        ),
        (NameRecordID::SampleText, &info.open_type_name_sample_text),
        (
            NameRecordID::WWSFamilyName,
            &info.open_type_name_wws_family_name,
        ),
        (
            NameRecordID::WWSSubfamilyName,
            &info.open_type_name_wws_subfamily_name,
        ),
    ] {
        if let Some(value) = field {
            records.push((*id, value.to_string()));
        }
    }
    for (id, string) in records {
        name.records.push(NameRecord::windows_unicode(id, string));
    }

    name
}

pub fn fill_tables(
    info: &norad::FontInfo,
    glyf_table: glyf::glyf,
    metrics: Vec<hmtx::Metric>,
    names: Vec<String>,
    mapping: BTreeMap<u32, u16>,
) -> Font {
    let mut font = Font::new(font::SfntVersion::TrueType);
    let head_table = compile_head(info, &glyf_table);
    let post_table = compile_post(info, &names);
    let (
        num_glyphs,
        max_points,
        max_contours,
        max_composite_points,
        max_composite_contours,
        max_component_elements,
        max_component_depth,
    ) = glyf_table.maxp_statistics();
    let maxp_table = maxp::new10(
        num_glyphs,
        max_points,
        max_contours,
        max_composite_points,
        max_composite_contours,
        max_component_elements,
        max_component_depth,
    );
    let os2_table = compile_os2(info, &metrics, &glyf_table, &mapping);
    let cmap_table = compile_cmap(mapping);
    let name_table = compile_name(info);
    let mut hhea_table = compile_hhea(info, &metrics, &glyf_table);
    let hmtx_table = hmtx::hmtx { metrics };
    let (hmtx_bytes, num_h_metrics) = hmtx_table.to_bytes();
    hhea_table.numberOfHMetrics = num_h_metrics;

    font.tables.insert(*b"head", Table::Head(head_table));
    font.tables.insert(*b"hhea", Table::Hhea(hhea_table));
    font.tables.insert(*b"maxp", Table::Maxp(maxp_table));
    font.tables.insert(*b"OS/2", Table::Os2(os2_table));
    font.tables.insert(*b"hmtx", Table::Unknown(hmtx_bytes));
    font.tables.insert(*b"cmap", Table::Cmap(cmap_table));
    font.tables.insert(*b"loca", Table::Unknown(vec![0]));
    font.tables.insert(*b"glyf", Table::Glyf(glyf_table));
    font.tables.insert(*b"name", Table::Name(name_table));
    font.tables.insert(*b"post", Table::Post(post_table));

    font
}
