mod glyph;
use rayon::prelude::*;

use crate::glyph::{
    babelfont_component_to_glyf_component, ConvertedMaster, GlyphForConversion, GlyphReadyToGo,
    UnconvertedMaster,
};
use babelfont::{Font, Layer};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::wrap_pyfunction;
use std::collections::{BTreeMap, HashMap, HashSet};

fn get_glyph_names_and_mapping(
    input: &babelfont::Font,
    codepoint_to_gid: &mut BTreeMap<u32, u16>,
    name_to_id: &mut BTreeMap<String, u16>,
    subset: &HashSet<&String>,
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
        if subset.contains(&name) {
            continue;
        }
        let glyph_id = names.len();
        names.push(name.clone());
        name_to_id.insert(name, glyph_id as u16);
        for cp in &glyf.codepoints {
            codepoint_to_gid.insert(*cp as u32, glyph_id as u16);
        }
    }
    names
}

fn make_unconverted_master<'a>(
    layer: &'a babelfont::Layer,
    mapping: &BTreeMap<String, u16>,
) -> UnconvertedMaster<'a> {
    UnconvertedMaster {
        babelfont_contours: layer.paths().collect(),
        components: layer
            .components()
            .flat_map(|component| babelfont_component_to_glyf_component(component, mapping))
            .collect(),
        width: layer.width,
    }
}

/// Compiles a set of glyphs into compatible contours
#[pyfunction]
pub fn _compile(
    font_name: String,
    glyphs: Vec<String>,
) -> PyResult<HashMap<String, Vec<Option<Py<PyDict>>>>> {
    let font: Font = babelfont::load(&font_name)
        .map_err(|e| PyValueError::new_err(format!("Could not load font {}: {}", font_name, e)))?;
    let mut result: HashMap<String, Vec<Option<Py<PyDict>>>> = HashMap::new();

    let subset: HashSet<&String> = glyphs.iter().collect();
    let mut codepoint_to_gid: BTreeMap<u32, u16> = BTreeMap::new();
    let mut name_to_id: BTreeMap<String, u16> = BTreeMap::new();
    let names = get_glyph_names_and_mapping(&font, &mut codepoint_to_gid, &mut name_to_id, &subset);
    let reverse_map: BTreeMap<u16, String> =
        name_to_id.iter().map(|(x, y)| (*y, x.clone())).collect();

    let default_master_ix = font
        .default_master_index()
        .ok_or_else(|| PyValueError::new_err("Could not find default master"))?;
    let variation_model = font
        .variation_model()
        .map_err(|_| PyValueError::new_err("Couldn't get variation model"))?;

    let dones: Vec<GlyphReadyToGo> = glyphs
        .par_iter()
        .map(|g| {
            let all_layers: Vec<Option<&Layer>> = font
                .masters
                .iter()
                .map(|master| font.master_layer_for(g, master))
                .collect();
            let for_conversion = GlyphForConversion {
                masters: all_layers
                    .iter()
                    .map(|maybe_x| maybe_x.map(|x| make_unconverted_master(x, &name_to_id)))
                    .collect(),
                default_master_ix,
                model: Some(&variation_model),
                glif_name: g,
            };
            for_conversion.convert()
        })
        .collect();
    for (done, g) in dones.iter().zip(&glyphs) {
        result.insert(
            g.clone(),
            done.masters
                .iter()
                .map(|x| to_python_dict(x, &reverse_map))
                .collect(),
        );
    }
    Ok(result)
}

fn to_python_dict(
    in_glyph: &Option<ConvertedMaster>,
    glyphlist: &BTreeMap<u16, String>,
) -> Option<Py<PyDict>> {
    if let Some(in_glyph) = in_glyph {
        Python::with_gil(|py| {
            let pydict = PyDict::new(py);
            let mut coordinates: Vec<(i16, i16)> = vec![];
            let mut flags: Vec<i8> = vec![];
            let mut endpoints: Vec<usize> = vec![];
            let mut components: Vec<&PyDict> = vec![];
            for component in in_glyph.components.iter() {
                let name = glyphlist.get(&component.glyph_index).unwrap_or_else(|| {
                    panic!(
                        "Can't find glyph name for component {}",
                        component.glyph_index
                    )
                });
                let py_component = PyDict::new(py);
                py_component
                    .set_item("glyphName".to_object(py), name.to_object(py))
                    .expect("Can't happen");
                let coeffs = component.transformation.as_coeffs();
                py_component
                    .set_item("transform".to_object(py), coeffs.to_object(py))
                    .expect("Can't happen");

                py_component
                    .set_item("x".to_object(py), coeffs[4].to_object(py))
                    .expect("Can't happen");
                py_component
                    .set_item("y".to_object(py), coeffs[5].to_object(py))
                    .expect("Can't happen");

                components.push(py_component)
            }
            for path in in_glyph.glyf_contours.iter() {
                for point in path {
                    coordinates.push((point.x, point.y));
                    flags.push(if point.on_curve { 1 } else { 0 });
                }
                endpoints.push(coordinates.len() - 1);
            }
            pydict
                .set_item("coordinates".to_object(py), coordinates.to_object(py))
                .expect("Can't happen");
            pydict
                .set_item("flags".to_object(py), flags.to_object(py))
                .expect("Can't happen");
            pydict
                .set_item("endPtsOfContours".to_object(py), endpoints.to_object(py))
                .expect("Can't happen");
            pydict
                .set_item("components".to_object(py), components.to_object(py))
                .expect("Can't happen");
            Some(pydict.into())
        })
    } else {
        None
    }
}

/// This module is implemented in Rust.
#[pymodule]
fn glyphcompiler(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_compile, m)?)?;
    Ok(())
}
