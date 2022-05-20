use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use fonttools::types::Tag;

const DFLT: Tag = fonttools::tag!("dflt");

#[derive(Default)]
pub struct I18NDictionary(pub HashMap<Tag, String>);

impl I18NDictionary {
    pub fn new() -> Self {
        I18NDictionary::default()
    }

    pub fn get_default(&self) -> Option<String> {
        self.0.get(b"dflt").map(|x| x.to_string())
    }

    pub fn set_default(&mut self, s: String) {
        self.0.insert(DFLT, s);
    }
}

impl Debug for I18NDictionary {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        fmt.write_str("<")?;
        let def = self.get_default();
        if def.is_some() {
            fmt.write_str(&def.unwrap())?;
        } else {
            fmt.write_str("no default")?;
        }
        fmt.write_str(">")
    }
}

impl From<String> for I18NDictionary {
    fn from(val: String) -> Self {
        let mut f = I18NDictionary::new();
        f.0.insert(DFLT, val);
        f
    }
}

impl From<&str> for I18NDictionary {
    fn from(val: &str) -> Self {
        let mut f = I18NDictionary::new();
        f.0.insert(DFLT, val.to_string());
        f
    }
}

impl From<&String> for I18NDictionary {
    fn from(val: &String) -> Self {
        let mut f = I18NDictionary::new();
        f.0.insert(DFLT, val.to_string());
        f
    }
}
