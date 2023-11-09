use darling::{ast::Data, Error, FromDeriveInput};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::fields::SqliteField;

/// Fallible entry point for generating a `FromRow`, `ToRow` implementation
pub(crate) fn try_derive_sqlite(input: &DeriveInput) -> std::result::Result<TokenStream, Error> {
    let derive = DeriveSqlite::from_derive_input(input)?;
    Ok(derive.generate()?)
}

/// Main struct for deriving `FromRow` for a struct.
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(sqlite),
    forward_attrs(allow, doc, cfg),
    supports(struct_named)
)]
pub(crate) struct DeriveSqlite {
    pub(crate) ident: syn::Ident,
    pub(crate) generics: syn::Generics,
    pub(crate) data: Data<(), SqliteField>,

    skip_from_row: Option<()>,
    skip_to_row: Option<()>,
}

impl DeriveSqlite {
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
    pub(crate) fn fields(&self) -> &[SqliteField] {
        match &self.data {
            Data::Struct(fields) => &fields.fields,
            _ => panic!("invalid shape"),
        }
    }

    /// Generate the `FromRow` implementation.
    fn generate(self) -> Result<TokenStream> {
        self.validate()?;

        let from_row = if self.skip_from_row.is_some() {
            quote! {}
        } else {
            self.generate_from_row()?
        };

        let to_row = if self.skip_to_row.is_some() {
            quote! {}
        } else {
            self.generate_to_row()?
        };

        Ok(quote! {
            #from_row
            #to_row
        }
        .into())
    }
}
