use crate::types::*;
use crate::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};

// These have to be serialized/deserialized by hand because of annoying
// bit-packing things.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct Device {
    pub startSize: uint16,
    pub endSize: uint16,
    pub deltaFormat: Option<uint16>, // Suggestion
    pub deltaValues: Vec<i8>,
}

impl Deserialize for Device {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let startSize: uint16 = c.de()?;
        let endSize: uint16 = c.de()?;
        let format: uint16 = c.de()?;
        let mut values: Vec<i8> = vec![];
        if format != 0x8000 {
            let mut count = endSize - startSize + 1;
            let num_bits = 1 << format;
            let minus_offset: i16 = 1 << num_bits;
            let mask = (1 << num_bits) - 1;
            let sign_mask = 1 << (num_bits - 1);
            let mut tmp: u16 = 0;
            let mut shift = 0;
            while count > 0 {
                if shift == 0 {
                    tmp = c.de()?;
                    shift = 16;
                }
                shift -= num_bits;
                let mut value: i16 = ((tmp >> shift) & mask) as i16;
                if (value & sign_mask) != 0 {
                    value -= minus_offset;
                }
                values.push(value as i8);
                count -= 1;
            }
        }
        Ok(Device {
            startSize,
            endSize,
            deltaFormat: Some(format),
            deltaValues: values,
        })
    }
}

impl Device {
    fn suggest_format(&self) -> uint16 {
        for &val in &self.deltaValues {
            if !(-9..=8).contains(&val) {
                return 3;
            }
            if !(-3..=2).contains(&val) {
                return 2;
            }
        }
        1
    }
}
impl Serialize for Device {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        data.put(self.startSize)?;
        data.put(self.endSize)?;
        let format = self.deltaFormat.unwrap_or_else(|| self.suggest_format());
        data.put(format)?;
        // Horrible bit-packing time
        let num_bits = 1 << format;
        let mask: i16 = (1 << num_bits) - 1;
        let mut tmp: uint16 = 0;
        let mut shift: uint16 = 16;
        for &value in &self.deltaValues {
            shift -= num_bits;
            tmp |= ((value as i16 & mask) as u16) << shift;
            if shift == 0 {
                data.put(tmp)?;
                tmp = 0;
                shift = 16;
            }
        }
        if shift != 16 {
            data.put(tmp)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_de() {
        let binary_device = vec![0x00, 0x0b, 0x00, 0x0f, 0x00, 0x01, 0xf5, 0x40];
        let deserialized: Device = otspec::de::from_bytes(&binary_device).unwrap();
        let expected = Device {
            startSize: 11,
            endSize: 15,
            deltaFormat: Some(1),
            deltaValues: vec![-1, -1, 1, 1, 1],
        };
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn device_ser() {
        let device = Device {
            startSize: 11,
            endSize: 15,
            deltaFormat: None,
            deltaValues: vec![-1, -1, 1, 1, 1],
        };
        let binary_device = vec![0x00, 0x0b, 0x00, 0x0f, 0x00, 0x01, 0xf5, 0x40];
        assert_eq!(otspec::ser::to_bytes(&device).unwrap(), binary_device);
    }
}
