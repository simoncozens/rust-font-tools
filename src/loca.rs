use otspec::read_field;
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use std::fmt;

extern crate otspec;

pub struct LocaDeserializer<'a, T: 'a> {
    locaIs32Bit: bool,
    _phantom: &'a std::marker::PhantomData<T>,
}

impl<T> LocaDeserializer<'_, T> {
    pub fn new(locaIs32Bit: bool) -> Self {
        LocaDeserializer {
            locaIs32Bit,
            _phantom: &std::marker::PhantomData,
        }
    }
}

impl<'de, 'a, T> DeserializeSeed<'de> for LocaDeserializer<'a, T>
where
    T: Deserialize<'de>,
{
    type Value = Vec<u32>;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct LocaDeserializerVisitor<'a, T: 'a> {
            locaIs32Bit: bool,
            _phantom: &'a std::marker::PhantomData<T>,
        }

        impl<'de, 'a, T> Visitor<'de> for LocaDeserializerVisitor<'a, T>
        where
            T: Deserialize<'de>,
        {
            type Value = Vec<u32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a loca table")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Vec<u32>, A::Error>
            where
                A: SeqAccess<'de>,
            {
                if self.locaIs32Bit {
                    Ok(read_field!(seq, Vec<u32>, "a glyph offset"))
                } else {
                    Ok(read_field!(seq, Vec<u16>, "a glyph offset")
                        .iter()
                        .map(|x| (*x as u32) * 2)
                        .collect())
                }
            }
        }

        deserializer.deserialize_seq(LocaDeserializerVisitor::<u32> {
            locaIs32Bit: self.locaIs32Bit,
            _phantom: &std::marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::loca;

    use otspec::de::Deserializer as OTDeserializer;

    use serde::de::DeserializeSeed;

    #[test]
    fn loca_de() {
        let binary_loca = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1a,
        ];
        let mut de = OTDeserializer::from_bytes(&binary_loca);
        let cs: loca::LocaDeserializer<u32> = loca::LocaDeserializer::new(false);
        let floca: Vec<u32> = cs.deserialize(&mut de).unwrap();
        println!("{:?}", floca);
        let mut de = OTDeserializer::from_bytes(&binary_loca);
        let cs: loca::LocaDeserializer<u32> = loca::LocaDeserializer::new(true);
        let floca: Vec<u32> = cs.deserialize(&mut de).unwrap();
        println!("{:?}", floca);
        assert!(false);
    }
}
