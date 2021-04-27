use clap::{App, Arg};
use fonttools::cmap;
use fonttools::font;
use fonttools::font::Table;
use fonttools::glyf;
use fonttools::head::head;
use fonttools::hhea;
use fonttools::hmtx;
use fonttools::maxp::maxp;
use fonttools::name::{name, NameRecord, NameRecordID};
use fonttools::post::post;
use fonttools_cli::font_info_data::*;
use lyon::geom::cubic_bezier::CubicBezierSegment;
use lyon::geom::euclid::TypedPoint2D;
use lyon::path::geom::cubic_to_quadratic::cubic_to_quadratics;
use norad::Font as Ufo;
use norad::PointType;
use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::marker::PhantomData;

type LyonPoint = TypedPoint2D<f32, lyon::geom::euclid::UnknownUnit>;

fn int_list_to_num(int_list: &[u8]) -> u32 {
    let mut flags = 0;
    for flag in int_list {
        flags += 1 << (flag + 1);
    }
    flags
}

fn compile_head(info: &norad::FontInfo, glyphs: &[Option<glyf::Glyph>]) -> head {
    let mut minor = info.version_minor.unwrap_or(0);
    while minor > 999 {
        minor /= 10;
    }
    let font_revision: f32 =
        (info.version_major.unwrap_or(1) as f32 * 1000.0 + minor as f32).round() / 1000.0;

    // bounding box
    let bounds: Vec<(i16, i16, i16, i16)> = glyphs
        .iter()
        .filter_map(|x| x.as_ref())
        .map(|x| (x.xMin, x.xMax, x.yMin, x.yMax))
        .collect();
    let mut head_table = head::new(
        font_revision,
        info.units_per_em.map_or(1000, |f| f.get() as u16),
        bounds.iter().map(|x| x.0).min().unwrap_or(0), /* xmin */
        bounds.iter().map(|x| x.1).max().unwrap_or(0), /* xmax */
        bounds.iter().map(|x| x.2).min().unwrap_or(0), /* ymin */
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

fn compile_post(info: &norad::FontInfo, names: &[String]) -> post {
    let upm = info.units_per_em.map_or(1000.0, |f| f.get());
    post::new(
        2.0,
        info.italic_angle.map_or(0.0, |f| f.get() as f32),
        info.postscript_underline_position
            .map_or_else(|| upm * -0.075, |f| f.get()) as i16,
        info.postscript_underline_thickness
            .map_or_else(|| upm * 0.05, |f| f.get()) as i16,
        info.postscript_is_fixed_pitch.unwrap_or(false),
        Some(names.to_vec()),
    )
}

fn compile_cmap(mapping: BTreeMap<u32, u16>) -> cmap::cmap {
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

fn compile_name(info: &norad::FontInfo) -> name {
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

fn glif_to_glyph(glif: &norad::Glyph, mapping: &BTreeMap<String, u16>) -> Option<glyf::Glyph> {
    let mut glyph = glyf::Glyph {
        xMin: 0,
        xMax: 0,
        yMin: 0,
        yMax: 0,
        contours: None,
        instructions: None,
        components: None,
        overlap: false,
    };
    if glif.components.is_empty() && glif.contours.is_empty() {
        return None;
    }

    if !glif.components.is_empty() && !glif.contours.is_empty() {
        // println!("Mixed glyph needs decomposition {:?}", glif.name);
        return Some(glyph);
    }

    /* Do components */
    let mut components: Vec<glyf::Component> = vec![];
    for component in &glif.components {
        if let Some(glyf_component) = norad_component_to_glyf_component(component, mapping) {
            components.push(glyf_component);
        }
    }
    if !components.is_empty() {
        glyph.components = Some(components);
    }

    /* Do outlines */
    let mut contours: Vec<Vec<glyf::Point>> = vec![];
    for contour in &glif.contours {
        if let Some(glyf_contour) = norad_contour_to_glyf_contour(contour) {
            contours.push(glyf_contour);
        }
    }
    if !contours.is_empty() {
        glyph.contours = Some(contours);
        glyph.recalc_bounds();
    }
    Some(glyph)
}

fn norad_contour_to_glyf_contour(contour: &norad::Contour) -> Option<Vec<glyf::Point>> {
    let cp = &contour.points;
    let mut points: Vec<glyf::Point> = vec![glyf::Point {
        x: cp[0].x as i16,
        y: cp[0].y as i16,
        on_curve: true, // I think?
    }];
    let mut i = 0;
    while i < cp.len() - 1 {
        i += 1;
        if cp[i].typ != PointType::OffCurve {
            points.push(glyf::Point {
                x: cp[i].x as i16,
                y: cp[i].y as i16,
                on_curve: true,
            });
            continue;
        } else {
            // Gonna assume cubic...
            points.pop(); // Drop last point, we'll add it
            let before_pt = &cp[i - 1];
            let this_pt = &cp[i];
            let next_handle = &cp[(i + 1) % cp.len()];
            let to_pt = &cp[(i + 2) % cp.len()];
            let seg = CubicBezierSegment {
                from: LyonPoint {
                    x: before_pt.x,
                    y: before_pt.y,
                    _unit: PhantomData,
                },
                ctrl1: LyonPoint {
                    x: this_pt.x,
                    y: this_pt.y,
                    _unit: PhantomData,
                },
                ctrl2: LyonPoint {
                    x: next_handle.x,
                    y: next_handle.y,
                    _unit: PhantomData,
                },
                to: LyonPoint {
                    x: to_pt.x,
                    y: to_pt.y,
                    _unit: PhantomData,
                },
            };
            cubic_to_quadratics(&seg, 1.0, &mut |quad| {
                points.push(glyf::Point {
                    x: quad.from.x as i16,
                    y: quad.from.y as i16,
                    on_curve: true,
                });
                points.push(glyf::Point {
                    x: quad.ctrl.x as i16,
                    y: quad.ctrl.y as i16,
                    on_curve: false,
                });
                points.push(glyf::Point {
                    x: quad.to.x as i16,
                    y: quad.to.y as i16,
                    on_curve: true,
                });
            });
            i += 2;
        }
    }
    Some(points)
}

fn norad_component_to_glyf_component(
    component: &norad::Component,
    mapping: &BTreeMap<String, u16>,
) -> Option<glyf::Component> {
    let maybe_id = mapping.get(&component.base.to_string());

    if maybe_id.is_none() {
        println!("Couldn't find component for {:?}", component.base);
        return None;
    }

    Some(glyf::Component {
        glyphIndex: *maybe_id.unwrap(),
        matchPoints: None,
        flags: glyf::ComponentFlags::empty(),
        transformation: component.transform.into(),
    })
}
fn main() {
    let matches = App::new("ufo2ttf")
        .about("Build TTF files from UFO")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Sets the output file to use")
                .required(false),
        )
        .get_matches();
    let filename = matches.value_of("INPUT").unwrap();
    let ufo = Ufo::load(filename).expect("failed to load font");
    let mut font = font::Font::new(font::SfntVersion::TrueType);

    let layer = ufo.default_layer();
    let info = ufo.font_info.as_ref().unwrap();

    let mut names: Vec<String> = vec![];
    let mut metrics: Vec<hmtx::Metric> = vec![];
    let mut glyph_id = 0;
    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();
    let mut glyphs: Vec<Option<glyf::Glyph>> = vec![];

    for glyf in layer.iter_contents() {
        let name = glyf.name.to_string();
        names.push(name.clone());
        name_to_id.insert(name, glyph_id);
        let cp = &glyf.codepoints;
        if !cp.is_empty() {
            mapping.insert(cp[0] as u32, glyph_id);
        }
        glyph_id += 1;
    }
    for glyf in layer.iter_contents() {
        let glyph = glif_to_glyph(&glyf, &name_to_id);
        metrics.push(hmtx::Metric {
            advanceWidth: glyf.width as u16,
            lsb: glyph.as_ref().map_or(0, |g| g.xMin),
        });
        glyphs.push(glyph);
    }

    let head_table = compile_head(info, &glyphs);
    let post_table = compile_post(info, &names);
    let maxp_table = maxp::new05(glyph_id);
    let cmap_table = compile_cmap(mapping);
    let name_table = compile_name(info);

    let mut hhea_table = hhea::hhea {
        majorVersion: 1,
        minorVersion: 0,
        ascender: info.ascender.map_or(600, |f| f.get() as i16),
        descender: info.descender.map_or(-200, |f| f.get() as i16),
        lineGap: info.open_type_hhea_line_gap.unwrap_or(0) as i16,
        advanceWidthMax: metrics.iter().map(|x| x.advanceWidth).max().unwrap_or(0),
        minLeftSideBearing: metrics.iter().map(|x| x.lsb).min().unwrap_or(0),
        minRightSideBearing: 0, // xxx
        xMaxExtent: glyphs
            .iter()
            .filter_map(|o| o.as_ref().map(|g| g.xMax))
            .max()
            .unwrap_or(0),
        caretSlopeRise: 1, // XXX
        caretSlopeRun: 0,  // XXX
        caretOffset: info.open_type_hhea_caret_offset.unwrap_or(0) as i16,
        reserved0: 0,
        reserved1: 0,
        reserved2: 0,
        reserved3: 0,
        metricDataFormat: 0,
        numberOfHMetrics: 0,
    };
    let glyf_table = glyf::glyf { glyphs };
    let hmtx_table = hmtx::hmtx { metrics };
    let (hmtx_bytes, num_h_metrics) = hmtx_table.to_bytes();
    hhea_table.numberOfHMetrics = num_h_metrics;

    font.tables.insert(*b"head", Table::Head(head_table));
    font.tables.insert(*b"hmtx", Table::Unknown(hmtx_bytes));
    font.tables.insert(*b"maxp", Table::Maxp(maxp_table));
    font.tables.insert(*b"post", Table::Post(post_table));
    font.tables.insert(*b"cmap", Table::Cmap(cmap_table));
    font.tables.insert(*b"glyf", Table::Glyf(glyf_table));
    font.tables.insert(*b"hhea", Table::Hhea(hhea_table));
    font.tables.insert(*b"name", Table::Name(name_table));

    if matches.is_present("OUTPUT") {
        let mut outfile = File::create(matches.value_of("OUTPUT").unwrap())
            .expect("Could not open file for writing");
        font.save(&mut outfile);
    } else {
        font.save(&mut io::stdout());
    };
}
