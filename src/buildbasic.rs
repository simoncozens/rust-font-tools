use crate::basictables::fill_tables;
use crate::glyph::glifs_to_glyph;
use fonttools::font;
use fonttools::font::Table;
use fonttools::glyf;
use fonttools::gvar::GlyphVariationData;
use fonttools::hmtx;
use fonttools::otvar::VariationModel;
use kurbo::{Affine, Point};
use norad::{Component, Contour, ContourPoint, Glyph, Layer};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

fn decomposed_components(glyph: &Glyph, glyphset: &Layer) -> Vec<Contour> {
    let mut contours = Vec::new();

    let mut stack: Vec<(&Component, Affine)> = Vec::new();

    for component in &glyph.components {
        stack.push((component, component.transform.into()));

        while let Some((component, transform)) = stack.pop() {
            let new_outline = match glyphset.get_glyph(&component.base) {
                Some(g) => g,
                None => continue,
            };

            for contour in &new_outline.contours {
                let mut decomposed_contour = Contour::default();
                for point in &contour.points {
                    let new_point = transform * Point::new(point.x as f64, point.y as f64);
                    decomposed_contour.points.push(ContourPoint::new(
                        new_point.x as f32,
                        new_point.y as f32,
                        point.typ.clone(),
                        point.smooth,
                        point.name.clone(),
                        None,
                        None,
                    ))
                }
                contours.push(decomposed_contour);
            }

            for new_component in new_outline.components.iter().rev() {
                let new_transform: Affine = new_component.transform.into();
                stack.push((new_component, transform * new_transform));
            }
        }
    }

    contours
}

fn decompose_mixed_glyphs(ufo: &mut norad::Font) {
    let layer = ufo.default_layer_mut();
    let mut decomposed: BTreeMap<String, Vec<norad::Contour>> = BTreeMap::new();
    for glif in layer.iter() {
        decomposed.insert(glif.name.to_string(), decomposed_components(glif, layer));
    }
    for glif in layer.iter_mut() {
        if glif.components.is_empty() || glif.contours.is_empty() {
            continue;
        }
        if let Some(contours) = decomposed.get(&glif.name.to_string()) {
            glif.contours.extend(contours.clone());
            glif.components.clear();
            log::info!("Decomposed mixed glyph {:?}", glif.name);
        }
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
    input: &babelfont::Font,
    mapping: &mut BTreeMap<u32, u16>,
    name_to_id: &mut BTreeMap<String, u16>,
    subset: &Option<HashSet<String>>,
) -> Vec<String> {
    let mut names: Vec<String> = vec![];
    for (glyph_id, glyf) in input.glyphs.iter().enumerate() {
        let name = glyf.name.to_string();
        if subset.is_some() && !subset.as_ref().unwrap().contains(&name) {
            continue;
        }
        names.push(name.clone());
        name_to_id.insert(name, glyph_id as u16);
        let cp = &glyf.codepoints;
        if !cp.is_empty() {
            mapping.insert(cp[0] as u32, glyph_id as u16);
        }
    }
    names
}

pub fn build_font(input: babelfont::Font, include: Option<HashSet<String>>) -> font::Font {
    // input.decompose_mixed_glyphs();

    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();

    let names = get_glyph_names_and_mapping(&input, &mut mapping, &mut name_to_id, &include);

    let mut glyphs: Vec<glyf::Glyph> = vec![];
    let mut metrics: Vec<hmtx::Metric> = vec![];
    let mut variations: Vec<Option<GlyphVariationData>> = vec![];
    let variation_model = input
        .variation_model()
        .expect("Couldn't get variation model");
    let default_master_ix = input
        .default_master_index()
        .expect("Couldn't find default master");
    for glif in input.glyphs.iter() {
        if include.is_some() && !include.as_ref().unwrap().contains(&glif.name.to_string()) {
            continue;
        }
        // Find other glyphs in designspace
        let mut glif_variations = vec![];
        for master in &input.masters {
            let layer = input.master_layer_for(&glif.name, master);
            glif_variations.push(layer);
        }
        let (glyph, variation) = glifs_to_glyph(
            default_master_ix,
            &name_to_id,
            &glif_variations,
            Some(&variation_model),
            &glif.name,
        );
        let lsb = 0; // glyph.xMin;
        let advance_width = input
            .master_layer_for(&glif.name, input.default_master().unwrap())
            .unwrap()
            .width as u16;
        glyphs.push(glyph);
        metrics.push(hmtx::Metric {
            advanceWidth: advance_width,
            lsb,
        });
        variations.push(variation);
    }

    let glyf_table = form_glyf_and_fix_bounds(glyphs, &mut metrics);
    let mut font = fill_tables(&input, glyf_table, metrics, names, mapping);
    let gvar_table = fonttools::gvar::gvar { variations };
    font.tables
        .insert(*b"gvar", Table::Unknown(gvar_table.to_bytes(None)));

    // No optimization by default

    font
}
