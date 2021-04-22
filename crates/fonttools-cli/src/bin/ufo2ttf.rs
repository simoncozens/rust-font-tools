use clap::{App, Arg};
use fonttools::cmap;
use fonttools::font;
use fonttools::font::Table;
use fonttools::glyf;
use fonttools::head::head;
use fonttools::hhea;
use fonttools::hmtx;
use fonttools::maxp::maxp;
use fonttools::post::post;
use kurbo::Affine;
use lyon::geom::cubic_bezier::CubicBezierSegment;
use lyon::geom::euclid::TypedPoint2D;
use std::marker::PhantomData;
type LyonPoint = TypedPoint2D<f32, lyon::geom::euclid::UnknownUnit>;
use lyon::path::geom::cubic_to_quadratic::cubic_to_quadratics;
use norad::glyph::PointType;
use norad::Ufo;
use std::collections::BTreeMap;
use std::fs::File;
use std::io;

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
    if let Some(outline) = &glif.outline {
        if outline.components.is_empty() && outline.contours.is_empty() {
            return None;
        }

        if !outline.components.is_empty() && !outline.contours.is_empty() {
            // println!("Mixed glyph needs decomposition {:?}", glif.name);
            return Some(glyph);
        }

        /* Do components */
        let mut components: Vec<glyf::Component> = vec![];
        for component in &outline.components {
            if let Some(glyf_component) = norad_component_to_glyf_component(component, mapping) {
                components.push(glyf_component);
            }
        }
        if !components.is_empty() {
            glyph.components = Some(components);
        }

        /* Do outlines */
        let mut contours: Vec<Vec<glyf::Point>> = vec![];
        for contour in &outline.contours {
            if let Some(glyf_contour) = norad_contour_to_glyf_contour(contour) {
                contours.push(glyf_contour);
            }
        }
        if !contours.is_empty() {
            glyph.contours = Some(contours);
            glyph.recalc_bounds();
        }
        return Some(glyph);
    }
    None
}

fn norad_contour_to_glyf_contour(contour: &norad::glyph::Contour) -> Option<Vec<glyf::Point>> {
    // Stupid implementation
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
    component: &norad::glyph::Component,
    mapping: &BTreeMap<String, u16>,
) -> Option<glyf::Component> {
    let maybe_id = mapping.get(&component.base.to_string());

    if maybe_id.is_none() {
        println!("Couldn't find component for {:?}", component.base);
        return None;
    }
    let transform = [
        component.transform.x_scale as f64,
        component.transform.xy_scale as f64,
        component.transform.y_scale as f64,
        component.transform.yx_scale as f64,
        component.transform.x_offset as f64,
        component.transform.y_offset as f64,
    ];

    Some(glyf::Component {
        glyphIndex: *maybe_id.unwrap(),
        matchPoints: None,
        flags: glyf::ComponentFlags::empty(),
        transformation: Affine::new(transform),
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

    let layer = ufo.get_default_layer().unwrap();
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
        if let Some(cp) = &glyf.codepoints {
            if !cp.is_empty() {
                mapping.insert(cp[0] as u32, glyph_id);
            }
        }
        glyph_id += 1;
    }
    for glyf in layer.iter_contents() {
        metrics.push(hmtx::Metric {
            advanceWidth: glyf.advance.as_ref().map_or(1000, |f| f.width as u16),
            lsb: 0,
        });
        glyphs.push(glif_to_glyph(&glyf, &name_to_id));
    }

    let head_table = head::new(
        info.version_major.unwrap_or(1) as f32,
        info.units_per_em.map_or(1000, |f| f.get() as u16),
        -200,
        500,
        -200,
        500,
    );

    let post_table = post::new(2.0, 0.0, 0, 0, false, Some(names));
    let maxp_table = maxp::new05(glyph_id);
    let cmap_table = cmap::cmap {
        subtables: vec![cmap::CmapSubtable {
            format: 4,
            platformID: 0,
            encodingID: 3,
            languageID: 0,

            mapping,
        }],
    };
    let mut hhea_table = hhea::hhea {
        majorVersion: 1,
        minorVersion: 0,
        ascender: info.ascender.map_or(600, |f| f.get() as i16),
        descender: info.descender.map_or(-200, |f| f.get() as i16),
        lineGap: 0,
        advanceWidthMax: metrics.iter().map(|x| x.advanceWidth).max().unwrap_or(0),
        minLeftSideBearing: metrics.iter().map(|x| x.lsb).min().unwrap_or(0),
        minRightSideBearing: 0, // xxx
        xMaxExtent: glyphs
            .iter()
            .filter_map(|o| o.as_ref().map(|g| g.xMax))
            .max()
            .unwrap_or(0),
        caretSlopeRise: 1,
        caretSlopeRun: 0,
        caretOffset: 0,
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

    if matches.is_present("OUTPUT") {
        let mut outfile = File::create(matches.value_of("OUTPUT").unwrap())
            .expect("Could not open file for writing");
        font.save(&mut outfile);
    } else {
        font.save(&mut io::stdout());
    };
}
