use crate::i18ndictionary::I18NDictionary;

#[derive(Debug, Clone)]
pub enum StyleMapStyle {
    BoldItalic,
    Bold,
    Regular,
    Italic,
}

#[derive(Debug, Default, Clone)]
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
