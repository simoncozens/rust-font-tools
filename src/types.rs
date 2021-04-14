use chrono::NaiveDateTime;
use serde::Deserialize;
use serde::Deserializer;
use std::fmt;

use serde::de::{self, Visitor};
use serde::{Serialize, Serializer};

pub type uint16 = u16;
#[derive(Debug, PartialEq)]
pub struct Fixed(pub f32);
pub type uint32 = u32;
pub type LONGDATETIME = NaiveDateTime;
pub type int16 = i16;

fn ot_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

impl Serialize for Fixed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fixed = ot_round(self.0 * 65536.0);
        serializer.serialize_i32(fixed)
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

impl<'de> Deserialize<'de> for Fixed {
    fn deserialize<D>(deserializer: D) -> Result<Fixed, D::Error>
    where
        D: Deserializer<'de>,
    {
        let orig = deserializer.deserialize_i32(I32Visitor)?;
        Ok(Fixed((orig as f32) / 65536.0))
    }
}

pub mod LONGDATETIMEshim {
    use crate::types::LONGDATETIME;
    use chrono::Duration;
    use chrono::NaiveDate;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(v: &LONGDATETIME, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let now = v.timestamp();
        let epoch = NaiveDate::from_ymd(1904, 1, 1).and_hms(0, 0, 0).timestamp();
        serializer.serialize_i64(now - epoch)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<LONGDATETIME, D::Error>
    where
        D: Deserializer<'de>,
    {
        let diff = i64::deserialize(d)?;
        let epoch = NaiveDate::from_ymd(1904, 1, 1).and_hms(0, 0, 0);
        let res = epoch + Duration::seconds(diff);
        Ok(res)
    }
}
