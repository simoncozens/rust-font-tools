use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};

/// The 'fpgm' OpenType tag.
pub const TAG: Tag = crate::tag!("fpgm");

/// Represents a font's fpgm (Font Program) table
#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub struct fpgm(Vec<uint8>);

impl Deserialize for fpgm {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        // This is very silly
        Ok(fpgm(c.input.clone()))
    }
}

impl Serialize for fpgm {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        data.put(&self.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tables::fpgm::fpgm;

    #[test]
    fn fpgm_deser() {
        let binary_fpgm = vec![0xb0, 0x00, 0x2c, 0x20, 0xb0, 0x00, 0x55, 0x58];
        let expected = vec![
            0xb0, 0x00, 0x2c, 0x20, 0xb0, 0x00, 0x55, 0x58, // I mean seriously
        ];
        let deserialized: super::fpgm = otspec::de::from_bytes(&binary_fpgm).unwrap();
        assert_eq!(deserialized, fpgm(expected));
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_fpgm);
    }
}
