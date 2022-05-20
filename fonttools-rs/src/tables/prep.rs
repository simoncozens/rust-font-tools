use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, ReaderContext, SerializationError, Serialize, Serializer,
};

/// The 'prep' OpenType tag.
pub const TAG: Tag = crate::tag!("prep");

/// Represents a font's prep (Font Program) table
#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub struct prep(Vec<uint8>);

impl Deserialize for prep {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        // This is very silly
        Ok(prep(c.input.clone()))
    }
}

impl Serialize for prep {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        data.put(&self.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tables::prep::prep;

    #[test]
    fn prep_deser() {
        let binary_prep = vec![0xb0, 0x00, 0x2c, 0x20, 0xb0, 0x00, 0x55, 0x58];
        let expected = vec![
            0xb0, 0x00, 0x2c, 0x20, 0xb0, 0x00, 0x55, 0x58, // I mean seriously
        ];
        let deserialized: super::prep = otspec::de::from_bytes(&binary_prep).unwrap();
        assert_eq!(deserialized, prep(expected));
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_prep);
    }
}
