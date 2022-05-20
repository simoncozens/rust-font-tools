use openstep_plist::Error;
use snafu::Snafu;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Snafu)]
pub enum BabelfontError {
    #[snafu(display("Unknown file type for file {}", path.display()))]
    UnknownFileType { path: PathBuf },

    #[snafu(display("Wrong convertor for file {}", path.display()))]
    WrongConvertor { path: PathBuf },

    #[snafu(display("Error parsing font: {}", msg))]
    General { msg: String },

    #[snafu(display("IO Error for file {}: {}", path.display(), source))]
    IO { source: io::Error, path: PathBuf },

    #[snafu(display("Could not parse plist file {}: {:?}", path.display(), orig))]
    PlistParse { orig: Error, path: PathBuf },

    #[snafu(display("Error loading UFO {}: {:?}", path, orig))]
    LoadingUFO { orig: norad::Error, path: String },

    #[snafu(display("Could not parse XML file {}: {:?}", path.display(), orig))]
    XMLParse {
        orig: serde_xml_rs::Error,
        path: PathBuf,
    },

    #[snafu(display("Could not find default master in {}",path.display()))]
    NoDefaultMaster { path: PathBuf },

    #[snafu(display("Ill-defined axis!: {:?}", axis_name))]
    IllDefinedAxis { axis_name: Option<String> },

    #[snafu(display("Ill-constructed path"))]
    BadPath,
}
