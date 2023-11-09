mod derive_from_row;
mod derive_to_row;

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Calls the fallible entry point and writes any errors to the tokenstream.
#[proc_macro_derive(FromRow, attributes(from_row))]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    match derive_from_row::try_derive_from_row(&derive_input) {
        Ok(result) => result,
        Err(err) => err.write_errors().into(),
    }
}

/// Calls the fallible entry point and writes any errors to the tokenstream.
#[proc_macro_derive(ToRow, attributes(to_row))]
pub fn derive_to_row(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    match derive_to_row::try_derive_to_row(&derive_input) {
        Ok(result) => result,
        Err(err) => err.write_errors().into(),
    }
}
