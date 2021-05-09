use crate::basictables::fill_tables;
use crate::glyph::glifs_to_glyph;
use fonttools::font;
use fonttools::font::Table;
use fonttools::glyf;
use fonttools::gvar::GlyphVariationData;
use fonttools::hmtx;
use fonttools::otvar::NormalizedLocation;

use rayon::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn build_font(ufo: norad::Font) -> font::Font {
    let layer = ufo.default_layer();
    let info = ufo.font_info.as_ref().unwrap();

    let mut names: Vec<String> = vec![];
    let mut glyph_id = 0;
    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();

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
    let glifs: Vec<Arc<norad::Glyph>> = layer.iter_contents().collect();
    let (mut glyphs, mut metrics): (Vec<glyf::Glyph>, Vec<hmtx::Metric>) = glifs
        .iter()
        .map({
            |glyf| {
                let (glyph, _) = glifs_to_glyph(&glyf, &name_to_id, vec![]);
                let lsb = glyph.xMin;
                let advanceWidth = glyf.width as u16;
                (glyph, hmtx::Metric { advanceWidth, lsb })
            }
        })
        .unzip();

    // Decompose mixed.
    let mut to_replace: Vec<(usize, glyf::Glyph)> = vec![];
    for (id, glyph) in glyphs.iter().enumerate() {
        if !glyph.components.is_empty() && !glyph.contours.is_empty() {
            log::info!("Decomposed mixed glyph {:?}", names[id]);
            to_replace.push((id, glyph.decompose(&glyphs)));
        }
    }
    for (id, glyph) in to_replace {
        glyphs[id] = glyph;
    }

    let mut glyf_table = glyf::glyf { glyphs };
    glyf_table.recalc_bounds();

    // Do LSBs again
    for (id, glyph) in glyf_table.glyphs.iter().enumerate() {
        metrics[id].lsb = glyph.xMin;
    }
    fill_tables(info, glyf_table, metrics, names, mapping)
}

pub fn build_fonts(
    default_master: &norad::Font,
    other_masters: Vec<(NormalizedLocation, &norad::Layer)>,
) -> font::Font {
    let layer = default_master.default_layer();
    let info = default_master.font_info.as_ref().unwrap();

    let mut names: Vec<String> = vec![];
    let mut glyph_id = 0;
    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();

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
    let glifs: Vec<Arc<norad::Glyph>> = layer.iter_contents().collect();

    let mut glyphs: Vec<glyf::Glyph> = vec![];
    let mut metrics: Vec<hmtx::Metric> = vec![];
    let mut variations: Vec<Option<GlyphVariationData>> = vec![];
    for glif in glifs {
        // Find other glyphs in designspace
        let mut glif_variations = vec![];
        for (location, layer) in &other_masters {
            if let Some(other_glif) = layer.get_glyph(&glif.name) {
                glif_variations.push((location, other_glif));
            }
        }
        let (glyph, variation) = glifs_to_glyph(&glif, &name_to_id, glif_variations);
        let lsb = glyph.xMin;
        let advanceWidth = glif.width as u16;
        glyphs.push(glyph);
        metrics.push(hmtx::Metric { advanceWidth, lsb });
        variations.push(variation);
    }

    // Decompose mixed.
    let mut to_replace: Vec<(usize, glyf::Glyph)> = vec![];
    for (id, glyph) in glyphs.iter().enumerate() {
        if !glyph.components.is_empty() && !glyph.contours.is_empty() {
            log::info!("Decomposed mixed glyph {:?}", names[id]);
            to_replace.push((id, glyph.decompose(&glyphs)));
        }
    }
    for (id, glyph) in to_replace {
        glyphs[id] = glyph;
    }

    let mut glyf_table = glyf::glyf { glyphs };
    glyf_table.recalc_bounds();

    // Do LSBs again
    for (id, glyph) in glyf_table.glyphs.iter().enumerate() {
        metrics[id].lsb = glyph.xMin;
    }
    let mut font = fill_tables(info, glyf_table, metrics, names, mapping);
    let gvar_table = fonttools::gvar::gvar { variations };
    font.tables
        .insert(*b"gvar", Table::Unknown(gvar_table.to_bytes()));

    font
}
