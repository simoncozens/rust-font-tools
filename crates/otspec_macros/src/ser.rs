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
            let sizes = serialize_sizes(fields);
            let offset_fields = serialize_offset_fields(fields);
            let embed_fields = serialize_embed_fields(fields);
            let serializer = serialize_fields(fields);
            let has_offsets = !offset_fields.is_empty();
            let prepare = if has_offsets && !cont.attrs.is_embedded {
                quote! {
                    let obj = otspec::offsetmanager::resolve_offsets(self);
                }
            } else {
                quote! { let obj = self; }
            };
            let descendants = if has_offsets && !cont.attrs.is_embedded {
                quote! {
                    otspec::offsetmanager::resolve_offsets_and_serialize(obj, data, false)?;
                }
            } else {
                quote! {}
            };
            Ok(quote! {
                #[automatically_derived]
                impl #impl_generics otspec::Serialize for #ident #ty_generics #where_clause {
                    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
                        #prepare
                        self.to_bytes_shallow(data)?;
                        #descendants
                        Ok(())
                    }

                    fn to_bytes_shallow(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
                        let obj = self;
                        #(#serializer)*
                        Ok(())
                    }
                    fn ot_binary_size(&self) -> usize {
                        0 #(#sizes)*
                    }

                    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
                        let mut v: Vec<&dyn OffsetMarkerTrait> = vec![ #(#offset_fields)* ];
                        #(#embed_fields)*
                        v
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
                        let wrapped = otspec::Counted(obj.#name.clone().into());
                        wrapped.to_bytes(data)?;
                    }
                } else {
                    quote! {
                        let wrapped = #path(obj.#name);
                        wrapped.to_bytes(data)?;
                    }
                }
            } else {
                quote! { obj.#name.to_bytes(data)?; }
            }
        })
        .collect()
}

fn serialize_sizes(fields: &[Field]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.original.ident;
            if let Some(path) = field.attrs.serialize_with() {
                if path.path.is_ident("Counted") {
                    quote! {
                         + {
                            let wrapped = otspec::Counted(self.#name.clone().into());
                            wrapped.ot_binary_size()
                        }
                    }
                } else {
                    quote! {
                        + { let wrapped = #path(self.#name);
                            wrapped.ot_binary_size()
                        }
                    }
                }
            } else {
                quote! { + self.#name.ot_binary_size() }
            }
        })
        .collect()
}

fn serialize_offset_fields(fields: &[Field]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.original.ident;
            let ty = &field.original.ty;
            if let syn::Type::Path(path) = ty {
                if path.path.segments.first().unwrap().ident == "Offset16" {
                    quote! { &self.#name, }
                } else {
                    quote! {}
                }
            } else {
                quote! {}
            }
        })
        .collect()
}

fn serialize_embed_fields(fields: &[Field]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.original.ident;
            let ty = &field.original.ty;
            let is_vec = if let syn::Type::Path(path) = ty {
                path.path.segments.first().unwrap().ident == "VecOffset16"
            } else {
                false
            };

            if field.attrs.embedded || is_vec {
                quote! {
                    v.extend(self.#name.offset_fields());
                }
            } else {
                quote! {}
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
