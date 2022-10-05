use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};

use designspace::{Axis as DSAxis, Designspace, Instance as DSInstance, Source};
use rayon::prelude::*;
use uuid::Uuid;

use crate::convertors::ufo::{
    load_font_info, load_glyphs, load_master_info, norad_glyph_to_babelfont_layer, stat,
};
use crate::{Axis, BabelfontError, Font, Location, Master};

pub fn load(path: PathBuf) -> Result<Font, BabelfontError> {
    let mut font = Font::new();

    let ds_file = File::open(&path).map_err(|source| BabelfontError::IO {
        path: path.clone(),
        source,
    })?;
    let ds: Designspace =
        designspace::from_reader(ds_file).map_err(|orig| BabelfontError::XMLParse {
            path: path.clone(),
            orig,
        })?;

    let default_source = ds
        .default_master()
        .ok_or_else(|| BabelfontError::NoDefaultMaster { path: path.clone() })?;

    let source_ufos = load_source_ufos(&path, &ds.sources.source)?;
    let default_ufo = &source_ufos[default_source.filename.as_str()];
    load_glyphs(&mut font, default_ufo);

    let info = &default_ufo.font_info;
    let created_time = stat(&path);
    load_font_info(&mut font, info, created_time);
    font.features = Some(default_ufo.features.clone());
    load_axes(&mut font, &ds.axes.axis);
    if let Some(instances) = &ds.instances {
        load_instances(&mut font, &instances.instance);
    }

    // Cache glyph indices for insertion, since getting a particular glyph from a
    // `font.glyph` scans the glyph list linearily. Clone glpyh names so we can pass the
    // actual font down mutably into load_master.
    let glyph_index: HashMap<String, usize> = font
        .glyphs
        .iter()
        .enumerate()
        .map(|(index, glyph)| (glyph.name.clone(), index))
        .collect();

    // todo: make [glyphname, mut glyph] mapping or else [glyphname, index] mapping for easy glyph lookup for insertion
    for source in &ds.sources.source {
        load_master(&mut font, &glyph_index, &ds, source, &source_ufos)?;
    }

    Ok(font)
}

/// Return mapping of source filenames to source UFOs.
///
/// Sources are loaded once per unique filename, not per unique canonical path.
fn load_source_ufos<'a>(
    designspace_path: &Path,
    sources: &'a [Source],
) -> Result<HashMap<&'a str, norad::Font>, BabelfontError> {
    // Distill the unique filenames in use. Keep it as a Vec with stable indices for
    // zipping later because error propagation in a (rayon) iterator context is
    // annoying.
    //
    // To be truly diligent, filenames should be canonicalized, but then we can't easily
    // use them as keys unless we canonicalize them in the Source struct.
    let unique_filenames: Vec<&str> = sources
        .iter()
        .map(|source| source.filename.as_str())
        .collect::<HashSet<&str>>()
        .into_iter()
        .collect();

    let source_ufos: Vec<norad::Font> = unique_filenames
        .par_iter()
        .map(|filename| construct_source_path(designspace_path, filename))
        .map(|path| {
            norad::Font::load(&path).map_err(|orig| BabelfontError::LoadingUFO {
                orig,
                path: path.display().to_string(),
            })
        })
        .collect::<Result<_, _>>()?;

    let source_ufos = HashMap::from_iter(unique_filenames.into_iter().zip(source_ufos));

    Ok(source_ufos)
}

/// Construct the path for a source relative to the Designspace file.
fn construct_source_path(designspace_path: &Path, filename: &str) -> PathBuf {
    match designspace_path.parent() {
        Some(parent_dir) => parent_dir.join(&filename),
        None => PathBuf::from(&filename),
    }
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
    font: &mut Font,
    glyph_index: &HashMap<String, usize>,
    ds: &Designspace,
    source: &Source,
    source_ufos: &HashMap<&str, norad::Font>,
) -> Result<(), BabelfontError> {
    let location = Location(
        ds.axes
            .axis
            .iter()
            .map(|x| x.tag.clone())
            .zip(ds.location_to_tuple(&source.location))
            .collect(),
    );
    let uuid = Uuid::new_v4().to_string();

    let mut master = Master::new(
        source
            .name
            .as_ref()
            .unwrap_or(&"Unnamed master".to_string()),
        source.name.as_ref().unwrap_or(&uuid),
        location,
    );

    let source_font = &source_ufos[source.filename.as_str()];
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

    let source_layer = get_source_layer(source, source_font)?;
    for source_glyph in source_layer.iter() {
        let glyph_name = source_glyph.name.as_str();
        if !glyph_index.contains_key(glyph_name) {
            log::warn!(
                "Glyph '{}' in source '{}' is not listed in the default source, skipping.",
                glyph_name,
                &source.filename
            );
            continue;
        }

        let converted_layer = norad_glyph_to_babelfont_layer(source_glyph, &master.id);
        let glyph = &mut font.glyphs.0[glyph_index[glyph_name]];
        glyph.layers.push(converted_layer);
    }

    font.masters.push(master);

    Ok(())
}

/// Fetch the default or given layer from the source font.
fn get_source_layer<'a>(
    source: &Source,
    source_font: &'a norad::Font,
) -> Result<&'a norad::Layer, BabelfontError> {
    match &source.layer {
        Some(layer_name) => {
            source_font
                .layers
                .get(layer_name)
                .ok_or(BabelfontError::UnknownSourceLayer {
                    layer_name: layer_name.clone(),
                    filename: source.filename.clone(),
                })
        }
        None => Ok(source_font.layers.default_layer()),
    }
}
