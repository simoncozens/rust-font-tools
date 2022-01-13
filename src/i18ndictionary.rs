use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Formatter;

use fonttools::types::Tag;

const DFLT: Tag = fonttools::tag!("dflt");

pub struct I18NDictionary(pub HashMap<Tag, String>);

impl I18NDictionary {
    pub fn new() -> Self {
        I18NDictionary(HashMap::<Tag, String>::new())
    }

    pub fn default(&self) -> Option<String> {
        self.0.get(b"dflt").map(|x| x.to_string())
    }

    pub fn set_default(&mut self, s: String) {
        self.0.insert(DFLT, s);
    }
}

impl Debug for I18NDictionary {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        fmt.write_str("<")?;
        let def = self.default();
        if def.is_some() {
            fmt.write_str(&def.unwrap())?;
        } else {
            fmt.write_str("no default")?;
        }
        fmt.write_str(">")
    }
}

impl Into<I18NDictionary> for String {
    fn into(self) -> I18NDictionary {
        let mut f = I18NDictionary::new();
        f.0.insert(DFLT, self);
        f
    }
}

impl Into<I18NDictionary> for &str {
    fn into(self) -> I18NDictionary {
        let mut f = I18NDictionary::new();
        f.0.insert(DFLT, self.to_string());
        f
    }
}

impl Into<I18NDictionary> for &String {
    fn into(self) -> I18NDictionary {
        let mut f = I18NDictionary::new();
        f.0.insert(DFLT, self.to_string());
        f
    }
}
