mod common;
pub mod glyphs2;
pub mod glyphs3;
mod upgrade;
use std::{fs, path};

use glyphs2::Glyphs2;
use glyphs3::Glyphs3;
use openstep_plist::de::Deserializer;
use openstep_plist::Plist;

fn is_glyphs3(plist: &Plist) -> bool {
    plist
        .as_dict()
        .map(|d| d.contains_key(".formatVersion"))
        .unwrap_or(false)
}

#[derive(Debug, Clone)]
pub enum Font {
    Glyphs2(Glyphs2),
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
        if is_glyphs3(&plist) {
            println!("Is glyphs 3");
            let glyphs3: Glyphs3 = serde_path_to_error::deserialize(deserializer)?;
            Ok(Font::Glyphs3(glyphs3))
        } else {
            println!("Is glyphs 2");
            let glyphs2: Glyphs2 = serde_path_to_error::deserialize(deserializer)?;
            Ok(Font::Glyphs2(glyphs2))
        }
    }
    pub fn as_glyphs3(&self) -> Option<&Glyphs3> {
        match self {
            Font::Glyphs3(glyphs3) => Some(glyphs3),
            _ => None,
        }
    }
    pub fn as_glyphs2(&self) -> Option<&Glyphs3> {
        match self {
            Font::Glyphs3(glyphs3) => Some(glyphs3),
            _ => None,
        }
    }
    pub fn upgrade(&self) -> Self {
        match self {
            Font::Glyphs2(glyphs2) => Font::Glyphs3(Into::into(glyphs2.clone())),
            Font::Glyphs3(_) => self.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use glyphs3::Shape;

    use super::*;
    #[test]
    fn test_load_everything() {
        let file = glob::glob("resources/*.glyphs");
        for entry in file.unwrap() {
            let path = entry.unwrap();
            println!("Loading {:?}", path);
            let font = Font::load(&path).unwrap();
            println!("Upgrading {:?}", path);
            let _ = font.upgrade();
        }
    }

    #[test]
    fn test_component() {
        let file = "resources/RadioCanadaDisplay.glyphs";
        let font = Font::load(path::Path::new(file)).unwrap();
        let glyphs3 = font.as_glyphs3().unwrap();
        if let Shape::Component(component) = glyphs3
            .glyphs
            .iter()
            .find(|g| g.name == "eacute")
            .unwrap()
            .layers
            .first()
            .unwrap()
            .shapes
            .get(1)
            .unwrap()
        {
            assert_eq!(component.component_glyph, "acutecomb");
            assert_eq!(component.position, (152.0, 0.0));
        }
    }
}
