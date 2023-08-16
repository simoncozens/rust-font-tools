use crate::constants::{NameID, RIBBI_STYLE_NAMES};
use fonttools::font::Font;
use std::error::Error;

#[derive(Debug)]
pub struct TestFont {
    pub filename: String,
    pub font: Font,
}

impl TestFont {
    pub fn new(filename: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            filename: filename.to_owned(),
            font: Font::load(filename)?,
        })
    }
    pub fn style(&self) -> Option<&str> {
        Some("Regular")
    }

    pub(crate) fn get_os2_fsselection(&self) -> Result<u16, Box<dyn Error>> {
        if let Some(os2) = self.font.tables.os2()? {
            Ok(os2.fsSelection)
        } else {
            Err("No OS2 table".into())
        }
    }
    pub fn get_name_entry_strings(&self, name_id: NameID) -> Vec<String> {
        if let Ok(Some(name_table)) = self.font.tables.name() {
            name_table
                .records
                .iter()
                .filter(|x| x.nameID == name_id as u16)
                .map(|x| x.string.clone())
                .collect()
        } else {
            vec![]
        }
    }
}

pub struct FontCollection<'a>(pub Vec<&'a TestFont>);

impl FontCollection<'_> {
    pub fn ribbi_fonts(&self) -> FontCollection {
        let filtered: Vec<&TestFont> = self
            .0
            .iter()
            .copied()
            .filter(|x| RIBBI_STYLE_NAMES.contains(&x.style().unwrap_or("None")))
            .collect();
        FontCollection(filtered)
    }
    pub fn iter(&self) -> std::slice::Iter<'_, &TestFont> {
        self.0.iter()
    }
}
