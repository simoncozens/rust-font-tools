use crate::i18ndictionary::I18NDictionary;

#[derive(Debug)]
pub enum StyleMapStyle {
    BoldItalic,
    Bold,
    Regular,
    Italic,
}

#[derive(Debug)]
pub struct Names {
    pub family_name: I18NDictionary,
    pub designer: I18NDictionary,
    pub designer_url: I18NDictionary,
    pub manufacturer: I18NDictionary,
    pub manufacturer_url: I18NDictionary,
    pub license: I18NDictionary,
    pub license_url: I18NDictionary,
    pub version: I18NDictionary,
    pub unique_id: I18NDictionary,
    pub description: I18NDictionary,
    pub typographic_family: I18NDictionary,
    pub typographic_subfamily: I18NDictionary,
    pub compatible_full_name: I18NDictionary,
    pub sample_text: I18NDictionary,
    pub w_w_s_family_name: I18NDictionary,
    pub w_w_s_subfamily_name: I18NDictionary,
    pub copyright: I18NDictionary,
    pub style_map_family_name: I18NDictionary,
    pub style_map_style_name: Option<StyleMapStyle>,
    pub trademark: I18NDictionary,
}

impl Names {
    pub fn new() -> Self {
        Names {
            family_name: I18NDictionary::new(),
            designer: I18NDictionary::new(),
            designer_url: I18NDictionary::new(),
            manufacturer: I18NDictionary::new(),
            manufacturer_url: I18NDictionary::new(),
            license: I18NDictionary::new(),
            license_url: I18NDictionary::new(),
            version: I18NDictionary::new(),
            unique_id: I18NDictionary::new(),
            description: I18NDictionary::new(),
            typographic_family: I18NDictionary::new(),
            typographic_subfamily: I18NDictionary::new(),
            compatible_full_name: I18NDictionary::new(),
            sample_text: I18NDictionary::new(),
            w_w_s_family_name: I18NDictionary::new(),
            w_w_s_subfamily_name: I18NDictionary::new(),
            copyright: I18NDictionary::new(),
            style_map_family_name: I18NDictionary::new(),
            style_map_style_name: None,
            trademark: I18NDictionary::new(),
        }
    }
}
