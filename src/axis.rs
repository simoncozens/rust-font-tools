use crate::common::Tag;
use crate::i18ndictionary::I18NDictionary;
use crate::BabelfontError;
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
    pub map: Option<HashMap<f32, f32>>,
    pub hidden: bool, // lib
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

    pub fn map_forward(&self, designspace: f32) -> f32 {
        designspace // This is evil and wrong
    }
    pub fn tag_as_tag(&self) -> Tag {
        self.tag.as_bytes()[0..4].try_into().unwrap()
    }

    fn normalize_userspace_value(&self, mut l: f32) -> Result<f32, BabelfontError> {
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
        let designspace_minimum = self
            .map
            .as_ref()
            .unwrap()
            .iter()
            .map(|m| *m.1)
            .fold(1. / 0., f32::min);
        let designspace_maximum = self
            .map
            .as_ref()
            .unwrap()
            .iter()
            .map(|m| *m.1)
            .fold(-1. / 0., f32::max);
        if l < designspace_minimum {
            l = designspace_minimum;
        }
        if l > designspace_maximum {
            l = designspace_maximum;
        }
        Ok((l - designspace_minimum) / (designspace_maximum - designspace_minimum))
    }
}
