use crate::interpolate::interpolate_field;
use norad::FontInfo;
use otmath::{Location, VariationModel};

pub(crate) fn interpolate_fontinfo(
    fontinfo: &mut FontInfo,
    others: &[Option<&FontInfo>],
    vm: &VariationModel<String>,
    target_location: &Location<String>,
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
