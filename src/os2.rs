#![allow(non_camel_case_types, non_snake_case)]

use otspec::types::*;
use otspec_macros::tables;
use serde::{Deserialize, Serialize};

tables!(
    Panose {
        u8 panose0
        u8 panose1
        u8 panose2
        u8 panose3
        u8 panose4
        u8 panose5
        u8 panose6
        u8 panose7
        u8 panose8
        u8 panose9
    }
    os2 {
        uint16	version
        int16	xAvgCharWidth
        uint16	usWeightClass
        uint16	usWidthClass
        uint16	fsType
        int16	ySubscriptXSize
        int16	ySubscriptYSize
        int16	ySubscriptXOffset
        int16	ySubscriptYOffset
        int16	ySuperscriptXSize
        int16	ySuperscriptYSize
        int16	ySuperscriptXOffset
        int16	ySuperscriptYOffset
        int16	yStrikeoutSize
        int16	yStrikeoutPosition
        int16	sFamilyClass
        Panose	panose
        uint32	ulUnicodeRange1
        uint32	ulUnicodeRange2
        uint32	ulUnicodeRange3
        uint32	ulUnicodeRange4
        Tag	achVendID
        uint16	fsSelection
        uint16	usFirstCharIndex
        uint16	usLastCharIndex
        int16	sTypoAscender
        int16	sTypoDescender
        int16	sTypoLineGap
        uint16	usWinAscent
        uint16	usWinDescent
        Maybe(uint32)	ulCodePageRange1
        Maybe(uint32)	ulCodePageRange2
        Maybe(int16)	sxHeight
        Maybe(int16)	sCapHeight
        Maybe(uint16)	usDefaultChar
        Maybe(uint16)	usBreakChar
        Maybe(uint16)	usMaxContext
        Maybe(uint16)	usLowerOpticalPointSize
        Maybe(uint16)	usUpperOpticalPointSize
    }
);
