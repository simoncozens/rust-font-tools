mod glyphs3;
use std::{fs, path};

use glyphs3::Glyphs3;
use openstep_plist::de::Deserializer;
use openstep_plist::Plist;
use serde::Deserialize;

enum Font {
    Glyphs3(Glyphs3),
}
impl Font {
    fn load(glyphs_file: &path::Path) -> Self {
        let raw_content = fs::read_to_string(glyphs_file).unwrap(); // I have no time to be tidy
        let plist = Plist::parse(&raw_content).unwrap();
        let mut deserializer = &mut Deserializer::from_plist(&plist);
        let result: Result<Glyphs3, _> = serde_path_to_error::deserialize(deserializer);
        let glyphs3 = result.unwrap();
        Font::Glyphs3(glyphs3)
    }

    fn as_glyphs3(&self) -> &Glyphs3 {
        match self {
            Font::Glyphs3(glyphs3) => glyphs3,
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
