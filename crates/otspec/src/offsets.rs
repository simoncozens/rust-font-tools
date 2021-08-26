use crate::uint16;
use crate::uint32;
use crate::Counted;
use crate::DeserializationError;
use crate::Deserialize;
use crate::Deserializer;
use crate::ReaderContext;
use crate::SerializationError;
use crate::Serialize;
use core::convert::TryFrom;
use fmt::Debug;
use fmt::Display;
use num::cast::AsPrimitive;
use num::traits::Unsigned;
use num::FromPrimitive;
use std::cell::RefCell;
use std::convert::TryInto;

pub trait OffsetType:
    Unsigned
    + Display
    + Copy
    + Serialize
    + Deserialize
    + FromPrimitive
    + TryFrom<u32>
    + Debug
    + AsPrimitive<usize>
{
}
impl<
        T: Unsigned
            + Display
            + Copy
            + Serialize
            + Deserialize
            + FromPrimitive
            + Debug
            + TryFrom<u32>
            + AsPrimitive<usize>,
    > OffsetType for T
{
}

/// Represents an offset within a table to another subtable
#[derive(Clone)]
pub struct Offset<T, U: OffsetType> {
    off: RefCell<Option<U>>,
    /// The subtable referred to by this offset. Can be `None` (e.g. `Script.defaultLangSysOffset`)
    pub link: Option<T>,
}

pub type Offset16<T> = Offset<T, uint16>;
pub type Offset32<T> = Offset<T, uint32>;

// This is purely internal but we need to make it pub because it's shared
// with otspec_macros. I'm not going to rustdoc it, though.
//
// The idea behind this is that we need to be able to build an object
// graph containing different subtable types. To do *that*, we need to erase
// the internal type of the `Offset16<T>`, and so turn it into a trait object.
// So we expose a portion of the Offset16's functionality inside this marker
// trait.
pub trait OffsetMarkerTrait: Serialize + Debug {
    fn children(&self) -> Vec<&dyn OffsetMarkerTrait>;
    fn object_size(&self) -> usize;
    fn total_size_with_descendants(&self) -> usize;
    fn needs_resolving(&self) -> bool;
    fn is_explicitly_zero(&self) -> bool;
    // This is gross. Having polymorphic offset marker traits would make everything horrible,
    // so we have to specify the highest offset we need and cast downwards.
    fn set(&self, off: u32);
    fn serialize_contents(&self, output: &mut Vec<u8>) -> Result<(), SerializationError>;
    fn serialize_offset(&self, output: &mut Vec<u8>) -> Result<(), SerializationError>;
}

impl<T, U> OffsetMarkerTrait for Offset<T, U>
where
    T: Serialize + Debug,
    U: OffsetType,
{
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

    fn is_explicitly_zero(&self) -> bool {
        self.link.is_none()
            && self.off.borrow().is_some()
            && self.off.borrow().unwrap() == U::zero()
    }

    // Finally, when we have resolved all the offsets, we use interior
    // mutability to replace the offset within the `Offset` struct.
    fn set(&self, off: u32) {
        self.off.replace(U::from_u32(off));
    }
    fn serialize_contents(&self, output: &mut Vec<u8>) -> Result<(), SerializationError> {
        if let Some(l) = self.link.as_ref() {
            l.to_bytes_shallow(output)?
        }
        Ok(())
    }
    fn serialize_offset(&self, output: &mut Vec<u8>) -> Result<(), SerializationError> {
        self.off.borrow().unwrap().to_bytes(output)
    }
}

impl<T, U: OffsetType> Offset<T, U> {
    /// Create a new offset pointing to a subtable. Its offset must be resolved
    /// before serialization using an `OffsetManager`.
    pub fn to(thing: T) -> Self {
        Self {
            off: RefCell::new(None),
            link: Some(thing),
        }
    }

    /// Create a new offset pointing to nothing.
    pub fn to_nothing() -> Self {
        Self {
            off: RefCell::new(Some(U::zero())),
            link: None,
        }
    }

    /// Returns the byte offset from the parent of this subtable, if set.
    pub fn offset_value(&self) -> Option<U> {
        *self.off.borrow()
    }
}

impl<T: PartialEq, U: OffsetType> PartialEq for Offset<T, U> {
    fn eq(&self, rhs: &Self) -> bool {
        self.link == rhs.link
    }
}

impl<T: Debug, U: OffsetType> Serialize for Offset<T, U>
where
    Offset<T, U>: Debug,
{
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        if let Some(v) = self.offset_value() {
            v.to_bytes(data)
        } else if self.link.is_none() {
            U::zero().to_bytes(data)
        } else {
            Err(SerializationError("Offset not set".to_string()))
        }
    }

    fn ot_binary_size(&self) -> usize {
        ::std::mem::size_of::<U>()
    }
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        vec![] // Maybe?
    }
}

impl<T: Deserialize + Debug, U: OffsetType> Deserialize for Offset<T, U> {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let off: U = c.de()?;
        if off == U::zero() {
            return Ok(Self {
                off: RefCell::new(None),
                link: None,
            });
        }
        let oldptr = c.ptr;
        c.ptr = c.top_of_table() + off.as_();
        let obj: T = c.de()?;
        c.ptr = oldptr;
        Ok(Self {
            off: RefCell::new(Some(off)),
            link: Some(obj),
        })
    }
}

use std::fmt;
use std::ops::Deref;

impl<T, U: OffsetType> Deref for Offset<T, U> {
    type Target = Option<T>;
    fn deref(&self) -> &Self::Target {
        &self.link
    }
}

impl<T, U> Debug for Offset<T, U>
where
    T: Debug,
    U: OffsetType,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<Obj")?;
        if let Some(off) = *self.off.borrow() {
            f.write_fmt(format_args!("@{:}", off))?;
        }
        if let Some(link) = &self.link {
            f.write_fmt(format_args!(" {:?}", link))?;
        }
        f.write_str(">")
    }
}

// Vector of offsets

#[derive(Debug, Clone, PartialEq)]
pub struct VecOffset<T, U: OffsetType> {
    pub v: Vec<Offset<T, U>>,
}
pub type VecOffset16<T> = VecOffset<T, u16>;
pub type VecOffset32<T> = VecOffset<T, u32>;

impl<T, U> Serialize for VecOffset<T, U>
where
    T: Serialize,
    U: OffsetType,
{
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        for el in &self.v {
            el.to_bytes(data)?
        }
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        std::mem::size_of::<U>() * self.v.len()
    }
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        let mut v: Vec<&dyn OffsetMarkerTrait> = vec![];
        for el in &self.v {
            v.push(el);
        }
        v
    }
}

impl<T, U: OffsetType> From<VecOffset<T, U>> for Vec<Offset<T, U>> {
    fn from(v: VecOffset<T, U>) -> Self {
        v.v
    }
}

impl<T, U: OffsetType> From<Vec<Offset<T, U>>> for VecOffset<T, U> {
    fn from(v: Vec<Offset<T, U>>) -> Self {
        VecOffset { v }
    }
}

impl<T, U: OffsetType> From<VecOffset<T, U>> for Counted<Offset<T, U>> {
    fn from(v: VecOffset<T, U>) -> Self {
        Counted(v.v)
    }
}

impl<T, U: OffsetType> From<Counted<Offset<T, U>>> for VecOffset<T, U> {
    fn from(v: Counted<Offset<T, U>>) -> Self {
        VecOffset { v: v.0 }
    }
}

impl<T, U> TryInto<Vec<T>> for Counted<Offset<T, U>>
where
    T: Clone,
    U: OffsetType,
{
    type Error = DeserializationError;
    fn try_into(self) -> Result<Vec<T>, DeserializationError> {
        self.0
            .iter()
            .map(|x| {
                x.link
                    .clone()
                    .ok_or_else(|| DeserializationError("Bad offset in offset array".to_string()))
            })
            .collect()
    }
}

impl<T, U> TryInto<Vec<T>> for VecOffset<T, U>
where
    T: Clone,
    U: OffsetType,
{
    type Error = DeserializationError;
    fn try_into(self) -> Result<Vec<T>, DeserializationError> {
        self.v
            .iter()
            .map(|x| {
                x.link
                    .clone()
                    .ok_or_else(|| DeserializationError("Bad offset in offset array".to_string()))
            })
            .collect()
    }
}

impl Serialize for Box<dyn OffsetMarkerTrait> {
    fn to_bytes(&self, output: &mut Vec<u8>) -> Result<(), SerializationError> {
        self.as_ref().to_bytes(output)
    }

    fn to_bytes_shallow(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        self.as_ref().serialize_offset(data)
    }

    fn ot_binary_size(&self) -> usize {
        0 // ?
    }

    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        vec![]
    }
}

impl<T, U> OffsetMarkerTrait for Option<Offset<T, U>>
where
    T: Serialize + Debug,
    U: OffsetType,
{
    fn children(&self) -> Vec<&dyn OffsetMarkerTrait> {
        self.as_ref().map_or_else(Vec::new, |x| x.children())
    }
    fn object_size(&self) -> usize {
        self.as_ref().map_or(0, |x| x.object_size())
    }
    fn total_size_with_descendants(&self) -> usize {
        self.as_ref().map_or(0, |x| x.total_size_with_descendants())
    }

    fn needs_resolving(&self) -> bool {
        self.as_ref().map_or(false, |x| x.needs_resolving())
    }

    fn is_explicitly_zero(&self) -> bool {
        self.as_ref().map_or(true, |x| x.is_explicitly_zero())
    }

    fn set(&self, off: u32) {
        if let Some(x) = self {
            let new_off: Result<U, <u32 as std::convert::TryInto<U>>::Error> = off.try_into();
            if let Ok(new_off) = new_off {
                x.off.replace(Some(new_off));
            } else {
                panic!("Oops, 32 bit offset didn't fit")
            }
        } else {
            panic!(
                "Attempted to set an offset on a None Option<Offset16> {:?}",
                self
            )
        }
    }

    fn serialize_contents(&self, output: &mut Vec<u8>) -> Result<(), SerializationError> {
        if let Some(x) = self {
            x.serialize_contents(output)
        } else {
            Ok(())
        }
    }
    fn serialize_offset(&self, output: &mut Vec<u8>) -> Result<(), SerializationError> {
        if let Some(x) = self {
            x.serialize_offset(output)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as otspec;
    use otspec::Deserializer;
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
