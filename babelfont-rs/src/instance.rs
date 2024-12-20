use crate::common::FormatSpecific;
use crate::i18ndictionary::I18NDictionary;
use crate::names::Names;
use fontdrasil::coords::DesignLocation;

#[derive(Debug, Clone, Default)]
pub struct Instance {
    pub name: I18NDictionary,
    pub location: DesignLocation,
    pub style_name: I18NDictionary,
    pub custom_names: Names,
    pub variable: bool,
    // lib
    pub format_specific: FormatSpecific,
}
