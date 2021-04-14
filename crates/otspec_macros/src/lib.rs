#![feature(proc_macro_diagnostic)]
#![feature(proc_macro_quote)]

use proc_macro::Delimiter;
use proc_macro::TokenStream;
use proc_macro::TokenTree;

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

fn special_type(t: &str) -> Option<String> {
    match t {
        "Fixed" => Some("f32".to_string()),
        "F2DOT16" => Some("f32".to_string()),
        "LONGDATETIME" => Some("chrono::NaiveDateTime".to_string()),
        _ => None,
    }
}

#[proc_macro]
pub fn table(item: TokenStream) -> TokenStream {
    let mut output = TokenStream::new();
    let mut iter = item.into_iter();

    // First parse table name
    let table_name = expect_ident(iter.next());
    assert!(table_name.len() == 4);
    let mut out_s = format!(
        "#[derive(Serialize, Debug, PartialEq)]\npub struct {} {{",
        table_name
    );

    let mut table_def = expect_group(iter.next(), Delimiter::Brace).into_iter();

    loop {
        let maybe_t = table_def.next();
        if maybe_t.is_none() {
            break;
        }
        let t = expect_ident(maybe_t);
        let name = expect_ident(table_def.next());
        if let Some(nonspecial_type) = special_type(&t) {
            out_s.push_str(&format!("#[serde(with = \"{}\")]\n", t));
            out_s.push_str(&format!("pub {} : {},\n", name, nonspecial_type))
        } else {
            out_s.push_str(&format!("pub {} : {},\n", name, t))
        }
    }
    out_s.push('}');
    let ts1: TokenStream = out_s.parse().unwrap();
    output.extend(ts1);
    output
}
