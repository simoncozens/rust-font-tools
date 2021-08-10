use std::fs::File;
use std::path::PathBuf;

use chrono::TimeZone;
use designspace::{Axis as DSAxis, Designspace, Instance as DSInstance};

use crate::glyph::GlyphCategory;
use crate::{
    Axis, BabelfontError, Component, Font, Glyph, Layer, Location, Master, OTScalar, Path, Shape,
};

pub fn load(path: PathBuf) -> Result<Font, BabelfontError> {
    let ds_file = File::open(path.clone()).map_err(|source| BabelfontError::IO {
        path: path.clone(),
        source,
    })?;
    let ds: Designspace =
        designspace::from_reader(ds_file).map_err(|orig| BabelfontError::XMLParse {
            path: path.clone(),
            orig,
        })?;
    let relative = path.parent();
    let mut font = Font::new();
    load_axes(&mut font, &ds.axes.axis);
    if let Some(instances) = &ds.instances {
        load_instances(&mut font, &instances.instance);
    }
    let default_master = ds
        .default_master()
        .ok_or_else(|| BabelfontError::NoDefaultMaster { path: path.clone() })?;
    let relative_path_to_default_master = if let Some(r) = relative {
        r.join(default_master.filename.clone())
    } else {
        default_master.filename.clone().into()
    };
    let default_ufo = norad::Font::load(relative_path_to_default_master).map_err(|e| {
        BabelfontError::LoadingUFO {
            orig: e,
            path: default_master.filename.clone(),
        }
    })?;
    load_glyphs(&mut font, &default_ufo);
    load_masters(&mut font, &ds, relative)?;
    if let Some(info) = default_ufo.font_info {
        load_font_info(&mut font, &info);
    }

    Ok(font)
}

fn load_axes(font: &mut Font, axes: &[DSAxis]) {
    for dsax in axes {
        let mut ax = Axis::new(dsax.name.clone(), dsax.tag.clone());
        ax.min = Some(dsax.minimum as f32);
        ax.max = Some(dsax.maximum as f32);
        ax.default = Some(dsax.default as f32);
        if let Some(map) = &dsax.map {
            ax.map = Some(map.iter().map(|x| (x.input, x.output)).collect());
        }
        font.axes.push(ax);
    }
}

fn load_masters(
    font: &mut Font,
    ds: &Designspace,
    relative: Option<&std::path::Path>,
) -> Result<(), BabelfontError> {
    for source in &ds.sources.source {
        let location = Location(
            ds.axes
                .axis
                .iter()
                .map(|x| x.tag.clone())
                .zip(ds.location_to_tuple(&source.location))
                .collect(),
        );

        let mut master = Master::new(
            source
                .name
                .as_ref()
                .unwrap_or(&"Unnamed master".to_string()),
            source
                .name
                .as_ref()
                .unwrap_or(&"Unnamed master".to_string()),
            location,
        );
        let relative_path_to_master = if let Some(r) = relative {
            r.join(source.filename.clone())
        } else {
            source.filename.clone().into()
        };

        let source_font =
            norad::Font::load(relative_path_to_master).map_err(|e| BabelfontError::LoadingUFO {
                path: source.filename.clone(),
                orig: e,
            })?;
        if let Some(ref info) = source_font.font_info {
            load_master_info(&mut master, &info);
        }
        if let Some(ref kerning) = source_font.kerning {
            for (left, right_dict) in kerning.iter() {
                for (right, value) in right_dict.iter() {
                    master
                        .kerning
                        .insert((left.clone(), right.clone()), *value as i16);
                }
            }
        }
        for layer in source_font.iter_layers() {
            for g in font.glyphs.iter_mut() {
                if let Some(norad_glyph) = layer.get_glyph(g.name.as_str()) {
                    g.layers
                        .push(norad_glyph_to_babelfont_layer(norad_glyph, &master.id))
                }
            }
        }
        font.masters.push(master);
    }
    Ok(())
}

fn norad_glyph_to_babelfont_layer(glyph: &norad::Glyph, master_id: &str) -> Layer {
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

fn load_component(c: &norad::Component) -> Component {
    Component {
        reference: c.base.to_string(),
        transform: c.transform.into(),
    }
}

fn load_path(c: &norad::Contour) -> Path {
    Path {
        nodes: c.points.iter().map(|p| p.into()).collect(),
        closed: c
            .points
            .first()
            .map_or(true, |v| v.typ != norad::PointType::Move),
        direction: crate::shape::PathDirection::Clockwise,
    }
}

fn load_master_info(master: &mut Master, info: &norad::FontInfo) {
    let metrics = &mut master.metrics;
    if let Some(v) = info.ascender {
        metrics.insert("ascender".to_string(), v.get() as i32);
    }
    if let Some(v) = info.cap_height {
        metrics.insert("capHeight".to_string(), v.get() as i32);
    }
    if let Some(v) = info.descender {
        metrics.insert("descender".to_string(), v.get() as i32);
    }
    if let Some(v) = &info.guidelines {
        for g in v.iter() {
            master.guides.push(g.into())
        }
    }
    if let Some(v) = info.italic_angle {
        metrics.insert("italic angle".to_string(), v.get() as i32);
    }
    if let Some(v) = info.x_height {
        metrics.insert("xHeight".to_string(), v.get() as i32);
    }
}

fn load_font_info(font: &mut Font, info: &norad::FontInfo) {
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
        font.date = chrono::NaiveDateTime::parse_from_str(v, "%Y/%m/%d %H:%m:%s")
            .map(|x| chrono::Local.from_utc_datetime(&x))
            .unwrap_or_else(|_| chrono::Local::now());
    }
    if let Some(v) = &info.open_type_head_flags {
        font.set_ot_value("head", "flags", OTScalar::BitField(v.to_vec()))
    }
    if let Some(v) = info.open_type_head_lowest_rec_ppem {
        font.set_ot_value("head", "lowestRecPPEM", OTScalar::Unsigned(v))
    }
    // XXX and much more
    if let Some(v) = &info.trademark {
        font.names.trademark = v.into();
    }

    if let Some(v) = info.units_per_em {
        font.upm = v.get() as u16;
    }
    if let Some(v) = info.version_major {
        font.version.0 = v as u16;
    }
    if let Some(v) = info.version_minor {
        font.version.1 = v as u16;
    }
}

fn load_instances(_font: &mut Font, _instances: &[DSInstance]) {
    // unimplemented!()
}

fn load_glyphs(font: &mut Font, ufo: &norad::Font) {
    let categories = ufo
        .lib
        .get("public.openTypeCategories")
        .and_then(|x| x.as_dictionary());
    let psnames = ufo
        .lib
        .get("public.postscriptNames")
        .and_then(|x| x.as_dictionary());
    for glyphname in ufo.iter_names() {
        if let Some(glyph) = ufo.get_glyph(&glyphname) {
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
                exported: true, // urgh
                direction: None,
            })
        }
    }
}
