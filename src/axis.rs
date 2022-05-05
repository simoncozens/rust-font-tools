use crate::i18ndictionary::I18NDictionary;
use crate::BabelfontError;
use core::cmp::Ordering;
use fonttools::{tables::fvar::VariationAxisRecord, types::Tag};
use uuid::Uuid;

#[derive(Debug)]
pub struct Axis {
    pub name: I18NDictionary,
    pub tag: String,
    pub id: Uuid,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub default: Option<f32>,
    pub map: Option<Vec<(f32, f32)>>,
    pub hidden: bool, // lib
}

fn ot_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

fn otcmp(a: f32, b: f32) -> Ordering {
    ot_round(a * 16384.0).cmp(&ot_round(b * 16384.0))
}

fn piecewise_linear_map(mapping: &[(f32, f32)], value: f32) -> f32 {
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

fn normalize_value(mut l: f32, min: f32, max: f32, default: f32) -> f32 {
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

impl Axis {
    pub fn new<T>(name: T, tag: String) -> Self
    where
        T: Into<I18NDictionary>,
    {
        Axis {
            name: name.into(),
            tag,
            id: Uuid::new_v4(),
            min: None,
            max: None,
            default: None,
            map: None,
            hidden: false,
        }
    }

    pub fn bounds(&self) -> Option<(f32, f32, f32)> {
        if self.min.is_none() || self.default.is_none() || self.max.is_none() {
            return None;
        }
        Some((self.min.unwrap(), self.default.unwrap(), self.max.unwrap()))
    }

    /// Converts a position on this axis from designspace coordinates to userspace coordinates
    pub fn designspace_to_userspace(&self, l: f32) -> f32 {
        if let Some(map) = &self.map {
            let inverted_map: Vec<(f32, f32)> = map.iter().map(|(a, b)| (*b, *a)).collect();
            piecewise_linear_map(&inverted_map, l)
        } else {
            l as f32
        }
    }

    /// Converts a position on this axis in userspace coordinates to designspace coordinates
    pub fn userspace_to_designspace(&self, l: f32) -> f32 {
        if let Some(map) = &self.map {
            piecewise_linear_map(map, l)
        } else {
            l as f32
        }
    }

    pub fn tag_as_tag(&self) -> Tag {
        Tag::from_raw(self.tag.as_bytes()).unwrap()
    }

    pub fn normalize_userspace_value(&self, l: f32) -> Result<f32, BabelfontError> {
        let min = self.min.ok_or_else(|| BabelfontError::IllDefinedAxis {
            axis_name: self.name.default(),
        })?;
        let max = self.max.ok_or_else(|| BabelfontError::IllDefinedAxis {
            axis_name: self.name.default(),
        })?;
        let default = self.default.ok_or_else(|| BabelfontError::IllDefinedAxis {
            axis_name: self.name.default(),
        })?;
        Ok(normalize_value(l, min, max, default))
    }
    pub fn normalize_designspace_value(&self, l: f32) -> Result<f32, BabelfontError> {
        if self.map.is_none() || self.map.as_ref().unwrap().is_empty() {
            return self.normalize_userspace_value(l);
        }
        let min = self.min.ok_or_else(|| BabelfontError::IllDefinedAxis {
            axis_name: self.name.default(),
        })?;
        let max = self.max.ok_or_else(|| BabelfontError::IllDefinedAxis {
            axis_name: self.name.default(),
        })?;
        let default = self.default.ok_or_else(|| BabelfontError::IllDefinedAxis {
            axis_name: self.name.default(),
        })?;
        Ok(normalize_value(
            l,
            self.userspace_to_designspace(min),
            self.userspace_to_designspace(max),
            self.userspace_to_designspace(default),
        ))
    }

    pub fn to_variation_axis_record(
        &self,
        name_id: u16,
    ) -> Result<VariationAxisRecord, BabelfontError> {
        if self.tag.len() != 4 {
            return Err(BabelfontError::General {
                msg: format!("Badly formatted axis tag: {}", self.tag),
            });
        }
        Ok(VariationAxisRecord {
            axisTag: Tag::from_raw(self.tag.as_bytes()).unwrap(),
            defaultValue: self.default.expect("Bad axis") as f32,
            maxValue: self.max.expect("Bad axis") as f32,
            minValue: self.min.expect("Bad axis") as f32,
            flags: if self.hidden { 0x0001 } else { 0x0000 },
            axisNameID: name_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! assert_ot_eq {
        ($left:expr, $right:expr) => {{
            match (&$left, &$right) {
                (left_val, right_val) => {
                    if otcmp(*left_val, *right_val) != Ordering::Equal {
                        panic!(
                            r#"assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{:?}`"#,
                            left_val, right_val
                        )
                    }
                }
            }
        }};
    }

    #[test]
    fn test_linear_map() {
        let mut weight = Axis::new("Weight".to_string(), "wght".to_string());
        weight.min = Some(100.0);
        weight.max = Some(900.0);
        weight.default = Some(100.0);
        weight.map = Some(vec![(100.0, 10.0), (900.0, 90.0)]);

        assert_eq!(weight.userspace_to_designspace(400.0), 40.0);
        assert_eq!(weight.designspace_to_userspace(40.0), 400.0);
    }

    #[test]
    fn test_nonlinear_map() {
        let mut weight = Axis::new("Weight".to_string(), "wght".to_string());
        weight.min = Some(200.0);
        weight.max = Some(1000.0);
        weight.default = Some(200.0);
        weight.map = Some(vec![
            (200.0, 42.0),
            (300.0, 61.0),
            (400.0, 81.0),
            (600.0, 101.0),
            (700.0, 125.0),
            (800.0, 151.0),
            (900.0, 178.0),
            (1000.0, 208.0),
        ]);

        assert_ot_eq!(weight.userspace_to_designspace(250.0), 51.5);
        assert_ot_eq!(weight.designspace_to_userspace(138.0), 750.0);
    }

    #[test]
    fn test_normalize_map() {
        let mut opsz = Axis::new("Optical Size".to_string(), "opsz".to_string());
        opsz.min = Some(17.0);
        opsz.max = Some(18.0);
        opsz.default = Some(18.0);
        opsz.map = Some(vec![(17.0, 17.0), (17.99, 17.1), (18.0, 18.0)]);
        assert_ot_eq!(opsz.normalize_userspace_value(17.99).unwrap(), -0.01);
        assert_ot_eq!(opsz.normalize_designspace_value(17.1).unwrap(), -0.9);
    }
}
