use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};

/// The 'cvt ' OpenType tag.
pub const TAG: Tag = crate::tag!("cvt ");

/// Represents a font's cvt (Control Value) table
#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub struct cvt(Vec<FWORD>);

impl Deserialize for cvt {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let mut v = vec![];
        while c.ptr < c.input.len() {
            let val: FWORD = c.de()?;
            v.push(val)
        }
        Ok(cvt(v))
    }
}

impl Serialize for cvt {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        for v in &self.0 {
            data.put(v)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tables::cvt::cvt;

    #[test]
    fn cvt_deser() {
        let binary_cvt = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x18,
            0x00, 0x18, 0x00, 0x18, 0x01, 0x93, 0xff, 0xf7, 0x01, 0x93, 0xff, 0xf7, 0x00, 0x4b,
            0x00, 0x4b, 0x00, 0x67, 0x00, 0x67, 0x02, 0x83, 0xff, 0xf3, 0x02, 0xa8, 0x01, 0x98,
            0xff, 0xf5, 0xfe, 0xec, 0x02, 0x87, 0xff, 0xf3, 0x02, 0xa8, 0x01, 0x98, 0xff, 0xf5,
            0xfe, 0xec, 0x00, 0x18, 0x00, 0x18, 0x00, 0x18, 0x00, 0x18, 0x02, 0xc9, 0x01, 0x2a,
            0x02, 0xc9, 0x01, 0x2a, 0x00, 0x4a, 0x00, 0x4a, 0x00, 0x6a, 0x00, 0x6a, 0x02, 0x0b,
            0x00, 0x01, 0x02, 0x65, 0xff, 0xf3,
        ];
        let expected = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 24, 24, 24, 24, 403, -9, 403,
            -9, 75, 75, 103, 103, 643, -13, 680, 408, -11, -276, 647, -13, 680, 408, -11, -276, 24,
            24, 24, 24, 713, 298, 713, 298, 74, 74, 106, 106, 523, 1, 613, -13,
        ];
        let deserialized: super::cvt = otspec::de::from_bytes(&binary_cvt).unwrap();
        assert_eq!(deserialized, cvt(expected));
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_cvt);
    }
}
