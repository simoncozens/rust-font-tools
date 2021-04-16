#![allow(unused_must_use, non_snake_case, non_camel_case_types)]
use std::convert::TryInto;
use std::fmt;

use serde::de::{self, Visitor};
use serde::{Serialize, Serializer};

pub type uint16 = u16;
pub type uint32 = u32;
pub type int16 = i16;
pub type Tag = [u8; 4];

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

pub mod F2DOT14 {
    use crate::types::ot_round;
    use crate::types::I32Visitor;
    use serde::{Deserializer, Serializer};
    use std::convert::TryInto;

    pub fn serialize<S>(v: &f32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fixed = ot_round(v * 16384.0);
        serializer.serialize_i16(fixed.try_into().unwrap())
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<f32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let orig = deserializer.deserialize_i32(I32Visitor)?;
        Ok((orig as f32) / 16384.0)
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
    use serde::{Deserialize, Deserializer, Serializer};

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

    struct CountVisitor;

    impl<'de> Visitor<'de> for CountVisitor {
        type Value = u16;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a u16")
        }

        fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }
    }

    struct SeqVisitor<T> {
        len: usize,
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> SeqVisitor<T> {
        fn new() -> Self {
            SeqVisitor {
                len: 0,
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
            write!(formatter, "A sequence of {} values", self.len)
        }

        fn visit_seq<A: SeqAccess<'de>>(mut self, mut seq: A) -> Result<Self::Value, A::Error> {
            self.len = seq
                .next_element::<u16>()?
                .ok_or_else(|| serde::de::Error::custom("Count type must begin with length"))?
                as usize;

            let mut result = Vec::with_capacity(self.len);
            for i in 0..self.len {
                let next = seq
                    .next_element::<T>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                result.push(next)
            }
            Ok(result)
        }
    }
}
pub mod Offset16 {
    use crate::error::Result;
    use crate::ser::SerializeOffsetStruct;
    use serde::Serialize;
    use serde::Serializer;

    pub fn serialize<S, T>(v: &T, mut serializer: S) -> Result<()>
    where
        S: Serializer + SerializeOffsetStruct,
        T: Serialize + Sized,
    {
        serializer.serialize_off16_struct(v)
    }
}

pub mod Offset32 {
    use crate::error::Result;
    use crate::ser::SerializeOffsetStruct;
    use serde::Serialize;
    use serde::Serializer;

    pub fn serialize<S, T>(v: &T, mut serializer: S) -> Result<()>
    where
        S: Serializer + SerializeOffsetStruct,
        T: Serialize + Sized,
    {
        serializer.serialize_off32_struct(v)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Counted;
    use crate::{de, ser};
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

    #[test]
    fn counted_ser() {
        let c = TestCounted {
            t: vec![0x10, 0x20],
        };
        let binary_c = vec![0x00, 0x02, 0x00, 0x10, 0x00, 0x20];
        assert_eq!(ser::to_bytes(&c).unwrap(), binary_c);
    }

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
