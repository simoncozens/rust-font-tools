use crate::fontinfo::*;
use crate::utils::adjust_offset;
use babelfont::OTScalar;
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
use std::collections::BTreeMap;
use std::convert::TryInto;

pub fn compile_head(font: &babelfont::Font, glyf: &glyf::glyf) -> head {
    let mut minor = font.version.1;
    while minor > 999 {
        minor /= 10;
    }
    let font_revision: f32 = (font.version.0 as f32 * 1000.0 + minor as f32).round() / 1000.0;

    // bounding box
    let bounds: Vec<(i16, i16, i16, i16)> = glyf
        .glyphs
        .iter()
        .map(|x| (x.xMin, x.xMax, x.yMin, x.yMax))
        .collect();
    let mut head_table = head::new(
        font_revision,
        font.upm,
        bounds.iter().map(|x| x.0).min().unwrap_or(0), /* xmin */
        bounds.iter().map(|x| x.2).min().unwrap_or(0), /* ymin */
        bounds.iter().map(|x| x.1).max().unwrap_or(0), /* xmax */
        bounds.iter().map(|x| x.3).max().unwrap_or(0), /* ymax */
    );

    // dates (modified is set to now by default)
    head_table.created = font.date.naive_local();

    // XXX
    // // mac style
    if let Some(lowest_rec_ppm) = font.ot_value("head", "lowestRecPPEM", true) {
        head_table.lowestRecPPEM = u16::from(lowest_rec_ppm);
    }

    // // misc
    // if let Some(flags) = &info.open_type_head_flags {
    //     head_table.flags = int_list_to_num(flags) as u16;
    // }

    head_table
}

pub fn compile_post(font: &babelfont::Font, names: &[String]) -> post {
    let upm = font.upm as f32;
    let default_master = font.default_master();
    post::new(
        2.0,
        *default_master
            .and_then(|x| x.metrics.get("italic angle"))
            .unwrap_or(&0) as f32,
        i16::from(
            font.ot_value("post", "underlinePosition", true)
                .unwrap_or_else(|| OTScalar::Float(upm * -0.075)),
        ),
        postscript_underline_thickness(font),
        font.ot_value("post", "isFixedPitch", true)
            .map(bool::from)
            .unwrap_or(false),
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
    input: &babelfont::Font,
    metrics: &[hmtx::Metric],
    glyf: &glyf::glyf,
) -> hhea::hhea {
    hhea::hhea {
        majorVersion: 1,
        minorVersion: 0,
        ascender: hhea_ascender(input),
        descender: hhea_descender(input),
        lineGap: input
            .ot_value("hhea", "lineGap", true)
            .map(i16::from)
            .unwrap_or(0),
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
        caretOffset: input
            .ot_value("hhea", "caretOffset", true)
            .map(|x| i16::from(x))
            .unwrap_or(0),
        reserved0: 0,
        reserved1: 0,
        reserved2: 0,
        reserved3: 0,
        metricDataFormat: 0,
        numberOfHMetrics: 0,
    }
}

pub fn compile_os2(
    input: &babelfont::Font,
    metrics: &[hmtx::Metric],
    _glyf: &glyf::glyf,
    mapping: &BTreeMap<u32, u16>,
) -> os2 {
    let upm = input.upm as f64;
    let italic_angle = input.default_metric("italic angle").unwrap_or(0) as f64;
    let x_height = input
        .default_metric("sxHeight")
        .unwrap_or((upm * 0.5) as i32);
    let subscript_y_offset = input
        .ot_value("OS2", "sSubscriptYOffset", true)
        .map(i16::from)
        .unwrap_or((upm * 0.075).round() as i16);
    let font_ascender = ascender(input);
    let font_descender = descender(input);
    let s_typo_ascender = input
        .ot_value("OS2", "sTypoAscender", true)
        .map(i16::from)
        .unwrap_or(font_ascender) as i16;
    let s_typo_descender = input
        .ot_value("OS2", "sTypoDescender", true)
        .map(i16::from)
        .unwrap_or_else(|| font_descender) as i16;
    let s_typo_line_gap = input
        .ot_value("OS2", "sTypoLineGap", true)
        .map(i32::from)
        .unwrap_or((upm * 1.2) as i32 + (font_ascender - font_descender) as i32)
        as i16;
    let superscript_y_offset = input
        .ot_value("OS2", "ySuperscriptYOffset", true)
        .map(i16::from)
        .unwrap_or((upm * 0.35).round() as i16);

    let subscript_x_size = input
        .ot_value("OS2", "ySubscriptXSize", true)
        .map_or((upm * 0.65).round() as i16, i16::from);

    let mut table = os2 {
        version: 4,
        xAvgCharWidth: (metrics.iter().map(|m| m.advanceWidth as f32).sum::<f32>()
            / metrics.iter().filter(|m| m.advanceWidth != 0).count() as f32)
            .round() as i16,
        usWeightClass: input
            .ot_value("OS2", "usWeightClass", true)
            .map_or(400, i16::from) as u16,
        usWidthClass: input
            .ot_value("OS2", "usWidthClass", true)
            .map_or(5, i16::from) as u16,
        // XXX OS2 fsType
        fsType: int_list_to_num(&[2]) as u16,
        ySubscriptXSize: subscript_x_size,
        ySubscriptYSize: input
            .ot_value("OS2", "ySubscriptYSize", true)
            .map_or((upm * 0.6).round() as i16, i16::from),
        ySubscriptYOffset: subscript_y_offset,
        ySubscriptXOffset: input
            .ot_value("OS2", "ySubscriptXOffset", true)
            .map_or_else(
                || adjust_offset(-subscript_y_offset, italic_angle),
                i32::from,
            ) as i16,

        ySuperscriptXSize: input
            .ot_value("OS2", "ySuperscriptXSize", true)
            .map_or((upm * 0.65).round() as i16, i16::from),
        ySuperscriptYSize: input
            .ot_value("OS2", "ySuperscriptYSize", true)
            .map_or((upm * 0.6).round() as i16, i16::from),
        ySuperscriptYOffset: superscript_y_offset,
        ySuperscriptXOffset: input
            .ot_value("OS2", "ySuperscriptXOffset", true)
            .map_or_else(
                || adjust_offset(-superscript_y_offset, italic_angle),
                i32::from,
            ) as i16,

        yStrikeoutSize: input
            .ot_value("OS2", "yStrikeoutSize", true)
            .map_or_else(|| postscript_underline_thickness(input).into(), i32::from)
            as i16,
        yStrikeoutPosition: input
            .ot_value("OS2", "yStrikeoutPosition", true)
            .map_or((x_height as f32 * 0.22) as i16, i16::from),

        sxHeight: Some(x_height as i16),
        achVendID: input
            .ot_value("OS2", "achVendID", true)
            .map_or(*b"NONE", |x| String::from(x).as_bytes().try_into().unwrap()),
        sCapHeight: Some(
            input
                .default_metric("cap height")
                .unwrap_or((upm * 0.7) as i32) as i16,
        ),
        sTypoAscender: s_typo_ascender,
        sTypoDescender: s_typo_descender,
        sTypoLineGap: s_typo_line_gap,
        usWinAscent: input
            .ot_value("OS2", "usWinAscent", true)
            .map_or(font_ascender + s_typo_line_gap, i16::from) as u16,
        usWinDescent: input
            .ot_value("OS2", "usWinDescent", true)
            .map_or(font_descender.abs(), i16::from) as u16,
        usBreakChar: Some(32),
        usMaxContext: Some(0),
        usDefaultChar: Some(0),
        // sFamilyClass: input.open_type_os2_family_class... (not public)
        sFamilyClass: 0,
        panose: get_panose(input),
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
        fsSelection: get_selection(input),
    };
    if let Some(OTScalar::BitField(page_ranges)) = input.ot_value("OS2", "codePageRanges", true) {
        table.int_list_to_code_page_ranges(&page_ranges);
    } else {
        table.calc_code_page_ranges(&mapping);
    }
    table
}

pub fn compile_name(input: &babelfont::Font) -> name {
    let mut name = name { records: vec![] };
    /* Ideally...
    if let Some(records) = &input.open_type_name_records {
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
    if let Some(copyright) = &input.names.copyright.default() {
        records.push((NameRecordID::Copyright, copyright.to_string()));
    }

    let family_name = style_map_family_name(input);
    let style_name = style_map_style_name(input);
    let pfn = preferred_family_name(input);
    let psfn = preferred_subfamily_name(input);
    records.extend(vec![
        (NameRecordID::FontFamilyName, family_name.clone()),
        (NameRecordID::FontSubfamilyName, style_name.clone()),
        (NameRecordID::UniqueID, unique_id(input)),
        (NameRecordID::FullFontName, format!("{0} {1}", pfn, psfn)),
        (NameRecordID::Version, name_version(input)),
        (NameRecordID::PostscriptName, postscript_font_name(input)),
    ]);
    for (id, field) in &[
        (NameRecordID::Trademark, &input.names.trademark),
        (NameRecordID::Manufacturer, &input.names.manufacturer),
        (NameRecordID::Designer, &input.names.designer),
        (NameRecordID::Description, &input.names.description),
        (NameRecordID::ManufacturerURL, &input.names.manufacturer_url),
        (NameRecordID::DesignerURL, &input.names.designer_url),
        (NameRecordID::License, &input.names.license),
        (NameRecordID::LicenseURL, &input.names.license_url),
    ] {
        if let Some(value) = field.default() {
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
            &input.names.compatible_full_name,
        ),
        (NameRecordID::SampleText, &input.names.sample_text),
        (NameRecordID::WWSFamilyName, &input.names.w_w_s_family_name),
        (
            NameRecordID::WWSSubfamilyName,
            &input.names.w_w_s_subfamily_name,
        ),
    ] {
        if let Some(value) = field.default() {
            records.push((*id, value.to_string()));
        }
    }
    for (id, string) in records {
        name.records.push(NameRecord::windows_unicode(id, string));
    }

    name
}

pub fn fill_tables(
    input: &babelfont::Font,
    glyf_table: glyf::glyf,
    metrics: Vec<hmtx::Metric>,
    names: Vec<String>,
    mapping: BTreeMap<u32, u16>,
) -> Font {
    let mut font = Font::new(font::SfntVersion::TrueType);
    let head_table = compile_head(input, &glyf_table);
    let post_table = compile_post(input, &names);
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
    let os2_table = compile_os2(input, &metrics, &glyf_table, &mapping);
    let cmap_table = compile_cmap(mapping);
    let name_table = compile_name(input);
    let mut hhea_table = compile_hhea(input, &metrics, &glyf_table);
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
