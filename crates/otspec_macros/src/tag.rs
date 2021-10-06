use std::{fmt::Display, iter::FromIterator};

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

/// Expand a macro of the form `tag!("test")`.
pub(crate) fn expand_tag(item: TokenStream) -> TokenStream {
    match expand_tag_impl(item) {
        Ok(tokens) => tokens,
        Err(e) => e.into_compile_error(),
    }
}

fn expand_tag_impl(item: TokenStream) -> Result<TokenStream, SyntaxError> {
    let input = match item.into_iter().next() {
        Some(TokenTree::Literal(lit)) => expect_tag_literal(lit)?,
        _other => {
            return Err(SyntaxError {
                span: Span::call_site(),
                message: "expected string literal".into(),
            })
        }
    };

    let padding = 4 - input.len();
    let padding = &"    "[..padding];
    let tag_lit = format!(
        "unsafe {{ otspec::types::Tag::from_raw_unchecked(*b\"{}{}\") }}",
        input, padding
    );
    Ok(tag_lit.parse().unwrap())
}

/// tag must be 1-4 bytes long, all in the printable range 0x20..=0xFE
fn expect_tag_literal(lit: Literal) -> Result<String, SyntaxError> {
    let span = lit.span();
    let repr = lit.to_string();
    if !repr.starts_with('"') || !repr.ends_with('"') {
        return Err(SyntaxError::new(span, "expected string literal"));
    }
    let repr = parse_lit_str_cooked(&lit.to_string());

    let repr = repr.trim_matches('"');
    if repr.is_empty() || repr.len() > 4 {
        return Err(SyntaxError::new(span, "tag must be 1..=4 bytes long"));
    }

    if let Some(idx) = repr.bytes().position(|b| !(0x20..=0x7E).contains(&b)) {
        return Err(SyntaxError::new(
            span,
            format!(
                "illegal byte '0x{:02X}' at position {}",
                repr.as_bytes()[idx],
                idx
            ),
        ));
    }

    Ok(repr.to_owned())
}

struct SyntaxError {
    message: String,
    span: Span,
}

impl SyntaxError {
    fn new(span: Span, message: impl Display) -> Self {
        Self {
            span,
            message: message.to_string(),
        }
    }
}

impl SyntaxError {
    fn into_compile_error(self) -> TokenStream {
        // compile_error! { $message }
        TokenStream::from_iter(vec![
            TokenTree::Ident(Ident::new("compile_error", self.span)),
            TokenTree::Punct({
                let mut punct = Punct::new('!', Spacing::Alone);
                punct.set_span(self.span);
                punct
            }),
            TokenTree::Group({
                let mut group = Group::new(Delimiter::Brace, {
                    TokenStream::from_iter(vec![TokenTree::Literal({
                        let mut string = Literal::string(&self.message);
                        string.set_span(self.span);
                        string
                    })])
                });
                group.set_span(self.span);
                group
            }),
        ])
    }
}

// taken directly from syn:
// https://github.com/dtolnay/syn/blob/69148aa2ff558bb4f10322ecc9ab505c4b835aba/src/lit.rs#L907-L1556
// Clippy false positive
// https://github.com/rust-lang-nursery/rust-clippy/issues/2329
#[allow(clippy::needless_continue)]
fn parse_lit_str_cooked(mut s: &str) -> Box<str> {
    fn byte(s: &str, idx: usize) -> u8 {
        s.as_bytes()[idx]
    }

    fn next_chr(s: &str) -> char {
        s.chars().next().unwrap_or('\0')
    }

    fn backslash_x(s: &str) -> (u8, &str) {
        let mut ch = 0;
        let b0 = byte(s, 0);
        let b1 = byte(s, 1);
        ch += 0x10
            * match b0 {
                b'0'..=b'9' => b0 - b'0',
                b'a'..=b'f' => 10 + (b0 - b'a'),
                b'A'..=b'F' => 10 + (b0 - b'A'),
                _ => panic!("unexpected non-hex character after \\x"),
            };
        ch += match b1 {
            b'0'..=b'9' => b1 - b'0',
            b'a'..=b'f' => 10 + (b1 - b'a'),
            b'A'..=b'F' => 10 + (b1 - b'A'),
            _ => panic!("unexpected non-hex character after \\x"),
        };
        (ch, &s[2..])
    }

    fn backslash_u(mut s: &str) -> (char, &str) {
        if byte(s, 0) != b'{' {
            panic!("{}", "expected { after \\u");
        }
        s = &s[1..];

        let mut ch = 0;
        let mut digits = 0;
        loop {
            let b = byte(s, 0);
            let digit = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => 10 + b - b'a',
                b'A'..=b'F' => 10 + b - b'A',
                b'_' if digits > 0 => {
                    s = &s[1..];
                    continue;
                }
                b'}' if digits == 0 => panic!("invalid empty unicode escape"),
                b'}' => break,
                _ => panic!("unexpected non-hex character after \\u"),
            };
            if digits == 6 {
                panic!("overlong unicode escape (must have at most 6 hex digits)");
            }
            ch *= 0x10;
            ch += u32::from(digit);
            digits += 1;
            s = &s[1..];
        }
        assert!(byte(s, 0) == b'}');
        s = &s[1..];

        if let Some(ch) = char::from_u32(ch) {
            (ch, s)
        } else {
            panic!("character code {:x} is not a valid unicode character", ch);
        }
    }

    assert_eq!(byte(s, 0), b'"');
    s = &s[1..];

    let mut content = String::new();
    'outer: loop {
        let ch = match byte(s, 0) {
            b'"' => break,
            b'\\' => {
                let b = byte(s, 1);
                s = &s[2..];
                match b {
                    b'x' => {
                        let (byte, rest) = backslash_x(s);
                        s = rest;
                        assert!(byte <= 0x80, "Invalid \\x byte in string literal");
                        char::from_u32(u32::from(byte)).unwrap()
                    }
                    b'u' => {
                        let (chr, rest) = backslash_u(s);
                        s = rest;
                        chr
                    }
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'\\' => '\\',
                    b'0' => '\0',
                    b'\'' => '\'',
                    b'"' => '"',
                    b'\r' | b'\n' => loop {
                        let ch = next_chr(s);
                        if ch.is_whitespace() {
                            s = &s[ch.len_utf8()..];
                        } else {
                            continue 'outer;
                        }
                    },
                    b => panic!("unexpected byte {:?} after \\ character in byte literal", b),
                }
            }
            b'\r' => {
                assert_eq!(byte(s, 1), b'\n', "Bare CR not allowed in string");
                s = &s[2..];
                '\n'
            }
            _ => {
                let ch = next_chr(s);
                s = &s[ch.len_utf8()..];
                ch
            }
        };
        content.push(ch);
    }

    assert!(s.starts_with('"'));
    content.into_boxed_str()
}
