use crate::convertors::ufo::stat;
use crate::glyph::GlyphList;
use crate::Layer;
use designspace::Source;
use rayon::prelude::*;
use std::fs::File;
use std::path::PathBuf;

use uuid::Uuid;

use crate::convertors::ufo::load_font_info;
use crate::convertors::ufo::load_glyphs;
use crate::convertors::ufo::load_master_info;
use crate::convertors::ufo::norad_glyph_to_babelfont_layer;
use crate::{Axis, BabelfontError, Font, Location, Master};

use designspace::{Axis as DSAxis, Designspace, Instance as DSInstance};

pub fn load(path: PathBuf) -> Result<Font, BabelfontError> {
    let created_time = stat(&path);
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
    let res: Vec<(Master, Vec<Vec<Layer>>)> = ds
        .sources
        .source
        .par_iter()
        .filter_map(|source| load_master(&font.glyphs, &ds, &source, relative).ok())
        .collect();
    for (master, mut layerset) in res {
        font.masters.push(master);
        for (g, mut l) in font.glyphs.iter_mut().zip(layerset.iter_mut()) {
            g.layers.append(&mut l);
        }
    }
    let info = default_ufo.font_info;
    load_font_info(&mut font, &info, created_time);
    font.features = Some(default_ufo.features);
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

pub(crate) fn load_instances(_font: &mut Font, _instances: &[DSInstance]) {
    // unimplemented!()
}

fn load_master(
    glyphs: &GlyphList,
    ds: &Designspace,
    source: &Source,
    relative: Option<&std::path::Path>,
) -> Result<(Master, Vec<Vec<Layer>>), BabelfontError> {
    let location = Location(
        ds.axes
            .axis
            .iter()
            .map(|x| x.tag.clone())
            .zip(ds.location_to_tuple(&source.location))
            .collect(),
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
            orig: e,
        })?;
    let info = &source_font.font_info;
    load_master_info(&mut master, info);
    let kerning = &source_font.kerning;
    for (left, right_dict) in kerning.iter() {
        for (right, value) in right_dict.iter() {
            master
                .kerning
                .insert((left.clone(), right.clone()), *value as i16);
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
