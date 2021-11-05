use crate::i18ndictionary::I18NDictionary;
use crate::BabelfontError;
use fonttools::tables::fvar::VariationAxisRecord;
use fonttools::types::Tag;
use std::collections::HashMap;
use std::convert::TryInto;
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

fn piecewise_linear_map(mapping: HashMap<i32, f32>, value: i32) -> f32 {
    if mapping.contains_key(&value) {
        return *mapping.get(&value).unwrap();
    }
    if mapping.keys().len() == 0 {
        return value as f32;
    }
    let min = *mapping.keys().min().unwrap();
    if value < min {
        return value as f32 + *mapping.get(&min).unwrap() - (min as f32);
    }
    let max = *mapping.keys().max().unwrap();
    if value > max {
        return value as f32 + mapping.get(&max).unwrap() - (max as f32);
    }
    let a = mapping.keys().filter(|k| *k < &value).max().unwrap();
    let b = mapping.keys().filter(|k| *k > &value).min().unwrap();
    let va = mapping.get(a).unwrap();
    let vb = mapping.get(b).unwrap();
    va + (vb - va) * (value - a) as f32 / (*b - *a) as f32
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
    pub fn designspace_to_userspace(&self, l: i32) -> f32 {
        let mut mapping: HashMap<i32, f32> = HashMap::new();
        if self.map.is_none() {
            return l as f32;
        }
        for m in self.map.as_ref().unwrap().iter() {
            mapping.insert(m.1 as i32, m.0);
        }
        piecewise_linear_map(mapping, l)
    }

    /// Converts a position on this axis in userspace coordinates to designspace coordinates
    pub fn userspace_to_designspace(&self, l: i32) -> f32 {
        let mut mapping: HashMap<i32, f32> = HashMap::new();
        if self.map.is_none() {
            return l as f32;
        }
        for m in self.map.as_ref().unwrap().iter() {
            mapping.insert(m.0 as i32, m.1);
        }

        piecewise_linear_map(mapping, l)
    }

    pub fn tag_as_tag(&self) -> Tag {
        Tag::from_raw(self.tag.as_bytes()).unwrap()
    }

    pub fn normalize_userspace_value(&self, mut l: f32) -> Result<f32, BabelfontError> {
        let min = self.min.ok_or_else(|| BabelfontError::IllDefinedAxis {
            axis_name: self.name.default(),
        })?;
        let max = self.max.ok_or_else(|| BabelfontError::IllDefinedAxis {
            axis_name: self.name.default(),
        })?;
        let default = self.default.ok_or_else(|| BabelfontError::IllDefinedAxis {
            axis_name: self.name.default(),
        })?;

        if l < min {
            l = min;
        }
        if l > max {
            l = max;
        }
        if l < default {
            Ok(-(default - l) / (default - min) as f32)
        } else if l > default {
            Ok((l - default) / (max - default) as f32)
        } else {
            Ok(0_f32)
        }
    }
    pub fn normalize_designspace_value(&self, mut l: f32) -> Result<f32, BabelfontError> {
        if self.map.is_none() || self.map.as_ref().unwrap().is_empty() {
            return self.normalize_userspace_value(l);
        }
        let designspace_min = self
            .map
            .as_ref()
            .unwrap()
            .iter()
            .map(|m| m.1)
            .fold(1. / 0., f32::min);
        let designspace_max = self
            .map
            .as_ref()
            .unwrap()
            .iter()
            .map(|m| m.1)
            .fold(-1. / 0., f32::max);
        if l < designspace_min {
            l = designspace_min;
        }
        if l > designspace_max {
            l = designspace_max;
        }
        Ok((l - designspace_min) / (designspace_max - designspace_min))
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
