use crate::interpolate::interpolate_field;
use norad::Glyph;
use otmath::{Location, VariationModel};

pub(crate) fn interpolate_glyph(
    g: &mut Glyph,
    others: &[Option<&Glyph>],
    vm: &VariationModel<String>,
    target_location: &Location<String>,
) {
    interpolate_field(
        g,
        others,
        vm,
        target_location,
        get_contour_numbers,
        set_contour_numbers,
    );
    interpolate_field(
        g,
        others,
        vm,
        target_location,
        get_anchor_numbers,
        set_anchor_numbers,
    );
    interpolate_field(
        g,
        others,
        vm,
        target_location,
        get_component_numbers,
        set_component_numbers,
    );
    interpolate_field(g, others, vm, target_location, get_width, set_width);
}

fn get_contour_numbers(g: &Glyph) -> ndarray::Array1<f64> {
    let mut v = vec![];
    for contour in g.contours.iter() {
        for p in &contour.points {
            v.push(p.x);
            v.push(p.y);
        }
    }
    let len = v.len();
    ndarray::Array1::from_shape_vec(len, v).unwrap()
}

fn set_contour_numbers(g: &mut Glyph, values: &[f64]) {
    let mut i = 0;
    for contour in g.contours.iter_mut() {
        for p in contour.points.iter_mut() {
            p.x = *values.get(i).unwrap();
            i += 1;
            p.y = *values.get(i).unwrap();
            i += 1;
        }
    }
}

fn get_anchor_numbers(g: &Glyph) -> ndarray::Array1<f64> {
    let mut v = vec![];
    for anchor in &g.anchors {
        v.push(anchor.x);
        v.push(anchor.y);
    }
    let len = v.len();
    ndarray::Array1::from_shape_vec(len, v).unwrap()
}

fn set_anchor_numbers(g: &mut Glyph, values: &[f64]) {
    let mut i = 0;
    for anchor in g.anchors.iter_mut() {
        anchor.x = *values.get(i).unwrap();
        i += 1;
        anchor.y = *values.get(i).unwrap();
        i += 1;
    }
}

fn get_component_numbers(g: &Glyph) -> ndarray::Array1<f64> {
    let mut v = vec![];
    for component in g.components.iter() {
        v.push(component.transform.x_scale);
        v.push(component.transform.xy_scale);
        v.push(component.transform.yx_scale);
        v.push(component.transform.y_scale);
        v.push(component.transform.x_offset);
        v.push(component.transform.y_offset);
    }
    let len = v.len();
    ndarray::Array1::from_shape_vec(len, v).unwrap()
}

fn set_component_numbers(g: &mut Glyph, values: &[f64]) {
    let mut i = 0;
    for component in g.components.iter_mut() {
        component.transform.x_scale = *values.get(i).unwrap();
        i += 1;
        component.transform.xy_scale = *values.get(i).unwrap();
        i += 1;
        component.transform.yx_scale = *values.get(i).unwrap();
        i += 1;
        component.transform.y_scale = *values.get(i).unwrap();
        i += 1;
        component.transform.x_offset = *values.get(i).unwrap();
        i += 1;
        component.transform.y_offset = *values.get(i).unwrap();
        i += 1;
    }
}

fn get_width(g: &Glyph) -> ndarray::Array1<f64> {
    ndarray::Array1::from_shape_vec(1, vec![g.width]).unwrap()
}

fn set_width(g: &mut Glyph, values: &[f64]) {
    g.width = values[0];
}
