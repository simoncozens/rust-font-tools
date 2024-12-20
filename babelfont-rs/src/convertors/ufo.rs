use crate::glyph::GlyphCategory;
use crate::{BabelfontError, Component, Font, Glyph, Layer, Master, Node, OTScalar, Path, Shape};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use fontdrasil::coords::Location;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

pub(crate) fn stat(path: &std::path::Path) -> Option<DateTime<chrono::Local>> {
    fs::metadata(path)
        .and_then(|x| x.created())
        .ok()
        .and_then(|x| {
            NaiveDateTime::from_timestamp_opt(
                x.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
                0,
            )
        })
        .map(|x| chrono::Local.from_utc_datetime(&x))
}

pub fn load(path: PathBuf) -> Result<Font, BabelfontError> {
    let mut font = Font::new();
    let created_time: Option<DateTime<Local>> = stat(&path);
    let ufo = norad::Font::load(&path).map_err(|e| BabelfontError::LoadingUFO {
        orig: Box::new(e),
        path: path.into_os_string().into_string().unwrap(),
    })?;
    load_glyphs(&mut font, &ufo);
    let info = &ufo.font_info;
    load_font_info(&mut font, info, created_time);
    let mut master = Master::new(
        info.family_name
            .as_ref()
            .unwrap_or(&"Unnamed master".to_string()),
        info.family_name
            .as_ref()
            .unwrap_or(&"Unnamed master".to_string()),
        Location::new(),
    );
    load_master_info(&mut master, info);
    load_kerning(&mut master, &ufo.kerning);
    (font.first_kern_groups, font.second_kern_groups) = load_kern_groups(&ufo.groups);

    for layer in ufo.iter_layers() {
        for g in font.glyphs.iter_mut() {
            if let Some(norad_glyph) = layer.get_glyph(g.name.as_str()) {
                g.layers
                    .push(norad_glyph_to_babelfont_layer(norad_glyph, &master.id))
            }
        }
    }
    font.features = Some(ufo.features);
    font.masters.push(master);
    Ok(font)
}

pub(crate) fn norad_glyph_to_babelfont_layer(glyph: &norad::Glyph, master_id: &str) -> Layer {
    let mut l = Layer::new(glyph.width as i32);
    l.id = Some(master_id.to_string());
    l.guides = glyph.guidelines.iter().map(|x| x.into()).collect();
    l.anchors = glyph.anchors.iter().map(|x| x.into()).collect();
    for comp in &glyph.components {
        l.shapes.push(Shape::ComponentShape(load_component(comp)));
    }
    for contour in &glyph.contours {
        l.shapes.push(Shape::PathShape(load_path(contour)));
    }
    l
}

pub(crate) fn load_component(c: &norad::Component) -> Component {
    let t = c.transform;
    Component {
        reference: c.base.to_string(),
        transform: kurbo::Affine::new([
            t.x_scale, t.xy_scale, t.yx_scale, t.y_scale, t.x_offset, t.y_offset,
        ]),
    }
}

pub(crate) fn load_path(c: &norad::Contour) -> Path {
    let mut nodes: Vec<Node> = c.points.iter().map(|p| p.into()).collect();
    // See https://github.com/simoncozens/rust-font-tools/issues/3
    nodes.rotate_left(1);
    Path {
        nodes,
        closed: c
            .points
            .first()
            .map_or(true, |v| v.typ != norad::PointType::Move),
        direction: crate::shape::PathDirection::Clockwise,
    }
}

pub(crate) fn load_master_info(master: &mut Master, info: &norad::FontInfo) {
    let metrics = &mut master.metrics;
    if let Some(v) = info.ascender {
        metrics.insert("ascender".to_string(), v as i32);
    }
    if let Some(v) = info.cap_height {
        metrics.insert("capHeight".to_string(), v as i32);
    }
    if let Some(v) = info.descender {
        metrics.insert("descender".to_string(), v as i32);
    }
    if let Some(v) = &info.guidelines {
        for g in v.iter() {
            master.guides.push(g.into())
        }
    }
    if let Some(v) = info.italic_angle {
        metrics.insert("italic angle".to_string(), v as i32);
    }
    if let Some(v) = info.x_height {
        metrics.insert("xHeight".to_string(), v as i32);
    }
}

pub(crate) fn load_font_info(
    font: &mut Font,
    info: &norad::FontInfo,
    created: Option<DateTime<Local>>,
) {
    if let Some(v) = &info.copyright {
        font.names.copyright = v.into();
    }
    if let Some(v) = &info.family_name {
        font.names.family_name = v.into();
    }
    if let Some(v) = &info.note {
        font.note = Some(v.clone());
    }
    if let Some(v) = &info.open_type_head_created {
        if let Ok(Some(date)) = NaiveDateTime::parse_from_str(v, "%Y/%m/%d %H:%M:%S")
            .map(|x| chrono::Local.from_local_datetime(&x).single())
        {
            font.date = date;
        } else {
            font.date = created.unwrap_or_else(chrono::Local::now);
        }
    }
    if let Some(v) = &info.open_type_head_flags {
        font.set_ot_value("head", "flags", OTScalar::BitField(v.to_vec()))
    }
    if let Some(v) = info.open_type_head_lowest_rec_ppem {
        font.set_ot_value("head", "lowestRecPPEM", OTScalar::Unsigned(v))
    }
    if let Some(v) = &info.open_type_os2_type {
        font.set_ot_value("OS/2", "fsType", OTScalar::BitField(v.to_vec()))
    }
    if let Some(v) = &info.postscript_underline_position {
        font.set_ot_value("post", "underlinePosition", OTScalar::Signed(*v as i32))
    }
    // XXX and much more
    if let Some(v) = &info.trademark {
        font.names.trademark = v.into();
    }

    if let Some(v) = info.units_per_em {
        font.upm = v.as_f64() as u16;
    }
    if let Some(v) = info.version_major {
        font.version.0 = v as u16;
    }
    if let Some(v) = info.version_minor {
        font.version.1 = v as u16;
    }
}

pub(crate) fn load_kerning(master: &mut Master, kerning: &norad::Kerning) {
    for (left, right_dict) in kerning.iter() {
        for (right, value) in right_dict.iter() {
            let left_maybe_group = if left.starts_with("public.kern") {
                format!("@{:}", left)
            } else {
                left.to_string()
            };
            let right_maybe_group = if right.starts_with("public.kern") {
                format!("@{:}", right)
            } else {
                right.to_string()
            };
            master
                .kerning
                .insert((left_maybe_group, right_maybe_group), *value as i16);
        }
    }
}

pub(crate) fn load_kern_groups(groups: &norad::Groups) -> HashMap<String, Vec<String>> {
    let mut hm: HashMap<String, Vec<String>> = HashMap::new();
    for (name, members) in groups.iter() {
        hm.insert(
            name.to_string(),
            members.iter().map(|x| x.to_string()).collect(),
        );
    }
    hm
}

pub(crate) fn load_glyphs(font: &mut Font, ufo: &norad::Font) {
    let categories = ufo
        .lib
        .get("public.openTypeCategories")
        .and_then(|x| x.as_dictionary());
    let psnames = ufo
        .lib
        .get("public.postscriptNames")
        .and_then(|x| x.as_dictionary());
    let skipped: HashSet<String> = ufo
        .lib
        .get("public.skipExportGlyphs")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(|x| x.as_string())
        .map(|x| x.to_string())
        .collect();
    let glyphorder: Vec<String> = ufo
        .lib
        .get("public.glyphOrder")
        .and_then(|x| x.as_array())
        .unwrap_or(&vec![])
        .iter()
        .flat_map(|x| x.as_string())
        .map(|x| x.to_string())
        .collect();
    let mut order: Vec<String> = vec![];
    let mut ufo_names: Vec<String> = ufo.iter_names().map(|x| x.to_string()).collect();
    if ufo_names.contains(&".notdef".to_string()) {
        order.push(".notdef".to_string());
        ufo_names.retain(|x| x != ".notdef");
    }
    for name in glyphorder {
        if !ufo_names.contains(&name) {
            continue;
        }
        ufo_names.retain(|x| x != &name);
        order.push(name);
    }
    order.append(&mut ufo_names);

    for glyphname in order {
        if let Some(glyph) = ufo.get_glyph(glyphname.as_str()) {
            let cat = if let Some(cats) = categories {
                match cats.get(&glyphname).and_then(|x| x.as_string()) {
                    Some("base") => GlyphCategory::Base,
                    Some("mark") => GlyphCategory::Mark,
                    Some("ligature") => GlyphCategory::Ligature,
                    _ => GlyphCategory::Base,
                }
            } else {
                GlyphCategory::Base
            };
            let production_name = psnames
                .and_then(|x| x.get(&glyphname))
                .and_then(|x| x.as_string())
                .map(|x| x.to_string());
            font.glyphs.push(Glyph {
                name: glyphname.to_string(),
                category: cat,
                production_name,
                codepoints: glyph.codepoints.iter().map(|x| *x as usize).collect(),
                layers: vec![],
                exported: !skipped.contains(&glyphname),
                direction: None,
            })
        }
    }
    add_uvs_sequences(font, ufo);
}

fn add_uvs_sequences(font: &mut Font, ufo: &norad::Font) {
    if let Some(uvs) = ufo
        .lib
        .get("public.unicodeVariationSequences")
        .and_then(|x| x.as_dictionary())
    {
        // Lasciate ogne speranza, voi ch'intrate
        for (selector_s, records_plist) in uvs.iter() {
            if let Ok(selector) = u32::from_str_radix(selector_s, 16) {
                if let Some(records) = records_plist.as_dictionary() {
                    for (codepoint_s, glyphname_plist) in records {
                        if let Ok(codepoint) = u32::from_str_radix(codepoint_s, 16) {
                            if let Some(glyphname) = glyphname_plist.as_string() {
                                font.variation_sequences
                                    .insert((selector, codepoint), glyphname.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
}
