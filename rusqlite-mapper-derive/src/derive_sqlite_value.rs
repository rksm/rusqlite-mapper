use darling::{Error, FromDeriveInput};
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

/// Fallible entry point for generating a `FromRow`, `ToRow` implementation
pub(crate) fn try_derive_sqlite_value(
    input: &DeriveInput,
) -> std::result::Result<TokenStream, Error> {
    let derive = DeriveSqliteValue::from_derive_input(input)?;
    Ok(derive.generate()?)
}

/// Main struct for deriving `SqliteValue` for a struct or enum. This will
/// implement `rusqlite::ToSql` and `rusqlite::types::FromSql` for the type as
/// well as `rusqlite_mapper::SqliteTypeInfo`.
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(rusqlite_value),
    forward_attrs(allow, doc, cfg),
    supports(any)
)]
pub(crate) struct DeriveSqliteValue {
    pub(crate) ident: syn::Ident,

    string: Option<()>,
    json: Option<()>,
}

impl DeriveSqliteValue {
    fn generate(self) -> Result<TokenStream> {
        match (self.string, self.json) {
            (Some(_), _) => self.generate_as_string(),
            (_, Some(_)) => self.generate_as_json(),
            _ => Err(Error::custom(
                r#"must specify one of `#[sqlite(as_string)]` or `#[sqlite(as_json)]`"#,
            )
            .into()),
        }
    }

    fn generate_as_string(self) -> Result<TokenStream> {
        let ident = &self.ident;

        Ok(quote! {
            impl rusqlite_mapper::SqliteTypeInfo for #ident {
                fn sqlite_type() -> &'static str {
                    "TEXT"
                }
            }

            impl rusqlite::ToSql for #ident {
                fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                    let s = self.to_string();
                    Ok(rusqlite::types::ToSqlOutput::from(s))
                }
            }

            impl rusqlite::types::FromSql for #ident {
                fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
                    let s = String::column_result(value)?;
                    let at = s
                        .parse::<#ident>()
                        .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))?;
                    Ok(at)
                }
            }

        }
        .into())
    }

    fn generate_as_json(self) -> Result<TokenStream> {
        let ident = &self.ident;

        Ok(quote! {
            impl rusqlite_mapper::SqliteTypeInfo for #ident {
                fn sqlite_type() -> &'static str {
                    "TEXT"
                }
            }

            impl rusqlite::ToSql for #ident {
                fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                    let s = serde_json::to_string(self)
                        .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))?;
                    Ok(rusqlite::types::ToSqlOutput::from(s))
                }
            }

            impl rusqlite::types::FromSql for #ident {
                fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
                    let s = String::column_result(value)?;
                    let val: #ident = serde_json::from_str(&s)
                        .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))?;
                    Ok(val)
                }
            }
        }
        .into())
    }
}
