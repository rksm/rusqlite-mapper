use darling::{ast::Data, Error, FromDeriveInput};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{DeriveInput, Result};

use crate::fields::SqliteField;

/// Fallible entry point for generating a `FromRow`, `ToRow` implementation
pub(crate) fn try_derive(input: &DeriveInput) -> std::result::Result<TokenStream, Error> {
    let derive = DeriveFromRow::from_derive_input(input)?;
    Ok(derive.generate()?)
}

/// Main struct for deriving `FromRow` for a struct.
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(rusqlite),
    forward_attrs(allow, doc, cfg),
    supports(struct_named)
)]
pub(crate) struct DeriveFromRow {
    pub(crate) ident: syn::Ident,
    pub(crate) generics: syn::Generics,
    pub(crate) data: Data<(), SqliteField>,
}

impl DeriveFromRow {
    /// Validates all fields
    fn validate(&self) -> Result<()> {
        for field in self.fields() {
            field.validate()?;
        }

        Ok(())
    }

    /// Generates any additional where clause predicates needed for the fields in this struct.
    pub(crate) fn predicates(&self) -> Result<Vec<TokenStream2>> {
        let mut predicates = Vec::new();

        for field in self.fields() {
            field.add_predicates(&mut predicates)?;
        }

        Ok(predicates)
    }

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

    pub(crate) fn all_fields(&self) -> &[SqliteField] {
        match &self.data {
            Data::Struct(fields) => &fields.fields,
            _ => panic!("invalid shape"),
        }
    }

    /// Generate the `FromRow` implementation.
    fn generate(self) -> Result<TokenStream> {
        self.validate()?;

        let ident = &self.ident;

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let original_predicates = where_clause.map(|w| &w.predicates).into_iter();
        let predicates = self.predicates()?;

        let is_all_null_fields = self
            .fields()
            .iter()
            .map(|f| f.generate_is_all_null())
            .collect::<syn::Result<Vec<_>>>()?;

        let try_from_row_fields = self
            .all_fields()
            .iter()
            .map(|f| f.generate_try_from_row())
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            impl #impl_generics rusqlite_mapper::FromRow for #ident #ty_generics where #(#original_predicates),* #(#predicates),* {
                fn try_from_row_prefixed(
                    row: &::rusqlite::Row,
                    prefix: Option<&str>
                ) -> std::result::Result<Self, ::rusqlite::Error> {
                    Ok(Self {
                        #(#try_from_row_fields),*
                    })
                }

                fn is_all_null(
                    row: &::rusqlite::Row,
                    prefix: Option<&str>
                ) -> std::result::Result<bool, ::rusqlite::Error> {
                    Ok(#(#is_all_null_fields)&&*)
                }
            }
        }
        .into())
    }
}

impl SqliteField {
    /// Checks wether this field has a valid combination of attributes
    fn validate(&self) -> Result<()> {
        if self.from.is_some() && self.try_from.is_some() {
            return Err(Error::custom(
                r#"can't combine `#[from_row(from = "..")]` with `#[from_row(try_from = "..")]`"#,
            )
            .into());
        }

        if self.rename.is_some() && self.flatten {
            return Err(Error::custom(
                r#"can't combine `#[from_row(flatten)]` with `#[from_row(rename = "..")]`"#,
            )
            .into());
        }

        Ok(())
    }

    /// Returns a tokenstream of the type that should be returned from either
    /// `FromRow` (when using `flatten`) or `FromSql`.
    fn target_ty(&self) -> Result<TokenStream2> {
        if let Some(from) = &self.from {
            Ok(from.parse()?)
        } else if let Some(try_from) = &self.try_from {
            Ok(try_from.parse()?)
        } else {
            Ok(self.ty.to_token_stream())
        }
    }

    fn generate_is_all_null(&self) -> Result<TokenStream2> {
        let column_name = self.column_name();
        let target_ty = self.target_ty()?;

        let line = if self.flatten {
            let prefix = if let Some(prefix) = &self.prefix {
                quote!(Some(&(prefix.unwrap_or("").to_string() + #prefix)))
            } else {
                quote!(prefix)
            };

            quote!(<#target_ty as rusqlite_mapper::FromRow>::is_all_null(row, #prefix)?)
        } else {
            quote! {
                ::rusqlite::Row::get_ref::<&str>(
                    row,
                    &(prefix.unwrap_or("").to_string() + #column_name)
                )? == ::rusqlite::types::ValueRef::Null
            }
        };

        Ok(line)
    }

    /// Pushes the needed where clause predicates for this field.
    ///
    /// By default this is `T: rusqlite::types::FromSql`,
    /// when using `flatten` it's: `T: rusqlite_mapper::FromRow`
    /// and when using either `from` or `try_from` attributes it additionally pushes this bound:
    /// `T: std::convert::From<R>`, where `T` is the type specified in the struct and `R` is the
    /// type specified in the `[try]_from` attribute.
    pub(crate) fn add_predicates(&self, predicates: &mut Vec<TokenStream2>) -> Result<()> {
        let target_ty = &self.target_ty()?;
        let ty = &self.ty;

        predicates.push(if self.flatten {
            quote! (#target_ty: rusqlite_mapper::FromRow)
        } else {
            quote! (#target_ty: ::rusqlite::types::FromSql)
        });

        if self.from.is_some() {
            predicates.push(quote!(#ty: std::convert::From<#target_ty>))
        } else if self.try_from.is_some() {
            let try_from = quote!(std::convert::TryFrom<#target_ty>);

            predicates.push(quote!(#ty: #try_from));
            predicates
                .push(quote!(::rusqlite::Error: std::convert::From<<#ty as #try_from>::Error>));
            predicates.push(quote!(<#ty as #try_from>::Error: std::fmt::Debug));
        }

        Ok(())
    }

    /// Generate the line needed to retrieve this field from a row when calling `try_from_row`.
    fn generate_try_from_row(&self) -> Result<TokenStream2> {
        let ident = self.ident.as_ref().unwrap();

        if self.skip.is_some() {
            return Ok(quote!(#ident: Default::default()));
        }

        let column_name = self.column_name();
        let field_ty = &self.ty;
        let target_ty = self.target_ty()?;

        let mut base = if self.flatten {
            let prefix = if let Some(prefix) = &self.prefix {
                quote!(Some(&(prefix.unwrap_or("").to_string() + #prefix)))
            } else {
                quote!(prefix)
            };

            quote!(<#target_ty as rusqlite_mapper::FromRow>::try_from_row_prefixed(row, #prefix)?)
        } else {
            quote!(::rusqlite::Row::get::<&str, #target_ty>(row, &(prefix.unwrap_or("").to_string() + #column_name))?)
        };

        if self.from.is_some() {
            base = quote!(<#field_ty as std::convert::From<#target_ty>>::from(#base));
        } else if self.try_from.is_some() {
            base = quote!(<#field_ty as std::convert::TryFrom<#target_ty>>::try_from(#base)?);
        };

        Ok(quote!(#ident: #base))
    }
}
