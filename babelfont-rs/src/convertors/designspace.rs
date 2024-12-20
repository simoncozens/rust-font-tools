use crate::common::tag_from_string;
use crate::convertors::ufo::stat;
use crate::glyph::GlyphList;
use crate::Layer;
use fontdrasil::coords::{DesignCoord, DesignLocation, UserCoord};
use norad::designspace::Source;
use std::collections::{BTreeMap, HashMap};
use write_fonts::types::Tag;
// use rayon::prelude::*;
use std::path::PathBuf;

use uuid::Uuid;

use crate::convertors::ufo::{
    load_font_info, load_glyphs, load_master_info, norad_glyph_to_babelfont_layer,
};
use crate::{Axis, BabelfontError, Font, Master};

use norad::designspace::{Axis as DSAxis, DesignSpaceDocument, Instance as DSInstance};

pub fn load(path: PathBuf) -> Result<Font, BabelfontError> {
    let created_time = stat(&path);
    // let ds_file = File::open(path.clone()).map_err(|source| BabelfontError::IO {
    //     path: path.clone(),
    //     source,
    // })?;
    let ds: DesignSpaceDocument = norad::designspace::DesignSpaceDocument::load(path.clone())
        .map_err(|orig| BabelfontError::XMLParse {
            path: path.clone(),
            orig,
        })?;
    let relative = path.parent();
    let mut font = Font::new();
    load_axes(&mut font, &ds.axes)?;
    // if let Some(instances) = &ds.instances {
    //     load_instances(&mut font, &instances.instance);
    // }
    let default_master = default_master(&ds, &font.axes)
        .ok_or_else(|| BabelfontError::NoDefaultMaster { path: path.clone() })?;
    let relative_path_to_default_master = if let Some(r) = relative {
        r.join(default_master.filename.clone())
    } else {
        default_master.filename.clone().into()
    };
    let default_ufo = norad::Font::load(relative_path_to_default_master).map_err(|e| {
        BabelfontError::LoadingUFO {
            orig: Box::new(e),
            path: default_master.filename.clone(),
        }
    })?;
    load_glyphs(&mut font, &default_ufo);
    let res: Vec<(Master, Vec<Vec<Layer>>)> = ds
        .sources
        .iter()
        .filter_map(|source| load_master(&font.glyphs, &ds, source, relative).ok())
        .collect();
    for (master, mut layerset) in res {
        font.masters.push(master);
        for (g, l) in font.glyphs.iter_mut().zip(layerset.iter_mut()) {
            g.layers.append(l);
        }
    }
    let info = default_ufo.font_info;
    load_font_info(&mut font, &info, created_time);
    font.features = Some(default_ufo.features);
    Ok(font)
}

fn load_axes(font: &mut Font, axes: &[DSAxis]) -> Result<(), BabelfontError> {
    for dsax in axes {
        let mut ax = Axis::new(dsax.name.clone(), tag_from_string(&dsax.tag)?);
        ax.min = dsax.minimum.map(UserCoord::new);
        ax.max = dsax.maximum.map(UserCoord::new);
        ax.default = Some(UserCoord::new(dsax.default));
        if let Some(map) = &dsax.map {
            ax.map = Some(
                map.iter()
                    .map(|x| (UserCoord::new(x.input), DesignCoord::new(x.output)))
                    .collect(),
            );
        }
        font.axes.push(ax);
    }
    Ok(())
}

pub(crate) fn load_instances(_font: &mut Font, _instances: &[DSInstance]) {
    // unimplemented!()
}

fn load_master(
    glyphs: &GlyphList,
    ds: &DesignSpaceDocument,
    source: &Source,
    relative: Option<&std::path::Path>,
) -> Result<(Master, Vec<Vec<Layer>>), BabelfontError> {
    #[warn(clippy::unwrap_used)] // XXX I am in a hurry
    let axis_names_to_tags: HashMap<String, Tag> = ds
        .axes
        .iter()
        .map(|x| (x.name.clone(), tag_from_string(&x.tag).unwrap()))
        .collect();
    let location = DesignLocation::from(
        source
            .location
            .iter()
            .map(|dimension| {
                (
                    *axis_names_to_tags
                        .get(dimension.name.as_str())
                        .unwrap_or_else(|| panic!("Axis name not found: {}", dimension.name)),
                    DesignCoord::new(dimension.uservalue.unwrap_or_default()),
                )
            })
            .collect::<Vec<_>>(),
    );
    let required_layer = &source.layer;
    let uuid = Uuid::new_v4().to_string();

    let mut master = Master::new(
        source
            .name
            .as_ref()
            .unwrap_or(&"Unnamed master".to_string()),
        source.name.as_ref().unwrap_or(&uuid),
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
            orig: Box::new(e),
        })?;
    let info = &source_font.font_info;
    load_master_info(&mut master, info);
    let kerning = &source_font.kerning;
    for (left, right_dict) in kerning.iter() {
        for (right, value) in right_dict.iter() {
            master
                .kerning
                .insert((left.to_string(), right.to_string()), *value as i16);
        }
    }
    let mut bf_layer_list = vec![];
    for g in glyphs.iter() {
        let mut glyph_layer_list = vec![];
        for layer in source_font.iter_layers() {
            let layername = layer.name().to_string();
            // We should probably keep all layers for interchange purposes,
            // but this is correct for compilation purposes
            if let Some(wanted) = &required_layer {
                if &layername != wanted {
                    continue;
                }
            }

            if let Some(norad_glyph) = layer.get_glyph(g.name.as_str()) {
                glyph_layer_list.push(norad_glyph_to_babelfont_layer(norad_glyph, &master.id))
            }
        }
        bf_layer_list.push(glyph_layer_list)
    }
    Ok((master, bf_layer_list))
}

fn default_master<'a>(ds: &'a DesignSpaceDocument, axes: &[Axis]) -> Option<&'a Source> {
    #[warn(clippy::unwrap_used)] // XXX I am in a hurry
    let defaults: BTreeMap<&String, DesignCoord> = axes
        .iter()
        .map(|ax| {
            (
                ax.name.get_default().unwrap(),
                ax.default
                    .map(|x| ax.userspace_to_designspace(x).unwrap())
                    .unwrap_or_default(),
            )
        })
        .collect();
    for source in ds.sources.iter() {
        let mut maybe = true;
        for loc in source.location.iter() {
            if defaults.get(&loc.name) != loc.xvalue.map(DesignCoord::new).as_ref() {
                maybe = false;
                break;
            }
        }
        if maybe {
            return Some(source);
        }
    }
    None
}
