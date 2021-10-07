//! OpenType tags.

use crate::{DeserializationError, Deserialize, ReaderContext, SerializationError, Serialize};
use std::{borrow::Borrow, ops::Deref, str::FromStr};

pub use otspec_macros::tag;

/// An OpenType tag.
///
/// A tag is a 4-byte array where each byte is in the printable ascii range
/// (0x20..=0x7E).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tag([u8; 4]);

impl Tag {
    /// Attempt to create a `Tag` from raw bytes.
    ///
    /// If the bytes are known at compile time, and you are using the `fonttools`
    /// crate, you should prefer the `tag!` macro.
    ///
    /// The argument may be a slice of bytes, a `&str`, or any other type that
    /// impls `AsRef<[u8]>`.
    ///
    /// The slice must contain between 1 and 4 characters, each in the printable
    /// ascii range (`0x20..=0x7E`).
    ///
    /// If the input has fewer than four bytes, spaces will be appended.
    pub fn from_raw(src: impl AsRef<[u8]>) -> Result<Self, InvalidTag> {
        let src = src.as_ref();
        if src.is_empty() || src.len() > 4 {
            return Err(InvalidTag::InvalidLength(src.len()));
        }
        if let Some(pos) = src.iter().position(|b| !(0x20..=0x7E).contains(b)) {
            let byte = src[pos];

            return Err(InvalidTag::InvalidByte { pos, byte });
        }
        let mut out = [b' '; 4];
        out[..src.len()].copy_from_slice(src);

        // I think this is all fine but I'm also frequently wrong, so
        debug_assert!(std::str::from_utf8(&out).is_ok());
        Ok(Tag(out))
    }

    /// Create a tag from a raw byte array.
    ///
    /// You probably do not want to use this function; you should use the
    /// `tag!` macro instead.
    ///
    /// # Safety
    ///
    /// The input array must be valid utf-8. In addition, it is *expected* to
    /// include only bytes in the printable ascii range. Passing other bytes
    /// is not *technically* unsafe, but does violate application invariants.
    pub const unsafe fn from_raw_unchecked(raw: [u8; 4]) -> Self {
        Tag(raw)
    }

    /// This tag as raw bytes.
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// This tag as a `&str`.
    pub fn as_str(&self) -> &str {
        // safety: tag can only be constructed from valid utf-8 (via FromStr)
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

/// An error representing an invalid tag.
#[derive(Clone)]
pub enum InvalidTag {
    InvalidLength(usize),
    InvalidByte { pos: usize, byte: u8 },
}

impl FromStr for Tag {
    type Err = InvalidTag;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Tag::from_raw(src)
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for Tag {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl Borrow<[u8; 4]> for Tag {
    fn borrow(&self) -> &[u8; 4] {
        &self.0
    }
}

impl Borrow<str> for Tag {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<[u8; 4]> for Tag {
    fn eq(&self, other: &[u8; 4]) -> bool {
        &self.0 == other
    }
}

impl PartialEq<str> for Tag {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Tag {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl Serialize for Tag {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        self.0[0].to_bytes(data)?;
        self.0[1].to_bytes(data)?;
        self.0[2].to_bytes(data)?;
        self.0[3].to_bytes(data)?;
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        4
    }
}

impl Deserialize for Tag {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let bytes = c.consume(4)?;
        Tag::from_raw(bytes).map_err(|e| DeserializationError(e.to_string()))
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Debug for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for InvalidTag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidLength(len) => write!(f, "length {} not in accepted range (1..=4)", len),
            Self::InvalidByte { pos, byte } => {
                write!(f, "invalid byte '0x{:02X}' at position {}", byte, pos)
            }
        }
    }
}

impl std::fmt::Debug for InvalidTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength(arg0) => f.debug_tuple("InvalidLength").field(arg0).finish(),
            Self::InvalidByte { pos, byte } => f
                .debug_struct("InvalidByte")
                .field("pos", pos)
                .field("byte", &format!("{:02X}", byte))
                .finish(),
        }
    }
}

impl std::error::Error for InvalidTag {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        assert!(Tag::from_raw("").is_err());
        assert!(Tag::from_raw("oopsy").is_err());
        assert!(Tag::from_raw("\nok").is_err());
        assert_eq!(Tag::from_raw("a").unwrap(), "a   ");
        assert_eq!(Tag::from_raw("aa").unwrap(), "aa  ");
        assert_eq!(Tag::from_raw("aaa").unwrap(), "aaa ");
        assert_eq!(Tag::from_raw("aaaa").unwrap(), "aaaa");
    }
}
