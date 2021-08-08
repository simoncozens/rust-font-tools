use crate::basictables::fill_tables;
use crate::glyph::layers_to_glyph;
use crate::kerning::build_kerning;
use babelfont::Layer;
use fonttools::font;
use fonttools::font::Table;
use fonttools::glyf;
use fonttools::gvar::GlyphVariationData;
use fonttools::hmtx;

use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::{BTreeMap, HashSet};
use unzip_n::unzip_n;

unzip_n!(3);

// We collect here the information for the `cmap` table (`codepoint_to_gid`); a
// mapping of glyph names to eventual glyph IDs (`name_to_id`) which will be used
// when resolving components; and the list of glyph names (return value) which
// will be used in the `post` table.
fn get_glyph_names_and_mapping(
    input: &babelfont::Font,
    codepoint_to_gid: &mut BTreeMap<u32, u16>,
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
            codepoint_to_gid.insert(cp[0] as u32, glyph_id as u16);
        }
    }
    names
}

// This builds a complete variable font
pub fn build_font(
    input: &babelfont::Font,
    subset: &Option<HashSet<String>>,
    just_one_master: Option<usize>,
) -> font::Font {
    // Previously, this function took a norad UFO font and could mutate it,
    // to decompose any mixed glyphs (see functions below). But now we have moved
    // to babelfont we would like to have a method on the font which does the
    // decomposition, but this method has not been written yet.

    // input.decompose_mixed_glyphs();

    // First, find the glyphs we're dealing with
    let mut codepoint_to_gid: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();
    let names =
        get_glyph_names_and_mapping(&input, &mut codepoint_to_gid, &mut name_to_id, &subset);

    let true_model = input
        .variation_model()
        .expect("Couldn't get variation model");

    let default_master_ix;
    let base_master;
    let variation_model;

    if let Some(master_ix) = just_one_master {
        // Oh, actually, we're not building a variable font. Just pick a master
        // and pretend that's the only thing in the font.
        default_master_ix = 0;
        base_master = input.masters.get(master_ix).unwrap();
        variation_model = None;
    } else {
        default_master_ix = input
            .default_master_index()
            .expect("Couldn't find default master");

        // Unused, but needs to have the same type...
        base_master = input.masters.get(default_master_ix).unwrap();

        variation_model = Some(&true_model);
    }

    // The guts of this thing is the big, parallel babelfont::Glyph to glyph::Glyph convertor.
    let result: Vec<(glyf::Glyph, hmtx::Metric, Option<GlyphVariationData>)> = input
        .glyphs
        .par_iter()
        .map(|glif| {
            // Check if we are included in the subset
            if subset.is_some() && !subset.as_ref().unwrap().contains(&glif.name.to_string()) {
                return None;
            }

            let all_layers: Vec<Option<&Layer>> = if just_one_master.is_none() {
                // Find all layers for this glyph across the designspace
                input
                    .masters
                    .iter()
                    .map(|master| input.master_layer_for(&glif.name, master))
                    .collect()
            } else {
                // Nobody here but us chickens
                vec![input.master_layer_for(&glif.name, base_master)]
            };

            // Convert them to OT glyph objects, plus variation data
            let (glyph, variation) = layers_to_glyph(
                default_master_ix,
                &name_to_id,
                &all_layers,
                variation_model,
                &glif.name,
            );

            // Build a basic hmtx entry
            let advance_width = input
                .master_layer_for(&glif.name, input.default_master().unwrap())
                .unwrap()
                .width as u16;
            let metric = hmtx::Metric {
                advanceWidth: advance_width,
                lsb: 0, // Dummy LSB because we will recalculate it later
            };

            // Return them all together
            Some((glyph, metric, variation))
        })
        .filter_map(|e| e)
        .collect();

    // We build the per-glyph data in parallel tuples, but now we want them
    // split into individual font-level vecs
    let (glyphs, mut metrics, variations) = result.into_iter().unzip_n_vec();

    let mut glyf_table = glyf::glyf { glyphs };

    // We built the glyphs in parallel (FOR SPEED) which means that some glyphs
    // which used components may have been built before the component glyphs that
    // they use. Obviously their glyph bounds will be undetermined until all the
    // components are available. Now that we're done building the glyphs, we have
    // to go over the whole glyf table again and recalculate the bounds.
    glyf_table.recalc_bounds();
    for (id, glyph) in glyf_table.glyphs.iter().enumerate() {
        metrics[id].lsb = glyph.xMin;
    }

    // Build the font with glyf + static metadata tables
    let mut font = fill_tables(&input, glyf_table, metrics, names, codepoint_to_gid);

    // Feature writers (temporary hack)
    let gpos_table = build_kerning(input, &name_to_id);
    font.tables.insert(*b"GPOS", Table::GPOS(gpos_table));

    if just_one_master.is_none() {
        // Put the gvar table in there
        let gvar_table = fonttools::gvar::gvar { variations };
        font.tables
            .insert(*b"gvar", Table::Unknown(gvar_table.to_bytes(None)));
        // No gvar optimization by default (use ttf-optimize-gvar for IUP)
    }

    font
}

// *This* function is unused, because...
/*
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

// ... this function needs to be adapted to Babelfont.
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
*/
