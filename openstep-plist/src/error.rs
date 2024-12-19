use serde::{de, ser};
use std::fmt::Display;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Unexpected character '{0}'")]
    UnexpectedChar(char),
    #[error("Unterminated string")]
    UnclosedString,
    #[error("Unterminated data block")]
    UnclosedData,
    #[error("Data block did not contain valid paired hex digits")]
    BadData,
    #[error("Unknown escape code")]
    UnknownEscape,
    #[error("Expected string, found '{token_name}")]
    NotAString { token_name: &'static str },
    #[error("Missing '='")]
    ExpectedEquals,
    #[error("Missing ','")]
    ExpectedComma,
    #[error("Missing ';'")]
    ExpectedSemicolon,
    #[error("Missing '{{'")]
    ExpectedOpenBrace,
    #[error("Missing '}}'")]
    ExpectedCloseBrace,
    #[error("Missing '('")]
    ExpectedOpenParen,
    #[error("Missing ')'")]
    ExpectedCloseParen,
    #[error("Expected character '{0}'")]
    ExpectedChar(char),
    #[error("Expected numeric value")]
    ExpectedNumber,
    #[error("Expected string value")]
    ExpectedString,
    #[error("Expected '{expected}', found '{found}'")]
    UnexpectedDataType {
        expected: &'static str,
        found: &'static str,
    },
    #[error("Unexpected token '{name}'")]
    UnexpectedToken { name: &'static str },
    #[error("parsing failed: '{0}'")]
    Parse(String),
    #[error("serializing failed: '{0}'")]
    Serialize(String),
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Serialize(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Parse(msg.to_string())
    }
}
