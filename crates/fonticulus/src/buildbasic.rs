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
    layer: &norad::Layer,
    mapping: &mut BTreeMap<u32, u16>,
    name_to_id: &mut BTreeMap<String, u16>,
    subset: &Option<HashSet<String>>,
) -> Vec<String> {
    let mut names: Vec<String> = vec![];
    for (glyph_id, glyf) in layer.iter_contents().enumerate() {
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

pub fn build_font(mut ufo: norad::Font, include: Option<HashSet<String>>) -> font::Font {
    decompose_mixed_glyphs(&mut ufo);
    let layer = ufo.default_layer();
    let info = ufo.font_info.as_ref().unwrap();

    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();

    let names = get_glyph_names_and_mapping(&layer, &mut mapping, &mut name_to_id, &include);
    let glifs: Vec<&Arc<norad::Glyph>> = layer.iter().collect();
    let (glyphs, mut metrics): (Vec<glyf::Glyph>, Vec<hmtx::Metric>) = glifs
        .iter()
        .filter(|g| include.is_none() || include.as_ref().unwrap().contains(&g.name.to_string()))
        .map({
            |glyf| {
                let (glyph, _) = glifs_to_glyph(0, &name_to_id, &[Some(&glyf)], None);
                let lsb = glyph.xMin;
                let advanceWidth = glyf.width as u16;
                (glyph, hmtx::Metric { advanceWidth, lsb })
            }
        })
        .unzip();
    let glyf_table = form_glyf_and_fix_bounds(glyphs, &mut metrics);
    fill_tables(info, glyf_table, metrics, names, mapping)
}

pub fn build_fonts(
    default_master: usize,
    mut fonts: Vec<norad::Font>,
    variation_model: VariationModel,
    include: Option<HashSet<String>>,
) -> font::Font {
    for f in fonts.iter_mut() {
        decompose_mixed_glyphs(f);
    }
    let layer = fonts[default_master].default_layer();
    let info = fonts[default_master].font_info.as_ref().unwrap();
    let mut mapping: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();

    let names = get_glyph_names_and_mapping(&layer, &mut mapping, &mut name_to_id, &include);

    let glifs: Vec<&Arc<norad::Glyph>> = layer.iter().collect();

    let mut glyphs: Vec<glyf::Glyph> = vec![];
    let mut metrics: Vec<hmtx::Metric> = vec![];
    let mut variations: Vec<Option<GlyphVariationData>> = vec![];
    for glif in glifs {
        if include.is_some() && !include.as_ref().unwrap().contains(&glif.name.to_string()) {
            continue;
        }
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

    let glyf_table = form_glyf_and_fix_bounds(glyphs, &mut metrics);
    let mut font = fill_tables(info, glyf_table, metrics, names, mapping);
    let gvar_table = fonttools::gvar::gvar { variations };
    font.tables
        .insert(*b"gvar", Table::Unknown(gvar_table.to_bytes(None)));
    // No optimization by default

    font
}
