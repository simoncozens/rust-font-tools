use otspec::read_field;
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::{Serialize, Serializer};
use std::fmt;
extern crate otspec;

#[derive(Debug, PartialEq)]
pub struct loca {
    pub indices: Vec<Option<u32>>,
}

pub struct LocaDeserializer {
    locaIs32Bit: bool,
}

impl<'de> DeserializeSeed<'de> for LocaDeserializer {
    type Value = loca;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct LocaDeserializerVisitor {
            locaIs32Bit: bool,
        }

        impl<'de> Visitor<'de> for LocaDeserializerVisitor {
            type Value = loca;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a loca table")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<loca, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut res = loca {
                    indices: Vec::new(),
                };
                let raw_indices: Vec<u32> = if self.locaIs32Bit {
                    read_field!(seq, Vec<u32>, "a glyph offset")
                } else {
                    read_field!(seq, Vec<u16>, "a glyph offset")
                        .iter()
                        .map(|x| (*x as u32) * 2)
                        .collect()
                };
                if raw_indices.is_empty() {
                    // No glyphs, eh?
                    return Ok(res);
                }
                for ab in raw_indices.windows(2) {
                    if let [a, b] = ab {
                        if *a == *b {
                            res.indices.push(None);
                        } else {
                            res.indices.push(Some(*a));
                        }
                    }
                }
                Ok(res)
            }
        }

        deserializer.deserialize_seq(LocaDeserializerVisitor {
            locaIs32Bit: self.locaIs32Bit,
        })
    }
}

impl Serialize for loca {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        panic!(
            "loca cannot be serialized directly. Call compile_glyf_loca_maxp on the font instead"
        )
        // But we still want an impl here to dispatch the Table serializer in Font
    }
}

pub fn from_bytes(s: &[u8], locaIs32Bit: bool) -> otspec::error::Result<loca> {
    let mut deserializer = otspec::de::Deserializer::from_bytes(s);
    let cs: LocaDeserializer = LocaDeserializer { locaIs32Bit };
    cs.deserialize(&mut deserializer)
}

#[cfg(test)]
mod tests {
    use crate::loca;

    #[test]
    fn loca_de_16bit() {
        let binary_loca = vec![0x00, 0x00, 0x01, 0x30, 0x01, 0x30, 0x01, 0x4c];
        let floca = loca::from_bytes(&binary_loca, false).unwrap();
        let locations = [Some(0), None, Some(608)];
        // println!("{:?}", floca);
        assert_eq!(floca.indices, locations);
    }
}
