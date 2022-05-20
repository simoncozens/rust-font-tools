//! This library is used by the otspec crate. No user-serviceable parts inside.
#![cfg_attr(nightly, feature(proc_macro_diagnostic))]
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

extern crate proc_macro;
extern crate proc_macro2;

mod internals;

use proc_macro::TokenStream;
use syn::DeriveInput;

#[macro_use]
mod bound;

mod de;
mod ser;

mod tables;
mod tag;

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}

#[proc_macro_derive(Serialize, attributes(otspec))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    ser::expand_derive_serialize(&mut input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

#[proc_macro_derive(Deserialize, attributes(otspec))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    de::expand_derive_deserialize(&mut input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

#[proc_macro]
pub fn tables(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    tables::expand_tables(input)
}

/// Generate a `Tag` from a string literal, verifying it conforms to the
/// OpenType spec.
///
/// The argument must be a non-empty string literal. Containing at most four
/// characters in the printable ascii range, `0x20..=0x7E`.
///
/// If the input has fewer than four characters, it will be padded with the space
/// (' ', `0x20`) character.
#[proc_macro]
pub fn tag(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    tag::expand_tag(input)
}
