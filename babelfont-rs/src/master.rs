use crate::common::{Location, OTValue};
use crate::guide::Guide;
use crate::i18ndictionary::I18NDictionary;
use crate::OTScalar;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Master {
    pub name: I18NDictionary,
    pub id: String,
    pub location: Location,
    pub guides: Vec<Guide>,
    pub metrics: HashMap<String, i32>,
    pub kerning: HashMap<(String, String), i16>,
    pub custom_ot_values: Vec<OTValue>,
    // lib
}

impl Master {
    pub fn new<T, U>(name: T, id: U, location: Location) -> Self
    where
        T: Into<I18NDictionary>,
        U: Into<String>,
    {
        Master {
            name: name.into(),
            id: id.into(),
            location,
            guides: vec![],
            metrics: HashMap::new(),
            kerning: HashMap::new(),
            custom_ot_values: vec![],
        }
    }

    pub fn ot_value(&self, table: &str, field: &str) -> Option<OTScalar> {
        for i in &self.custom_ot_values {
            if i.table == table && i.field == field {
                return Some(i.value.clone());
            }
        }
        None
    }

    pub fn set_ot_value(&mut self, table: &str, field: &str, value: OTScalar) {
        self.custom_ot_values.push(OTValue {
            table: table.to_string(),
            field: field.to_string(),
            value,
        })
    }
}
