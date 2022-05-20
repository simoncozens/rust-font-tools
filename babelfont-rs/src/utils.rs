use core::cmp::Ordering;

pub(crate) fn ot_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

pub(crate) fn otcmp(a: f32, b: f32) -> Ordering {
    ot_round(a * 16384.0).cmp(&ot_round(b * 16384.0))
}

pub(crate) fn piecewise_linear_map(mapping: &[(f32, f32)], value: f32) -> f32 {
    if let Some(exact) = mapping
        .iter()
        .find(|(a, _b)| otcmp(*a, value) == Ordering::Equal)
    {
        return exact.1;
    }
    if mapping.is_empty() {
        return value;
    }
    let (min, mapped_min) = mapping.first().unwrap();
    if otcmp(value, *min) == Ordering::Less {
        return value + mapped_min - min;
    }
    let (max, mapped_max) = mapping.last().unwrap();
    if otcmp(value, *max) == Ordering::Greater {
        return value + mapped_max - max;
    }
    let (a, va) = mapping
        .iter()
        .filter(|(k, _v)| otcmp(*k, value) == Ordering::Less)
        .max_by(|(k1, _v1), (k2, _v2)| otcmp(*k1, *k2))
        .unwrap();

    let (b, vb) = mapping
        .iter()
        .filter(|(k, _v)| otcmp(*k, value) == Ordering::Greater)
        .min_by(|(k1, _v1), (k2, _v2)| otcmp(*k1, *k2))
        .unwrap();
    va + (vb - va) * (value - a) / (*b - *a)
}

pub(crate) fn normalize_value(mut l: f32, min: f32, max: f32, default: f32) -> f32 {
    if l < min {
        l = min;
    }
    if l > max {
        l = max;
    }
    if l < default {
        -(default - l) / (default - min) as f32
    } else if l > default {
        (l - default) / (max - default) as f32
    } else {
        0_f32
    }
}
