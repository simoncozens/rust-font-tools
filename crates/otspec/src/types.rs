use crate::DeserializationError;
use crate::Deserialize;
use crate::Deserializer;
use crate::ReaderContext;
use crate::SerializationError;
use crate::Serialize;

use std::cell::RefCell;
use std::convert::TryInto;

#[allow(non_camel_case_types)]
pub type uint16 = u16;
#[allow(non_camel_case_types)]
pub type uint32 = u32;
#[allow(non_camel_case_types)]
pub type int16 = i16;
#[allow(clippy::upper_case_acronyms)]
pub type FWORD = i16;
#[allow(clippy::upper_case_acronyms)]
pub type UFWORD = u16;
pub type Tag = [u8; 4];

pub use fixed::types::U16F16;

pub fn tag(s: &str) -> Tag {
    (*s).as_bytes().try_into().unwrap()
}

impl Serialize for Tag {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        self[0].to_bytes(data)?;
        self[1].to_bytes(data)?;
        self[2].to_bytes(data)?;
        self[3].to_bytes(data)?;
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        4
    }
}

impl Deserialize for Tag {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        Ok(c.consume(4)?.try_into().unwrap())
    }
}

#[derive(Shrinkwrap, Debug, PartialEq, Copy, Clone)]
pub struct Fixed(pub f32);

pub type Tuple = Vec<f32>;

fn ot_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

impl Serialize for Fixed {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let packed: i32 = ot_round(self.0 * 65536.0);
        packed.to_bytes(data)
    }
    fn ot_binary_size(&self) -> usize {
        4
    }
}
impl Deserialize for Fixed {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let packed: i32 = c.de()?;
        Ok(Fixed(packed as f32 / 65536.0))
    }
}

impl From<f32> for Fixed {
    fn from(num: f32) -> Self {
        Self(num)
    }
}
impl From<Fixed> for f32 {
    fn from(num: Fixed) -> Self {
        num.0
    }
}

#[derive(Shrinkwrap, Debug, Copy, Clone)]
pub struct F2DOT14(pub f32);

impl F2DOT14 {
    pub fn as_packed(&self) -> Result<i16, std::num::TryFromIntError> {
        ot_round(self.0 * 16384.0).try_into()
    }
    pub fn from_packed(packed: i16) -> Self {
        F2DOT14(packed as f32 / 16384.0)
    }
}
impl PartialEq for F2DOT14 {
    fn eq(&self, other: &Self) -> bool {
        self.as_packed() == other.as_packed()
    }
}
impl Eq for F2DOT14 {}

impl std::hash::Hash for F2DOT14 {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.as_packed().unwrap().hash(state)
    }
}

impl Serialize for F2DOT14 {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let packed: i16 = self
            .as_packed()
            .map_err(|_| SerializationError("Value didn't fit into a F2DOT14".to_string()))?;
        packed.to_bytes(data)
    }
    fn ot_binary_size(&self) -> usize {
        2
    }
}
impl Deserialize for F2DOT14 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let packed: i16 = c.de()?;
        Ok(F2DOT14::from_packed(packed))
    }
}

impl From<f32> for F2DOT14 {
    fn from(num: f32) -> Self {
        Self(num)
    }
}
impl From<F2DOT14> for f32 {
    fn from(num: F2DOT14) -> Self {
        num.0
    }
}

#[derive(Shrinkwrap, Debug, PartialEq)]
pub struct Version16Dot16(pub U16F16);

impl Serialize for Version16Dot16 {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let major = self.0.floor().to_num::<u8>();
        let minor = (self.0.frac().to_num::<f32>() * 160.0) as u8;
        0_u8.to_bytes(data)?;
        major.to_bytes(data)?;
        minor.to_bytes(data)?;
        0_u8.to_bytes(data)
    }
    fn ot_binary_size(&self) -> usize {
        2
    }
}
impl Deserialize for Version16Dot16 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let packed: i32 = c.de()?;
        let orig = packed.to_be_bytes();
        let major = orig[1] as f32;
        let minor = orig[2] as f32 / 160.0;
        Ok(Self(U16F16::from_num(major + minor)))
    }
}

impl From<U16F16> for Version16Dot16 {
    fn from(num: U16F16) -> Self {
        Self(num)
    }
}
impl From<Version16Dot16> for U16F16 {
    fn from(num: Version16Dot16) -> Self {
        num.0
    }
}
#[derive(Shrinkwrap, Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub struct LONGDATETIME(pub chrono::NaiveDateTime);

use chrono::Duration;
use chrono::NaiveDate;

impl Serialize for LONGDATETIME {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let now = self.timestamp();
        let epoch = NaiveDate::from_ymd(1904, 1, 1).and_hms(0, 0, 0).timestamp();
        (now - epoch).to_bytes(data)
    }
    fn ot_binary_size(&self) -> usize {
        8
    }
}
impl Deserialize for LONGDATETIME {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let diff: i64 = c.de()?;
        let epoch = NaiveDate::from_ymd(1904, 1, 1).and_hms(0, 0, 0);
        let res = epoch + Duration::seconds(diff);
        Ok(LONGDATETIME(res))
    }
}

impl From<chrono::NaiveDateTime> for LONGDATETIME {
    fn from(num: chrono::NaiveDateTime) -> Self {
        Self(num)
    }
}
impl From<LONGDATETIME> for chrono::NaiveDateTime {
    fn from(num: LONGDATETIME) -> Self {
        num.0
    }
}

// OK, the offset type is going to be terrifying.

/// Represents an offset within a table to another subtable
#[derive(Debug, Clone)]
pub struct Offset16<T> {
    off: RefCell<Option<u16>>,
    /// The subtable referred to by this offset. Can be `None` (e.g. `Script.defaultLangSysOffset`)
    pub link: Option<T>,
}

// This is purely internal but we need to make it pub because it's shared
// with otspec_macros. I'm not going to rustdoc it, though.
//
// The idea behind this is that we need to be able to build an object
// graph containing different subtable types. To do *that*, we need to erase
// the internal type of the `Offset16<T>`, and so turn it into a trait object.
// So we expose a portion of the Offset16's functionality inside this marker
// trait.
pub trait OffsetMarkerTrait: Serialize + std::fmt::Debug {
    fn children(&self) -> Vec<&dyn OffsetMarkerTrait>;
    fn object_size(&self) -> usize;
    fn total_size_with_descendants(&self) -> usize;
    fn needs_resolving(&self) -> bool;
    fn set(&self, off: uint16);
    fn serialize_contents(&self, output: &mut Vec<u8>) -> Result<(), SerializationError>;
}

impl<T: Serialize + std::fmt::Debug> OffsetMarkerTrait for Offset16<T> {
    // When building the tree, we need to know which of my fields also have
    // offsets.
    fn children(&self) -> Vec<&dyn OffsetMarkerTrait> {
        self.link.as_ref().map_or(vec![], |l| l.offset_fields())
    }
    // And when computing the offset for the *next* link, we need to know
    // how big this object is.
    fn object_size(&self) -> usize {
        self.link.as_ref().map_or(0, |l| l.ot_binary_size())
    }
    fn total_size_with_descendants(&self) -> usize {
        let me: usize = self.object_size();
        let them: usize = self
            .children()
            .iter()
            .map(|l| l.total_size_with_descendants())
            .sum();
        me + them
    }

    fn needs_resolving(&self) -> bool {
        if self.off.borrow().is_none() {
            return true;
        }
        for f in self.children() {
            if f.needs_resolving() {
                return true;
            }
        }
        false
    }

    // Finally, when we have resolved all the offsets, we use interior
    // mutability to replace the offset within the `Offset16` struct.
    fn set(&self, off: uint16) {
        self.off.replace(Some(off));
    }
    fn serialize_contents(&self, output: &mut Vec<u8>) -> Result<(), SerializationError> {
        if let Some(l) = self.link.as_ref() {
            l.to_bytes_shallow(output)?
        }
        Ok(())
    }
}

impl<T> Offset16<T> {
    /// Create a new offset pointing to a subtable. Its offset must be resolved
    /// before serialization using an `OffsetManager`.
    pub fn to(thing: T) -> Self {
        Offset16 {
            off: RefCell::new(None),
            link: Some(thing),
        }
    }

    /// Create a new offset pointing to nothing.
    pub fn to_nothing() -> Self {
        Offset16 {
            off: RefCell::new(None),
            link: None,
        }
    }

    /// Returns the byte offset from the parent of this subtable, if set.
    pub fn offset_value(&self) -> Option<uint16> {
        *self.off.borrow()
    }
}

impl<T: PartialEq> PartialEq for Offset16<T> {
    fn eq(&self, rhs: &Offset16<T>) -> bool {
        self.link == rhs.link
    }
}

impl<T: std::fmt::Debug> Serialize for Offset16<T> {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        if let Some(v) = self.offset_value() {
            v.to_bytes(data)
        } else if self.link.is_none() {
            0_u16.to_bytes(data)
        } else {
            Err(SerializationError("Offset not set".to_string()))
        }
    }

    fn ot_binary_size(&self) -> usize {
        2
    }
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        vec![] // Maybe?
    }
}

impl<T: Deserialize + std::fmt::Debug> Deserialize for Offset16<T> {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let off: uint16 = c.de()?;
        if off == 0 {
            return Ok(Offset16 {
                off: RefCell::new(None),
                link: None,
            });
        }
        let oldptr = c.ptr;
        c.ptr = c.top_of_table() + off as usize;
        let obj: T = c.de()?;
        c.ptr = oldptr;
        Ok(Offset16 {
            off: RefCell::new(Some(off)),
            link: Some(obj),
        })
    }
}

use std::ops::Deref;

impl<T> Deref for Offset16<T> {
    type Target = Option<T>;
    fn deref(&self) -> &Self::Target {
        &self.link
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as otspec;
    use otspec_macros::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    struct One {
        thing: uint16,
        off: Offset16<Two>,
        other: uint16,
    }

    #[derive(Deserialize, Debug, PartialEq, Serialize, Clone)]
    struct Two {
        #[serde(offset_base)]
        test1: uint16,
        deep: Offset16<Three>,
        test2: uint16,
    }

    #[derive(Deserialize, Debug, PartialEq, Serialize, Clone)]
    struct Three {
        blah: uint16,
    }

    #[test]
    fn test_de_off16() {
        let bytes = vec![
            0x00, 0x01, // thing
            0x00, 0x08, // off
            0x00, 0x02, // other
            0xff, 0xff, // filler
            0x00, 0x0a, // test1
            0x00, 0x06, // deep
            0x00, 0x0b, // test2
            0x00, 0xaa,
        ];
        let mut rc = ReaderContext::new(bytes);
        let one: One = rc.de().unwrap();
        assert_eq!(one.other, 0x02);
        assert_eq!(one.thing, 0x01);
        assert_eq!(one.off.as_ref().unwrap().test1, 0x0a);
        assert_eq!(
            one.off.link,
            Some(Two {
                test1: 0x0a,
                deep: Offset16::to(Three { blah: 0xaa }),
                test2: 0x0b
            })
        );
    }
}
