//! Procedural macros for the `serde-evolve` crate.
//!
//! This crate provides the `Versioned` derive macro for generating versioned type
//! conversions and serialization/deserialization implementations.

#![allow(clippy::option_if_let_else)] // `darling` expands field defaults into if-let/else; suppress noisy lint.

mod emit;
mod parse;
mod validate;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Derive macro for versioned data structures.
///
/// See the `serde-evolve` crate documentation for usage examples.
#[proc_macro_derive(Versioned, attributes(versioned))]
pub fn derive_versioned(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match parse::parse_input(&input) {
        Ok(parsed) => match validate::validate(parsed) {
            Ok(validated) => emit::generate(&validated).into(),
            Err(err) => err.to_compile_error().into(),
        },
        Err(err) => err.write_errors().into(),
    }
}
