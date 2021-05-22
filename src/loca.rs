use otspec::{read_field, stateful_deserializer};
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::{Serialize, Serializer};

#[allow(non_snake_case, non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub struct loca {
    pub indices: Vec<Option<u32>>,
}

stateful_deserializer!(
    loca,
    LocaDeserializer,
    { loca_is_32bit: bool },
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<loca, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut res = loca {
            indices: Vec::new(),
        };
        let raw_indices: Vec<u32> = if self.loca_is_32bit {
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
);

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

pub fn from_bytes(s: &[u8], loca_is_32bit: bool) -> otspec::error::Result<loca> {
    let mut deserializer = otspec::de::Deserializer::from_bytes(s);
    let cs: LocaDeserializer = LocaDeserializer { loca_is_32bit };
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
