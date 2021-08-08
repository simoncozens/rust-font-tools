use crate::basictables::fill_tables;
use crate::glyph::glifs_to_glyph;
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
unzip_n!(2);

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

// We are going to be building the glyphs in parallel (FOR SPEED) which means
// that some glyphs which use components might be built before the component
// glyphs that they use. Obviously their glyph bounds will be undetermined
// until the components are available. This means that once we're done building
// the glyphs, we have to go over the whole glyf table again and recalculate the
// bounds.
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

// We collect here the information for the `cmap` table (`mapping`); a mapping
// of glyph names to eventual glyph IDs (`name_to_id`) which will be used when
// resolving components; and the list of glyph names (return value) which will
// be used in the `post` table.
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

// This builds a complete variable font
pub fn build_font(input: &babelfont::Font, subset: &Option<HashSet<String>>) -> font::Font {
    // input.decompose_mixed_glyphs();

    // First, find the glyphs we're dealing with
    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();
    let names = get_glyph_names_and_mapping(&input, &mut mapping, &mut name_to_id, &subset);

    let variation_model = input
        .variation_model()
        .expect("Couldn't get variation model");

    let default_master_ix = input
        .default_master_index()
        .expect("Couldn't find default master");

    // The guts of this thing is the big, parallel babelfont::Glyph to glyph::Glyph convertor.
    let result: Vec<(glyf::Glyph, hmtx::Metric, Option<GlyphVariationData>)> = input
        .glyphs
        .par_iter()
        .map(|glif| {
            // Check if we are included in the subset
            if subset.is_some() && !subset.as_ref().unwrap().contains(&glif.name.to_string()) {
                return None;
            }

            // Find all glyph layers
            let all_layers: Vec<Option<&Layer>> = input
                .masters
                .iter()
                .map(|master| input.master_layer_for(&glif.name, master))
                .collect();

            // Convert them to OT glyph objects, plus variation data
            let (glyph, variation) = glifs_to_glyph(
                default_master_ix,
                &name_to_id,
                &all_layers,
                Some(&variation_model),
                &glif.name,
            );

            // Build a basic hmtx entry
            let advance_width = input
                .master_layer_for(&glif.name, input.default_master().unwrap())
                .unwrap()
                .width as u16;

            // Return them all together
            Some((
                glyph,
                hmtx::Metric {
                    advanceWidth: advance_width,
                    lsb: 0, // Dummy LSB because we will recalculate it later
                },
                variation,
            ))
        })
        .filter_map(|e| e)
        .collect();

    let (glyphs, mut metrics, variations) = result.into_iter().unzip_n_vec();

    let glyf_table = form_glyf_and_fix_bounds(glyphs, &mut metrics);

    // Build the font with glyf + static metadata tables
    let mut font = fill_tables(&input, glyf_table, metrics, names, mapping);

    // Feature writers (temporary hack)
    let gpos_table = build_kerning(input, &name_to_id);
    font.tables.insert(*b"GPOS", Table::GPOS(gpos_table));

    // Put the gvar table in there
    let gvar_table = fonttools::gvar::gvar { variations };
    font.tables
        .insert(*b"gvar", Table::Unknown(gvar_table.to_bytes(None)));

    // No optimization by default

    font
}

// Basically the same as the variable version, but without the variations...
pub fn build_static_master(
    input: &babelfont::Font,
    subset: &Option<HashSet<String>>,
    master: usize,
) -> font::Font {
    // input.decompose_mixed_glyphs();

    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();
    let master = input.masters.get(master).expect("This can't be");
    let names = get_glyph_names_and_mapping(&input, &mut mapping, &mut name_to_id, &subset);
    let result: Vec<(glyf::Glyph, hmtx::Metric)> = input
        .glyphs
        .par_iter()
        .map(|glif| {
            if subset.is_some() && !subset.as_ref().unwrap().contains(&glif.name.to_string()) {
                return None;
            }
            let all_layers = vec![input.master_layer_for(&glif.name, master)];
            let (glyph, _) = glifs_to_glyph(0, &name_to_id, &all_layers, None, &glif.name);
            let lsb = glyph.xMin;
            let advance_width = input
                .master_layer_for(&glif.name, input.default_master().unwrap())
                .unwrap()
                .width as u16;
            Some((
                glyph,
                hmtx::Metric {
                    advanceWidth: advance_width,
                    lsb,
                },
            ))
        })
        .filter_map(|e| e)
        .collect();
    let (glyphs, mut metrics) = result.into_iter().unzip_n_vec();

    let glyf_table = form_glyf_and_fix_bounds(glyphs, &mut metrics);
    let mut font = fill_tables(&input, glyf_table, metrics, names, mapping);
    let gpos_table = build_kerning(input, &name_to_id);
    font.tables.insert(*b"GPOS", Table::GPOS(gpos_table));
    font
}
