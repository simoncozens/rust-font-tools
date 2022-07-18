use std::fmt::Write as _; // import without risk of name clashing

use proc_macro::{Delimiter, TokenStream, TokenTree};

#[cfg(nightly)]
fn expect_group(item: Option<TokenTree>, delimiter: Delimiter) -> TokenStream {
    match item {
        Some(TokenTree::Group(i)) => {
            if i.delimiter() == delimiter {
                return i.stream();
            }
            let err = i.span().error(format!(
                "Expected an {:?}, saw {:?} ",
                delimiter,
                i.delimiter()
            ));
            err.emit();
            panic!("Syntax error");
        }
        None => {
            panic!("Expected delimiter, found end of macro")
        }
        Some(i) => {
            let err = i.span().error("Expected an ident");
            err.emit();
            panic!("Syntax error");
        }
    }
}

#[cfg(not(nightly))]
fn expect_group(item: Option<TokenTree>, delimiter: Delimiter) -> TokenStream {
    match item {
        Some(TokenTree::Group(i)) => {
            if i.delimiter() == delimiter {
                i.stream()
            } else {
                let tokens =
                    quote::quote_spanned!(i.span().into()=>compile_error!("expected bool"));
                tokens.into()
            }
        }
        None => {
            let tokens = quote::quote! {
                compile_error!("Expected delimiter, found end of macro")
            };
            tokens.into()
        }
        Some(i) => {
            let tokens =
                quote::quote_spanned!(i.span().into()=>compile_error!("expected an ident"));
            tokens.into()
        }
    }
}

#[cfg(nightly)]
fn expect_ident(item: Option<TokenTree>) -> String {
    match item {
        Some(TokenTree::Ident(i)) => i.to_string(),
        None => {
            panic!("Expected identifier, found end of macro")
        }
        Some(i) => {
            let err = i.span().error("Expected an ident");
            err.emit();
            panic!("Syntax error");
        }
    }
}

#[cfg(not(nightly))]
fn expect_ident(item: Option<TokenTree>) -> String {
    match item {
        Some(TokenTree::Ident(i)) => i.to_string(),
        None => {
            panic!("Expected identifier, found end of macro")
        }
        Some(i) => {
            panic!("Syntax error: expected ident, found tokens: '{:?}'", i);
        }
    }
}

fn has_pragma(item: &Option<&TokenTree>) -> Option<String> {
    match item {
        Some(TokenTree::Group(i)) => {
            if i.delimiter() == Delimiter::Bracket {
                return Some(i.to_string());
            }
            None
        }
        _ => None,
    }
}

fn special_type(t: &str) -> Option<String> {
    match t {
        /* We don't use types from the fixed crate here because fixed-point
        arithmetic is an artefact of the storage format of OpenType, and
        not something we want to foist on the user. It's more ergonomic
        for them to be able to manipulate plain f32s. */
        "Fixed" => Some("f32".to_string()),
        "F2DOT14" => Some("f32".to_string()),
        /* But we *do* use fixed point here, because we want to be able to
        compare fractional version numbers for equality without having to
        do epsilon dances. */
        "Version16Dot16" => Some("U16F16".to_string()),
        "LONGDATETIME" => Some("chrono::NaiveDateTime".to_string()),
        _ => None,
    }
}

pub fn expand_tables(item: TokenStream) -> TokenStream {
    let mut output = TokenStream::new();
    let mut iter = item.into_iter().peekable();
    let mut out_s = String::new();

    loop {
        let mut do_debug = "Debug,";
        let mut do_default = "";
        let mut do_serialize = "otspec_macros::Serialize,";
        let mut do_deserialize = "otspec_macros::Deserialize,";
        let mut embed_attr = "";
        // First parse table name
        let maybe_table_name = iter.next();
        if maybe_table_name.is_none() {
            break;
        }

        let table_name = expect_ident(maybe_table_name);

        loop {
            let next = iter.peek();
            if let Some(pragma) = has_pragma(&next) {
                if pragma == "[embedded]" {
                    embed_attr = "#[otspec(embedded)]";
                } else if pragma == "[nodebug]" {
                    do_debug = "";
                } else if pragma == "[noserialize]" {
                    do_serialize = "";
                } else if pragma == "[default]" {
                    do_default = "Default,";
                } else if pragma == "[nodeserialize]" {
                    do_deserialize = "";
                } else {
                    panic!("Unknown pragma '{:?}'", pragma);
                }
                iter.next();
                continue;
            }
            break;
        }

        write!(
            out_s,
            "/// Low-level structure used for serializing/deserializing table\n\
            #[allow(missing_docs, non_snake_case, non_camel_case_types)]\n\
            #[derive({} {} {} {} PartialEq, Clone)]\n\
            {}\n\
            pub struct {} {{",
            do_serialize, do_deserialize, do_debug, do_default, embed_attr, table_name
        )
        .unwrap();

        let mut table_def = expect_group(iter.next(), Delimiter::Brace).into_iter();

        loop {
            let maybe_t = table_def.next();
            if maybe_t.is_none() {
                break;
            }
            if let Some(pragma) = has_pragma(&maybe_t.as_ref()) {
                if pragma == "[offset_base]" {
                    out_s.push_str("#[otspec(offset_base)]");
                } else if pragma == "[embed]" {
                    out_s.push_str("#[otspec(embed)]");
                } else {
                    panic!("Unknown pragma '{:?}'", pragma);
                }
                continue;
            }
            let t = expect_ident(maybe_t);
            if t == "Maybe" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                writeln!(out_s, "pub {} : Option<{}>,", name, subtype).unwrap();
            } else if t == "Counted" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                out_s.push_str("#[otspec(with = \"Counted\")]\n");
                writeln!(out_s, "pub {} : Vec<{}>,", name, subtype).unwrap();
            } else if t == "Counted32" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                out_s.push_str("#[otspec(with = \"Counted32\")]\n");
                writeln!(out_s, "pub {} : Vec<{}>,", name, subtype).unwrap();
            } else if t == "Offset16" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                writeln!(out_s, "pub {} : Offset16<{}>,", name, subtype).unwrap();
            } else if t == "Offset32" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                writeln!(out_s, "pub {} : Offset32<{}>,", name, subtype).unwrap();
            } else if t == "CountedOffset16" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                out_s.push_str("#[otspec(with = \"Counted\")]\n");
                writeln!(out_s, "pub {} : VecOffset16<{}>,", name, subtype).unwrap();
            } else if t == "CountedOffset32" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                out_s.push_str("#[otspec(with = \"Counted\")]\n");
                writeln!(out_s, "pub {} : VecOffset32<{}>,", name, subtype).unwrap();
            } else if let Some(nonspecial_type) = special_type(&t) {
                writeln!(out_s, "#[otspec(with = \"{}\")]", t).unwrap();
                let name = expect_ident(table_def.next());
                writeln!(out_s, "pub {} : {},", name, nonspecial_type).unwrap();
            } else {
                let name = expect_ident(table_def.next());
                writeln!(out_s, "pub {} : {},", name, t).unwrap();
            }
        }
        out_s.push('}');
    }
    let ts1: TokenStream = out_s.parse().unwrap();
    output.extend(ts1);
    output
}
