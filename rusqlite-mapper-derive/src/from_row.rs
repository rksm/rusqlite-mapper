use darling::ToTokens;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Result;

use crate::{derive_sqlite::DeriveSqlite, fields::SqliteField};

impl DeriveSqlite {
    /// Generate the `FromRow` implementation.
    pub(crate) fn generate_from_row(&self) -> Result<TokenStream2> {
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
            .fields()
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
        .to_token_stream())
    }
}

impl SqliteField {
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

    /// Generate the line needed to retrieve this field from a row when calling `try_from_row`.
    fn generate_try_from_row(&self) -> Result<TokenStream2> {
        let ident = self.ident.as_ref().unwrap();
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
