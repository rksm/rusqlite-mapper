use darling::{ast::Data, Error, FromDeriveInput};
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{DeriveInput, Result};

use crate::fields::SqliteField;

/// Fallible entry point for generating a `FromRow`, `ToRow` implementation
pub(crate) fn try_derive(input: &DeriveInput) -> std::result::Result<TokenStream, Error> {
    let derive = DeriveToRow::from_derive_input(input)?;
    Ok(derive.generate()?)
}

/// Main struct for deriving `ToRow` for a struct.
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(rusqlite),
    forward_attrs(allow, doc, cfg),
    supports(struct_named)
)]
pub(crate) struct DeriveToRow {
    pub(crate) ident: syn::Ident,
    pub(crate) generics: syn::Generics,
    pub(crate) data: Data<(), SqliteField>,
}

impl DeriveToRow {
    /// Provides a slice of this struct's fields.
    pub(crate) fn fields(&self) -> Vec<&SqliteField> {
        match &self.data {
            Data::Struct(fields) => fields
                .fields
                .iter()
                .filter(|f| f.skip.is_none())
                .collect::<Vec<_>>(),
            _ => panic!("invalid shape"),
        }
    }

    /// Generate the `FromRow` implementation.
    fn generate(self) -> Result<TokenStream> {
        let ident = &self.ident;

        let (impl_generics, ty_generics, _where_clause) = self.generics.split_for_impl();

        let params = self
            .fields()
            .iter()
            .map(|field| {
                let ty = field.param_ty()?;
                Ok(quote! { #ty })
            })
            .collect::<Result<Vec<_>>>()?;

        let param_values = self
            .fields()
            .iter()
            .map(|field| field.param_ref())
            .collect::<Result<Vec<_>>>()?;

        let column_names = self
            .fields()
            .iter()
            .map(|field| field.column_name())
            .collect::<Vec<_>>();

        let table_name = self.ident.to_string().to_snake_case();

        let sql_types = self
            .fields()
            .iter()
            .map(|field| {
                let ty = &field.ty;
                let primary_key = field.is_primary_key();
                quote! { (
                    <#ty as rusqlite_mapper::SqliteTypeInfo>::sqlite_type().to_string(),
                    <#ty as rusqlite_mapper::SqliteTypeInfo>::optional(),
                    #primary_key,
                ) }
            })
            .collect::<Vec<_>>();

        Ok(quote! {
            impl #impl_generics rusqlite_mapper::ToRow for #ident #ty_generics {
                type Params<'a> = (#(#params),*,)
                where
                    Self: 'a;

                fn table_name() -> &'static str {
                    #table_name
                }

                fn column_names() -> &'static [&'static str] {
                    &[
                        #(
                            #column_names,
                        )*
                    ]
                }

                fn to_params(&self) -> Self::Params<'_> {
                    (#(#param_values),*,)
                }

                fn sql_types() -> Vec<(String, bool, bool)> {
                    vec![
                        #(
                            #sql_types,
                        )*
                    ]
                }
            }
        }
        .into())
    }
}

impl SqliteField {
    /// The Rust type of when this field is converted to a param
    fn param_ty(&self) -> Result<TokenStream2> {
        type_to_param_ty(&self.ty, self.value.is_some())
    }

    /// The Rust expression to access this field as a param
    fn param_ref(&self) -> Result<TokenStream2> {
        let Some(ident) = &self.ident else {
            return Ok(quote! {
                ()
            });
        };

        if self.value.is_some() {
            return Ok(quote! {
                self.#ident
            });
        }

        let outer_type_ident = if let syn::Type::Path(syn::TypePath {
            path: syn::Path { segments, .. },
            ..
        }) = &self.ty
        {
            segments.last().map(|s| &s.ident)
        } else {
            None
        };

        let res = match outer_type_ident {
            Some(id)
                if id == "i8"
                    || id == "i16"
                    || id == "i32"
                    || id == "i64"
                    || id == "isize"
                    || id == "u8"
                    || id == "u16"
                    || id == "u32"
                    || id == "f32"
                    || id == "f64"
                    || id == "u64"
                    || id == "usize"
                    || id == "bool" =>
            {
                quote! { self.#ident }
            }
            _ => quote! { &self.#ident },
        };

        Ok(res)
    }
}

fn type_to_param_ty(ty: &syn::Type, force_as_value: bool) -> Result<TokenStream2> {
    if force_as_value {
        return Ok(quote! { #ty });
    }

    let ty = match &ty {
        syn::Type::Path(syn::TypePath {
            path: syn::Path { segments, .. },
            ..
        }) => match segments {
            segments
                if segments.len() == 1
                    && matches!(
                        segments[0].ident.to_string().as_str(),
                        "i8" | "i16"
                            | "i32"
                            | "i64"
                            | "isize"
                            | "u8"
                            | "u16"
                            | "u32"
                            | "f32"
                            | "f64"
                            | "u64"
                            | "usize"
                            | "bool"
                    ) =>
            {
                let ident = &segments[0].ident;
                quote! { #ident }
            }

            segments if segments.last().map_or(false, |s| s.ident == "Vec") => {
                let segment = segments.last().unwrap();
                let ty = match &segment.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        let ty = args.args.first().unwrap();
                        match ty {
                            syn::GenericArgument::Type(ty) => ty,
                            _ => panic!("invalid shape"),
                        }
                    }
                    _ => panic!("invalid shape"),
                };
                quote! { &'a [#ty] }
            }

            _ => quote! { &'a #ty },
        },

        _ => ty.to_token_stream(),
    };

    Ok(ty)
}
