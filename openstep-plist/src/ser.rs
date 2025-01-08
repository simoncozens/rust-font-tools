use serde::{ser, Serialize};

use crate::error::{Error, Result};
use crate::is_alnum_strict;

pub struct Serializer {
    output: String,
}

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        output: String::new(),
    };
    value.serialize(&mut serializer)?;
    serializer.output.push(';');
    Ok(serializer.output)
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output += if v { "1" } else { "0" };
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.output.push_str(&format!("{v}"));
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output.push_str(&format!("{v}"));
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.output.push_str(&format!("{v}"));
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.output.push(v);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        escape_string(&mut self.output, v);
        Ok(())
    }

    fn serialize_bytes(self, data: &[u8]) -> Result<()> {
        self.output.push('<');
        for byte in data {
            self.output.extend(hex_digits_for_byte(*byte))
        }
        self.output.push('>');
        Ok(())
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

    fn serialize_unit(self) -> Result<()> {
        // ????
        self.output += "null";
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += "{";
        variant.serialize(&mut *self)?;
        self.output += " = ";
        value.serialize(&mut *self)?;
        self.output += ";}";
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.output += "(";
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    // Tuple structs look just like sequences in JSON.
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
        self.output += "{";
        variant.serialize(&mut *self)?;
        self.output += " = (";
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.output += "{";
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.output += "{structvariant";
        variant.serialize(&mut *self)?;
        self.output += " = {";
        Ok(self)
    }
}

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
        if !self.output.ends_with('(') {
            self.output += ", ";
        }
        value.serialize(&mut **self)
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        self.output += ")";
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
        if !self.output.ends_with('(') {
            self.output += ", ";
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += ")";
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
        if !self.output.ends_with('(') {
            self.output += ", ";
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += ")";
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
        if !self.output.ends_with('(') {
            self.output += ", ";
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += ");}";
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // if !self.output.ends_with('{') {
        //     self.output += ";";
        // }
        self.output += "\n";
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += " = ";
        value.serialize(&mut **self)?;
        self.output += ";";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += "\n}";
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += "\n";
        key.serialize(&mut **self)?;
        self.output += " = ";
        value.serialize(&mut **self)?;
        self.output += "; ";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output = self.output.trim_end().to_string() + "\n}";
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if !self.output.ends_with('{') {
            self.output += "; ";
        }
        key.serialize(&mut **self)?;
        self.output += " = ";
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += "};}";
        Ok(())
    }
}

fn escape_string(buf: &mut String, s: &str) {
    if !s.is_empty() && s.as_bytes().iter().all(|&b| is_alnum_strict(b)) {
        buf.push_str(s);
    } else {
        buf.push('"');
        let mut start = 0;
        let mut ix = start;
        while ix < s.len() {
            let b = s.as_bytes()[ix];
            match b {
                b'"' | b'\\' => {
                    buf.push_str(&s[start..ix]);
                    buf.push('\\');
                    start = ix;
                }
                _ => (),
            }
            ix += 1;
        }
        buf.push_str(&s[start..]);
        buf.push('"');
    }
}

fn hex_digits_for_byte(byte: u8) -> [char; 2] {
    fn to_hex_digit(val: u8) -> char {
        match val {
            0..=9 => ('0' as u32 as u8 + val).into(),
            10..=15 => (('a' as u32 as u8) + val - 10).into(),
            _ => unreachable!("only called with values in range 0..=15"),
        }
    }

    [to_hex_digit(byte >> 4), to_hex_digit(byte & 0x0f)]
}

#[cfg(test)]
mod tests {
    use crate::Plist;

    use super::*;

    #[test]
    fn hex_to_ascii() {
        assert_eq!(hex_digits_for_byte(0x01), ['0', '1']);
        assert_eq!(hex_digits_for_byte(0x00), ['0', '0']);
        assert_eq!(hex_digits_for_byte(0xff), ['f', 'f']);
        assert_eq!(hex_digits_for_byte(0xf0), ['f', '0']);
        assert_eq!(hex_digits_for_byte(0x0f), ['0', 'f']);
    }

    #[test]
    fn test_serialize() {
        let plist: Plist = vec![
            Plist::String("hello".to_string()),
            Plist::String("world".to_string()),
        ]
        .into();
        let s = to_string(&plist).unwrap();
        assert_eq!(s, r#"(hello, world);"#);
    }

    #[test]
    fn test_serialize_map() {
        let plist_str = r#"{array = (1, 2);foo = bar;hello = world;};"#;
        let plist: Plist = Plist::parse(plist_str).unwrap();
        let s = to_string(&plist).unwrap().replace("\n", "");
        assert_eq!(s, plist_str);
    }

    #[test]
    fn test_serialize_struct() {
        let plist_str = r#"
{
axes = (
{
hidden = 1;
name = Weight;
tag = wght;
}
);
};"#
        .replace("\n", "");
        let plist: Plist = Plist::parse(&plist_str).unwrap();
        let s = to_string(&plist).unwrap().replace("\n", "");
        assert_eq!(s, plist_str);
    }

    #[test]
    fn test_vec_axis() {
        #[derive(Serialize, Debug, Default, Clone)]
        struct Axis {
            /// If the axis should be visible in the UI.
            #[serde(default)]
            pub hidden: bool,
            /// The name of the axis (e.g. `Weight``)
            pub name: String,
            /// The axis tag (e.g. `wght`)
            pub tag: String,
        }
        let foo = vec![Axis {
            hidden: true,
            name: "Weight".to_string(),
            tag: "wght".to_string(),
        }];
        let s = to_string(&foo).unwrap().replace("\n", "");
        assert_eq!(s, r#"({hidden = 1; name = Weight; tag = wght;});"#);
    }
}
