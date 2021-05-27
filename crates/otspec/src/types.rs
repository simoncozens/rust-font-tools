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

#[derive(Shrinkwrap, Debug, PartialEq)]
pub struct Tag(pub [u8; 4]);
#[macro_export]
macro_rules! tag {
    ($e: expr) => {
        crate::types::Tag((*$e).as_bytes().try_into().unwrap())
    };
}

impl Serialize for Tag {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        self.0[0].to_bytes(data)?;
        self.0[1].to_bytes(data)?;
        self.0[2].to_bytes(data)?;
        self.0[3].to_bytes(data)?;
        Ok(())
    }
}

impl Deserialize for Tag {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        Ok(Tag(c.consume(4)?.try_into().unwrap()))
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
