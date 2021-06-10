use crate::bound;
use crate::internals::ast::{Container, Data, Field, Style};
use crate::internals::{attr, replace_receiver, Ctxt, Derive};
use proc_macro2::TokenStream;
use syn::{self};

pub fn expand_derive_serialize(
    input: &mut syn::DeriveInput,
) -> Result<TokenStream, Vec<syn::Error>> {
    replace_receiver(input);

    let ctxt = Ctxt::new();
    let cont = match Container::from_ast(&ctxt, input, Derive::Serialize) {
        Some(cont) => cont,
        None => return Err(ctxt.check().unwrap_err()),
    };
    ctxt.check()?;

    let ident = &cont.ident;
    let params = Parameters::new(&cont);
    let (impl_generics, ty_generics, where_clause) = params.generics.split_for_impl();
    match &cont.data {
        Data::Struct(Style::Struct, fields) => {
            let body = serialize_fields(fields);
            Ok(quote! {
                #[automatically_derived]
                impl #impl_generics otspec::Serialize for #ident #ty_generics #where_clause {
                    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
                        #(#body)*
                        Ok(())
                    }
                }
            })
        }
        _ => panic!("Can't auto-serialize a non-struct type"),
    }
}

fn serialize_fields(fields: &[Field]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.original.ident;
            if let Some(path) = field.attrs.serialize_with() {
                if path.path.is_ident("Counted") {
                    quote! {
                        let wrapped = otspec::Counted(self.#name.clone());
                        wrapped.to_bytes(data)?;
                    }
                } else {
                    quote! {
                        let wrapped = #path(self.#name);
                        wrapped.to_bytes(data)?;
                    }
                }
            } else {
                quote! { self.#name.to_bytes(data)?; }
            }
        })
        .collect()
}

struct Parameters {
    generics: syn::Generics,
}

impl Parameters {
    fn new(cont: &Container) -> Self {
        let generics = build_generics(cont);

        Parameters { generics }
    }
}

fn build_generics(cont: &Container) -> syn::Generics {
    let generics = bound::without_defaults(cont.generics);

    bound::with_bound(
        cont,
        &generics,
        needs_serialize_bound,
        &parse_quote!(_serde::Serialize),
    )
}
fn needs_serialize_bound(field: &attr::Field, _variant: Option<&attr::Variant>) -> bool {
    field.serialize_with().is_none()
}
