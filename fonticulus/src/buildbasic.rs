use crate::basictables::fill_tables;
use crate::glyph::layers_to_glyph;
use crate::kerning::build_kerning;
use crate::notdef::add_notdef;
use babelfont::{Component, Font, Layer, Node, Path};
use fonttools::tables::gvar::GlyphVariationData;
use fonttools::tables::{glyf, hmtx};
use fonttools::{font, tag};
use std::collections::{BTreeMap, HashSet};
use unzip_n::unzip_n;

#[cfg(not(debug_assertions))]
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

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

    // If we have an explicit notdef, that must be first in the glyph order.
    if input.glyphs.get(".notdef").is_some() {
        let name = ".notdef".to_string();
        name_to_id.insert(name.clone(), 0);
        names.push(name);
    }
    for glyf in input.glyphs.iter() {
        let name = glyf.name.to_string();
        if name == ".notdef" {
            continue;
        }
        if subset.is_some() && !subset.as_ref().unwrap().contains(&name) {
            continue;
        }
        let glyph_id = names.len();
        names.push(name.clone());
        log::debug!("Assigning GID {:} to {:}", glyph_id, name);
        name_to_id.insert(name, glyph_id as u16);
        for cp in &glyf.codepoints {
            codepoint_to_gid.insert(*cp as u32, glyph_id as u16);
            log::debug!("Mapping U+{:04X} to {:}", cp, glyph_id);
        }
    }
    names
}

// This builds a complete variable font
pub fn build_font(
    input: &mut babelfont::Font,
    subset: &Option<HashSet<String>>,
    just_one_master: Option<usize>,
) -> font::Font {
    add_notdef(input);
    decompose_mixed_glyphs(input);

    // First, find the glyphs we're dealing with
    let mut codepoint_to_gid: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();
    let names = get_glyph_names_and_mapping(input, &mut codepoint_to_gid, &mut name_to_id, subset);

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

    // The guts of this thing is the big, parallel babelfont::Glyph to glyf::Glyph convertor.
    #[cfg(debug_assertions)]
    let glyph_iter = names.iter().map(|n| input.glyphs.get(n).unwrap());

    #[cfg(not(debug_assertions))]
    let glyph_iter = names.par_iter().map(|n| input.glyphs.get(n).unwrap());

    // This statement reads quite differently in release versus debug
    #[allow(clippy::needless_collect)]
    #[allow(clippy::filter_map_identity)]
    let result: Vec<(glyf::Glyph, hmtx::Metric, Option<GlyphVariationData>)> = glyph_iter
        .map(|glif| {
            // If we are subsetting, check if we are included in the subset
            if subset.is_some() && !subset.as_ref().unwrap().contains(&glif.name.to_string()) {
                return None;
            }

            // We do need to do this at some point, but we have to get decomposition
            // working properly first. That can come later, but for now it's
            // just breaking things, so export everything.

            // if !glif.exported {
            //     return None;
            // }

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

    // We built the per-glyph data in parallel tuples, but now we want them
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
    let mut font = fill_tables(input, glyf_table, metrics, names, codepoint_to_gid);

    // Feature writers (temporary hack)
    let gpos_table = build_kerning(input, &name_to_id);
    font.tables.insert(gpos_table);

    if just_one_master.is_none() && variations.iter().any(|x| x.is_some()) {
        // Put the gvar table in there
        let gvar_table = fonttools::tables::gvar::gvar { variations };
        font.tables
            .insert_raw(tag!("gvar"), gvar_table.to_bytes(None));
        // No gvar optimization by default (use ttf-optimize-gvar for IUP)
    }

    font
}

fn decomposed_components(layer: &Layer, font: &Font) -> Vec<Path> {
    let mut contours = Vec::new();

    let mut stack: Vec<(&Component, kurbo::Affine)> = Vec::new();

    for component in layer.components() {
        stack.push((component, component.transform));

        while let Some((component, transform)) = stack.pop() {
            let referenced_glyph = match font.glyphs.get(&component.reference) {
                Some(g) => g,
                None => continue,
            };
            let new_outline = match referenced_glyph.get_layer(layer.id.as_ref().unwrap()) {
                Some(g) => g,
                None => continue,
            };

            for contour in new_outline.paths() {
                let mut decomposed_contour = Path::default();
                for node in &contour.nodes {
                    let new_point = transform * kurbo::Point::new(node.x as f64, node.y as f64);
                    decomposed_contour.nodes.push(Node {
                        x: new_point.x as f32,
                        y: new_point.y as f32,
                        nodetype: node.nodetype,
                    })
                }
                decomposed_contour.closed = contour.closed;
                contours.push(decomposed_contour);
            }

            // We need to do this backwards.
            for new_component in new_outline.components().rev() {
                let new_transform: kurbo::Affine = new_component.transform;
                stack.push((new_component, transform * new_transform));
            }
        }
    }

    contours
}

fn decompose_mixed_glyphs(input: &mut babelfont::Font) {
    for master in &input.masters {
        let mut decomposed: BTreeMap<String, Vec<babelfont::Path>> = BTreeMap::new();
        for glif in input.glyphs.iter() {
            if let Some(layer) = glif.get_layer(&master.id) {
                decomposed.insert(glif.name.to_string(), decomposed_components(layer, input));
            }
        }
        for glif in input.glyphs.iter_mut() {
            let name = glif.name.to_string();
            if let Some(layer) = glif.get_layer_mut(&master.id) {
                if !layer.has_components() || !layer.has_paths() {
                    continue;
                }
                if let Some(contours) = decomposed.get(&name) {
                    for c in contours {
                        layer.push_path(c.clone());
                    }
                    layer.clear_components();
                    log::info!("Decomposed mixed glyph {:?}", glif.name);
                }
            }
        }
    }
}
