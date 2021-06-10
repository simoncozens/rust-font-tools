use proc_macro2::TokenStream;

use syn::{self};

use crate::bound;
use crate::internals::ast::{Container, Data, Field, Style};
use crate::internals::{attr, replace_receiver, Ctxt, Derive};

pub fn expand_derive_deserialize(
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
            let body = deserialize_fields(fields);
            let names = fields.iter().map(|f| &f.original.ident);
            let do_pop = if fields.iter().any(|f| f.attrs.offset_base) {
                quote!(c.pop();)
            } else {
                quote!()
            };
            Ok(quote! {
                #[automatically_derived]
                impl #impl_generics otspec::Deserialize for #ident #ty_generics #where_clause {
                    fn from_bytes(c: &mut otspec::ReaderContext) -> Result<Self, otspec::DeserializationError> {
                        #(#body)*
                        #do_pop
                        Ok(#ident { #(#names,)* })
                    }
                }
            })
        }
        _ => panic!("Can't auto-serialize a non-struct type"),
    }
}

fn deserialize_fields(fields: &[Field]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.original.ident;
            let ty = &field.original.ty;
            let start = if field.attrs.offset_base {
                quote!(c.push();)
            } else {
                quote!()
            };
            if let Some(path) = field.attrs.deserialize_with() {
                if path.path.is_ident("Counted") {
                    if let syn::Type::Path(subvec) = ty {
                        let subpath = get_vector_arg(subvec);
                        quote! {
                            #start
                            let wrapped: otspec::Counted<#subpath> = c.de()?;
                            let #name: #ty = wrapped.into();
                        }
                    } else {
                        panic!("Can't happen");
                    }
                } else {
                    quote! {
                        #start
                        let wrapped: #path = c.de()?;
                        let #name: #ty = wrapped.into();
                    }
                }
            } else {
                quote! {
                    #start
                    let #name: #ty = c.de()?;
                }
            }
        })
        .collect()
}

use quote::ToTokens;
fn get_vector_arg(path: &syn::TypePath) -> TokenStream {
    if let syn::PathArguments::AngleBracketed(brackets) =
        &path.path.segments.first().unwrap().arguments
    {
        let g = brackets.args.first().unwrap();
        let mut t = TokenStream::new();
        g.to_tokens(&mut t);
        t
    } else {
        let mut t = TokenStream::new();
        path.to_tokens(&mut t);
        panic!("Vector wasn't generic in {:?}", t);
    }
}

// #[proc_macro_derive(Deserialize)]
// pub fn deserialize_derive(input: TokenStream) -> TokenStream {
//     let ast: syn::DeriveInput = syn::parse(input).unwrap();

//     let fields = match &ast.data {
//         Data::Struct(DataStruct {
//             fields: Fields::Named(fields),
//             ..
//         }) => &fields.named,
//         _ => panic!("expected a struct with named fields"),
//     };
//     let field_name1 = fields.iter().map(|field| &field.ident);
//     let field_name2 = fields.iter().map(|field| &field.ident);
//     let field_type = fields.iter().map(|field| &field.ty);

//     let name = &ast.ident;

//     TokenStream::from(quote! {
//         impl otspec::Deserialize for #name {
//                     fn from_bytes(c: &mut otspec::ReaderContext) -> Result<Self, otspec::DeserializationError> {

//                 #(
//                         let #field_name1: #field_type = c.de()?;
//                       )*

//                       Ok(Self {
//                               #(#field_name2, )*
//                       })
//           }
//         }

//     })
// }

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
