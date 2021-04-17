use crate::error::{Error, Result};
use serde::{ser, Serialize};
use std::convert::TryInto;

pub struct Serializer {
    output: Vec<u8>,
    add_later: Vec<OffsetEntry>,
}

struct OffsetEntry {
    where_to_insert_offset: usize,
    offset_base: usize,
    child: Option<Serializer>,
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        output: vec![],
        add_later: vec![],
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

macro_rules! serialize_number_type {
    ($func:ident, $type:ty) => {
        fn $func(self, v: $type) -> Result<()> {
            self.output.extend_from_slice(&v.to_be_bytes());
            Ok(())
        }
    };
}

pub trait SerializeOffsetStruct {
    // add code here
    fn serialize_off16_struct<T>(&mut self, value: &T) -> Result<()>;
    fn serialize_off32_struct<T>(&mut self, value: &T) -> Result<()>;
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    // Associated types for keeping track of additional state while serializing
    // compound data structures like sequences and maps. In this case no
    // additional state is required beyond what is already stored in the
    // Serializer struct.
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    serialize_number_type!(serialize_i8, i8);
    serialize_number_type!(serialize_i16, i16);
    serialize_number_type!(serialize_i32, i32);
    serialize_number_type!(serialize_i64, i64);
    serialize_number_type!(serialize_u8, u8);
    serialize_number_type!(serialize_u16, u16);
    serialize_number_type!(serialize_u32, u32);
    serialize_number_type!(serialize_u64, u64);
    serialize_number_type!(serialize_f32, f32);
    serialize_number_type!(serialize_f64, f64);

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.output.extend_from_slice(v.as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        use serde::ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    // But we will adopt the convention that a *tuple* is not counted.
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(None)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        variant.serialize(&mut *self)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(self)
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        // println!("Serializing struct {}", name);
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(self)
    }
    fn serialize_bool(self, _: bool) -> Result<()> {
        todo!()
    }
    fn serialize_unit_struct(self, _: &str) -> Result<()> {
        todo!()
    }
    fn serialize_unit(self) -> Result<()> {
        todo!()
    }
    fn serialize_unit_variant(self, _: &str, _: u32, _: &str) -> Result<()> {
        todo!()
    }

    fn serialize_newtype_variant<T>(self, _: &str, _: u32, _: &str, _: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        todo!()
    }
    fn serialize_newtype_struct<T>(self, _: &'static str, _: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        todo!()
    }
}

// The following 7 impls deal with the serialization of compound types like
// sequences and maps. Serialization of such types is begun by a Serializer
// method and followed by zero or more calls to serialize individual elements of
// the compound type and one call to end the compound type.
//
// This impl is SerializeSeq so these methods are called after `serialize_seq`
// is called on the Serializer.
impl<'a> ser::SerializeSeq for &'a mut Serializer {
    // Must match the `Ok` type of the serializer.
    type Ok = ();
    // Must match the `Error` type of the serializer.
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Same thing but for tuples.
impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Same thing but for tuple structs.
impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    // The Serde data model allows map keys to be any serializable type. JSON
    // only allows string keys so the implementation below will produce invalid
    // JSON if the key serializes as something other than a string.
    //
    // A real JSON serializer would need to validate that map keys are strings.
    // This can be done by using a different Serializer to serialize the key
    // (instead of `&mut **self`) and having that other serializer only
    // implement `serialize_str` and return an error on any other data type.
    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Ok(())
    }

    // It doesn't make a difference whether the colon is printed at the end of
    // `serialize_key` or at the beginning of `serialize_value`. In this case
    // the code is a bit simpler having it here.
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        for todo in &self.add_later {
            println!("Fixing offset for entry at {}", todo.where_to_insert_offset);
        }
        Ok(())
    }
}

// Similar to `SerializeTupleVariant`, here the `end` method is responsible for
// closing both of the curly braces opened by `serialize_struct_variant`.
impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> SerializeOffsetStruct for &'a mut Serializer {
    fn serialize_off16_struct<T>(&mut self, _value: &T) -> Result<()> {
        let pos = self.output.len();
        // Two-byte placeholder offset
        self.output.extend(vec![0, 0]);
        self.add_later.push(OffsetEntry {
            where_to_insert_offset: pos,
            offset_base: 0,
            child: None, // XXX
        });
        Ok(())
    }

    fn serialize_off32_struct<T>(&mut self, _value: &T) -> Result<()> {
        let pos = self.output.len();
        // Four-byte placeholder offset
        self.output.extend(vec![0, 0, 0, 0]);
        self.add_later.push(OffsetEntry {
            where_to_insert_offset: pos,
            offset_base: 0,
            child: None, // XXX
        });
        Ok(())
    }
}
