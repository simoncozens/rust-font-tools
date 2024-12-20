#[cfg(feature = "glyphs")]
type GlyphsError = Box<dyn std::error::Error>;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BabelfontError {
    #[error("Unknown file type for file {path}")]
    UnknownFileType { path: PathBuf },

    #[error("Wrong convertor for file {path}")]
    WrongConvertor { path: PathBuf },

    #[error("Error parsing font: {}", msg)]
    General { msg: String },

    #[error("IO Error for file {path}: '{source}'")]
    IO {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[cfg(feature = "ufo")]
    #[error("Error parsing XML file {}: {:?}", path.display(), orig)]
    XMLParse {
        #[source]
        orig: norad::error::DesignSpaceLoadError,
        path: PathBuf,
    },

    #[cfg(feature = "glyphs")]
    #[error("Could not parse plist file {}: {:?}", path.display(), source)]
    PlistParse {
        #[source]
        source: GlyphsError,
        path: PathBuf,
    },

    #[cfg(feature = "ufo")]
    #[error("Error loading UFO {}: {:?}", path, orig)]
    LoadingUFO {
        orig: Box<norad::error::FontLoadError>,
        path: String,
    },

    #[error("Could not find default master in {path}")]
    NoDefaultMaster { path: PathBuf },

    #[error("Ill-defined axis!: {axis_name}")]
    IllDefinedAxis { axis_name: String },

    #[error("Ill-constructed path")]
    BadPath,

    #[error("Called a method which requires a decomposed layer on a layer which had components")]
    NeedsDecomposition,
}

// impl<T> From<T> for BabelfontError
// where
//     T: std::error::Error,
// {
//     fn from(e: T) -> Self {
//         BabelfontError::General { msg: e.to_string() }
//     }
// }
