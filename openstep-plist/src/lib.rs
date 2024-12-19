use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Debug;

use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

pub mod de;
pub mod error;
pub mod ser;


use crate::error::Error;

/// A plist dictionary
pub type Dictionary = BTreeMap<SmolStr, Plist>;

/// An array of plist values
pub type Array = Vec<Plist>;

/// An enum representing a property list.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Plist {
    Dictionary(Dictionary),
    Array(Array),
    String(String),
    Integer(i64),
    Float(OrderedFloat<f64>),
    Data(Vec<u8>),
}

#[derive(Debug)]
pub(crate) enum Token<'a> {
    Eof,
    OpenBrace,
    OpenParen,
    Data(Vec<u8>),
    String(Cow<'a, str>),
    Atom(&'a str),
}

fn is_numeric(b: u8) -> bool {
    b.is_ascii_digit() || b == b'.' || b == b'-'
}

fn is_alnum(b: u8) -> bool {
    // https://github.com/opensource-apple/CF/blob/3cc41a76b1491f50813e28a4ec09954ffa359e6f/CFOldStylePList.c#L79
    is_numeric(b)
        || b.is_ascii_uppercase()
        || b.is_ascii_lowercase()
        || b == b'_'
        || b == b'$'
        || b == b'/'
        || b == b':'
        || b == b'.'
        || b == b'-'
}

// Used for serialization; make sure UUID's get quoted
fn is_alnum_strict(b: u8) -> bool {
    is_alnum(b) && b != b'-'
}

fn is_hex_upper(b: u8) -> bool {
    b.is_ascii_digit() || (b'A'..=b'F').contains(&b)
}

fn is_ascii_whitespace(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\r' || b == b'\n'
}

fn numeric_ok(s: &str) -> bool {
    let s = s.as_bytes();
    if s.is_empty() {
        return false;
    }
    let s = if s.len() > 1 && (*s.first().unwrap(), *s.last().unwrap()) == (b'"', b'"') {
        &s[1..s.len()]
    } else {
        s
    };
    if s.iter().all(|&b| is_hex_upper(b)) && !s.iter().all(|&b| b.is_ascii_digit()) {
        return false;
    }
    if s.len() > 1 && s[0] == b'0' {
        return !s.iter().all(|&b| b.is_ascii_digit());
    }
    // Prevent parsing of "infinity", "inf", "nan" as numbers, we
    // want to keep them as strings (e.g. glyphname)
    // https://doc.rust-lang.org/std/primitive.f64.html#grammar
    if s.eq_ignore_ascii_case(b"infinity")
        || s.eq_ignore_ascii_case(b"inf")
        || s.eq_ignore_ascii_case(b"nan")
    {
        return false;
    }
    true
}

fn skip_ws(s: &str, mut ix: usize) -> usize {
    while ix < s.len() && is_ascii_whitespace(s.as_bytes()[ix]) {
        ix += 1;
    }
    ix
}

impl Plist {
    pub fn parse(s: &str) -> Result<Plist, Error> {
        let (plist, _ix) = Plist::parse_rec(s, 0)?;
        // TODO: check that we're actually at eof
        Ok(plist)
    }

    fn name(&self) -> &'static str {
        match self {
            Plist::Array(..) => "array",
            Plist::Dictionary(..) => "dictionary",
            Plist::Float(..) => "float",
            Plist::Integer(..) => "integer",
            Plist::String(..) => "string",
            Plist::Data(..) => "data",
        }
    }

    pub fn get(&self, key: &str) -> Option<&Plist> {
        match self {
            Plist::Dictionary(d) => d.get(key),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&BTreeMap<SmolStr, Plist>> {
        match self {
            Plist::Dictionary(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Plist]> {
        match self {
            Plist::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Plist::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Plist::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Plist::Integer(i) => Some(*i as f64),
            Plist::Float(f) => Some((*f).into_inner()),
            _ => None,
        }
    }

    pub fn flatten_to_integer(&self) -> Plist {
        match self {
            Plist::Float(f) => {
                if f.fract() == 0.0 {
                    Plist::Integer(f.into_inner() as i64)
                } else {
                    Plist::Float(*f)
                }
            }
            Plist::String(s) => {
                if let Ok(num) = s.parse() {
                    Plist::Integer(num)
                } else {
                    self.clone()
                }
            }
            _ => self.clone(),
        }
    }
    pub fn flatten_to_string(&self) -> Plist {
        match self {
            Plist::Integer(i) => Plist::String(i.to_string()),
            Plist::Float(f) => {
                if f.fract() == 0.0 {
                    Plist::String((f.into_inner() as i64).to_string())
                } else {
                    Plist::String(f.to_string())
                }
            }
            _ => self.clone(),
        }
    }

    pub fn expect_dict(self) -> Result<Dictionary, Error> {
        match self {
            Plist::Dictionary(dict) => Ok(dict),
            _other => Err(Error::UnexpectedDataType {
                expected: "dictionary",
                found: _other.name(),
            }),
        }
    }

    pub fn expect_array(self) -> Result<Array, Error> {
        match self {
            Plist::Array(array) => Ok(array),
            _other => Err(Error::UnexpectedDataType {
                expected: "array",
                found: _other.name(),
            }),
        }
    }

    pub fn expect_string(self) -> Result<String, Error> {
        match self {
            Plist::String(string) => Ok(string),
            _other => Err(Error::UnexpectedDataType {
                expected: "string",
                found: _other.name(),
            }),
        }
    }

    pub fn expect_data(self) -> Result<Vec<u8>, Error> {
        match self {
            Plist::Data(bytes) => Ok(bytes),
            _other => Err(Error::UnexpectedDataType {
                expected: "data",
                found: _other.name(),
            }),
        }
    }

    fn parse_rec(s: &str, ix: usize) -> Result<(Plist, usize), Error> {
        let (tok, mut ix) = Token::lex(s, ix)?;
        match tok {
            Token::Atom(s) => Ok((Plist::parse_atom(s), ix)),
            Token::String(s) => Ok((Plist::String(s.into()), ix)),
            Token::Data(bytes) => Ok((Plist::Data(bytes), ix)),
            Token::OpenBrace => {
                let mut dict = BTreeMap::new();
                loop {
                    if let Some(ix) = Token::expect(s, ix, b'}') {
                        return Ok((Plist::Dictionary(dict), ix));
                    }
                    let (key, next) = Token::lex(s, ix)?;
                    let key_str = Token::try_into_smolstr(key)?;
                    let next = Token::expect(s, next, b'=');
                    if next.is_none() {
                        return Err(Error::ExpectedEquals);
                    }
                    let (val, next) = Self::parse_rec(s, next.unwrap())?;
                    dict.insert(key_str, val);
                    if let Some(next) = Token::expect(s, next, b';') {
                        ix = next;
                    } else {
                        return Err(Error::ExpectedSemicolon);
                    }
                }
            }
            Token::OpenParen => {
                let mut list = Vec::new();
                loop {
                    if let Some(ix) = Token::expect(s, ix, b')') {
                        return Ok((Plist::Array(list), ix));
                    }
                    let (val, next) = Self::parse_rec(s, ix)?;
                    list.push(val);
                    if let Some(ix) = Token::expect(s, next, b')') {
                        return Ok((Plist::Array(list), ix));
                    }
                    if let Some(next) = Token::expect(s, next, b',') {
                        ix = next;
                        if let Some(next) = Token::expect(s, next, b')') {
                            return Ok((Plist::Array(list), next));
                        }
                    } else {
                        return Err(Error::ExpectedComma);
                    }
                }
            }
            _ => Err(Error::UnexpectedToken { name: tok.name() }),
        }
    }

    fn parse_atom(s: &str) -> Plist {
        if numeric_ok(s) {
            if let Ok(num) = s.parse() {
                return Plist::Integer(num);
            }
            if let Ok(num) = s.parse() {
                return Plist::Float(num);
            }
        }
        Plist::String(s.into())
    }

    #[allow(clippy::inherent_to_string, unused)]
    pub fn to_string(&self) -> String {
        crate::ser::to_string(&self).unwrap()
    }

    pub fn is_meaningful(&self) -> bool {
        match self {
            Plist::Array(a) => !a.is_empty(),
            Plist::Dictionary(d) => !d.is_empty(),
            Plist::String(s) => !s.is_empty(),
            Plist::Integer(i) => *i != 0,
            Plist::Float(f) => f.into_inner() != 0.0,
            Plist::Data(d) => !d.is_empty(),
        }
    }
}

impl Default for Plist {
    fn default() -> Self {
        // kind of arbitrary but seems okay
        Plist::Array(Vec::new())
    }
}

fn byte_from_hex(hex: [u8; 2]) -> Result<u8, Error> {
    fn hex_digit_to_byte(digit: u8) -> Result<u8, Error> {
        match digit {
            b'0'..=b'9' => Ok(digit - b'0'),
            b'a'..=b'f' => Ok(digit - b'a' + 10),
            b'A'..=b'F' => Ok(digit - b'A' + 10),
            _ => Err(Error::BadData),
        }
    }
    let maj = hex_digit_to_byte(hex[0])? << 4;
    let min = hex_digit_to_byte(hex[1])?;
    Ok(maj | min)
}

impl<'a> Token<'a> {
    fn lex(s: &'a str, ix: usize) -> Result<(Token<'a>, usize), Error> {
        let start = skip_ws(s, ix);
        if start == s.len() {
            return Ok((Token::Eof, start));
        }
        let b = s.as_bytes()[start];
        match b {
            b'{' => Ok((Token::OpenBrace, start + 1)),
            b'(' => Ok((Token::OpenParen, start + 1)),
            b'<' => {
                let data_start = start + 1;
                let data_end = data_start
                    + s.as_bytes()[data_start..]
                        .iter()
                        .position(|b| *b == b'>')
                        .ok_or(Error::UnclosedData)?;
                let chunks = s.as_bytes()[data_start..data_end].chunks_exact(2);
                if !chunks.remainder().is_empty() {
                    return Err(Error::BadData);
                }
                let data = chunks
                    .map(|x| byte_from_hex(x.try_into().unwrap()))
                    .collect::<Result<_, _>>()?;
                Ok((Token::Data(data), data_end + 1))
            }
            b'"' => {
                let mut ix = start + 1;
                let mut cow_start = ix;
                let mut buf = String::new();
                while ix < s.len() {
                    let b = s.as_bytes()[ix];
                    match b {
                        b'"' => {
                            // End of string
                            let string = if buf.is_empty() {
                                s[cow_start..ix].into()
                            } else {
                                buf.push_str(&s[cow_start..ix]);
                                buf.into()
                            };
                            return Ok((Token::String(string), ix + 1));
                        }
                        b'\\' => {
                            buf.push_str(&s[cow_start..ix]);
                            ix += 1;
                            if ix == s.len() {
                                return Err(Error::UnclosedString);
                            }
                            let b = s.as_bytes()[ix];
                            match b {
                                b'"' | b'\\' => cow_start = ix,
                                b'n' => {
                                    buf.push('\n');
                                    cow_start = ix + 1;
                                }
                                b'r' => {
                                    buf.push('\r');
                                    cow_start = ix + 1;
                                }
                                _ => {
                                    if (b'0'..=b'3').contains(&b) && ix + 2 < s.len() {
                                        // octal escape
                                        let b1 = s.as_bytes()[ix + 1];
                                        let b2 = s.as_bytes()[ix + 2];
                                        if (b'0'..=b'7').contains(&b1)
                                            && (b'0'..=b'7').contains(&b2)
                                        {
                                            let oct =
                                                (b - b'0') * 64 + (b1 - b'0') * 8 + (b2 - b'0');
                                            buf.push(oct as char);
                                            ix += 2;
                                            cow_start = ix + 1;
                                        } else {
                                            return Err(Error::UnknownEscape);
                                        }
                                    } else {
                                        return Err(Error::UnknownEscape);
                                    }
                                }
                            }
                            ix += 1;
                        }
                        _ => ix += 1,
                    }
                }
                Err(Error::UnclosedString)
            }
            _ => {
                if is_alnum(b) {
                    let mut ix = start + 1;
                    while ix < s.len() {
                        if !is_alnum(s.as_bytes()[ix]) {
                            break;
                        }
                        ix += 1;
                    }
                    Ok((Token::Atom(&s[start..ix]), ix))
                } else {
                    Err(Error::UnexpectedChar(s[start..].chars().next().unwrap()))
                }
            }
        }
    }

    fn try_into_smolstr(self) -> Result<SmolStr, Error> {
        match self {
            Token::Atom(s) => Ok(s.into()),
            Token::String(s) => Ok(s.into()),
            _ => Err(Error::NotAString {
                token_name: self.name(),
            }),
        }
    }

    fn expect(s: &str, ix: usize, delim: u8) -> Option<usize> {
        let ix = skip_ws(s, ix);
        if ix < s.len() {
            let b = s.as_bytes()[ix];
            if b == delim {
                return Some(ix + 1);
            }
        }
        None
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            Token::Atom(..) => "Atom",
            Token::String(..) => "String",
            Token::Eof => "Eof",
            Token::OpenBrace => "OpenBrace",
            Token::OpenParen => "OpenParen",
            Token::Data(_) => "Data",
        }
    }
}

impl From<bool> for Plist {
    fn from(x: bool) -> Plist {
        Plist::Integer(x as i64)
    }
}

impl From<String> for Plist {
    fn from(x: String) -> Plist {
        Plist::String(x)
    }
}

impl From<SmolStr> for Plist {
    fn from(x: SmolStr) -> Plist {
        Plist::String(x.into())
    }
}

impl From<i64> for Plist {
    fn from(x: i64) -> Plist {
        Plist::Integer(x)
    }
}

impl From<f64> for Plist {
    fn from(x: f64) -> Plist {
        Plist::Float(x.into())
    }
}

impl From<ordered_float::OrderedFloat<f64>> for Plist {
    fn from(x: ordered_float::OrderedFloat<f64>) -> Plist {
        Plist::Float(f64::from(x).into())
    }
}

impl From<Dictionary> for Plist {
    fn from(x: Dictionary) -> Plist {
        Plist::Dictionary(x)
    }
}

impl<T> From<Vec<T>> for Plist
where
    T: Into<Plist>,
{
    fn from(x: Vec<T>) -> Plist {
        Plist::Array(x.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn parse_unquoted_strings() {
        let contents = r#"
        {
            name = "UFO Filename";
            value1 = ../../build/instance_ufos/Testing_Rg.ufo;
            value2 = _;
            value3 = $;
            value4 = /;
            value5 = :;
            value6 = .;
            value7 = -;
        }
        "#;

        let plist = Plist::parse(contents).unwrap();
        let plist_expected = Plist::Dictionary(BTreeMap::from_iter([
            ("name".into(), Plist::String("UFO Filename".into())),
            (
                "value1".into(),
                Plist::String("../../build/instance_ufos/Testing_Rg.ufo".into()),
            ),
            ("value2".into(), Plist::String("_".into())),
            ("value3".into(), Plist::String("$".into())),
            ("value4".into(), Plist::String("/".into())),
            ("value5".into(), Plist::String(":".into())),
            ("value6".into(), Plist::String(".".into())),
            ("value7".into(), Plist::String("-".into())),
        ]));
        assert_eq!(plist, plist_expected);
    }

    #[test]
    fn parse_binary_data() {
        let contents = r#"
        {
            mydata = <deadbeef>;
        }
            "#;
        let plist = Plist::parse(contents).unwrap();
        let data = plist.get("mydata").unwrap().clone().expect_data().unwrap();
        assert_eq!(data, [0xde, 0xad, 0xbe, 0xef])
    }

    #[test]
    fn ascii_to_hex() {
        assert_eq!(byte_from_hex([b'0', b'1']), Ok(0x01));
        assert_eq!(byte_from_hex([b'0', b'0']), Ok(0x00));
        assert_eq!(byte_from_hex([b'f', b'f']), Ok(0xff));
        assert_eq!(byte_from_hex([b'f', b'0']), Ok(0xf0));
        assert_eq!(byte_from_hex([b'0', b'f']), Ok(0x0f));
    }

    #[test]
    fn parse_to_plist_type() {
        let plist_str = r#"
        {
            name = "meta";
            value = (
                {
                    data = latn;
                    tag = dlng;
                    num = 5;
                },
                {
                    data = "latn,cyrl";
                    tag = slng;
                    num = -3.0;
                }
            );
        }"#;

        let plist = Plist::parse(plist_str).unwrap();
        let root = plist.expect_dict().unwrap();
        assert_eq!(root.get("name").unwrap().as_str(), Some("meta"));
        let value = root.get("value").unwrap().as_array().unwrap();
        assert_eq!(value.len(), 2);
        let first = value[0].as_dict().unwrap();
        assert_eq!(first.get("data").and_then(Plist::as_str), Some("latn"));
        assert_eq!(first.get("tag").and_then(Plist::as_str), Some("dlng"));
        assert_eq!(first.get("num").and_then(Plist::as_i64), Some(5));
        let second = value[1].as_dict().unwrap();
        assert_eq!(
            second.get("data").and_then(Plist::as_str),
            Some("latn,cyrl")
        );
        assert_eq!(second.get("tag").and_then(Plist::as_str), Some("slng"));
        assert_eq!(second.get("num").and_then(Plist::as_f64), Some(-3.0));
    }
}
