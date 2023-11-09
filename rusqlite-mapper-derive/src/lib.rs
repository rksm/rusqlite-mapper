mod derive_sqlite;
mod derive_sqlite_value;
mod fields;
mod from_row;
mod to_row;

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Calls the fallible entry point and writes any errors to the tokenstream.
#[proc_macro_derive(Sqlite, attributes(sqlite))]
pub fn derive_sqlite(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    match derive_sqlite::try_derive_sqlite(&derive_input) {
        Ok(result) => result,
        Err(err) => err.write_errors().into(),
    }
}

/// Calls the fallible entry point and writes any errors to the tokenstream.
#[proc_macro_derive(SqliteValue, attributes(sqlite))]
pub fn derive_sqlite_value(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    match derive_sqlite_value::try_derive_sqlite_value(&derive_input) {
        Ok(result) => result,
        Err(err) => err.write_errors().into(),
    }
}
