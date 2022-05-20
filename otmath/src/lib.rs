/// Mathematical operations useful for OpenType fonts
use core::cmp::Ordering;

/// Round a number to a 32 bit integer using OpenType rounding
///
/// The OpenType spec (in the section on ["normalization" of OpenType Font
/// Variations](https://docs.microsoft.com/en-us/typography/opentype/spec/otvaroverview#coordinate-scales-and-normalization>)
/// defines the required method for converting floating point values to
/// fixed-point. In particular it specifies the following rounding strategy:
/// > for fractional values of 0.5 and higher, take the next higher integer;
/// > for other fractional values, truncate.
/// This function rounds the floating-point value according to this strategy
/// in preparation for conversion to fixed-point.
pub fn ot_round<T>(value: T) -> i32
where
    T: Into<f64>,
{
    (value.into() as f32 + 0.5_f32).floor() as i32
}

/// Compare two floating point values using their OpenType fixed-point
/// equivalents.
pub fn ot_cmp(a: f32, b: f32) -> Ordering {
    ot_round(a * 16384.0).cmp(&ot_round(b * 16384.0))
}

/// Perform a piecewise linear mapping of a value.
///
/// This is most used to remap the design space of a variable font.
pub fn piecewise_linear_map(mapping: &[(f32, f32)], value: f32) -> f32 {
    if let Some(exact) = mapping
        .iter()
        .find(|(a, _b)| ot_cmp(*a, value) == Ordering::Equal)
    {
        return exact.1;
    }
    if mapping.is_empty() {
        return value;
    }
    let (min, mapped_min) = mapping.first().unwrap();
    if ot_cmp(value, *min) == Ordering::Less {
        return value + mapped_min - min;
    }
    let (max, mapped_max) = mapping.last().unwrap();
    if ot_cmp(value, *max) == Ordering::Greater {
        return value + mapped_max - max;
    }
    let (a, va) = mapping
        .iter()
        .filter(|(k, _v)| ot_cmp(*k, value) == Ordering::Less)
        .max_by(|(k1, _v1), (k2, _v2)| ot_cmp(*k1, *k2))
        .unwrap();

    let (b, vb) = mapping
        .iter()
        .filter(|(k, _v)| ot_cmp(*k, value) == Ordering::Greater)
        .min_by(|(k1, _v1), (k2, _v2)| ot_cmp(*k1, *k2))
        .unwrap();
    va + (vb - va) * (value - a) / (*b - *a)
}

/// Normalize a value along a min/default/max triplet to the range -1.0/0.0/1.0.
pub fn normalize_value(mut l: f32, min: f32, max: f32, default: f32) -> f32 {
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
