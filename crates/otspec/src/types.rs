#![allow(
    unused_must_use,
    non_snake_case,
    non_camel_case_types,
    clippy::clippy::upper_case_acronyms
)]

use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::Deserialize;
use serde::Deserializer;
use std::fmt;

use serde::de::{self, Visitor};

pub type uint16 = u16;
pub type uint32 = u32;
pub type int16 = i16;
pub type Tag = [u8; 4];
pub type FWORD = i16;
pub type UFWORD = u16;
pub type Tuple = Vec<f32>;

pub use fixed::types::U16F16;

fn ot_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

pub mod Fixed {
    use crate::types::ot_round;
    use crate::types::I32Visitor;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(v: &f32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fixed = ot_round(v * 65536.0);
        serializer.serialize_i32(fixed)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<f32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let orig = deserializer.deserialize_i32(I32Visitor)?;
        Ok((orig as f32) / 65536.0)
    }
}

pub mod Version16Dot16 {
    extern crate fixed;

    use crate::types::I32Visitor;
    use fixed::types::U16F16;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(v: &U16F16, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let major = v.floor().to_num::<u8>();
        let minor = (v.frac().to_num::<f32>() * 160.0) as u8;
        serializer.serialize_bytes(&[0, major, minor, 0])
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<U16F16, D::Error>
    where
        D: Deserializer<'de>,
    {
        let orig = deserializer.deserialize_i32(I32Visitor)?.to_be_bytes();
        let major = orig[1] as f32;
        let minor = orig[2] as f32 / 160.0;
        Ok(U16F16::from_num(major + minor))
    }
}

pub mod F2DOT14 {
    use crate::types::ot_round;
    use crate::types::I16Visitor;

    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serializer};
    use std::convert::TryInto;

    pub fn unpack(v: i16) -> f32 {
        (v as f32) / 16384.0
    }
    pub fn pack(v: f32) -> i16 {
        ot_round(v * 16384.0).try_into().unwrap()
    }

    pub fn serialize<S>(v: &f32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i16(pack(*v))
    }

    pub fn serialize_element<S>(
        v: &f32,
        seq: &mut S,
    ) -> std::result::Result<(), <S as serde::ser::SerializeSeq>::Error>
    where
        S: SerializeSeq,
    {
        seq.serialize_element::<i16>(&pack(*v))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<f32, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(unpack(deserializer.deserialize_i16(I16Visitor)?))
    }
}

struct I32Visitor;

impl<'de> Visitor<'de> for I32Visitor {
    type Value = i32;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }
    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value)
    }
}

struct I16Visitor;

impl<'de> Visitor<'de> for I16Visitor {
    type Value = i16;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between -2^15 and 2^15")
    }
    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value)
    }
}

pub mod LONGDATETIME {
    use chrono::Duration;
    use chrono::NaiveDate;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(v: &chrono::NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let now = v.timestamp();
        let epoch = NaiveDate::from_ymd(1904, 1, 1).and_hms(0, 0, 0).timestamp();
        serializer.serialize_i64(now - epoch)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<chrono::NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let diff = i64::deserialize(d)?;
        let epoch = NaiveDate::from_ymd(1904, 1, 1).and_hms(0, 0, 0);
        let res = epoch + Duration::seconds(diff);
        Ok(res)
    }
}
pub mod Counted {
    use serde::de::{SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::Serialize;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S, T>(v: &[T], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        let mut my_seq = serializer.serialize_seq(Some(v.len()))?;
        my_seq.serialize_element(&(v.len() as u16));
        for k in v {
            my_seq.serialize_element(&k)?;
        }
        my_seq.end()
    }
    pub fn deserialize<'de, D, T: serde::Deserialize<'de>>(d: D) -> Result<Vec<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_seq(SeqVisitor::new())
    }

    pub struct SeqVisitor<T> {
        len: Option<usize>,
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> SeqVisitor<T> {
        fn new() -> Self {
            SeqVisitor {
                len: None,
                _phantom: std::marker::PhantomData,
            }
        }
        pub fn with_len(len: usize) -> Self {
            SeqVisitor {
                len: Some(len),
                _phantom: std::marker::PhantomData,
            }
        }
    }

    impl<'de, T> Visitor<'de> for SeqVisitor<T>
    where
        T: serde::Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "A sequence of {:?} values", self.len)
        }

        fn visit_seq<A: SeqAccess<'de>>(mut self, mut seq: A) -> Result<Self::Value, A::Error> {
            if self.len.is_none() {
                self.len =
                    Some(seq.next_element::<u16>()?.ok_or_else(|| {
                        serde::de::Error::custom("Count type must begin with length")
                    })? as usize);
            }
            let expected = self.len.unwrap();
            let mut result = Vec::with_capacity(expected);
            for i in 0..expected {
                let next = seq
                    .next_element::<T>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                result.push(next)
            }
            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Counted;
    use crate::types::Version16Dot16;
    use crate::{de, ser};
    use fixed::types::U16F16;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestCounted {
        #[serde(with = "Counted")]
        t: Vec<u16>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestCounted2 {
        t0: u32,
        #[serde(with = "Counted")]
        t1: Vec<u16>,
        t2: u16,
        #[serde(with = "Counted")]
        t3: Vec<TestCounted>,
    }

    // #[derive(Serialize, Debug, PartialEq)]
    // struct TestOffset {
    //     t0: u32,
    //     #[serde(with = "Offset16")]
    //     t1: Vec<u16>,
    //     t2: u16,
    // }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestVersion {
        #[serde(with = "Version16Dot16")]
        version: U16F16,
    }

    #[test]
    fn version_ser() {
        let c05 = TestVersion {
            version: U16F16::from_num(0.5),
        };
        let binary_c05 = vec![0x00, 0x00, 0x50, 0x00];
        let c11 = TestVersion {
            version: U16F16::from_num(1.1),
        };
        let binary_c11 = vec![0x00, 0x01, 0x10, 0x00];

        assert_eq!(ser::to_bytes(&c05).unwrap(), binary_c05);
        assert_eq!(ser::to_bytes(&c11).unwrap(), binary_c11);
    }

    #[test]
    fn version_de() {
        let c05 = TestVersion {
            version: U16F16::from_num(0.5),
        };
        let binary_c05 = vec![0x00, 0x00, 0x50, 0x00];
        let c11 = TestVersion {
            version: U16F16::from_num(1.1),
        };
        let binary_c11 = vec![0x00, 0x01, 0x10, 0x00];

        assert_eq!(de::from_bytes::<TestVersion>(&binary_c05).unwrap(), c05);
        assert_eq!(de::from_bytes::<TestVersion>(&binary_c11).unwrap(), c11);
    }

    // #[test]
    // fn counted_ser() {
    //     let c = TestCounted {
    //         t: vec![0x10, 0x20],
    //     };
    //     let binary_c = vec![0x00, 0x02, 0x00, 0x10, 0x00, 0x20];
    //     assert_eq!(ser::to_bytes(&c).unwrap(), binary_c);
    // }

    #[test]
    fn counted_de() {
        let c = TestCounted {
            t: vec![0x10, 0x20],
        };
        let binary_c = vec![0x00, 0x02, 0x00, 0x10, 0x00, 0x20];
        assert_eq!(de::from_bytes::<TestCounted>(&binary_c).unwrap(), c);
    }

    #[test]
    fn counted2_serde() {
        let c1a = TestCounted {
            t: vec![0xaa, 0xbb, 0xcc],
        };
        let c1b = TestCounted {
            t: vec![0xdd, 0xee],
        };
        let c2 = TestCounted2 {
            t0: 0x01020304,
            t1: vec![0x10, 0x20],
            t2: 0x1,
            t3: vec![c1a, c1b],
        };
        let binary_c2 = vec![
            0x01, 0x02, 0x03, 0x04, /* t0 */
            0x00, 0x02, /* count */
            0x00, 0x10, 0x00, 0x20, /* t1 */
            0x00, 0x01, /* t2 */
            0x00, 0x02, /* count */
            0x00, 0x03, /* c1a count */
            0x00, 0xaa, 0x00, 0xbb, 0x00, 0xcc, /* c1a */
            0x00, 0x02, /* c1b count */
            0x00, 0xdd, 0x00, 0x0ee, /* c1b*/
        ];
        assert_eq!(ser::to_bytes(&c2).unwrap(), binary_c2);
        assert_eq!(de::from_bytes::<TestCounted2>(&binary_c2).unwrap(), c2);
    }
}
