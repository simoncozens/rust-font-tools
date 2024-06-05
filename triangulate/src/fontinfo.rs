use crate::interpolate::interpolate_field;
use crate::noradextensions::FromString;
use norad::designspace::Instance;
use norad::fontinfo::StyleMapStyle;
use norad::FontInfo;
use otmath::{Location, VariationModel};

pub(crate) fn interpolate_fontinfo(
    fontinfo: &mut FontInfo,
    others: &[Option<&FontInfo>],
    vm: &VariationModel<String>,
    target_location: &Location<String>,
    instance: Option<&Instance>,
) {
    if fontinfo.postscript_blue_values.is_some() {
        interpolate_field(
            fontinfo,
            others,
            vm,
            target_location,
            get_postscript_blues,
            set_postscript_blues,
        );
    }
    if fontinfo.postscript_other_blues.is_some() {
        interpolate_field(
            fontinfo,
            others,
            vm,
            target_location,
            get_postscript_other_blues,
            set_postscript_other_blues,
        );
    }
    if fontinfo.postscript_family_blues.is_some() {
        interpolate_field(
            fontinfo,
            others,
            vm,
            target_location,
            get_postscript_family_blues,
            set_postscript_family_blues,
        );
    }
    if fontinfo.x_height.is_some() {
        interpolate_field(
            fontinfo,
            others,
            vm,
            target_location,
            get_x_height,
            set_x_height,
        );
    }
    if fontinfo.postscript_family_other_blues.is_some() {
        interpolate_field(
            fontinfo,
            others,
            vm,
            target_location,
            get_postscript_family_other_blues,
            set_postscript_family_other_blues,
        );
    }

    if fontinfo.postscript_stem_snap_h.is_some() {
        interpolate_field(
            fontinfo,
            others,
            vm,
            target_location,
            get_postscript_stem_snap_h,
            set_postscript_stem_snap_h,
        );
    }
    if fontinfo.postscript_stem_snap_v.is_some() {
        interpolate_field(
            fontinfo,
            others,
            vm,
            target_location,
            get_postscript_stem_snap_v,
            set_postscript_stem_snap_v,
        );
    }
    if let Some(instance) = instance {
        fontinfo.style_name = instance.stylename.clone();
        fontinfo.style_map_family_name = instance.stylemapfamilyname.clone();
        fontinfo.style_map_style_name = instance
            .stylemapstylename
            .as_ref()
            .map(|x| StyleMapStyle::from_string(x));
        fontinfo.postscript_font_name = instance.postscriptfontname.clone();
        // openTypeOS2Panose
        // openTypeOS2WeightClass
        // openTypeOS2WidthClass
        // styleMapFamilyName
        // styleName
    }

    // openTypeOS2Panose
    // openTypeOS2WeightClass
    // openTypeOS2WidthClass
    // styleMapFamilyName
    // styleName
}

fn get_postscript_blues(fi: &FontInfo) -> ndarray::Array1<f64> {
    fi.postscript_blue_values.clone().unwrap_or_default().into()
}

fn set_postscript_blues(fi: &mut FontInfo, values: &[f64]) {
    fi.postscript_blue_values = Some(values.to_vec());
}

fn get_postscript_other_blues(fi: &FontInfo) -> ndarray::Array1<f64> {
    fi.postscript_other_blues.clone().unwrap_or_default().into()
}

fn set_postscript_other_blues(fi: &mut FontInfo, values: &[f64]) {
    fi.postscript_other_blues = Some(values.to_vec());
}

fn get_postscript_family_blues(fi: &FontInfo) -> ndarray::Array1<f64> {
    fi.postscript_family_blues
        .clone()
        .unwrap_or_default()
        .into()
}

fn set_postscript_family_blues(fi: &mut FontInfo, values: &[f64]) {
    fi.postscript_family_blues = Some(values.to_vec());
}

fn get_postscript_family_other_blues(fi: &FontInfo) -> ndarray::Array1<f64> {
    fi.postscript_family_other_blues
        .clone()
        .unwrap_or_default()
        .into()
}

fn set_postscript_family_other_blues(fi: &mut FontInfo, values: &[f64]) {
    fi.postscript_family_other_blues = Some(values.to_vec());
}

fn get_postscript_stem_snap_h(fi: &FontInfo) -> ndarray::Array1<f64> {
    fi.postscript_stem_snap_h.clone().unwrap_or_default().into()
}

fn set_postscript_stem_snap_h(fi: &mut FontInfo, values: &[f64]) {
    fi.postscript_stem_snap_h = Some(values.to_vec());
}

fn get_postscript_stem_snap_v(fi: &FontInfo) -> ndarray::Array1<f64> {
    fi.postscript_stem_snap_v.clone().unwrap_or_default().into()
}

fn set_postscript_stem_snap_v(fi: &mut FontInfo, values: &[f64]) {
    fi.postscript_stem_snap_v = Some(values.to_vec());
}

fn get_x_height(fi: &FontInfo) -> ndarray::Array1<f64> {
    ndarray::Array1::from_shape_vec(1, vec![fi.x_height.unwrap_or_default()]).unwrap()
}

fn set_x_height(fi: &mut FontInfo, values: &[f64]) {
    fi.x_height = Some(values[0]);
}
