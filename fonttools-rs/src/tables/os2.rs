use otspec::types::*;
use otspec::utils::filtered_bitset_to_num;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::tables;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};

/// The 'OS/2' OpenType tag.
pub const TAG: Tag = crate::tag!("OS/2");

// Unicode ranges data from the OpenType OS/2 table specification v1.8.4
// https://docs.microsoft.com/en-us/typography/opentype/spec/os2#ulunicoderange1-bits-031ulunicoderange2-bits-3263ulunicoderange3-bits-6495ulunicoderange4-bits-96127
const OS2_UNICODE_RANGES: [(u32, &str, u32, u32); 169] = [
    (0, "Basic Latin", 0x0000, 0x007F),
    (1, "Latin-1 Supplement", 0x0080, 0x00FF),
    (2, "Latin Extended-A", 0x0100, 0x017F),
    (3, "Latin Extended-B", 0x0180, 0x024F),
    (4, "IPA Extensions", 0x0250, 0x02AF),
    (4, "Phonetic Extensions", 0x1D00, 0x1D7F), // Added in OpenType 1.5 for OS/2 version 4.
    (4, "Phonetic Extensions Supplement", 0x1D80, 0x1DBF), // Added in OpenType 1.5 for OS/2 version 4.
    (5, "Spacing Modifier Letters", 0x02B0, 0x02FF),
    (5, "Modifier Tone Letters", 0xA700, 0xA71F), // Added in OpenType 1.5 for OS/2 version 4.
    (6, "Combining Diacritical Marks", 0x0300, 0x036F),
    (6, "Combining Diacritical Marks Supplement", 0x1DC0, 0x1DFF), // Added in OpenType 1.5 for OS/2 version 4.
    (7, "Greek and Coptic", 0x0370, 0x03FF),
    (8, "Coptic", 0x2C80, 0x2CFF), // Added in OpenType 1.5 for OS/2 version 4. See below for other version differences.
    (9, "Cyrillic", 0x0400, 0x04FF),
    (9, "Cyrillic Supplement", 0x0500, 0x052F), // Added in OpenType 1.4 for OS/2 version 3.
    (9, "Cyrillic Extended-A", 0x2DE0, 0x2DFF), // Added in OpenType 1.5 for OS/2 version 4.
    (9, "Cyrillic Extended-B", 0xA640, 0xA69F), // Added in OpenType 1.5 for OS/2 version 4.
    (10, "Armenian", 0x0530, 0x058F),
    (11, "Hebrew", 0x0590, 0x05FF),
    (12, "Vai", 0xA500, 0xA63F), // Added in OpenType 1.5 for OS/2 version 4. See below for other version differences.
    (13, "Arabic", 0x0600, 0x06FF),
    (13, "Arabic Supplement", 0x0750, 0x077F), // Added in OpenType 1.5 for OS/2 version 4.
    (14, "NKo", 0x07C0, 0x07FF), // Added in OpenType 1.5 for OS/2 version 4. See below for other version differences.
    (15, "Devanagari", 0x0900, 0x097F),
    (16, "Bengali", 0x0980, 0x09FF),
    (17, "Gurmukhi", 0x0A00, 0x0A7F),
    (18, "Gujarati", 0x0A80, 0x0AFF),
    (19, "Oriya", 0x0B00, 0x0B7F),
    (20, "Tamil", 0x0B80, 0x0BFF),
    (21, "Telugu", 0x0C00, 0x0C7F),
    (22, "Kannada", 0x0C80, 0x0CFF),
    (23, "Malayalam", 0x0D00, 0x0D7F),
    (24, "Thai", 0x0E00, 0x0E7F),
    (25, "Lao", 0x0E80, 0x0EFF),
    (26, "Georgian", 0x10A0, 0x10FF),
    (26, "Georgian Supplement", 0x2D00, 0x2D2F), // Added in OpenType 1.5 for OS/2 version 4.
    (27, "Balinese", 0x1B00, 0x1B7F), // Added in OpenType 1.5 for OS/2 version 4. See below for other version differences.
    (28, "Hangul Jamo", 0x1100, 0x11FF),
    (29, "Latin Extended Additional", 0x1E00, 0x1EFF),
    (29, "Latin Extended-C", 0x2C60, 0x2C7F), // Added in OpenType 1.5 for OS/2 version 4.
    (29, "Latin Extended-D", 0xA720, 0xA7FF), // Added in OpenType 1.5 for OS/2 version 4.
    (30, "Greek Extended", 0x1F00, 0x1FFF),
    (31, "General Punctuation", 0x2000, 0x206F),
    (31, "Supplemental Punctuation", 0x2E00, 0x2E7F), // Added in OpenType 1.5 for OS/2 version 4.
    (32, "Superscripts And Subscripts", 0x2070, 0x209F),
    (33, "Currency Symbols", 0x20A0, 0x20CF),
    (
        34,
        "Combining Diacritical Marks For Symbols",
        0x20D0,
        0x20FF,
    ),
    (35, "Letterlike Symbols", 0x2100, 0x214F),
    (36, "Number Forms", 0x2150, 0x218F),
    (37, "Arrows", 0x2190, 0x21FF),
    (37, "Supplemental Arrows-A", 0x27F0, 0x27FF), // Added in OpenType 1.4 for OS/2 version 3.
    (37, "Supplemental Arrows-B", 0x2900, 0x297F), // Added in OpenType 1.4 for OS/2 version 3.
    (37, "Miscellaneous Symbols and Arrows", 0x2B00, 0x2BFF), // Added in OpenType 1.5 for OS/2 version 4.
    (38, "Mathematical Operators", 0x2200, 0x22FF),
    (38, "Supplemental Mathematical Operators", 0x2A00, 0x2AFF), // Added in OpenType 1.4 for OS/2 version 3.
    (38, "Miscellaneous Mathematical Symbols-A", 0x27C0, 0x27EF), // Added in OpenType 1.4 for OS/2 version 3.
    (38, "Miscellaneous Mathematical Symbols-B", 0x2980, 0x29FF), // Added in OpenType 1.4 for OS/2 version 3.
    (39, "Miscellaneous Technical", 0x2300, 0x23FF),
    (40, "Control Pictures", 0x2400, 0x243F),
    (41, "Optical Character Recognition", 0x2440, 0x245F),
    (42, "Enclosed Alphanumerics", 0x2460, 0x24FF),
    (43, "Box Drawing", 0x2500, 0x257F),
    (44, "Block Elements", 0x2580, 0x259F),
    (45, "Geometric Shapes", 0x25A0, 0x25FF),
    (46, "Miscellaneous Symbols", 0x2600, 0x26FF),
    (47, "Dingbats", 0x2700, 0x27BF),
    (48, "CJK Symbols And Punctuation", 0x3000, 0x303F),
    (49, "Hiragana", 0x3040, 0x309F),
    (50, "Katakana", 0x30A0, 0x30FF),
    (50, "Katakana Phonetic Extensions", 0x31F0, 0x31FF), // Added in OpenType 1.4 for OS/2 version 3.
    (51, "Bopomofo", 0x3100, 0x312F),
    (51, "Bopomofo Extended", 0x31A0, 0x31BF), // Added in OpenType 1.3, extending OS/2 version 2.
    (52, "Hangul Compatibility Jamo", 0x3130, 0x318F),
    (53, "Phags-pa", 0xA840, 0xA87F), // Added in OpenType 1.5 for OS/2 version 4. See below for other version differences.
    (54, "Enclosed CJK Letters And Months", 0x3200, 0x32FF),
    (55, "CJK Compatibility", 0x3300, 0x33FF),
    (56, "Hangul Syllables", 0xAC00, 0xD7AF),
    (57, "Non-Plane 0", 0x10000, 0x10FFFF), // Implies at least one character beyond the Basic Multilingual Plane. First assigned in OpenType 1.3 for OS/2 version 2.
    (58, "Phoenician", 0x10900, 0x1091F),   // First assigned in OpenType 1.5 for OS/2 version 4.
    (59, "CJK Unified Ideographs", 0x4E00, 0x9FFF),
    (59, "CJK Radicals Supplement", 0x2E80, 0x2EFF), // Added in OpenType 1.3 for OS/2 version 2.
    (59, "Kangxi Radicals", 0x2F00, 0x2FDF),         // Added in OpenType 1.3 for OS/2 version 2.
    (59, "Ideographic Description Characters", 0x2FF0, 0x2FFF), // Added in OpenType 1.3 for OS/2 version 2.
    (59, "CJK Unified Ideographs Extension A", 0x3400, 0x4DBF), // Added in OpenType 1.3 for OS/2 version 2.
    (59, "CJK Unified Ideographs Extension B", 0x20000, 0x2A6DF), // Added in OpenType 1.4 for OS/2 version 3.
    (59, "Kanbun", 0x3190, 0x319F), // Added in OpenType 1.4 for OS/2 version 3.
    (60, "Private Use Area (plane 0)", 0xE000, 0xF8FF),
    (61, "CJK Strokes", 0x31C0, 0x31EF), // Range added in OpenType 1.5 for OS/2 version 4.
    (61, "CJK Compatibility Ideographs", 0xF900, 0xFAFF),
    (
        61,
        "CJK Compatibility Ideographs Supplement",
        0x2F800,
        0x2FA1F,
    ), // Added in OpenType 1.4 for OS/2 version 3.
    (62, "Alphabetic Presentation Forms", 0xFB00, 0xFB4F),
    (63, "Arabic Presentation Forms-A", 0xFB50, 0xFDFF),
    (64, "Combining Half Marks", 0xFE20, 0xFE2F),
    (65, "Vertical Forms", 0xFE10, 0xFE1F), // Range added in OpenType 1.5 for OS/2 version 4.
    (65, "CJK Compatibility Forms", 0xFE30, 0xFE4F),
    (66, "Small Form Variants", 0xFE50, 0xFE6F),
    (67, "Arabic Presentation Forms-B", 0xFE70, 0xFEFF),
    (68, "Halfwidth And Fullwidth Forms", 0xFF00, 0xFFEF),
    (69, "Specials", 0xFFF0, 0xFFFF),
    (70, "Tibetan", 0x0F00, 0x0FFF), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (71, "Syriac", 0x0700, 0x074F),  // First assigned in OpenType 1.3, extending OS/2 version 2.
    (72, "Thaana", 0x0780, 0x07BF),  // First assigned in OpenType 1.3, extending OS/2 version 2.
    (73, "Sinhala", 0x0D80, 0x0DFF), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (74, "Myanmar", 0x1000, 0x109F), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (75, "Ethiopic", 0x1200, 0x137F), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (75, "Ethiopic Supplement", 0x1380, 0x139F), // Added in OpenType 1.5 for OS/2 version 4.
    (75, "Ethiopic Extended", 0x2D80, 0x2DDF), // Added in OpenType 1.5 for OS/2 version 4.
    (76, "Cherokee", 0x13A0, 0x13FF), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (77, "Unified Canadian Aboriginal Syllabics", 0x1400, 0x167F), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (78, "Ogham", 0x1680, 0x169F), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (79, "Runic", 0x16A0, 0x16FF), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (80, "Khmer", 0x1780, 0x17FF), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (80, "Khmer Symbols", 0x19E0, 0x19FF), // Added in OpenType 1.5 for OS/2 version 4.
    (81, "Mongolian", 0x1800, 0x18AF), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (82, "Braille Patterns", 0x2800, 0x28FF), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (83, "Yi Syllables", 0xA000, 0xA48F), // First assigned in OpenType 1.3, extending OS/2 version 2.
    (83, "Yi Radicals", 0xA490, 0xA4CF),  // Added in OpenType 1.3, extending OS/2 version 2.
    (84, "Tagalog", 0x1700, 0x171F),      // First assigned in OpenType 1.4 for OS/2 version 3.
    (84, "Hanunoo", 0x1720, 0x173F),      // Added in OpenType 1.4 for OS/2 version 3.
    (84, "Buhid", 0x1740, 0x175F),        // Added in OpenType 1.4 for OS/2 version 3.
    (84, "Tagbanwa", 0x1760, 0x177F),     // Added in OpenType 1.4 for OS/2 version 3.
    (85, "Old Italic", 0x10300, 0x1032F), // First assigned in OpenType 1.4 for OS/2 version 3.
    (86, "Gothic", 0x10330, 0x1034F),     // First assigned in OpenType 1.4 for OS/2 version 3.
    (87, "Deseret", 0x10400, 0x1044F),    // First assigned in OpenType 1.4 for OS/2 version 3.
    (88, "Byzantine Musical Symbols", 0x1D000, 0x1D0FF), // First assigned in OpenType 1.4 for OS/2 version 3.
    (88, "Musical Symbols", 0x1D100, 0x1D1FF), // Added in OpenType 1.4 for OS/2 version 3.
    (88, "Ancient Greek Musical Notation", 0x1D200, 0x1D24F), // Added in OpenType 1.5 for OS/2 version 4.
    (89, "Mathematical Alphanumeric Symbols", 0x1D400, 0x1D7FF), // First assigned in OpenType 1.4 for OS/2 version 3.
    (90, "Private Use (plane 15)", 0xF0000, 0xFFFFD), // First assigned in OpenType 1.4 for OS/2 version 3.
    (90, "Private Use (plane 16)", 0x100000, 0x10FFFD), // Added in OpenType 1.4 for OS/2 version 3.
    (91, "Variation Selectors", 0xFE00, 0xFE0F), // First assigned in OpenType 1.4 for OS/2 version 3.
    (91, "Variation Selectors Supplement", 0xE0100, 0xE01EF), // Added in OpenType 1.4 for OS/2 version 3.
    (92, "Tags", 0xE0000, 0xE007F), // First assigned in OpenType 1.4 for OS/2 version 3.
    (93, "Limbu", 0x1900, 0x194F),  // First assigned in OpenType 1.5 for OS/2 version 4.
    (94, "Tai Le", 0x1950, 0x197F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (95, "New Tai Lue", 0x1980, 0x19DF), // First assigned in OpenType 1.5 for OS/2 version 4.
    (96, "Buginese", 0x1A00, 0x1A1F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (97, "Glagolitic", 0x2C00, 0x2C5F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (98, "Tifinagh", 0x2D30, 0x2D7F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (99, "Yijing Hexagram Symbols", 0x4DC0, 0x4DFF), // First assigned in OpenType 1.5 for OS/2 version 4.
    (100, "Syloti Nagri", 0xA800, 0xA82F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (101, "Linear B Syllabary", 0x10000, 0x1007F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (101, "Linear B Ideograms", 0x10080, 0x100FF), // Added in OpenType 1.5 for OS/2 version 4.
    (101, "Aegean Numbers", 0x10100, 0x1013F),     // Added in OpenType 1.5 for OS/2 version 4.
    (102, "Ancient Greek Numbers", 0x10140, 0x1018F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (103, "Ugaritic", 0x10380, 0x1039F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (104, "Old Persian", 0x103A0, 0x103DF), // First assigned in OpenType 1.5 for OS/2 version 4.
    (105, "Shavian", 0x10450, 0x1047F),  // First assigned in OpenType 1.5 for OS/2 version 4.
    (106, "Osmanya", 0x10480, 0x104AF),  // First assigned in OpenType 1.5 for OS/2 version 4.
    (107, "Cypriot Syllabary", 0x10800, 0x1083F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (108, "Kharoshthi", 0x10A00, 0x10A5F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (109, "Tai Xuan Jing Symbols", 0x1D300, 0x1D35F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (110, "Cuneiform", 0x12000, 0x123FF), // First assigned in OpenType 1.5 for OS/2 version 4.
    (110, "Cuneiform Numbers and Punctuation", 0x12400, 0x1247F), // Added in OpenType 1.5 for OS/2 version 4.
    (111, "Counting Rod Numerals", 0x1D360, 0x1D37F), // First assigned in OpenType 1.5 for OS/2 version 4.
    (112, "Sundanese", 0x1B80, 0x1BBF), // First assigned in OpenType 1.5 for OS/2 version 4.
    (113, "Lepcha", 0x1C00, 0x1C4F),    // First assigned in OpenType 1.5 for OS/2 version 4.
    (114, "Ol Chiki", 0x1C50, 0x1C7F),  // First assigned in OpenType 1.5 for OS/2 version 4.
    (115, "Saurashtra", 0xA880, 0xA8DF), // First assigned in OpenType 1.5 for OS/2 version 4.
    (116, "Kayah Li", 0xA900, 0xA92F),  // First assigned in OpenType 1.5 for OS/2 version 4.
    (117, "Rejang", 0xA930, 0xA95F),    // First assigned in OpenType 1.5 for OS/2 version 4.
    (118, "Cham", 0xAA00, 0xAA5F),      // First assigned in OpenType 1.5 for OS/2 version 4.
    (119, "Ancient Symbols", 0x10190, 0x101CF), // First assigned in OpenType 1.5 for OS/2 version 4.
    (120, "Phaistos Disc", 0x101D0, 0x101FF), // First assigned in OpenType 1.5 for OS/2 version 4.
    (121, "Carian", 0x102A0, 0x102DF),        // First assigned in OpenType 1.5 for OS/2 version 4.
    (121, "Lycian", 0x10280, 0x1029F),        // Added in OpenType 1.5 for OS/2 version 4.
    (121, "Lydian", 0x10920, 0x1093F),        // Added in OpenType 1.5 for OS/2 version 4.
    (122, "Domino Tiles", 0x1F030, 0x1F09F),  // First assigned in OpenType 1.5 for OS/2 version 4.
    (122, "Mahjong Tiles", 0x1F000, 0x1F02F), // First assigned in OpenType 1.5 for OS/2 version 4.
                                              // (123-127, Reserved for process-internal usage")
];

/// Determine a glyph's unicode range by using binary search so we get a O(m log n) runtime
fn glyph_unicode_range(target: &u32) -> u32 {
    let index = OS2_UNICODE_RANGES
        .binary_search_by(|(_bit, _name, low, high)| {
            if target < low {
                Ordering::Greater
            } else if target > high {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .unwrap_or(0);
    OS2_UNICODE_RANGES[index].0
}

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
#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types, non_snake_case)]
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
    /// Calculate and set the Unicode ranges from a mapping of codepoints to glyph IDs
    pub fn calc_unicode_ranges(&mut self, mapping: &BTreeMap<u32, u16>) {
        let mut unicode_ranges: HashSet<u8> = HashSet::new();
        // we need to sort OS2_UNICODE_RANGES by the 3rd column which is the starting code point
        // for each range because we use binary search.
        for uni in mapping.keys() {
            unicode_ranges.insert(glyph_unicode_range(uni) as u8);
        }
        self.ulUnicodeRange1 = filtered_bitset_to_num(unicode_ranges.iter(), 0, 31) as u32;
        self.ulUnicodeRange2 = filtered_bitset_to_num(unicode_ranges.iter(), 32, 63) as u32;
        self.ulUnicodeRange3 = filtered_bitset_to_num(unicode_ranges.iter(), 64, 95) as u32;
        self.ulUnicodeRange4 = filtered_bitset_to_num(unicode_ranges.iter(), 96, 127) as u32;
    }

    /// Calculate and set the code page ranges from a mapping of codepoints to glyph IDs
    // implementation based on ufo2ft:
    // https://github.com/googlefonts/ufo2ft/blob/main/lib/ufo2ft/util.py#l307
    pub fn calc_code_page_ranges(&mut self, mapping: &BTreeMap<u32, u16>) {
        let unicodes = mapping.keys().copied().collect::<HashSet<_>>();
        let mut code_page_ranges: HashSet<u8> = HashSet::default();

        let unicodes_contains = |char| unicodes.contains(&(char as u32));

        let has_ascii = (0x20..0x7E).all(|x| unicodes.contains(&x));
        let has_lineart = unicodes_contains('┤');

        if unicodes_contains('Þ') && has_ascii {
            code_page_ranges.insert(0); // Latin 1
        }
        if unicodes_contains('Ľ') && has_ascii {
            code_page_ranges.insert(1); // Latin 2
        }
        if unicodes_contains('Б') {
            code_page_ranges.insert(2); // Cyrillic
            if unicodes_contains('Ѕ') && has_lineart {
                code_page_ranges.insert(57); // IBM Cyrillic
            }
            if unicodes_contains('╜') && has_lineart {
                code_page_ranges.insert(49); // MS-DOS Russian
            }
        }
        if unicodes_contains('Ά') {
            code_page_ranges.insert(3); // Greek
            if unicodes_contains('½') && has_lineart {
                code_page_ranges.insert(48); // IBM Greek
            }
            if unicodes_contains('√') && has_lineart {
                code_page_ranges.insert(60); // Greek, former 437 G
            }
        }
        if unicodes_contains('İ') && has_ascii {
            code_page_ranges.insert(4); //  Turkish
            if has_lineart {
                code_page_ranges.insert(56); //  IBM turkish
            }
        }
        if unicodes_contains('א') {
            code_page_ranges.insert(5); //  Hebrew
            if has_lineart && unicodes_contains('√') {
                code_page_ranges.insert(53); //  Hebrew
            }
        }
        if unicodes_contains('ر') {
            code_page_ranges.insert(6); //  Arabic
            if unicodes_contains('√') {
                code_page_ranges.insert(51); //  Arabic
            }
            if has_lineart {
                code_page_ranges.insert(61); //  Arabic; ASMO 708
            }
        }
        if unicodes_contains('ŗ') && has_ascii {
            code_page_ranges.insert(7); //  Windows Baltic
            if has_lineart {
                code_page_ranges.insert(59); //  MS-DOS Baltic
            }
        }
        if unicodes_contains('₫') && has_ascii {
            code_page_ranges.insert(8); //  Vietnamese
        }
        if unicodes_contains('ๅ') {
            code_page_ranges.insert(16); //  Thai
        }
        if unicodes_contains('エ') {
            code_page_ranges.insert(17); //  JIS/Japan
        }
        if unicodes_contains('ㄅ') {
            code_page_ranges.insert(18); //  Chinese: Simplified chars
        }
        if unicodes_contains('ㄱ') {
            code_page_ranges.insert(19); //  Korean wansung
        }
        if unicodes_contains('央') {
            code_page_ranges.insert(20); //  Chinese: Traditional chars
        }
        if unicodes_contains('곴') {
            code_page_ranges.insert(21); //  Korean Johab
        }
        if unicodes_contains('♥') && has_ascii {
            code_page_ranges.insert(30); //  OEM Character Set
                                         //  TODO: Symbol bit has a special meaning (check the spec), we need
                                         //  to confirm if this is wanted by default.
                                         //  elif chr(0xF000) <= char <= chr(0xF0FF):
                                         //     code_page_ranges.insert(31)          //  Symbol Character Set
        }
        if unicodes_contains('þ') && has_ascii && has_lineart {
            code_page_ranges.insert(54); //  MS-DOS Icelandic
        }
        if unicodes_contains('╚') && has_ascii {
            code_page_ranges.insert(62); //  WE/Latin 1
            code_page_ranges.insert(63); //  US
        }
        if has_ascii && has_lineart && unicodes_contains('√') {
            if unicodes_contains('Å') {
                code_page_ranges.insert(50); //  MS-DOS Nordic
            }
            if unicodes_contains('é') {
                code_page_ranges.insert(52); //  MS-DOS Canadian French
            }
            if unicodes_contains('õ') {
                code_page_ranges.insert(55); //  MS-DOS Portuguese
            }
        }
        if has_ascii && unicodes_contains('‰') && unicodes_contains('∑') {
            code_page_ranges.insert(29); // Macintosh Character Set (US Roman)
        }
        // when no codepage ranges can be enabled, fall back to enabling bit 0
        // (Latin 1) so that the font works in MS Word:
        // https://github.com/googlei18n/fontmake/issues/468
        if code_page_ranges.is_empty() {
            code_page_ranges.insert(0);
        }
        self.ulCodePageRange1 = Some(filtered_bitset_to_num(code_page_ranges.iter(), 0, 31) as u32);
        self.ulCodePageRange2 =
            Some(filtered_bitset_to_num(code_page_ranges.iter(), 32, 63) as u32);
    }
}
