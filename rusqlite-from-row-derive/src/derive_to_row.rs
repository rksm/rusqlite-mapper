use darling::{ast::Data, Error, FromDeriveInput, FromField, ToTokens};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Result};

/// Fallible entry point for generating a `ToRow` implementation
pub(crate) fn try_derive_to_row(input: &DeriveInput) -> std::result::Result<TokenStream, Error> {
    let to_row_derive = DeriveToRow::from_derive_input(input)?;
    Ok(to_row_derive.generate()?)
}

/// Main struct for deriving `ToRow` for a struct.
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(to_row),
    forward_attrs(allow, doc, cfg),
    supports(struct_named)
)]
struct DeriveToRow {
    ident: syn::Ident,
    generics: syn::Generics,
    data: Data<(), ToRowField>,
}
impl DeriveToRow {
    /// Provides a slice of this struct's fields.
    fn fields(&self) -> &[ToRowField] {
        match &self.data {
            Data::Struct(fields) => &fields.fields,
            _ => panic!("invalid shape"),
        }
    }

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

        let table_name = self.ident.to_string();

        let sql_types = self
            .fields()
            .iter()
            .map(|field| {
                let ty = &field.ty;
                let primary_key = field.is_primary_key();
                quote! { (
                    <#ty as rusqlite_from_row::SqliteTypeInfo>::sqlite_type().to_string(),
                    <#ty as rusqlite_from_row::SqliteTypeInfo>::optional(),
                    #primary_key,
                ) }
            })
            .collect::<Vec<_>>();

        Ok(quote! {
            impl #impl_generics rusqlite_from_row::ToRow for #ident #ty_generics {
                type Params<'a> = (#(#params),*)
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
                    (#(#param_values),*)
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

/// A single field inside of a struct that derives `ToRow`
#[derive(Debug, FromField)]
#[darling(attributes(to_row), forward_attrs(allow, doc, cfg))]
struct ToRowField {
    /// The identifier of this field.
    ident: Option<syn::Ident>,
    /// The type specified in this field.
    ty: syn::Type,
    /// Override the name of the actual sql column instead of using `self.ident`.
    /// Is not compatible with `flatten` since no column is needed there.
    rename: Option<String>,

    primary_key: Option<()>,
}

impl ToRowField {
    fn param_ty(&self) -> Result<TokenStream2> {
        type_to_param_ty(&self.ty)
    }

    fn param_ref(&self) -> Result<TokenStream2> {
        let Some(ident) = &self.ident else {
            return Ok(quote! {
                ()
            });
        };

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
            // Some(id) if id == "String" => quote! { &self.#ident },
            // Some(id) if id == "Option" => quote! { &self.#ident },
            _ => quote! { &self.#ident },
        };

        Ok(res)
    }

    // fn column_type(&self) -> String {
    //     self.rename
    //         .as_ref()
    //         .map(Clone::clone)
    //         .unwrap_or_else(|| self.ident.as_ref().unwrap().to_string())
    // }

    /// Returns the name that maps to the actuall sql column
    /// By default this is the same as the rust field name but can be overwritten by `#[from_row(rename = "..")]`.
    fn column_name(&self) -> String {
        self.rename
            .as_ref()
            .map(Clone::clone)
            .unwrap_or_else(|| self.ident.as_ref().unwrap().to_string())
    }

    fn is_primary_key(&self) -> bool {
        self.primary_key.is_some()
    }
}

fn type_to_param_ty(ty: &syn::Type) -> Result<TokenStream2> {
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

            // segments if segments.last().map_or(false, |s| s.ident == "Option") => {
            //     let segment = segments.last().unwrap();
            //     let ty = match &segment.arguments {
            //         syn::PathArguments::AngleBracketed(args) => {
            //             let ty = args.args.first().unwrap();
            //             match ty {
            //                 syn::GenericArgument::Type(ty) => ty,
            //                 _ => panic!("invalid shape"),
            //             }
            //         }
            //         _ => panic!("invalid shape"),
            //     };
            //     let ty = type_to_param_ty(ty)?;
            //     quote! { Option<#ty> }
            // }
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
