use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

static DFLT: &str = "dflt";

#[derive(Default, Clone)]
pub struct I18NDictionary(pub HashMap<String, String>);

impl I18NDictionary {
    pub fn new() -> Self {
        I18NDictionary::default()
    }

    pub fn get_default(&self) -> Option<&String> {
        self.0.get(DFLT)
    }

    pub fn set_default(&mut self, s: String) {
        self.0.insert(DFLT.to_string(), s);
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
        f.0.insert(DFLT.to_string(), val);
        f
    }
}

impl From<&str> for I18NDictionary {
    fn from(val: &str) -> Self {
        let mut f = I18NDictionary::new();
        f.0.insert(DFLT.to_string(), val.to_string());
        f
    }
}

impl From<&String> for I18NDictionary {
    fn from(val: &String) -> Self {
        let mut f = I18NDictionary::new();
        f.0.insert(DFLT.to_string(), val.to_string());
        f
    }
}
