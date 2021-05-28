use crate::DeserializationError;
use crate::Deserialize;
use crate::Deserializer;
use crate::ReaderContext;
use crate::SerializationError;
use crate::Serialize;
use std::convert::TryInto;

pub type uint16 = u16;
pub type uint32 = u32;
pub type int16 = i16;
pub type FWORD = i16;
pub type UFWORD = u16;

#[derive(Shrinkwrap, Debug, PartialEq)]
pub struct Tuple(Vec<f32>);
impl Serialize for Tuple {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        for var in &self.0 {
            let packed: i32 = ot_round(var * 65536.0);
            packed.to_bytes(data)?
        }
        Ok(())
    }
}

pub type Tag = [u8; 4];
#[macro_export]
macro_rules! tag {
    ($e: expr) => {
        (*$e).as_bytes().try_into().unwrap(): Tag
    };
}

impl Serialize for Tag {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        self[0].to_bytes(data)?;
        self[1].to_bytes(data)?;
        self[2].to_bytes(data)?;
        self[3].to_bytes(data)?;
        Ok(())
    }
}

impl Deserialize for Tag {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        Ok(c.consume(4)?.try_into().unwrap())
    }
}

#[derive(Shrinkwrap, Debug, PartialEq)]
pub struct Fixed(pub f32);

fn ot_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

impl Serialize for Fixed {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let packed: i32 = ot_round(self.0 * 65536.0);
        packed.to_bytes(data)
    }
}
impl Deserialize for Fixed {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let packed: i32 = c.de()?;
        Ok(Fixed(packed as f32 / 65536.0))
    }
}

impl From<f32> for Fixed {
    fn from(num: f32) -> Self {
        Self(num)
    }
}
impl From<Fixed> for f32 {
    fn from(num: Fixed) -> Self {
        num.0
    }
}

#[derive(Shrinkwrap, Debug, PartialEq)]
pub struct F2DOT14(pub f32);

impl Serialize for F2DOT14 {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let packed: i16 = ot_round(self.0 * 16384.0)
            .try_into()
            .map_err(|_| SerializationError("Value didn't fit into a F2DOT14".to_string()))?;
        packed.to_bytes(data)
    }
}
impl Deserialize for F2DOT14 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let packed: i16 = c.de()?;
        Ok(F2DOT14(packed as f32 / 16384.0))
    }
}

impl From<f32> for F2DOT14 {
    fn from(num: f32) -> Self {
        Self(num)
    }
}
impl From<F2DOT14> for f32 {
    fn from(num: F2DOT14) -> Self {
        num.0
    }
}

#[derive(Shrinkwrap, Debug, PartialEq)]
pub struct LONGDATETIME(pub chrono::NaiveDateTime);

use chrono::Duration;
use chrono::NaiveDate;

impl Serialize for LONGDATETIME {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let now = self.timestamp();
        let epoch = NaiveDate::from_ymd(1904, 1, 1).and_hms(0, 0, 0).timestamp();
        (now - epoch).to_bytes(data)
    }
}
impl Deserialize for LONGDATETIME {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let diff: i64 = c.de()?;
        let epoch = NaiveDate::from_ymd(1904, 1, 1).and_hms(0, 0, 0);
        let res = epoch + Duration::seconds(diff);
        Ok(LONGDATETIME(res))
    }
}

impl From<chrono::NaiveDateTime> for LONGDATETIME {
    fn from(num: chrono::NaiveDateTime) -> Self {
        Self(num)
    }
}
impl From<LONGDATETIME> for chrono::NaiveDateTime {
    fn from(num: LONGDATETIME) -> Self {
        num.0
    }
}

pub struct Offset16<T> {
    off: Option<u16>,
    link: Option<T>,
}

impl<T> Serialize for Offset16<T> {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        match self.off {
            Some(x) => x.to_bytes(data),
            None => Err(SerializationError("Offset not set".to_string())),
        }
    }
}

impl<T: Deserialize> Deserialize for Offset16<T> {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let off: uint16 = c.de()?;
        c.push(c.start_of_struct() + off as usize);
        let obj: T = c.de()?;
        c.pop();
        Ok(Offset16 {
            off: Some(off),
            link: Some(obj),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as otspec;
    use otspec_macros::Deserialize;

    #[derive(Deserialize)]
    struct One {
        thing: uint16,
        off: Offset16<Two>,
        other: uint16,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Two {
        test1: uint16,
        test2: uint16,
    }

    #[test]
    fn test_de_off16() {
        let bytes = vec![
            0x00, 0x01, // thing
            0x00, 0x08, // off
            0x00, 0x02, // other
            0xff, 0xff, // filler
            0x00, 0x0a, // test1
            0x00, 0x0b, // test2
        ];
        let mut rc = ReaderContext::new(bytes);
        let one: One = rc.de().unwrap();
        assert_eq!(one.other, 0x02);
        assert_eq!(
            one.off.link,
            Some(Two {
                test1: 0x0a,
                test2: 0x0b
            })
        );
    }
}
