use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
extern crate otspec;

use otspec::types::*;
use otspec::{deserialize_visitor, read_field};
use otspec_macros::tables;

tables!(
maxp05 {
    uint16  numGlyphs
}

maxp10 {
    uint16  numGlyphs
    uint16  maxPoints
    uint16  maxContours
    uint16  maxCompositePoints
    uint16  maxCompositeContours
    uint16  maxZones
    uint16  maxTwilightPoints
    uint16  maxStorage
    uint16  maxFunctionDefs
    uint16  maxInstructionDefs
    uint16  maxStackElements
    uint16  maxSizeOfInstructions
    uint16  maxComponentElements
    uint16  maxComponentDepth
});

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum MaxpVariant {
    Maxp05(maxp05),
    Maxp10(maxp10),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct maxp {
    #[serde(with = "Version16Dot16")]
    pub version: U16F16,
    #[serde(flatten)]
    pub table: MaxpVariant,
}

impl maxp {
    pub fn new05(num_glyphs: u16) -> maxp {
        maxp {
            version: U16F16::from_num(0.5),
            table: MaxpVariant::Maxp05(maxp05 {
                numGlyphs: num_glyphs,
            }),
        }
    }
    pub fn new10(
        numGlyphs: u16,
        maxPoints: u16,
        maxContours: u16,
        maxCompositePoints: u16,
        maxCompositeContours: u16,
        maxComponentElements: u16,
        maxComponentDepth: u16,
    ) -> maxp {
        maxp {
            version: U16F16::from_num(1.0),
            table: MaxpVariant::Maxp10(maxp10 {
                numGlyphs,
                maxPoints,
                maxContours,
                maxCompositePoints,
                maxCompositeContours,
                maxZones: 2,
                maxTwilightPoints: 0,
                maxStorage: 0,
                maxFunctionDefs: 0,
                maxInstructionDefs: 0,
                maxStackElements: 0,
                maxSizeOfInstructions: 0,
                maxComponentElements,
                maxComponentDepth,
            }),
        }
    }
    pub fn num_glyphs(&self) -> u16 {
        match &self.table {
            MaxpVariant::Maxp05(s) => s.numGlyphs,
            MaxpVariant::Maxp10(s) => s.numGlyphs,
        }
    }
    pub fn set_num_glyphs(&mut self, num: u16) {
        match &mut self.table {
            MaxpVariant::Maxp05(s) => s.numGlyphs = num,
            MaxpVariant::Maxp10(s) => s.numGlyphs = num,
        }
    }
}

deserialize_visitor!(
    maxp,
    MaxpVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let version = read_field!(seq, i32, "a maxp version");
        if version == 0x00005000 {
            return Ok(maxp {
                version: U16F16::from_num(0.5),
                table: MaxpVariant::Maxp05(read_field!(seq, maxp05, "a maxp05 table")),
            });
        }
        if version == 0x00010000 {
            return Ok(maxp {
                version: U16F16::from_num(1.0),
                table: MaxpVariant::Maxp10(read_field!(seq, maxp10, "a maxp05 table")),
            });
        }
        Err(serde::de::Error::custom("Unknown maxp version"))
    }
);

#[cfg(test)]
mod tests {
    use crate::maxp;

    use otspec::ser;
    use otspec::types::U16F16;

    #[test]
    fn maxp_ser_v05() {
        let v = maxp::maxp {
            version: U16F16::from_num(0.5),
            table: maxp::MaxpVariant::Maxp05(maxp::maxp05 { numGlyphs: 935 }),
        };
        let binary_maxp = ser::to_bytes(&v).unwrap();
        let maxp_expectation = vec![0x00, 0x00, 0x50, 0x00, 0x03, 0xa7];
        assert_eq!(binary_maxp, maxp_expectation);
        // let deserialized: maxp::maxp = otspec::de::from_bytes(&binary_maxp).unwrap();
        // assert_eq!(deserialized, v);
    }

    #[test]
    fn maxp_ser_v10() {
        let v = maxp::maxp {
            version: U16F16::from_num(1.0),
            table: maxp::MaxpVariant::Maxp10(maxp::maxp10 {
                numGlyphs: 1117,
                maxPoints: 98,
                maxContours: 7,
                maxCompositePoints: 0,
                maxCompositeContours: 0,
                maxZones: 2,
                maxTwilightPoints: 0,
                maxStorage: 0,
                maxFunctionDefs: 0,
                maxInstructionDefs: 0,
                maxStackElements: 0,
                maxSizeOfInstructions: 0,
                maxComponentElements: 0,
                maxComponentDepth: 0,
            }),
        };
        let binary_maxp = ser::to_bytes(&v).unwrap();
        let maxp_expectation = vec![
            0x00, 0x01, 0x00, 0x00, 0x04, 0x5d, 0x00, 0x62, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(binary_maxp, maxp_expectation);
    }

    #[test]
    fn maxp_de_v05() {
        let v = maxp::maxp {
            version: U16F16::from_num(0.5),
            table: maxp::MaxpVariant::Maxp05(maxp::maxp05 { numGlyphs: 935 }),
        };
        let binary_maxp = vec![0x00, 0x00, 0x50, 0x00, 0x03, 0xa7];
        let deserialized: maxp::maxp = otspec::de::from_bytes(&binary_maxp).unwrap();
        assert_eq!(deserialized, v);
    }
}
