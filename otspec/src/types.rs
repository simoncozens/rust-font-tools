pub use crate::offsets::OffsetMarkerTrait;
use crate::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otmath::ot_round;
use std::convert::TryInto;

#[allow(non_camel_case_types)]
pub type uint16 = u16;
#[allow(non_camel_case_types)]
pub type uint8 = u8;
#[allow(non_camel_case_types)]
pub type uint32 = u32;
#[allow(non_camel_case_types)]
pub type int16 = i16;
#[allow(clippy::upper_case_acronyms)]
pub type FWORD = i16;
#[allow(clippy::upper_case_acronyms)]
pub type UFWORD = u16;
#[allow(non_camel_case_types)]
pub type GlyphID = u16;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct uint24(u32);

pub use super::tag::{InvalidTag, Tag};

impl Serialize for uint24 {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        if self.0 > (1 << 24) - 1 {
            return Err(SerializationError(format!(
                "Could not fit {:} into uint24",
                self.0
            )));
        }
        data.extend(&self.0.to_be_bytes()[1..]);
        Ok(())
    }
}

impl From<u32> for uint24 {
    fn from(val: u32) -> Self {
        uint24(val)
    }
}

impl From<uint24> for u32 {
    fn from(val: uint24) -> Self {
        val.0
    }
}

impl Deserialize for uint24 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let bytes: Vec<u8> = c.de_counted(3)?;
        Ok(uint24(
            ((bytes[0] as u32) << 16) + ((bytes[1] as u32) << 8) + bytes[2] as u32,
        ))
    }
}

pub use fixed::types::U16F16;

#[derive(Shrinkwrap, Debug, PartialEq, Copy, Clone)]
pub struct Fixed(pub f32);

pub type Tuple = Vec<f32>;

impl Fixed {
    pub fn as_packed(&self) -> i32 {
        ot_round(self.0 * 65536.0)
    }
    pub fn from_packed(packed: i32) -> Self {
        Fixed(packed as f32 / 65536.0)
    }

    pub fn round(f: f32) -> f32 {
        Fixed::from_packed(Fixed(f).as_packed()).0
    }
}
impl Serialize for Fixed {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let packed: i32 = self.as_packed();
        packed.to_bytes(data)
    }
    fn ot_binary_size(&self) -> usize {
        4
    }
}
impl Deserialize for Fixed {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let packed: i32 = c.de()?;
        Ok(Fixed::from_packed(packed))
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

#[derive(Shrinkwrap, Debug, Copy, Clone)]
pub struct F2DOT14(pub f32);

impl F2DOT14 {
    const MAX: f32 = 1.999939;

    pub fn as_packed(&self) -> Result<i16, std::num::TryFromIntError> {
        ot_round(self.0 * 16384.0).try_into()
    }

    pub fn from_packed(packed: i16) -> Self {
        F2DOT14(packed as f32 / 16384.0)
    }

    pub fn round(f: f32) -> f32 {
        F2DOT14::from_packed(F2DOT14(f).as_packed().unwrap()).0
    }
}
impl PartialEq for F2DOT14 {
    fn eq(&self, other: &Self) -> bool {
        self.as_packed() == other.as_packed()
    }
}
impl Eq for F2DOT14 {}
impl PartialOrd for F2DOT14 {
    fn partial_cmp(&self, other: &Self) -> std::option::Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for F2DOT14 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_packed().unwrap().cmp(&other.as_packed().unwrap())
    }
}

impl std::hash::Hash for F2DOT14 {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.as_packed().unwrap().hash(state)
    }
}

impl Serialize for F2DOT14 {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let packed: i16 = self
            .as_packed()
            .map_err(|_| SerializationError("Value didn't fit into a F2DOT14".to_string()))?;
        packed.to_bytes(data)
    }
    fn ot_binary_size(&self) -> usize {
        2
    }
}
impl Deserialize for F2DOT14 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let packed: i16 = c.de()?;
        Ok(F2DOT14::from_packed(packed))
    }
}

impl From<f32> for F2DOT14 {
    /// Convert a f32 into an F2DOT14.
    ///
    /// The target type's upper bound is 1.999939 rather than 2, so as a special case,
    /// values 1.999939 > v <= 2 are clamped to 1.999939. This allows us to keep some
    /// composites as is when one of their scaling values happens to be exactly 2.0,
    /// with no perceptual loss.
    ///
    /// The valid range is [-2.0, 2.0]. This should be enforced in the future.
    fn from(num: f32) -> Self {
        if num > Self::MAX && num <= 2.0 {
            Self(Self::MAX)
        } else {
            Self(num)
        }
    }
}
impl From<F2DOT14> for f32 {
    fn from(num: F2DOT14) -> Self {
        num.0
    }
}

#[derive(Shrinkwrap, Debug, PartialEq, Eq)]
pub struct Version16Dot16(pub U16F16);

impl Serialize for Version16Dot16 {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let major = self.0.floor().to_num::<u8>();
        let minor = (self.0.frac().to_num::<f32>() * 160.0) as u8;
        0_u8.to_bytes(data)?;
        major.to_bytes(data)?;
        minor.to_bytes(data)?;
        0_u8.to_bytes(data)
    }
    fn ot_binary_size(&self) -> usize {
        2
    }
}
impl Deserialize for Version16Dot16 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let packed: i32 = c.de()?;
        let orig = packed.to_be_bytes();
        let major = orig[1] as f32;
        let minor = orig[2] as f32 / 160.0;
        Ok(Self(U16F16::from_num(major + minor)))
    }
}

impl From<U16F16> for Version16Dot16 {
    fn from(num: U16F16) -> Self {
        Self(num)
    }
}
impl From<Version16Dot16> for U16F16 {
    fn from(num: Version16Dot16) -> Self {
        num.0
    }
}
#[derive(Shrinkwrap, Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub struct LONGDATETIME(pub chrono::NaiveDateTime);

use chrono::{Duration, NaiveDate};

impl Serialize for LONGDATETIME {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let now = self.timestamp();
        let epoch = NaiveDate::from_ymd_opt(1904, 1, 1)
            .expect("The world is broken")
            .and_hms_opt(0, 0, 0)
            .expect("The world is broken")
            .timestamp();
        (now - epoch).to_bytes(data)
    }
    fn ot_binary_size(&self) -> usize {
        8
    }
}
impl Deserialize for LONGDATETIME {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let diff: i64 = c.de()?;
        let epoch = NaiveDate::from_ymd_opt(1904, 1, 1)
            .expect("The world is broken")
            .and_hms_opt(0, 0, 0)
            .expect("The world is broken");
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

pub use crate::offsets::{Offset16, Offset32, VecOffset, VecOffset16, VecOffset32};
// OK, the offset type is going to be terrifying.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f2dot14_range() {
        assert_eq!(F2DOT14::from_packed(i16::MAX).0, F2DOT14::MAX);
        assert_eq!(F2DOT14::from_packed(0x7000).0, 1.75);
        assert_eq!(F2DOT14::from_packed(0x0000).0, 0.0);
        assert_eq!(F2DOT14::from_packed(i16::MIN).0, -2.0);

        assert_eq!(F2DOT14::from(2.0), F2DOT14(F2DOT14::MAX));
        assert_eq!(F2DOT14::from(1.99999), F2DOT14(F2DOT14::MAX));
        assert_eq!(F2DOT14::from(1.9), F2DOT14(1.9));
    }
}
