use openstep_plist::PlistError;
use snafu::Snafu;
use std::{io, path::PathBuf};

#[derive(Debug, Snafu)]
pub enum BabelfontError {
    #[snafu(display("Wrong convertor for file {}", path.display()))]
    WrongConvertor { path: PathBuf },

    #[snafu(display("Error parsing font: {}", msg))]
    General { msg: String },

    #[snafu(display("IO Error for file {}: {}", path.display(), source))]
    IO { source: io::Error, path: PathBuf },

    #[snafu(display("Could not parse plist file {}: {}", path.display(), source))]
    PlistParse { source: PlistError, path: PathBuf },

    #[snafu(display("Ill-defined axis!: {:?}", axis_name))]
    IllDefinedAxis { axis_name: Option<String> },

    #[snafu(display("Ill-constructed path"))]
    BadPath,
}

type Result<T, E = BabelfontError> = std::result::Result<T, E>;
