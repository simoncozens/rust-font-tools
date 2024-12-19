pub mod glyphs3;
use std::{fs, path};

use glyphs3::Glyphs3;
use openstep_plist::de::Deserializer;
use openstep_plist::Plist;

pub enum Font {
    Glyphs3(Glyphs3),
}
impl Font {
    pub fn load(glyphs_file: &path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let raw_content = fs::read_to_string(glyphs_file)?;
        Self::load_str(&raw_content)
    }
    pub fn load_str(raw_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let plist = Plist::parse(raw_content).unwrap();
        let deserializer = &mut Deserializer::from_plist(&plist);
        let result: Result<Glyphs3, _> = serde_path_to_error::deserialize(deserializer);
        let glyphs3 = result?;
        Ok(Font::Glyphs3(glyphs3))
    }
    pub fn as_glyphs3(&self) -> Option<&Glyphs3> {
        match self {
            Font::Glyphs3(glyphs3) => Some(glyphs3),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load() {
        let file = "resources/Oswald-AE-comb.glyphs";
        let font = Font::load(path::Path::new(file));
    }
}
