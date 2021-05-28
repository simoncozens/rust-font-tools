use crate::internals::respan::respan;
use crate::internals::symbol::*;
use crate::internals::{ungroup, Ctxt};
use proc_macro2::{Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use std::borrow::Cow;
use std::collections::BTreeSet;
use syn;
use syn::parse::{self, Parse, ParseStream};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::Ident;
use syn::Meta::{List, NameValue, Path};
use syn::NestedMeta::{Lit, Meta};
use syn::Token;

// This module handles parsing of `#[serde(...)]` attributes. The entrypoints
// are `attr::Container::from_ast`, `attr::Variant::from_ast`, and
// `attr::Field::from_ast`. Each returns an instance of the corresponding
// struct. Note that none of them return a Result. Unrecognized, malformed, or
// duplicated attributes result in a span_err but otherwise are ignored. The
// user will see errors simultaneously for all bad attributes in the crate
// rather than just the first.

struct Attr<'c, T> {
    cx: &'c Ctxt,
    name: Symbol,
    tokens: TokenStream,
    value: Option<T>,
}

impl<'c, T> Attr<'c, T> {
    fn none(cx: &'c Ctxt, name: Symbol) -> Self {
        Attr {
            cx,
            name,
            tokens: TokenStream::new(),
            value: None,
        }
    }

    fn set<A: ToTokens>(&mut self, obj: A, value: T) {
        let tokens = obj.into_token_stream();

        if self.value.is_some() {
            self.cx
                .error_spanned_by(tokens, format!("duplicate serde attribute `{}`", self.name));
        } else {
            self.tokens = tokens;
            self.value = Some(value);
        }
    }

    fn set_opt<A: ToTokens>(&mut self, obj: A, value: Option<T>) {
        if let Some(value) = value {
            self.set(obj, value);
        }
    }

    fn set_if_none(&mut self, value: T) {
        if self.value.is_none() {
            self.value = Some(value);
        }
    }

    fn get(self) -> Option<T> {
        self.value
    }

    fn get_with_tokens(self) -> Option<(TokenStream, T)> {
        match self.value {
            Some(v) => Some((self.tokens, v)),
            None => None,
        }
    }
}

struct BoolAttr<'c>(Attr<'c, ()>);

impl<'c> BoolAttr<'c> {
    fn none(cx: &'c Ctxt, name: Symbol) -> Self {
        BoolAttr(Attr::none(cx, name))
    }

    fn set_true<A: ToTokens>(&mut self, obj: A) {
        self.0.set(obj, ());
    }

    fn get(&self) -> bool {
        self.0.value.is_some()
    }
}

struct VecAttr<'c, T> {
    cx: &'c Ctxt,
    name: Symbol,
    first_dup_tokens: TokenStream,
    values: Vec<T>,
}

impl<'c, T> VecAttr<'c, T> {
    fn none(cx: &'c Ctxt, name: Symbol) -> Self {
        VecAttr {
            cx,
            name,
            first_dup_tokens: TokenStream::new(),
            values: Vec::new(),
        }
    }

    fn insert<A: ToTokens>(&mut self, obj: A, value: T) {
        if self.values.len() == 1 {
            self.first_dup_tokens = obj.into_token_stream();
        }
        self.values.push(value);
    }

    fn at_most_one(mut self) -> Result<Option<T>, ()> {
        if self.values.len() > 1 {
            let dup_token = self.first_dup_tokens;
            self.cx.error_spanned_by(
                dup_token,
                format!("duplicate serde attribute `{}`", self.name),
            );
            Err(())
        } else {
            Ok(self.values.pop())
        }
    }

    fn get(self) -> Vec<T> {
        self.values
    }
}

#[allow(deprecated)]
fn unraw(ident: &Ident) -> String {
    // str::trim_start_matches was added in 1.30, trim_left_matches deprecated
    // in 1.33. We currently support rustc back to 1.15 so we need to continue
    // to use the deprecated one.
    ident.to_string().trim_left_matches("r#").to_owned()
}

/// Represents struct or enum attribute information.
pub struct Container {
    transparent: bool,
    deny_unknown_fields: bool,
    default: Default,
    ser_bound: Option<Vec<syn::WherePredicate>>,
    de_bound: Option<Vec<syn::WherePredicate>>,
    tag: TagType,
    type_from: Option<syn::Type>,
    type_try_from: Option<syn::Type>,
    type_into: Option<syn::Type>,
    remote: Option<syn::Path>,
    identifier: Identifier,
    has_flatten: bool,
    serde_path: Option<syn::Path>,
    is_packed: bool,
    /// Error message generated when type can't be deserialized
    expecting: Option<String>,
}

/// Styles of representing an enum.
pub enum TagType {
    /// The default.
    ///
    /// ```json
    /// {"variant1": {"key1": "value1", "key2": "value2"}}
    /// ```
    External,

    /// `#[serde(tag = "type")]`
    ///
    /// ```json
    /// {"type": "variant1", "key1": "value1", "key2": "value2"}
    /// ```
    Internal { tag: String },

    /// `#[serde(tag = "t", content = "c")]`
    ///
    /// ```json
    /// {"t": "variant1", "c": {"key1": "value1", "key2": "value2"}}
    /// ```
    Adjacent { tag: String, content: String },

    /// `#[serde(untagged)]`
    ///
    /// ```json
    /// {"key1": "value1", "key2": "value2"}
    /// ```
    None,
}

/// Whether this enum represents the fields of a struct or the variants of an
/// enum.
#[derive(Copy, Clone)]
pub enum Identifier {
    /// It does not.
    No,

    /// This enum represents the fields of a struct. All of the variants must be
    /// unit variants, except possibly one which is annotated with
    /// `#[serde(other)]` and is a newtype variant.
    Field,

    /// This enum represents the variants of an enum. All of the variants must
    /// be unit variants.
    Variant,
}

impl Identifier {
    #[cfg(feature = "deserialize_in_place")]
    pub fn is_some(self) -> bool {
        match self {
            Identifier::No => false,
            Identifier::Field | Identifier::Variant => true,
        }
    }
}

impl Container {
    /// Extract out the `#[serde(...)]` attributes from an item.
    pub fn from_ast(cx: &Ctxt, item: &syn::DeriveInput) -> Self {
        let mut ser_name = Attr::none(cx, RENAME);
        let mut de_name = Attr::none(cx, RENAME);
        let transparent = BoolAttr::none(cx, TRANSPARENT);
        let mut deny_unknown_fields = BoolAttr::none(cx, DENY_UNKNOWN_FIELDS);
        let mut default = Attr::none(cx, DEFAULT);
        let mut ser_bound = Attr::none(cx, BOUND);
        let mut de_bound = Attr::none(cx, BOUND);
        let mut untagged = BoolAttr::none(cx, UNTAGGED);
        let mut internal_tag = Attr::none(cx, TAG);
        let mut content = Attr::none(cx, CONTENT);
        let type_from = Attr::none(cx, FROM);
        let type_try_from = Attr::none(cx, TRY_FROM);
        let type_into = Attr::none(cx, INTO);
        let remote = Attr::none(cx, REMOTE);
        let field_identifier = BoolAttr::none(cx, FIELD_IDENTIFIER);
        let variant_identifier = BoolAttr::none(cx, VARIANT_IDENTIFIER);
        let serde_path = Attr::none(cx, CRATE);
        let expecting = Attr::none(cx, EXPECTING);

        for meta_item in item
            .attrs
            .iter()
            .flat_map(|attr| get_serde_meta_items(cx, attr))
            .flatten()
        {
            match &meta_item {
                // Parse `#[serde(rename = "foo")]`
                Meta(NameValue(m)) if m.path == RENAME => {
                    if let Ok(s) = get_lit_str(cx, RENAME, &m.lit) {
                        ser_name.set(&m.path, s.value());
                        de_name.set(&m.path, s.value());
                    }
                }

                // Parse `#[serde(deny_unknown_fields)]`
                Meta(Path(word)) if word == DENY_UNKNOWN_FIELDS => {
                    deny_unknown_fields.set_true(word);
                }

                // Parse `#[serde(default)]`
                Meta(Path(word)) if word == DEFAULT => match &item.data {
                    syn::Data::Struct(syn::DataStruct { fields, .. }) => match fields {
                        syn::Fields::Named(_) => {
                            default.set(word, Default::Default);
                        }
                        syn::Fields::Unnamed(_) | syn::Fields::Unit => cx.error_spanned_by(
                            fields,
                            "#[serde(default)] can only be used on structs with named fields",
                        ),
                    },
                    syn::Data::Enum(syn::DataEnum { enum_token, .. }) => cx.error_spanned_by(
                        enum_token,
                        "#[serde(default)] can only be used on structs with named fields",
                    ),
                    syn::Data::Union(syn::DataUnion { union_token, .. }) => cx.error_spanned_by(
                        union_token,
                        "#[serde(default)] can only be used on structs with named fields",
                    ),
                },

                // Parse `#[serde(default = "...")]`
                Meta(NameValue(m)) if m.path == DEFAULT => {
                    if let Ok(path) = parse_lit_into_expr_path(cx, DEFAULT, &m.lit) {
                        match &item.data {
                            syn::Data::Struct(syn::DataStruct { fields, .. }) => {
                                match fields {
                                    syn::Fields::Named(_) => {
                                        default.set(&m.path, Default::Path(path));
                                    }
                                    syn::Fields::Unnamed(_) | syn::Fields::Unit => cx
                                        .error_spanned_by(
                                            fields,
                                            "#[serde(default = \"...\")] can only be used on structs with named fields",
                                        ),
                                }
                            }
                            syn::Data::Enum(syn::DataEnum { enum_token, .. }) => cx
                                .error_spanned_by(
                                    enum_token,
                                    "#[serde(default = \"...\")] can only be used on structs with named fields",
                                ),
                            syn::Data::Union(syn::DataUnion {
                                union_token, ..
                            }) => cx.error_spanned_by(
                                union_token,
                                "#[serde(default = \"...\")] can only be used on structs with named fields",
                            ),
                        }
                    }
                }

                // Parse `#[serde(bound = "T: SomeBound")]`
                Meta(NameValue(m)) if m.path == BOUND => {
                    if let Ok(where_predicates) = parse_lit_into_where(cx, BOUND, BOUND, &m.lit) {
                        ser_bound.set(&m.path, where_predicates.clone());
                        de_bound.set(&m.path, where_predicates);
                    }
                }

                // Parse `#[serde(bound(serialize = "...", deserialize = "..."))]`
                Meta(List(m)) if m.path == BOUND => {
                    if let Ok((ser, de)) = get_where_predicates(cx, &m.nested) {
                        ser_bound.set_opt(&m.path, ser);
                        de_bound.set_opt(&m.path, de);
                    }
                }

                // Parse `#[serde(untagged)]`
                Meta(Path(word)) if word == UNTAGGED => match item.data {
                    syn::Data::Enum(_) => {
                        untagged.set_true(word);
                    }
                    syn::Data::Struct(syn::DataStruct { struct_token, .. }) => {
                        cx.error_spanned_by(
                            struct_token,
                            "#[serde(untagged)] can only be used on enums",
                        );
                    }
                    syn::Data::Union(syn::DataUnion { union_token, .. }) => {
                        cx.error_spanned_by(
                            union_token,
                            "#[serde(untagged)] can only be used on enums",
                        );
                    }
                },

                // Parse `#[serde(tag = "type")]`
                Meta(NameValue(m)) if m.path == TAG => {
                    if let Ok(s) = get_lit_str(cx, TAG, &m.lit) {
                        match &item.data {
                            syn::Data::Enum(_) => {
                                internal_tag.set(&m.path, s.value());
                            }
                            syn::Data::Struct(syn::DataStruct { fields, .. }) => match fields {
                                syn::Fields::Named(_) => {
                                    internal_tag.set(&m.path, s.value());
                                }
                                syn::Fields::Unnamed(_) | syn::Fields::Unit => {
                                    cx.error_spanned_by(
                                            fields,
                                            "#[serde(tag = \"...\")] can only be used on enums and structs with named fields",
                                        );
                                }
                            },
                            syn::Data::Union(syn::DataUnion { union_token, .. }) => {
                                cx.error_spanned_by(
                                    union_token,
                                    "#[serde(tag = \"...\")] can only be used on enums and structs with named fields",
                                );
                            }
                        }
                    }
                }

                // Parse `#[serde(content = "c")]`
                Meta(NameValue(m)) if m.path == CONTENT => {
                    if let Ok(s) = get_lit_str(cx, CONTENT, &m.lit) {
                        match &item.data {
                            syn::Data::Enum(_) => {
                                content.set(&m.path, s.value());
                            }
                            syn::Data::Struct(syn::DataStruct { struct_token, .. }) => {
                                cx.error_spanned_by(
                                    struct_token,
                                    "#[serde(content = \"...\")] can only be used on enums",
                                );
                            }
                            syn::Data::Union(syn::DataUnion { union_token, .. }) => {
                                cx.error_spanned_by(
                                    union_token,
                                    "#[serde(content = \"...\")] can only be used on enums",
                                );
                            }
                        }
                    }
                }

                Meta(meta_item) => {
                    let path = meta_item
                        .path()
                        .into_token_stream()
                        .to_string()
                        .replace(' ', "");
                    cx.error_spanned_by(
                        meta_item.path(),
                        format!("unknown serde container attribute `{}`", path),
                    );
                }

                Lit(lit) => {
                    cx.error_spanned_by(lit, "unexpected literal in serde container attribute");
                }
            }
        }

        let mut is_packed = false;
        for attr in &item.attrs {
            if attr.path.is_ident("repr") {
                let _ = attr.parse_args_with(|input: ParseStream| {
                    while let Some(token) = input.parse()? {
                        if let TokenTree::Ident(ident) = token {
                            is_packed |= ident == "packed";
                        }
                    }
                    Ok(())
                });
            }
        }

        Container {
            transparent: transparent.get(),
            deny_unknown_fields: deny_unknown_fields.get(),
            default: default.get().unwrap_or(Default::None),
            ser_bound: ser_bound.get(),
            de_bound: de_bound.get(),
            tag: decide_tag(cx, item, untagged, internal_tag, content),
            type_from: type_from.get(),
            type_try_from: type_try_from.get(),
            type_into: type_into.get(),
            remote: remote.get(),
            identifier: decide_identifier(cx, item, field_identifier, variant_identifier),
            has_flatten: false,
            serde_path: serde_path.get(),
            is_packed,
            expecting: expecting.get(),
        }
    }
    pub fn default(&self) -> &Default {
        &self.default
    }

    pub fn ser_bound(&self) -> Option<&[syn::WherePredicate]> {
        self.ser_bound.as_ref().map(|vec| &vec[..])
    }
}

fn decide_tag(
    cx: &Ctxt,
    item: &syn::DeriveInput,
    untagged: BoolAttr,
    internal_tag: Attr<String>,
    content: Attr<String>,
) -> TagType {
    match (
        untagged.0.get_with_tokens(),
        internal_tag.get_with_tokens(),
        content.get_with_tokens(),
    ) {
        (None, None, None) => TagType::External,
        (Some(_), None, None) => TagType::None,
        (None, Some((_, tag)), None) => {
            // Check that there are no tuple variants.
            if let syn::Data::Enum(data) = &item.data {
                for variant in &data.variants {
                    match &variant.fields {
                        syn::Fields::Named(_) | syn::Fields::Unit => {}
                        syn::Fields::Unnamed(fields) => {
                            if fields.unnamed.len() != 1 {
                                cx.error_spanned_by(
                                    variant,
                                    "#[serde(tag = \"...\")] cannot be used with tuple variants",
                                );
                                break;
                            }
                        }
                    }
                }
            }
            TagType::Internal { tag }
        }
        (Some((untagged_tokens, _)), Some((tag_tokens, _)), None) => {
            cx.error_spanned_by(
                untagged_tokens,
                "enum cannot be both untagged and internally tagged",
            );
            cx.error_spanned_by(
                tag_tokens,
                "enum cannot be both untagged and internally tagged",
            );
            TagType::External // doesn't matter, will error
        }
        (None, None, Some((content_tokens, _))) => {
            cx.error_spanned_by(
                content_tokens,
                "#[serde(tag = \"...\", content = \"...\")] must be used together",
            );
            TagType::External
        }
        (Some((untagged_tokens, _)), None, Some((content_tokens, _))) => {
            cx.error_spanned_by(
                untagged_tokens,
                "untagged enum cannot have #[serde(content = \"...\")]",
            );
            cx.error_spanned_by(
                content_tokens,
                "untagged enum cannot have #[serde(content = \"...\")]",
            );
            TagType::External
        }
        (None, Some((_, tag)), Some((_, content))) => TagType::Adjacent { tag, content },
        (Some((untagged_tokens, _)), Some((tag_tokens, _)), Some((content_tokens, _))) => {
            cx.error_spanned_by(
                untagged_tokens,
                "untagged enum cannot have #[serde(tag = \"...\", content = \"...\")]",
            );
            cx.error_spanned_by(
                tag_tokens,
                "untagged enum cannot have #[serde(tag = \"...\", content = \"...\")]",
            );
            cx.error_spanned_by(
                content_tokens,
                "untagged enum cannot have #[serde(tag = \"...\", content = \"...\")]",
            );
            TagType::External
        }
    }
}

fn decide_identifier(
    cx: &Ctxt,
    item: &syn::DeriveInput,
    field_identifier: BoolAttr,
    variant_identifier: BoolAttr,
) -> Identifier {
    match (
        &item.data,
        field_identifier.0.get_with_tokens(),
        variant_identifier.0.get_with_tokens(),
    ) {
        (_, None, None) => Identifier::No,
        (_, Some((field_identifier_tokens, _)), Some((variant_identifier_tokens, _))) => {
            cx.error_spanned_by(
                field_identifier_tokens,
                "#[serde(field_identifier)] and #[serde(variant_identifier)] cannot both be set",
            );
            cx.error_spanned_by(
                variant_identifier_tokens,
                "#[serde(field_identifier)] and #[serde(variant_identifier)] cannot both be set",
            );
            Identifier::No
        }
        (syn::Data::Enum(_), Some(_), None) => Identifier::Field,
        (syn::Data::Enum(_), None, Some(_)) => Identifier::Variant,
        (syn::Data::Struct(syn::DataStruct { struct_token, .. }), Some(_), None) => {
            cx.error_spanned_by(
                struct_token,
                "#[serde(field_identifier)] can only be used on an enum",
            );
            Identifier::No
        }
        (syn::Data::Union(syn::DataUnion { union_token, .. }), Some(_), None) => {
            cx.error_spanned_by(
                union_token,
                "#[serde(field_identifier)] can only be used on an enum",
            );
            Identifier::No
        }
        (syn::Data::Struct(syn::DataStruct { struct_token, .. }), None, Some(_)) => {
            cx.error_spanned_by(
                struct_token,
                "#[serde(variant_identifier)] can only be used on an enum",
            );
            Identifier::No
        }
        (syn::Data::Union(syn::DataUnion { union_token, .. }), None, Some(_)) => {
            cx.error_spanned_by(
                union_token,
                "#[serde(variant_identifier)] can only be used on an enum",
            );
            Identifier::No
        }
    }
}

/// Represents variant attribute information
pub struct Variant {
    ser_bound: Option<Vec<syn::WherePredicate>>,
    de_bound: Option<Vec<syn::WherePredicate>>,
    skip_deserializing: bool,
    skip_serializing: bool,
    other: bool,
    serialize_with: Option<syn::ExprPath>,
    deserialize_with: Option<syn::ExprPath>,
    borrow: Option<syn::Meta>,
}

impl Variant {
    pub fn from_ast(cx: &Ctxt, variant: &syn::Variant) -> Self {
        let skip_deserializing = BoolAttr::none(cx, SKIP_DESERIALIZING);
        let skip_serializing = BoolAttr::none(cx, SKIP_SERIALIZING);
        let ser_bound = Attr::none(cx, BOUND);
        let de_bound = Attr::none(cx, BOUND);
        let other = BoolAttr::none(cx, OTHER);
        let mut serialize_with = Attr::none(cx, SERIALIZE_WITH);
        let mut deserialize_with = Attr::none(cx, DESERIALIZE_WITH);
        let mut borrow = Attr::none(cx, BORROW);

        for meta_item in variant
            .attrs
            .iter()
            .flat_map(|attr| get_serde_meta_items(cx, attr))
            .flatten()
        {
            match &meta_item {
                // Parse `#[serde(with = "...")]`
                Meta(NameValue(m)) if m.path == WITH => {
                    if let Ok(path) = parse_lit_into_expr_path(cx, WITH, &m.lit) {
                        let ser_path = path.clone();
                        serialize_with.set(&m.path, ser_path);
                        let de_path = path;
                        deserialize_with.set(&m.path, de_path);
                    }
                }

                // Defer `#[serde(borrow)]` and `#[serde(borrow = "'a + 'b")]`
                Meta(m) if m.path() == BORROW => match &variant.fields {
                    syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                        borrow.set(m.path(), m.clone());
                    }
                    _ => {
                        cx.error_spanned_by(
                            variant,
                            "#[serde(borrow)] may only be used on newtype variants",
                        );
                    }
                },

                Meta(meta_item) => {
                    let path = meta_item
                        .path()
                        .into_token_stream()
                        .to_string()
                        .replace(' ', "");
                    cx.error_spanned_by(
                        meta_item.path(),
                        format!("unknown serde variant attribute `{}`", path),
                    );
                }

                Lit(lit) => {
                    cx.error_spanned_by(lit, "unexpected literal in serde variant attribute");
                }
            }
        }

        Variant {
            ser_bound: ser_bound.get(),
            de_bound: de_bound.get(),
            skip_deserializing: skip_deserializing.get(),
            skip_serializing: skip_serializing.get(),
            other: other.get(),
            serialize_with: serialize_with.get(),
            deserialize_with: deserialize_with.get(),
            borrow: borrow.get(),
        }
    }
}

/// Represents field attribute information
pub struct Field {
    pub offset_base: bool,
    default: Default,
    serialize_with: Option<syn::ExprPath>,
    deserialize_with: Option<syn::ExprPath>,
    ser_bound: Option<Vec<syn::WherePredicate>>,
}

/// Represents the default to use for a field when deserializing.
pub enum Default {
    /// Field must always be specified because it does not have a default.
    None,
    /// The default is given by `std::default::Default::default()`.
    Default,
    /// The default is given by this function.
    Path(syn::ExprPath),
}

impl Field {
    /// Extract out the `#[serde(...)]` attributes from a struct field.
    pub fn from_ast(
        cx: &Ctxt,
        index: usize,
        field: &syn::Field,
        attrs: Option<&Variant>,
        container_default: &Default,
    ) -> Self {
        let skip_deserializing = BoolAttr::none(cx, SKIP_DESERIALIZING);
        let mut default = Attr::none(cx, DEFAULT);
        let mut serialize_with = Attr::none(cx, SERIALIZE_WITH);
        let mut deserialize_with = Attr::none(cx, DESERIALIZE_WITH);
        let ser_bound = Attr::none(cx, BOUND);
        let mut borrowed_lifetimes = Attr::none(cx, BORROW);
        let mut offset_base = BoolAttr::none(cx, OFFSET_BASE);

        let ident = match &field.ident {
            Some(ident) => unraw(ident),
            None => index.to_string(),
        };

        let variant_borrow = attrs
            .and_then(|variant| variant.borrow.as_ref())
            .map(|borrow| Meta(borrow.clone()));

        for meta_item in field
            .attrs
            .iter()
            .flat_map(|attr| get_serde_meta_items(cx, attr))
            .flatten()
            .chain(variant_borrow)
        {
            match &meta_item {
                // Parse `#[serde(serialize_with = "...")]`
                Meta(NameValue(m)) if m.path == SERIALIZE_WITH => {
                    if let Ok(path) = parse_lit_into_expr_path(cx, SERIALIZE_WITH, &m.lit) {
                        serialize_with.set(&m.path, path);
                    }
                }

                // Parse `#[serde(deserialize_with = "...")]`
                Meta(NameValue(m)) if m.path == DESERIALIZE_WITH => {
                    if let Ok(path) = parse_lit_into_expr_path(cx, DESERIALIZE_WITH, &m.lit) {
                        deserialize_with.set(&m.path, path);
                    }
                }

                // Parse `#[serde(with = "...")]`
                Meta(NameValue(m)) if m.path == WITH => {
                    if let Ok(path) = parse_lit_into_expr_path(cx, WITH, &m.lit) {
                        let ser_path = path.clone();
                        serialize_with.set(&m.path, ser_path);
                        let de_path = path;
                        deserialize_with.set(&m.path, de_path);
                    }
                }

                // Parse `#[serde(offset_base)]`
                Meta(Path(word)) if word == OFFSET_BASE => {
                    offset_base.set_true(word);
                }

                // Parse `#[serde(borrow)]`
                Meta(Path(word)) if word == BORROW => {
                    if let Ok(borrowable) = borrowable_lifetimes(cx, &ident, field) {
                        borrowed_lifetimes.set(word, borrowable);
                    }
                }

                Meta(meta_item) => {
                    let path = meta_item
                        .path()
                        .into_token_stream()
                        .to_string()
                        .replace(' ', "");
                    cx.error_spanned_by(
                        meta_item.path(),
                        format!("unknown serde field attribute `{}`", path),
                    );
                }

                Lit(lit) => {
                    cx.error_spanned_by(lit, "unexpected literal in serde field attribute");
                }
            }
        }

        // Is skip_deserializing, initialize the field to Default::default() unless a
        // different default is specified by `#[serde(default = "...")]` on
        // ourselves or our container (e.g. the struct we are in).
        if let Default::None = *container_default {
            if skip_deserializing.0.value.is_some() {
                default.set_if_none(Default::Default);
            }
        }

        Field {
            offset_base: offset_base.get(),
            default: default.get().unwrap_or(Default::None),
            serialize_with: serialize_with.get(),
            deserialize_with: deserialize_with.get(),
            ser_bound: ser_bound.get(),
        }
    }

    pub fn serialize_with(&self) -> Option<&syn::ExprPath> {
        self.serialize_with.as_ref()
    }

    pub fn deserialize_with(&self) -> Option<&syn::ExprPath> {
        self.deserialize_with.as_ref()
    }

    pub fn ser_bound(&self) -> Option<&[syn::WherePredicate]> {
        self.ser_bound.as_ref().map(|vec| &vec[..])
    }
}

type SerAndDe<T> = (Option<T>, Option<T>);

fn get_ser_and_de<'a, 'b, T, F>(
    cx: &'b Ctxt,
    attr_name: Symbol,
    metas: &'a Punctuated<syn::NestedMeta, Token![,]>,
    f: F,
) -> Result<(VecAttr<'b, T>, VecAttr<'b, T>), ()>
where
    T: 'a,
    F: Fn(&Ctxt, Symbol, Symbol, &'a syn::Lit) -> Result<T, ()>,
{
    let mut ser_meta = VecAttr::none(cx, attr_name);
    let mut de_meta = VecAttr::none(cx, attr_name);

    for meta in metas {
        match meta {
            Meta(NameValue(meta)) if meta.path == SERIALIZE => {
                if let Ok(v) = f(cx, attr_name, SERIALIZE, &meta.lit) {
                    ser_meta.insert(&meta.path, v);
                }
            }

            Meta(NameValue(meta)) if meta.path == DESERIALIZE => {
                if let Ok(v) = f(cx, attr_name, DESERIALIZE, &meta.lit) {
                    de_meta.insert(&meta.path, v);
                }
            }

            _ => {
                cx.error_spanned_by(
                    meta,
                    format!(
                        "malformed {0} attribute, expected `{0}(serialize = ..., deserialize = ...)`",
                        attr_name
                    ),
                );
                return Err(());
            }
        }
    }

    Ok((ser_meta, de_meta))
}

fn get_where_predicates(
    cx: &Ctxt,
    items: &Punctuated<syn::NestedMeta, Token![,]>,
) -> Result<SerAndDe<Vec<syn::WherePredicate>>, ()> {
    let (ser, de) = get_ser_and_de(cx, BOUND, items, parse_lit_into_where)?;
    Ok((ser.at_most_one()?, de.at_most_one()?))
}

pub fn get_serde_meta_items(cx: &Ctxt, attr: &syn::Attribute) -> Result<Vec<syn::NestedMeta>, ()> {
    if attr.path != SERDE {
        return Ok(Vec::new());
    }

    match attr.parse_meta() {
        Ok(List(meta)) => Ok(meta.nested.into_iter().collect()),
        Ok(other) => {
            cx.error_spanned_by(other, "expected #[serde(...)]");
            Err(())
        }
        Err(err) => {
            cx.syn_error(err);
            Err(())
        }
    }
}

fn get_lit_str<'a>(cx: &Ctxt, attr_name: Symbol, lit: &'a syn::Lit) -> Result<&'a syn::LitStr, ()> {
    get_lit_str2(cx, attr_name, attr_name, lit)
}

fn get_lit_str2<'a>(
    cx: &Ctxt,
    attr_name: Symbol,
    meta_item_name: Symbol,
    lit: &'a syn::Lit,
) -> Result<&'a syn::LitStr, ()> {
    if let syn::Lit::Str(lit) = lit {
        Ok(lit)
    } else {
        cx.error_spanned_by(
            lit,
            format!(
                "expected serde {} attribute to be a string: `{} = \"...\"`",
                attr_name, meta_item_name
            ),
        );
        Err(())
    }
}

fn parse_lit_into_expr_path(
    cx: &Ctxt,
    attr_name: Symbol,
    lit: &syn::Lit,
) -> Result<syn::ExprPath, ()> {
    let string = get_lit_str(cx, attr_name, lit)?;
    parse_lit_str(string).map_err(|_| {
        cx.error_spanned_by(lit, format!("failed to parse path: {:?}", string.value()))
    })
}

fn parse_lit_into_where(
    cx: &Ctxt,
    attr_name: Symbol,
    meta_item_name: Symbol,
    lit: &syn::Lit,
) -> Result<Vec<syn::WherePredicate>, ()> {
    let string = get_lit_str2(cx, attr_name, meta_item_name, lit)?;
    if string.value().is_empty() {
        return Ok(Vec::new());
    }

    let where_string = syn::LitStr::new(&format!("where {}", string.value()), string.span());

    parse_lit_str::<syn::WhereClause>(&where_string)
        .map(|wh| wh.predicates.into_iter().collect())
        .map_err(|err| cx.error_spanned_by(lit, err))
}

// All lifetimes that this type could borrow from a Deserializer.
//
// For example a type `S<'a, 'b>` could borrow `'a` and `'b`. On the other hand
// a type `for<'a> fn(&'a str)` could not borrow `'a` from the Deserializer.
//
// This is used when there is an explicit or implicit `#[serde(borrow)]`
// attribute on the field so there must be at least one borrowable lifetime.
fn borrowable_lifetimes(
    cx: &Ctxt,
    name: &str,
    field: &syn::Field,
) -> Result<BTreeSet<syn::Lifetime>, ()> {
    let mut lifetimes = BTreeSet::new();
    collect_lifetimes(&field.ty, &mut lifetimes);
    if lifetimes.is_empty() {
        cx.error_spanned_by(
            field,
            format!("field `{}` has no lifetimes to borrow", name),
        );
        Err(())
    } else {
        Ok(lifetimes)
    }
}

fn collect_lifetimes(ty: &syn::Type, out: &mut BTreeSet<syn::Lifetime>) {
    match ty {
        syn::Type::Slice(ty) => {
            collect_lifetimes(&ty.elem, out);
        }
        syn::Type::Array(ty) => {
            collect_lifetimes(&ty.elem, out);
        }
        syn::Type::Ptr(ty) => {
            collect_lifetimes(&ty.elem, out);
        }
        syn::Type::Reference(ty) => {
            out.extend(ty.lifetime.iter().cloned());
            collect_lifetimes(&ty.elem, out);
        }
        syn::Type::Tuple(ty) => {
            for elem in &ty.elems {
                collect_lifetimes(elem, out);
            }
        }
        syn::Type::Path(ty) => {
            if let Some(qself) = &ty.qself {
                collect_lifetimes(&qself.ty, out);
            }
            for seg in &ty.path.segments {
                if let syn::PathArguments::AngleBracketed(bracketed) = &seg.arguments {
                    for arg in &bracketed.args {
                        match arg {
                            syn::GenericArgument::Lifetime(lifetime) => {
                                out.insert(lifetime.clone());
                            }
                            syn::GenericArgument::Type(ty) => {
                                collect_lifetimes(ty, out);
                            }
                            syn::GenericArgument::Binding(binding) => {
                                collect_lifetimes(&binding.ty, out);
                            }
                            syn::GenericArgument::Constraint(_)
                            | syn::GenericArgument::Const(_) => {}
                        }
                    }
                }
            }
        }
        syn::Type::Paren(ty) => {
            collect_lifetimes(&ty.elem, out);
        }
        syn::Type::Group(ty) => {
            collect_lifetimes(&ty.elem, out);
        }
        syn::Type::Macro(ty) => {
            collect_lifetimes_from_tokens(ty.mac.tokens.clone(), out);
        }
        syn::Type::BareFn(_)
        | syn::Type::Never(_)
        | syn::Type::TraitObject(_)
        | syn::Type::ImplTrait(_)
        | syn::Type::Infer(_)
        | syn::Type::Verbatim(_) => {}

        #[cfg(test)]
        syn::Type::__TestExhaustive(_) => unimplemented!(),
        #[cfg(not(test))]
        _ => {}
    }
}

fn collect_lifetimes_from_tokens(tokens: TokenStream, out: &mut BTreeSet<syn::Lifetime>) {
    let mut iter = tokens.into_iter();
    while let Some(tt) = iter.next() {
        match &tt {
            TokenTree::Punct(op) if op.as_char() == '\'' && op.spacing() == Spacing::Joint => {
                if let Some(TokenTree::Ident(ident)) = iter.next() {
                    out.insert(syn::Lifetime {
                        apostrophe: op.span(),
                        ident,
                    });
                }
            }
            TokenTree::Group(group) => {
                let tokens = group.stream();
                collect_lifetimes_from_tokens(tokens, out);
            }
            _ => {}
        }
    }
}

fn parse_lit_str<T>(s: &syn::LitStr) -> parse::Result<T>
where
    T: Parse,
{
    let tokens = spanned_tokens(s)?;
    syn::parse2(tokens)
}

fn spanned_tokens(s: &syn::LitStr) -> parse::Result<TokenStream> {
    let stream = syn::parse_str(&s.value())?;
    Ok(respan(stream, s.span()))
}
