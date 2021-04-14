#![allow(non_camel_case_types, non_snake_case)]

use crate::types::{int16, uint16, uint32, Fixed, LONGDATETIMEshim, LONGDATETIME};
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct head {
    pub majorVersion: uint16,
    pub minorVersion: uint16,
    pub fontRevision: Fixed,
    pub checksumAdjustment: uint32,
    pub magicNumber: uint32,
    pub flags: uint16,
    pub unitsPerEm: uint16,
    #[serde(with = "LONGDATETIMEshim")]
    pub created: LONGDATETIME,
    #[serde(with = "LONGDATETIMEshim")]
    pub modified: LONGDATETIME,
    pub xMin: int16,
    pub yMin: int16,
    pub xMax: int16,
    pub yMax: int16,
    pub macStyle: uint16,
    pub lowestRecPPEM: uint16,
    pub fontDirectionHint: int16,
    pub indexToLocFormat: int16,
    pub glyphDataFormat: int16,
}
