//! Procedural macros for the `serde-evolve` crate.
//!
//! This crate provides the `Versioned` derive macro for generating versioned type
//! conversions and serialization/deserialization implementations.

#![allow(clippy::option_if_let_else)] // `darling` expands field defaults into if-let/else; suppress noisy lint.

use darling::{FromDeriveInput, FromMeta};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::convert::TryFrom;
use syn::{DeriveInput, parse_macro_input};

/// Derive macro for versioned data structures.
///
/// See the `serde-evolve` crate documentation for usage examples.
#[proc_macro_derive(Versioned, attributes(versioned))]
pub fn derive_versioned(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match VersionedReceiver::from_derive_input(&input) {
        Ok(receiver) => receiver.generate().into(),
        Err(err) => err.write_errors().into(),
    }
}

// `darling` expands the `default` attribute into an `if let / else` block that triggers
// Clippy's option-if-let-else lint; suppress it locally so callers do not need to.
#[allow(clippy::option_if_let_else)]
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(versioned), supports(struct_any))]
struct VersionedReceiver {
    ident: syn::Ident,

    /// Name of the generated representation enum (defaults to {Type}Versions)
    #[darling(default)]
    rep: Option<syn::Ident>,

    /// Mode: "infallible" or "fallible" (defaults to "fallible")
    #[darling(default)]
    mode: Option<String>,

    /// Error type for fallible mode
    #[darling(default)]
    error: Option<syn::Path>,

    /// Enable transparent serde support (serialize/deserialize domain type directly)
    #[darling(default)]
    transparent: Option<bool>,

    /// Chain of version types
    chain: ChainList,
}

#[derive(Debug, Clone)]
struct ChainList(Vec<syn::Path>);

impl FromMeta for ChainList {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        items
            .iter()
            .map(|item| match item {
                darling::ast::NestedMeta::Meta(meta) => {
                    if let syn::Meta::Path(path) = meta {
                        Ok(path.clone())
                    } else {
                        Err(darling::Error::unexpected_type("path"))
                    }
                }
                darling::ast::NestedMeta::Lit(_) => Err(darling::Error::unexpected_type("path")),
            })
            .collect::<darling::Result<Vec<_>>>()
            .map(ChainList)
    }
}

impl ChainList {
    const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl VersionedReceiver {
    fn generate(&self) -> proc_macro2::TokenStream {
        let domain_type = &self.ident;
        let rep_name = self
            .rep
            .clone()
            .unwrap_or_else(|| format_ident!("{}Versions", domain_type));

        // Validate mode (default to fallible)
        let mode = match self.mode.as_deref().unwrap_or("fallible") {
            "infallible" => Mode::Infallible,
            "fallible" => Mode::Fallible,
            other => {
                return syn::Error::new_spanned(
                    &self.ident,
                    format!("invalid mode '{other}', expected 'infallible' or 'fallible'"),
                )
                .to_compile_error();
            }
        };

        // Validate error type for fallible mode
        if matches!(mode, Mode::Fallible) && self.error.is_none() {
            return syn::Error::new_spanned(
                &self.ident,
                "fallible mode requires 'error' attribute",
            )
            .to_compile_error();
        }

        if self.chain.is_empty() {
            return syn::Error::new_spanned(
                &self.ident,
                "chain must contain at least one version type",
            )
            .to_compile_error();
        }

        let version_types = &self.chain.0;

        // Generate representation enum
        let rep_enum = Self::generate_rep_enum(&rep_name, version_types);

        // Generate conversions
        let conversions = self.generate_conversions(&mode, domain_type, &rep_name, version_types);

        // Generate transparent serde impls if requested
        let transparent_serde = if self.transparent.unwrap_or(false) {
            Self::generate_transparent_serde(&mode, domain_type, &rep_name)
        } else {
            quote! {}
        };

        quote! {
            #rep_enum
            #conversions
            #transparent_serde
        }
    }

    fn generate_rep_enum(
        rep_name: &syn::Ident,
        version_types: &[syn::Path],
    ) -> proc_macro2::TokenStream {
        let num_versions = version_types.len();
        let current_version =
            u32::try_from(num_versions).expect("too many versions for u32 discriminant");

        // Generate enum variants
        let variants: Vec<proc_macro2::TokenStream> = version_types
            .iter()
            .enumerate()
            .map(|(idx, ty)| {
                let variant_name = format_ident!("V{}", idx + 1);
                let version_str = (idx + 1).to_string();
                quote! {
                    #[serde(rename = #version_str)]
                    #variant_name(#ty)
                }
            })
            .collect();

        // Generate version() match arms
        let version_match_arms: Vec<proc_macro2::TokenStream> = (0..num_versions)
            .map(|idx| {
                let variant_name = format_ident!("V{}", idx + 1);
                let version_num =
                    u32::try_from(idx + 1).expect("too many versions for u32 discriminant");
                quote! {
                    Self::#variant_name(_) => #version_num
                }
            })
            .collect();

        // Generate From impls for each version into representation enum
        let from_impls: Vec<proc_macro2::TokenStream> = version_types
            .iter()
            .enumerate()
            .map(|(idx, ty)| {
                let variant_name = format_ident!("V{}", idx + 1);
                quote! {
                    impl From<#ty> for #rep_name {
                        fn from(v: #ty) -> Self {
                            Self::#variant_name(v)
                        }
                    }
                }
            })
            .collect();

        let latest_variant = format_ident!("V{}", num_versions);

        quote! {
            #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
            #[serde(tag = "_version")]
            pub enum #rep_name {
                #(#variants),*
            }

            impl #rep_name {
                /// The current version number.
                pub const CURRENT: u32 = #current_version;

                /// Get the version number of this instance.
                pub const fn version(&self) -> u32 {
                    match self {
                        #(#version_match_arms),*
                    }
                }

                /// Check if this is the current version.
                pub const fn is_current(&self) -> bool {
                    matches!(self, Self::#latest_variant(_))
                }
            }

            // Boilerplate From impls
            #(#from_impls)*
        }
    }

    fn generate_conversions(
        &self,
        mode: &Mode,
        domain_type: &syn::Ident,
        rep_name: &syn::Ident,
        version_types: &[syn::Path],
    ) -> proc_macro2::TokenStream {
        let num_versions = version_types.len();

        let rep_to_domain = match mode {
            Mode::Infallible => {
                // Generate From<Rep> for Domain
                let variant_conversions: Vec<proc_macro2::TokenStream> = (0..num_versions)
                    .map(|idx| {
                        let variant_name = format_ident!("V{}", idx + 1);

                        let chain = Self::build_infallible_chain(domain_type, version_types, idx);

                        quote! {
                            #rep_name::#variant_name(v) => {
                                #chain
                            }
                        }
                    })
                    .collect();

                quote! {
                    impl From<#rep_name> for #domain_type {
                        fn from(rep: #rep_name) -> Self {
                            match rep {
                                #(#variant_conversions),*
                            }
                        }
                    }
                }
            }
            Mode::Fallible => {
                let error_type = self.error.as_ref().unwrap();

                // Generate TryFrom<Rep> for Domain
                let variant_conversions: Vec<proc_macro2::TokenStream> = (0..num_versions)
                    .map(|idx| {
                        let variant_name = format_ident!("V{}", idx + 1);

                        let chain = Self::build_fallible_chain(domain_type, version_types, idx);

                        quote! {
                            #rep_name::#variant_name(v) => {
                                #chain
                            }
                        }
                    })
                    .collect();

                quote! {
                    impl core::convert::TryFrom<#rep_name> for #domain_type {
                        type Error = #error_type;

                        fn try_from(rep: #rep_name) -> Result<Self, Self::Error> {
                            match rep {
                                #(#variant_conversions),*
                            }
                        }
                    }
                }
            }
        };

        // Generate From<&Domain> for representation (always uses latest version)
        let latest_version_type = &version_types[num_versions - 1];
        let latest_variant = format_ident!("V{}", num_versions);

        let domain_to_rep = quote! {
            impl From<&#domain_type> for #rep_name {
                fn from(domain: &#domain_type) -> Self {
                    let latest = #latest_version_type::from(domain);
                    Self::#latest_variant(latest)
                }
            }
        };

        quote! {
            #rep_to_domain
            #domain_to_rep
        }
    }

    fn generate_transparent_serde(
        mode: &Mode,
        domain_type: &syn::Ident,
        rep_name: &syn::Ident,
    ) -> proc_macro2::TokenStream {
        // Generate Serialize impl
        let serialize_impl = quote! {
            impl serde::Serialize for #domain_type {
                fn serialize<__S>(&self, __serializer: __S) -> core::result::Result<__S::Ok, __S::Error>
                where
                    __S: serde::Serializer,
                {
                    #rep_name::from(self).serialize(__serializer)
                }
            }
        };

        // Generate Deserialize impl (differs based on mode)
        let deserialize_impl = match mode {
            Mode::Infallible => {
                quote! {
                    impl<'de> serde::Deserialize<'de> for #domain_type {
                        fn deserialize<__D>(__deserializer: __D) -> core::result::Result<Self, __D::Error>
                        where
                            __D: serde::Deserializer<'de>,
                        {
                            Ok(#rep_name::deserialize(__deserializer)?.into())
                        }
                    }
                }
            }
            Mode::Fallible => {
                quote! {
                    impl<'de> serde::Deserialize<'de> for #domain_type {
                        fn deserialize<__D>(__deserializer: __D) -> core::result::Result<Self, __D::Error>
                        where
                            __D: serde::Deserializer<'de>,
                        {
                            #rep_name::deserialize(__deserializer)?
                                .try_into()
                                .map_err(serde::de::Error::custom)
                        }
                    }
                }
            }
        };

        quote! {
            #serialize_impl
            #deserialize_impl
        }
    }

    fn build_infallible_chain(
        domain_type: &syn::Ident,
        version_types: &[syn::Path],
        start_idx: usize,
    ) -> proc_macro2::TokenStream {
        let mut expr = quote! { v };

        for ty in version_types.iter().skip(start_idx + 1) {
            expr = quote! {{
                let next: #ty = #expr.into();
                next
            }};
        }

        quote! {{
            let next: #domain_type = #expr.into();
            next
        }}
    }

    fn build_fallible_chain(
        domain_type: &syn::Ident,
        version_types: &[syn::Path],
        start_idx: usize,
    ) -> proc_macro2::TokenStream {
        let mut expr = quote! { v };

        for ty in version_types.iter().skip(start_idx + 1) {
            expr = quote! {{
                let next: #ty = #expr.try_into()?;
                next
            }};
        }

        quote! {{
            let next: #domain_type = #expr.try_into()?;
            Ok(next)
        }}
    }
}

enum Mode {
    Infallible,
    Fallible,
}
