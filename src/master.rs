use crate::common::Location;
use crate::guide::Guide;
use crate::i18ndictionary::I18NDictionary;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Master {
    pub name: I18NDictionary,
    pub id: String,
    pub location: Location,
    pub guides: Vec<Guide>,
    pub metrics: HashMap<String, i32>,
    pub kerning: HashMap<(String, String), i32>,
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
        }
    }
}
