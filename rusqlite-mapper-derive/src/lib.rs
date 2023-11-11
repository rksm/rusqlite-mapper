mod derive_from_row;
mod derive_sqlite_value;
mod derive_to_row;
mod fields;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(FromRow, attributes(rusqlite))]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    match derive_from_row::try_derive(&derive_input) {
        Ok(result) => result,
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(ToRow, attributes(rusqlite))]
pub fn derive_to_row(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    match derive_to_row::try_derive(&derive_input) {
        Ok(result) => result,
        Err(err) => err.write_errors().into(),
    }
}

#[proc_macro_derive(SqliteValue, attributes(sqlite))]
pub fn derive_sqlite_value(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    match derive_sqlite_value::try_derive_sqlite_value(&derive_input) {
        Ok(result) => result,
        Err(err) => err.write_errors().into(),
    }
}
