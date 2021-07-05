use itertools::put_back;
use itertools::structs::PutBack;
use snafu::Snafu;
use std::collections::HashMap;
use std::vec::IntoIter;

#[derive(Debug, PartialEq)]
pub enum Plist {
    Null,
    String(String),
    Array(Vec<Plist>),
    Dictionary(PlistDictionary),
    Integer(i32),
    Float(f32),
}

pub type PlistDictionary = HashMap<String, Plist>;

impl Plist {
    pub fn dict(&self) -> Option<&PlistDictionary> {
        match self {
            Plist::Dictionary(d) => Some(d),
            _ => None,
        }
    }
    pub fn array(&self) -> Option<&Vec<Plist>> {
        match self {
            Plist::Array(d) => Some(d),
            _ => None,
        }
    }
    pub fn string(&self) -> Option<&String> {
        match self {
            Plist::String(d) => Some(d),
            _ => None,
        }
    }

    pub fn iter_array_of_dicts<'a>(&'a self) -> Box<dyn Iterator<Item = &'a PlistDictionary> + 'a> {
        if let Some(a) = self.array() {
            return Box::new(a.iter().map(|l| l.dict()).flatten());
        }
        Box::new(std::iter::empty())
    }
}

impl From<&Plist> for f32 {
    fn from(p: &Plist) -> f32 {
        match p {
            Plist::Integer(i) => *i as f32,
            Plist::Float(f) => *f,
            _ => 0.0,
        }
    }
}

impl From<&Plist> for i32 {
    fn from(p: &Plist) -> i32 {
        match p {
            Plist::Integer(i) => *i,
            Plist::Float(f) => *f as i32,
            _ => 0,
        }
    }
}

#[derive(Debug, Snafu, PartialEq)]
pub enum PlistError {
    #[snafu(display("Unexpected end of file"))]
    Eof {},

    #[snafu(display("Invalid string character at line {}: {}", line, bad_char))]
    InvalidString { line: usize, bad_char: char },

    #[snafu(display("Unexpected character at line {}: {}", line, bad_char))]
    UnexpectedChar { line: usize, bad_char: char },

    #[snafu(display("Unterminated string beginning at line {}", line))]
    UnterminatedString { line: usize },

    #[snafu(display("Missing comma for array at line {}", line))]
    MissingComma { line: usize },

    #[snafu(display("Expected terminating ')' for array at line {}", line))]
    MissingArrayTerminator { line: usize },

    #[snafu(display("Expected terminating '}}' for dictionary at line {}", line))]
    MissingDictTerminator { line: usize },

    #[snafu(display("Missing ';' on line {}", line))]
    MissingSemicolon { line: usize },

    #[snafu(display("Tried to use something as a number that wasn't a number"))]
    NotANumber,
}

type Result<T, E = PlistError> = std::result::Result<T, E>;

pub struct PlistParser {
    line_no: usize,
    i: PutBack<IntoIter<char>>,
    use_numbers: bool,
}

const NEXT_STEP_DECODING_TABLE: [u32; 128] = [
    0xA0, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCC, 0xCD, 0xCE, 0xCF,
    0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD9, 0xDA, 0xDB, 0xDC, 0xDD, 0xDE, 0xB5, 0xD7, 0xF7,
    0xA9, 0xA1, 0xA2, 0xA3, 0x2044, 0xA5, 0x192, 0xA7, 0xA4, 0x2019, 0x201C, 0xAB, 0x2039, 0x203A,
    0xFB01, 0xFB02, 0xAE, 0x2013, 0x2020, 0x2021, 0xB7, 0xA6, 0xB6, 0x2022, 0x201A, 0x201E, 0x201D,
    0xBB, 0x2026, 0x2030, 0xAC, 0xBF, 0xB9, 0x2CB, 0xB4, 0x2C6, 0x2DC, 0xAF, 0x2D8, 0x2D9, 0xA8,
    0xB2, 0x2DA, 0xB8, 0xB3, 0x2DD, 0x2DB, 0x2C7, 0x2014, 0xB1, 0xBC, 0xBD, 0xBE, 0xE0, 0xE1, 0xE2,
    0xE3, 0xE4, 0xE5, 0xE7, 0xE8, 0xE9, 0xEA, 0xEB, 0xEC, 0xC6, 0xED, 0xAA, 0xEE, 0xEF, 0xF0, 0xF1,
    0x141, 0xD8, 0x152, 0xBA, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xE6, 0xF9, 0xFA, 0xFB, 0x131, 0xFC,
    0xFD, 0x142, 0xF8, 0x153, 0xDF, 0xFE, 0xFF, 0xFFFD, 0xFFFD,
];

fn is_linebreak(c: char) -> bool {
    c == '\n' || c == '\r' || c == '\u{2028}' || c == '\u{2029}'
}

fn is_valid_unquoted_string_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || c == '_'
        || c == '$'
        || c == '/'
        || c == ':'
        || c == '.'
        || c == '-'
}

fn parse_unquoted_string(s: &str) -> Plist {
    if let Ok(i) = s.parse::<i32>() {
        Plist::Integer(i)
    } else if let Ok(f) = s.parse::<f32>() {
        Plist::Float(f)
    } else {
        Plist::String(s.to_string())
    }
}

impl PlistParser {
    fn next_nonspace(&mut self) -> Option<char> {
        while let Some(c) = self.i.next() {
            if ('\x09'..='\x0d').contains(&c) {
                if c == '\x0a' {
                    self.line_no += 1;
                }
                continue;
            }
            if c == '\u{2028}' || c == '\u{2029}' {
                self.line_no += 1;
                continue;
            }
            if c == ' ' {
                continue;
            } else if c == '/' {
                let c2 = self.i.next();
                if c2.is_none() {
                    return Some('/');
                }
                if c2 == Some('/') {
                    while let Some(c3) = self.i.next() {
                        if is_linebreak(c3) {
                            self.line_no += 1;
                            break;
                        }
                    }
                } else if c2 == Some('*') {
                    // Handle C style comments
                    while let Some(c2) = self.i.next() {
                        if is_linebreak(c2) {
                            self.line_no += 1;
                        } else if c2 == '*' {
                            let c3 = self.i.next();
                            if c3 == Some('/') {
                                break;
                            }
                        }
                    }
                } else {
                    return Some(c);
                }
            } else {
                return Some(c);
            }
        }
        None
    }

    fn next_is_digit(&mut self, radix: u32) -> Option<u32> {
        if let Some(c) = self.i.next() {
            if let Some(res) = c.to_digit(radix) {
                return Some(res);
            }
            self.i.put_back(c);
            return None;
        }
        None
    }

    fn get_slashed_char(&mut self) -> char {
        if let Some(mut num) = self.next_is_digit(8) {
            if let Some(num2) = self.next_is_digit(8) {
                num = num * 8 + num2;
                if let Some(num3) = self.next_is_digit(8) {
                    num = num * 8 + num3;
                }
            }
            if num < 128 {
                return num as u8 as char;
            }
            return char::from_u32(NEXT_STEP_DECODING_TABLE[(num - 128) as usize]).unwrap();
        }
        let ch = self.i.next();
        if ch == Some('U') {
            let mut unum = 0;
            while let Some(num) = self.next_is_digit(16) {
                unum = unum * 16 + num;
            }
            return char::from_u32(unum).unwrap();
        }
        match ch {
            Some('a') => '\x07',
            Some('b') => '\x08',
            Some('f') => '\x0c',
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some('\n') => '\n',
            _ => ch.unwrap(),
        }
    }

    fn parse_quoted_plist_string(&mut self, quote: char) -> Result<Plist> {
        let mut s: Vec<char> = vec![];
        let start_line = self.line_no;
        loop {
            if let Some(c) = self.i.next() {
                if c == quote {
                    break;
                }
                if c == '\\' {
                    let ch = self.get_slashed_char();
                    s.push(ch);
                } else {
                    if is_linebreak(c) {
                        self.line_no += 1;
                    }
                    s.push(c);
                }
            } else {
                return Err(PlistError::UnterminatedString { line: start_line });
            }
        }
        Ok(Plist::String(s.into_iter().collect()))
    }

    fn parse_unquoted_plist_string(&mut self, ensure_string: bool) -> Result<Plist> {
        let mut s: Vec<char> = vec![];
        while let Some(c) = self.i.next() {
            if !is_valid_unquoted_string_char(c) {
                self.i.put_back(c);
                break;
            }
            s.push(c);
        }
        let s: String = s.into_iter().collect();
        if !ensure_string && self.use_numbers {
            return Ok(parse_unquoted_string(&s));
        }
        if s.is_empty() {
            return Err(PlistError::Eof {});
        }
        Ok(Plist::String(s))
    }

    fn parse_plist_string(&mut self, required: bool) -> Result<Plist> {
        if let Some(c) = self.next_nonspace() {
            if c == '\'' || c == '"' {
                return self.parse_quoted_plist_string(c);
            } else if is_valid_unquoted_string_char(c) {
                // println!("Parsing unquoted string beginning {}", c);
                self.i.put_back(c);
                return self.parse_unquoted_plist_string(true);
            } else if required {
                return Err(PlistError::InvalidString {
                    line: self.line_no,
                    bad_char: c,
                });
            } else {
                self.i.put_back(c);
            }
        }
        Err(PlistError::Eof {})
    }

    fn parse_plist_array(&mut self) -> Result<Plist> {
        let mut v = vec![];
        // println!("Start of array loop");
        loop {
            let o = self.parse_plist_object(false)?;
            // println!("Got {:?}", o);
            v.push(o);
            if let Some(c) = self.next_nonspace() {
                // print!("Next char was {}", c);
                if c != ',' {
                    self.i.put_back(c);
                    break;
                }
            } else {
                return Err(PlistError::MissingComma { line: self.line_no });
            }
            // println!("Going for another");
        }
        let c = self.next_nonspace();
        // println!("Looking for end, got {:?}", c);
        if c.is_none() || c.unwrap() != ')' {
            return Err(PlistError::MissingArrayTerminator { line: self.line_no });
        }
        Ok(Plist::Array(v))
    }

    fn parse_plist_dict(&mut self) -> Result<Plist> {
        let mut dict: HashMap<String, Plist> = HashMap::new();
        loop {
            let key = self.parse_plist_string(false);
            if let Ok(Plist::String(key)) = key {
                let c = self.next_nonspace();
                if c.is_none() {
                    return Err(PlistError::MissingSemicolon { line: self.line_no });
                }
                let c = c.unwrap();
                if c == ';' {
                    let value = Plist::String(key.clone());
                    dict.insert(key, value);
                } else {
                    let value = if c == '=' {
                        self.parse_plist_object(true)?
                    } else {
                        return Err(PlistError::UnexpectedChar {
                            line: self.line_no,
                            bad_char: c,
                        });
                    };
                    dict.insert(key, value);
                    let c = self.next_nonspace();
                    if c.is_none() || c.unwrap() != ';' {
                        return Err(PlistError::MissingSemicolon { line: self.line_no });
                    }
                }
            } else {
                break;
            }
        }
        let c = self.next_nonspace();
        if c.is_none() || c.unwrap() != '}' {
            return Err(PlistError::MissingDictTerminator { line: self.line_no });
        }

        Ok(Plist::Dictionary(dict))
    }

    fn parse_plist_object(&mut self, required: bool) -> Result<Plist> {
        if let Some(c) = self.next_nonspace() {
            if c == '\'' || c == '"' {
                return self.parse_quoted_plist_string(c);
            } else if c == '{' {
                return self.parse_plist_dict();
            } else if c == '(' {
                return self.parse_plist_array();
            } else if is_valid_unquoted_string_char(c) {
                self.i.put_back(c);
                return self.parse_unquoted_plist_string(false);
            } else {
                self.i.put_back(c);
                if required {
                    return Err(PlistError::UnexpectedChar {
                        bad_char: c,
                        line: self.line_no,
                    });
                }
                return Ok(Plist::Null);
            }
        }
        Err(PlistError::Eof {})
    }

    pub fn parse(data: String, use_numbers: bool) -> Result<Plist> {
        let iter = data.chars().collect::<Vec<_>>().into_iter();
        let mut parser = Self {
            line_no: 1,
            i: put_back(iter),
            use_numbers,
        };
        parser.parse_plist_object(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::Plist;
    use crate::PlistError;
    use crate::PlistParser;
    use std::fs;

    use std::iter::FromIterator;

    macro_rules! hashmap {
            ($($k:expr => $v:expr),* $(,)?) => {
                std::collections::HashMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
            };
        }

    #[test]
    fn test_strings() {
        let input = "'foo'".to_string();
        let res = PlistParser::parse(input, true).unwrap();
        assert_eq!(res, Plist::String("foo".to_string()));
        let input = "'foo\"".to_string();
        let res = PlistParser::parse(input, true);
        assert_eq!(res.unwrap_err(), PlistError::UnterminatedString { line: 1 });
    }

    #[test]
    fn test_numbers() {
        let input = "123".to_string();
        let res = PlistParser::parse(input, true).unwrap();
        assert_eq!(res, Plist::Integer(123));
        let input = "-123.45".to_string();
        let res = PlistParser::parse(input, true).unwrap();
        assert_eq!(res, Plist::Float(-123.45));
    }

    #[test]
    fn test_skipping() {
        let input = "         'foo'".to_string();
        let res = PlistParser::parse(input, true).unwrap();
        assert_eq!(res, Plist::String("foo".to_string()));

        let input = " /* Comment */ 'foo'".to_string();
        let res = PlistParser::parse(input, true).unwrap();
        assert_eq!(res, Plist::String("foo".to_string()));

        let input = "\n\n'foo'".to_string();
        let res = PlistParser::parse(input, true).unwrap();
        assert_eq!(res, Plist::String("foo".to_string()));
    }

    #[test]
    fn test_array() {
        let input = "(123,456)".to_string();
        let res = PlistParser::parse(input, true).unwrap();
        assert_eq!(
            res,
            Plist::Array(vec![Plist::Integer(123), Plist::Integer(456)])
        );
    }

    #[test]
    fn test_dict() {
        let input = "{a = \"x123\";}".to_string();
        let res = PlistParser::parse(input, true).expect("Whatever");
        assert_eq!(
            res,
            Plist::Dictionary(hashmap!("a".to_string() => Plist::String("x123".to_string())))
        );

        let t_e = vec![
            (
                "{a=1;}",
                Plist::Dictionary(hashmap!("a".to_string() => Plist::String("1".to_string()))),
            ),
            (
                "{\"a\"=\"1\";}",
                Plist::Dictionary(hashmap!("a".to_string() => Plist::String("1".to_string()))),
            ),
            (
                "{'a'='1';}",
                Plist::Dictionary(hashmap!("a".to_string() => Plist::String("1".to_string()))),
            ),
            (
                "{\na = 1;\n}",
                Plist::Dictionary(hashmap!("a".to_string() => Plist::String("1".to_string()))),
            ),
            (
                "{\na\n=\n1;\n}",
                Plist::Dictionary(hashmap!("a".to_string() => Plist::String("1".to_string()))),
            ),
            (
                "{a=1;b;}",
                Plist::Dictionary(
                    hashmap!("a".to_string() => Plist::String("1".to_string()), "b".to_string() => Plist::String("b".to_string())),
                ),
            ),
        ];

        for (t, e) in t_e.iter() {
            let res = PlistParser::parse(t.to_string(), false).expect("Whatever");
            assert_eq!(res, *e);
        }
    }
}
