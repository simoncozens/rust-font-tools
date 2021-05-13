use crate::basictables::fill_tables;
use crate::glyph::glifs_to_glyph;
use fonttools::font;
use fonttools::font::Table;
use fonttools::glyf;
use fonttools::gvar::GlyphVariationData;
use fonttools::hmtx;
use fonttools::otvar::{NormalizedLocation, VariationModel};
use std::collections::BTreeMap;
use std::sync::Arc;

fn decompose_mixed_glyphs(glyphs: &mut Vec<glyf::Glyph>, names: &[String]) {
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
}

fn form_glyf_and_fix_bounds(
    glyphs: Vec<glyf::Glyph>,
    metrics: &mut Vec<hmtx::Metric>,
) -> glyf::glyf {
    let mut glyf_table = glyf::glyf { glyphs };
    glyf_table.recalc_bounds();

    // Do LSBs again
    for (id, glyph) in glyf_table.glyphs.iter().enumerate() {
        metrics[id].lsb = glyph.xMin;
    }
    glyf_table
}

fn get_glyph_names_and_mapping(
    layer: &norad::Layer,
    mapping: &mut BTreeMap<u32, u16>,
    name_to_id: &mut BTreeMap<String, u16>,
) -> Vec<String> {
    let mut names: Vec<String> = vec![];
    for (glyph_id, glyf) in layer.iter_contents().enumerate() {
        let name = glyf.name.to_string();
        names.push(name.clone());
        name_to_id.insert(name, glyph_id as u16);
        let cp = &glyf.codepoints;
        if !cp.is_empty() {
            mapping.insert(cp[0] as u32, glyph_id as u16);
        }
    }
    names
}

pub fn build_font(ufo: norad::Font) -> font::Font {
    let layer = ufo.default_layer();
    let info = ufo.font_info.as_ref().unwrap();

    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();

    let names = get_glyph_names_and_mapping(&layer, &mut mapping, &mut name_to_id);
    let glifs: Vec<Arc<norad::Glyph>> = layer.iter_contents().collect();
    let (mut glyphs, mut metrics): (Vec<glyf::Glyph>, Vec<hmtx::Metric>) = glifs
        .iter()
        .map({
            |glyf| {
                let (glyph, _) = glifs_to_glyph(0, &name_to_id, &[Some(&glyf)], None);
                let lsb = glyph.xMin;
                let advanceWidth = glyf.width as u16;
                (glyph, hmtx::Metric { advanceWidth, lsb })
            }
        })
        .unzip();
    decompose_mixed_glyphs(&mut glyphs, &names);
    let glyf_table = form_glyf_and_fix_bounds(glyphs, &mut metrics);
    fill_tables(info, glyf_table, metrics, names, mapping)
}

pub fn build_fonts(
    default_master: usize,
    fonts: Vec<norad::Font>,
    variation_model: VariationModel,
) -> font::Font {
    let layer = fonts[default_master].default_layer();
    let info = fonts[default_master].font_info.as_ref().unwrap();
    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();

    let names = get_glyph_names_and_mapping(&layer, &mut mapping, &mut name_to_id);

    let glifs: Vec<Arc<norad::Glyph>> = layer.iter_contents().collect();

    let mut glyphs: Vec<glyf::Glyph> = vec![];
    let mut metrics: Vec<hmtx::Metric> = vec![];
    let mut variations: Vec<Option<GlyphVariationData>> = vec![];
    for glif in glifs {
        // Find other glyphs in designspace
        let mut glif_variations = vec![];
        for font in &fonts {
            if let Some(other_glif) = font.default_layer().get_glyph(&glif.name) {
                glif_variations.push(Some(other_glif));
            } else {
                glif_variations.push(None);
            }
        }
        let (glyph, variation) = glifs_to_glyph(
            default_master,
            &name_to_id,
            &glif_variations,
            Some(&variation_model),
        );
        let lsb = glyph.xMin;
        let advanceWidth = glif.width as u16;
        glyphs.push(glyph);
        metrics.push(hmtx::Metric { advanceWidth, lsb });
        variations.push(variation);
    }

    // XXX, you can't do this here.
    decompose_mixed_glyphs(&mut glyphs, &names);
    let glyf_table = form_glyf_and_fix_bounds(glyphs, &mut metrics);
    let mut font = fill_tables(info, glyf_table, metrics, names, mapping);
    let gvar_table = fonttools::gvar::gvar { variations };
    font.tables
        .insert(*b"gvar", Table::Unknown(gvar_table.to_bytes(None)));
    // No optimization by default

    font
}
