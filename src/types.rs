use chrono::NaiveDateTime;

use serde::Serialize;
use serde::Serializer;

pub type uint16 = u16;
#[derive(Debug)]
pub struct Fixed(pub f32);
pub type uint32 = u32;
pub type LONGDATETIME = NaiveDateTime;
pub type int16 = i16;

fn otRound(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

impl Serialize for Fixed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fixed = otRound(self.0 * 65536.0);
        serializer.serialize_i32(fixed)
    }
}

pub mod LONGDATETIMEshim {
    use crate::types::LONGDATETIME;
    use chrono::NaiveDate;
    use serde::Serializer;

    pub fn serialize<S>(v: &LONGDATETIME, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let now = v.timestamp();
        let epoch = NaiveDate::from_ymd(1904, 1, 1).and_hms(0, 0, 0).timestamp();
        serializer.serialize_i64(now - epoch)
    }
}
