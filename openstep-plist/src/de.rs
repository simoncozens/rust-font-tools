use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;
use smol_str::SmolStr;

use crate::error::{Error, Result};
use crate::Plist;

enum PathElement {
    Key(SmolStr),
    Index(usize),
}

pub struct Deserializer<'de> {
    input: &'de Plist,
    path: Vec<PathElement>,
}

impl<'de> Deserializer<'de> {
    pub fn from_plist(input: &'de Plist) -> Self {
        Deserializer {
            input,
            path: Vec::new(),
        }
    }

    fn element(&self) -> &'de Plist {
        let mut element = self.input;
        for path_element in &self.path {
            match path_element {
                PathElement::Key(key) => {
                    element = element.as_dict().unwrap().get(key).unwrap();
                }
                PathElement::Index(index) => {
                    element = element.as_array().unwrap().get(*index).unwrap();
                }
            }
        }
        element
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // Look at the input data to decide what Serde data model type to
    // deserialize as. Not all data formats are able to support this operation.
    // Formats that support `deserialize_any` are known as self-describing.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.element() {
            Plist::String(_) => self.deserialize_string(visitor),
            Plist::Integer(_) => self.deserialize_i64(visitor),
            Plist::Float(_) => self.deserialize_f64(visitor),
            Plist::Dictionary(_) => self.deserialize_map(visitor),
            Plist::Array(_) => self.deserialize_seq(visitor),
            Plist::Data(_) => self.deserialize_byte_buf(visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.element() {
            Plist::Integer(i) => visitor.visit_bool(*i != 0),
            _ => Err(Error::UnexpectedDataType {
                expected: "integer",
                found: self.element().name(),
            }),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    forward_to_deserialize_any! {i8 i16 i32 u8 u16 u32 u64 f32 char str unit unit_struct}
    forward_to_deserialize_any! {bytes}
    forward_to_deserialize_any! {tuple tuple_struct struct enum identifier ignored_any}

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.element() {
            Plist::Integer(i) => visitor.visit_i64(*i),
            _ => Err(Error::UnexpectedDataType {
                expected: "integer",
                found: self.element().name(),
            }),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.element() {
            Plist::Float(f) => visitor.visit_f64(f.into_inner()),
            _ => Err(Error::UnexpectedDataType {
                expected: "float",
                found: self.element().name(),
            }),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.element() {
            Plist::String(s) => visitor.visit_borrowed_str(s),
            _ => Err(Error::UnexpectedDataType {
                expected: "string",
                found: self.element().name(),
            }),
        }
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.element() {
            Plist::Array(_) => visitor.visit_seq(ArrayDeserializer::new(self)),
            _ => Err(Error::UnexpectedDataType {
                expected: "array",
                found: self.element().name(),
            }),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.element() {
            Plist::Dictionary(_) => visitor.visit_map(DictDeserializer::new(self)),
            _ => Err(Error::UnexpectedDataType {
                expected: "dictionary",
                found: self.element().name(),
            }),
        }
    }
}

struct ArrayDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    index: usize,
    len: usize,
}

impl<'a, 'de> ArrayDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        let len = de.element().as_array().unwrap().len();
        ArrayDeserializer { de, index: 0, len }
    }
}

impl<'de, 'a> SeqAccess<'de> for ArrayDeserializer<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.index == self.len {
            return Ok(None);
        }
        self.de.path.push(PathElement::Index(self.index));
        let result = seed.deserialize(&mut *self.de).map(Some);
        self.de.path.pop();
        self.index += 1;
        result
    }
}

struct DictDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    index: usize,
    keys: Vec<&'a SmolStr>,
}

impl<'a, 'de> DictDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        let keys = de.element().as_dict().unwrap().keys().collect();
        DictDeserializer { de, index: 0, keys }
    }
}

impl<'de, 'a> MapAccess<'de> for DictDeserializer<'a, 'de> {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.index == self.keys.len() {
            return Ok(None);
        }
        let key = self.keys[self.index].clone();
        self.de.path.push(PathElement::Key(key.clone()));
        let key_deserializer = serde::de::value::StringDeserializer::new(key.to_string());
        seed.deserialize(key_deserializer).map(Some)
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let result = seed.deserialize(&mut *self.de);
        self.de.path.pop();
        self.index += 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_basic() {
        let plist = Plist::String("hello".to_string());
        let mut deserializer = Deserializer::from_plist(&plist);
        let value: String = String::deserialize(&mut deserializer).unwrap();
        assert_eq!(value, "hello");
    }

    #[test]
    fn simple_seq() {
        let plist = Plist::Array(vec![
            Plist::Integer(1),
            Plist::Integer(2),
            Plist::Integer(3),
        ]);

        #[derive(Deserialize, Debug, PartialEq)]
        struct Foo(Vec<i64>);

        let mut deserializer = Deserializer::from_plist(&plist);
        let value: Foo = Foo::deserialize(&mut deserializer).unwrap();
        assert_eq!(value.0, vec![1, 2, 3]);
    }

    #[test]
    fn simple_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Foo {
            b: i64,
            a: i64,
        }
        let plist = Plist::Dictionary(
            vec![
                (SmolStr::new("a"), Plist::Integer(2)),
                (SmolStr::new("b"), Plist::Integer(1)),
            ]
            .into_iter()
            .collect(),
        );
        let mut deserializer = Deserializer::from_plist(&plist);
        let value: Foo = Foo::deserialize(&mut deserializer).unwrap();
        assert_eq!(value, Foo { a: 2, b: 1 });
    }

    #[test]
    fn nested_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Foo {
            a: i64,
            b: Bar,
            s: String,
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct Bar {
            c: i64,
        }
        let plist = Plist::Dictionary(
            vec![
                (SmolStr::new("s"), Plist::String("hello".to_string())),
                (SmolStr::new("a"), Plist::Integer(1)),
                (
                    SmolStr::new("b"),
                    Plist::Dictionary(
                        vec![(SmolStr::new("c"), Plist::Integer(2))]
                            .into_iter()
                            .collect(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        );
        let mut deserializer = Deserializer::from_plist(&plist);
        let value: Foo = Foo::deserialize(&mut deserializer).unwrap();
        assert_eq!(
            value,
            Foo {
                a: 1,
                b: Bar { c: 2 },
                s: "hello".to_string()
            }
        );
    }

    #[test]
    fn nested_everything() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Foo {
            a: i64,
            b: Vec<Bar>,
            #[serde(default)]
            s: Option<String>,
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct Bar {
            c: i64,
            d: Vec<String>,
        }
        let plist_str = r#"
        {
            a = 1;
            b = (
                {
                    c = 2;
                    d = ("hello", "world");
                },
                {
                    c = 3;
                    d = ("foo", "bar");
                }
            );
        }
        "#;
        let plist: Plist = Plist::parse(plist_str).unwrap();
        let mut deserializer = Deserializer::from_plist(&plist);
        let value: Foo = Foo::deserialize(&mut deserializer).unwrap();
        assert_eq!(
            value,
            Foo {
                a: 1,
                b: vec![
                    Bar {
                        c: 2,
                        d: vec!["hello".to_string(), "world".to_string()]
                    },
                    Bar {
                        c: 3,
                        d: vec!["foo".to_string(), "bar".to_string()]
                    }
                ],
                s: None
            }
        );
    }
}
