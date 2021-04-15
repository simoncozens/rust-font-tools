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
    use serde::ser::SerializeSeq;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;

    pub fn serialize<S, T>(v: &[T], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        let mut my_seq = serializer.serialize_seq(Some(v.len()))?;
        for k in v {
            my_seq.serialize_element(&k)?;
        }
        my_seq.end()
    }
    pub fn deserialize<'de, D, T>(d: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        unimplemented!()
    }
}
