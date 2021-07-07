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
        "Offset16" => Some("u16".to_string()),
        "Offset32" => Some("u32".to_string()),
        "LONGDATETIME" => Some("chrono::NaiveDateTime".to_string()),
        _ => None,
    }
}

pub fn expand_tables(item: TokenStream) -> TokenStream {
    let mut output = TokenStream::new();
    let mut iter = item.into_iter().peekable();
    let mut out_s = String::new();

    loop {
        let mut do_debug = true;
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
                    out_s.push_str("#[serde(embedded)]");
                } else if pragma == "[nodebug]" {
                    do_debug = false;
                } else {
                    panic!("Unknown pragma '{:?}'", pragma);
                }
                iter.next();
                continue;
            }
            break;
        }

        out_s.push_str(&format!(
            "/// Low-level structure used for serializing/deserializing table\n\
            #[allow(missing_docs, non_snake_case, non_camel_case_types)]\n\
            #[derive(otspec_macros::Serialize, otspec_macros::Deserialize, {} PartialEq, Clone)]\n\
            pub struct {} {{",
            if do_debug { "Debug," } else { "" },
            table_name,
        ));

        let mut table_def = expect_group(iter.next(), Delimiter::Brace).into_iter();

        loop {
            let maybe_t = table_def.next();
            if maybe_t.is_none() {
                break;
            }
            if let Some(pragma) = has_pragma(&maybe_t.as_ref()) {
                if pragma == "[offset_base]" {
                    out_s.push_str("#[serde(offset_base)]");
                } else if pragma == "[embed]" {
                    out_s.push_str("#[serde(embed)]");
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
                out_s.push_str(&format!("pub {} : Option<{}>,\n", name, subtype))
            } else if t == "Counted" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                out_s.push_str(&"#[serde(with = \"Counted\")]\n".to_string());
                out_s.push_str(&format!("pub {} : Vec<{}>,\n", name, subtype))
            } else if t == "Offset16" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                out_s.push_str(&format!("pub {} : Offset16<{}>,\n", name, subtype))
            } else if t == "CountedOffset16" {
                let subtype = expect_group(table_def.next(), Delimiter::Parenthesis)
                    .into_iter()
                    .next()
                    .unwrap()
                    .to_string();
                let name = expect_ident(table_def.next());
                out_s.push_str(&"#[serde(with = \"Counted\")]\n".to_string());
                out_s.push_str(&format!("pub {} : VecOffset16<{}>,\n", name, subtype))
            } else if let Some(nonspecial_type) = special_type(&t) {
                out_s.push_str(&format!("#[serde(with = \"{}\")]\n", t));
                let name = expect_ident(table_def.next());
                out_s.push_str(&format!("pub {} : {},\n", name, nonspecial_type))
            } else {
                let name = expect_ident(table_def.next());
                out_s.push_str(&format!("pub {} : {},\n", name, t))
            }
        }
        out_s.push('}');
    }
    let ts1: TokenStream = out_s.parse().unwrap();
    output.extend(ts1);
    output
}
