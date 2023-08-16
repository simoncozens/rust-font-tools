pub const RIBBI_STYLE_NAMES: [&str; 5] = ["Regular", "Italic", "Bold", "BoldItalic", "Bold Italic"];
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum NameID {
    CopyrightNotice = 0,
    FontFamilyName = 1,
    FontSubfamilyName = 2,
    UniqueFontIdentifier = 3,
    FullFontName = 4,
    VersionString = 5,
    PostscriptName = 6,
    Trademark = 7,
    ManufacturerName = 8,
    Designer = 9,
    Description = 10,
    VendorURL = 11,
    DesignerURL = 12,
    LicenseDescription = 13,
    LicenseInfoURL = 14,
    TypographicFamilyName = 16,
    TypographicSubfamilyName = 17,
    CompatibleFullMaconly = 18,
    SampleText = 19,
    PostscriptCidName = 20,
    WwsFamilyName = 21,
    WwsSubfamilyName = 22,
    LightBackgroundPalette = 23,
    DarkBackgroudPalette = 24,
}
