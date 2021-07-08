use crate::uint16;
use crate::Counted;
use crate::DeserializationError;
use crate::Deserialize;
use crate::Deserializer;
use crate::ReaderContext;
use crate::SerializationError;
use crate::Serialize;
use std::cell::RefCell;
use std::convert::TryInto;

/// Represents an offset within a table to another subtable
#[derive(Clone)]
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
    fn serialize_offset(&self, output: &mut Vec<u8>) -> Result<(), SerializationError>;
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
    fn serialize_offset(&self, output: &mut Vec<u8>) -> Result<(), SerializationError> {
        self.off.borrow().unwrap().to_bytes(output)
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

impl<T: std::fmt::Debug> std::fmt::Debug for Offset16<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
pub struct VecOffset16<T>(pub Vec<Offset16<T>>);

impl<T> Serialize for VecOffset16<T>
where
    T: Serialize,
{
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        for el in &self.0 {
            el.to_bytes(data)?
        }
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        2 * self.0.len()
    }
    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        let mut v: Vec<&dyn OffsetMarkerTrait> = vec![];
        for el in &self.0 {
            v.push(el);
        }
        v
    }
}

impl<T> From<VecOffset16<T>> for Vec<Offset16<T>> {
    fn from(v: VecOffset16<T>) -> Self {
        v.0
    }
}

impl<T> From<Vec<Offset16<T>>> for VecOffset16<T> {
    fn from(v: Vec<Offset16<T>>) -> Self {
        VecOffset16(v)
    }
}

impl<T> From<VecOffset16<T>> for Counted<Offset16<T>> {
    fn from(v: VecOffset16<T>) -> Self {
        Counted(v.0)
    }
}

impl<T> From<Counted<Offset16<T>>> for VecOffset16<T> {
    fn from(v: Counted<Offset16<T>>) -> Self {
        VecOffset16(v.0)
    }
}

impl<T> TryInto<Vec<T>> for Counted<Offset16<T>>
where
    T: Clone,
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

impl<T> TryInto<Vec<T>> for VecOffset16<T>
where
    T: Clone,
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
