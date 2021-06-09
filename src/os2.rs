#![allow(non_camel_case_types, non_snake_case)]

use crate::utils::int_list_to_num;
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::tables;
use std::collections::{BTreeMap, HashSet};

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
    os2core {
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
    }

    os2v1 {
        uint32   ulCodePageRange1
        uint32   ulCodePageRange2
    }

    os2v2 {
        int16	sxHeight
        int16	sCapHeight
        uint16	usDefaultChar
        uint16	usBreakChar
        uint16	usMaxContext
    }
    os2v5 {
        uint16	usLowerOpticalPointSize
        uint16	usUpperOpticalPointSize
    }
);

/// Represents a font's OS/2 (OS/2 and Windows Metrics) table
#[derive(Debug, PartialEq)]
pub struct os2 {
    /// Table version (between 0 and 5)
    pub version: uint16,
    /// Average width (xMax-xMin) of all non-empty glyphs
    pub xAvgCharWidth: int16,
    /// Visual weight class (0-1000)
    pub usWeightClass: uint16,
    /// Visual width class (1=Ultra-Condensed <-> 9=Ultra-Expanded)
    pub usWidthClass: uint16,
    /// Font embedding permissions bit field
    pub fsType: uint16,
    /// Horizontal size of subscript glyphs
    pub ySubscriptXSize: int16,
    /// Vertical size of subscript glyphs
    pub ySubscriptYSize: int16,
    /// Horizontal offset of subscript glyphs
    pub ySubscriptXOffset: int16,
    /// Vertical offset of subscript glyphs
    pub ySubscriptYOffset: int16,
    /// Horizontal size of superscript glyphs
    pub ySuperscriptXSize: int16,
    /// Vertical size of superscript glyphs
    pub ySuperscriptYSize: int16,
    /// Horizontal offset of superscript glyphs
    pub ySuperscriptXOffset: int16,
    /// Vertical offset of superscript glyphs
    pub ySuperscriptYOffset: int16,
    /// Thickness of strikeout dash (usually same as em dash thickness)
    pub yStrikeoutSize: int16,
    /// Strikeout dash position above baseline
    pub yStrikeoutPosition: int16,
    /// IBM font class parameter. See <https://docs.microsoft.com/en-us/typography/opentype/spec/ibmfc>.
    pub sFamilyClass: int16,
    /// PANOSE metrics. See <https://monotype.github.io/panose/pan1.htm>.
    pub panose: Panose,
    /// Supported unicode range (bitfield)
    pub ulUnicodeRange1: uint32,
    /// Supported unicode range (bitfield)
    pub ulUnicodeRange2: uint32,
    /// Supported unicode range (bitfield)
    pub ulUnicodeRange3: uint32,
    /// Supported unicode range (bitfield)
    pub ulUnicodeRange4: uint32,
    /// Registered vendor ID. See <https://docs.microsoft.com/en-gb/typography/vendors/>.
    pub achVendID: Tag,
    /// Font selection bitfield
    pub fsSelection: uint16,
    /// Minimum Unicode codepoint supported by font
    pub usFirstCharIndex: uint16,
    /// Maximum Unicode codepoint supported by font
    pub usLastCharIndex: uint16,
    /// Typographic ascender
    pub sTypoAscender: int16,
    /// Typographic descender
    pub sTypoDescender: int16,
    /// Typographic line gap
    pub sTypoLineGap: int16,
    /// Windows clipping region ascender
    pub usWinAscent: uint16,
    /// Windows clipping region descender (Usually positive!)
    pub usWinDescent: uint16,
    /// Bitfield of supported codepages (Version >=1)
    pub ulCodePageRange1: Option<uint32>,
    /// Bitfield of supported codepages (Version >=1)
    pub ulCodePageRange2: Option<uint32>,
    /// x-Height (Version >= 2)
    pub sxHeight: Option<int16>,
    /// Cap height (Version >= 2)
    pub sCapHeight: Option<int16>,
    /// GID used for undefined glyph (Version >= 2)
    pub usDefaultChar: Option<uint16>,
    /// GID used for word break glyph (Version >= 2)
    pub usBreakChar: Option<uint16>,
    /// Length of largest contextual lookup (Version >= 2)
    pub usMaxContext: Option<uint16>,
    /// Lowest supported optical point size. Deprecated, use STAT instead (Version >= 5)
    pub usLowerOpticalPointSize: Option<uint16>,
    /// Highest supported optical point size. Deprecated, use STAT instead (Version >= 5)
    pub usUpperOpticalPointSize: Option<uint16>,
}

impl Serialize for os2 {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        os2core {
            version: self.version,
            xAvgCharWidth: self.xAvgCharWidth,
            usWeightClass: self.usWeightClass,
            usWidthClass: self.usWidthClass,
            fsType: self.fsType,
            ySubscriptXSize: self.ySubscriptXSize,
            ySubscriptYSize: self.ySubscriptYSize,
            ySubscriptXOffset: self.ySubscriptXOffset,
            ySubscriptYOffset: self.ySubscriptYOffset,
            ySuperscriptXSize: self.ySuperscriptXSize,
            ySuperscriptYSize: self.ySuperscriptYSize,
            ySuperscriptXOffset: self.ySuperscriptXOffset,
            ySuperscriptYOffset: self.ySuperscriptYOffset,
            yStrikeoutSize: self.yStrikeoutSize,
            yStrikeoutPosition: self.yStrikeoutPosition,
            sFamilyClass: self.sFamilyClass,
            panose: Panose {
                panose0: self.panose.panose0,
                panose1: self.panose.panose1,
                panose2: self.panose.panose2,
                panose3: self.panose.panose3,
                panose4: self.panose.panose4,
                panose5: self.panose.panose5,
                panose6: self.panose.panose6,
                panose7: self.panose.panose7,
                panose8: self.panose.panose8,
                panose9: self.panose.panose9,
            },
            ulUnicodeRange1: self.ulUnicodeRange1,
            ulUnicodeRange2: self.ulUnicodeRange2,
            ulUnicodeRange3: self.ulUnicodeRange3,
            ulUnicodeRange4: self.ulUnicodeRange4,
            achVendID: self.achVendID,
            fsSelection: self.fsSelection,
            usFirstCharIndex: self.usFirstCharIndex,
            usLastCharIndex: self.usLastCharIndex,
            sTypoAscender: self.sTypoAscender,
            sTypoDescender: self.sTypoDescender,
            sTypoLineGap: self.sTypoLineGap,
            usWinAscent: self.usWinAscent,
            usWinDescent: self.usWinDescent,
        }
        .to_bytes(data)?;
        if self.version > 0 {
            (&os2v1 {
                ulCodePageRange1: self.ulCodePageRange1.unwrap_or(0),
                ulCodePageRange2: self.ulCodePageRange2.unwrap_or(0),
            })
                .to_bytes(data)?;
        }
        if self.version > 1 {
            (&os2v2 {
                sxHeight: self.sxHeight.unwrap_or(0),
                sCapHeight: self.sCapHeight.unwrap_or(0),
                usDefaultChar: self.usDefaultChar.unwrap_or(0),
                usBreakChar: self.usBreakChar.unwrap_or(0),
                usMaxContext: self.usMaxContext.unwrap_or(0),
            })
                .to_bytes(data)?;
        }
        if self.version > 4 {
            (&os2v5 {
                usLowerOpticalPointSize: self.usLowerOpticalPointSize.unwrap_or(0),
                usUpperOpticalPointSize: self.usUpperOpticalPointSize.unwrap_or(0),
            })
                .to_bytes(data)?;
        }
        Ok(())
    }
}

impl Deserialize for os2 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let core: os2core = c.de()?;
        let mut res = os2 {
            version: core.version,
            xAvgCharWidth: core.xAvgCharWidth,
            usWeightClass: core.usWeightClass,
            usWidthClass: core.usWidthClass,
            fsType: core.fsType,
            ySubscriptXSize: core.ySubscriptXSize,
            ySubscriptYSize: core.ySubscriptYSize,
            ySubscriptXOffset: core.ySubscriptXOffset,
            ySubscriptYOffset: core.ySubscriptYOffset,
            ySuperscriptXSize: core.ySuperscriptXSize,
            ySuperscriptYSize: core.ySuperscriptYSize,
            ySuperscriptXOffset: core.ySuperscriptXOffset,
            ySuperscriptYOffset: core.ySuperscriptYOffset,
            yStrikeoutSize: core.yStrikeoutSize,
            yStrikeoutPosition: core.yStrikeoutPosition,
            sFamilyClass: core.sFamilyClass,
            panose: core.panose,
            ulUnicodeRange1: core.ulUnicodeRange1,
            ulUnicodeRange2: core.ulUnicodeRange2,
            ulUnicodeRange3: core.ulUnicodeRange3,
            ulUnicodeRange4: core.ulUnicodeRange4,
            achVendID: core.achVendID,
            fsSelection: core.fsSelection,
            usFirstCharIndex: core.usFirstCharIndex,
            usLastCharIndex: core.usLastCharIndex,
            sTypoAscender: core.sTypoAscender,
            sTypoDescender: core.sTypoDescender,
            sTypoLineGap: core.sTypoLineGap,
            usWinAscent: core.usWinAscent,
            usWinDescent: core.usWinDescent,
            ulCodePageRange1: None,
            ulCodePageRange2: None,
            sxHeight: None,
            sCapHeight: None,
            usDefaultChar: None,
            usBreakChar: None,
            usMaxContext: None,
            usLowerOpticalPointSize: None,
            usUpperOpticalPointSize: None,
        };
        if core.version > 0 {
            let v1: os2v1 = c.de()?;
            res.ulCodePageRange1 = Some(v1.ulCodePageRange1);
            res.ulCodePageRange2 = Some(v1.ulCodePageRange2);
        }
        if core.version > 1 {
            let v2: os2v2 = c.de()?;
            res.sxHeight = Some(v2.sxHeight);
            res.sCapHeight = Some(v2.sCapHeight);
            res.usDefaultChar = Some(v2.usDefaultChar);
            res.usBreakChar = Some(v2.usBreakChar);
            res.usMaxContext = Some(v2.usMaxContext);
        }
        if core.version > 4 {
            let v5: os2v5 = c.de()?;
            res.usLowerOpticalPointSize = Some(v5.usLowerOpticalPointSize);
            res.usUpperOpticalPointSize = Some(v5.usUpperOpticalPointSize);
        }
        Ok(res)
    }
}

impl os2 {
    /// Populate ulCodePageRange fields using a
    pub fn int_list_to_code_page_ranges(&mut self, bitlist: &Vec<u8>) {
        let mut code_pages1 = bitlist.clone();
        code_pages1.sort_unstable();
        let split_at = code_pages1
            .iter()
            .position(|&x| x >= 32)
            .unwrap_or_else(|| bitlist.len());
        let mut code_pages2 = code_pages1.split_off(split_at);
        code_pages2.iter_mut().for_each(|x| *x -= 32);

        self.ulCodePageRange1 = Some(int_list_to_num(&code_pages1) as u32);
        self.ulCodePageRange2 = Some(int_list_to_num(&code_pages2) as u32);
    }

    /// implementation based on ufo2ft:
    /// https://github.com/googlefonts/ufo2ft/blob/main/lib/ufo2ft/util.py#l307
    pub fn calc_code_page_ranges(&mut self, mapping: &BTreeMap<u32, u16>) {
        let unicodes = mapping.keys().copied().collect::<HashSet<_>>();
        let mut code_page_ranges: Vec<u8> = vec![];

        let unicodes_contains = |char| unicodes.contains(&(char as u32));

        let has_ascii = (0x20..0x7E).all(|x| unicodes.contains(&x));
        let has_lineart = unicodes_contains('┤');

        if unicodes_contains('Þ') && has_ascii {
            code_page_ranges.push(0); // Latin 1
        }
        if unicodes_contains('Ľ') && has_ascii {
            code_page_ranges.push(1); // Latin 2
        }
        if unicodes_contains('Б') {
            code_page_ranges.push(2); // Cyrillic
            if unicodes_contains('Ѕ') && has_lineart {
                code_page_ranges.push(57); // IBM Cyrillic
            }
            if unicodes_contains('╜') && has_lineart {
                code_page_ranges.push(49); // MS-DOS Russian
            }
        }
        if unicodes_contains('Ά') {
            code_page_ranges.push(3); // Greek
            if unicodes_contains('½') && has_lineart {
                code_page_ranges.push(48); // IBM Greek
            }
            if unicodes_contains('√') && has_lineart {
                code_page_ranges.push(60); // Greek, former 437 G
            }
        }
        if unicodes_contains('İ') && has_ascii {
            code_page_ranges.push(4); //  Turkish
            if has_lineart {
                code_page_ranges.push(56); //  IBM turkish
            }
        }
        if unicodes_contains('א') {
            code_page_ranges.push(5); //  Hebrew
            if has_lineart && unicodes_contains('√') {
                code_page_ranges.push(53); //  Hebrew
            }
        }
        if unicodes_contains('ر') {
            code_page_ranges.push(6); //  Arabic
            if unicodes_contains('√') {
                code_page_ranges.push(51); //  Arabic
            }
            if has_lineart {
                code_page_ranges.push(61); //  Arabic; ASMO 708
            }
        }
        if unicodes_contains('ŗ') && has_ascii {
            code_page_ranges.push(7); //  Windows Baltic
            if has_lineart {
                code_page_ranges.push(59); //  MS-DOS Baltic
            }
        }
        if unicodes_contains('₫') && has_ascii {
            code_page_ranges.push(8); //  Vietnamese
        }
        if unicodes_contains('ๅ') {
            code_page_ranges.push(16); //  Thai
        }
        if unicodes_contains('エ') {
            code_page_ranges.push(17); //  JIS/Japan
        }
        if unicodes_contains('ㄅ') {
            code_page_ranges.push(18); //  Chinese: Simplified chars
        }
        if unicodes_contains('ㄱ') {
            code_page_ranges.push(19); //  Korean wansung
        }
        if unicodes_contains('央') {
            code_page_ranges.push(20); //  Chinese: Traditional chars
        }
        if unicodes_contains('곴') {
            code_page_ranges.push(21); //  Korean Johab
        }
        if unicodes_contains('♥') && has_ascii {
            code_page_ranges.push(30); //  OEM Character Set
                                       //  TODO: Symbol bit has a special meaning (check the spec), we need
                                       //  to confirm if this is wanted by default.
                                       //  elif chr(0xF000) <= char <= chr(0xF0FF):
                                       //     code_page_ranges.push(31)          //  Symbol Character Set
        }
        if unicodes_contains('þ') && has_ascii && has_lineart {
            code_page_ranges.push(54); //  MS-DOS Icelandic
        }
        if unicodes_contains('╚') && has_ascii {
            code_page_ranges.push(62); //  WE/Latin 1
            code_page_ranges.push(63); //  US
        }
        if has_ascii && has_lineart && unicodes_contains('√') {
            if unicodes_contains('Å') {
                code_page_ranges.push(50); //  MS-DOS Nordic
            }
            if unicodes_contains('é') {
                code_page_ranges.push(52); //  MS-DOS Canadian French
            }
            if unicodes_contains('õ') {
                code_page_ranges.push(55); //  MS-DOS Portuguese
            }
        }
        if has_ascii && unicodes_contains('‰') && unicodes_contains('∑') {
            code_page_ranges.push(29); // Macintosh Character Set (US Roman)
        }
        // when no codepage ranges can be enabled, fall back to enabling bit 0
        // (Latin 1) so that the font works in MS Word:
        // https://github.com/googlei18n/fontmake/issues/468
        if code_page_ranges.is_empty() {
            code_page_ranges.push(0);
        }
        self.int_list_to_code_page_ranges(&code_page_ranges);
    }
}
