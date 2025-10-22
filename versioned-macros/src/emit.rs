use crate::validate::{Mode, ValidatedInput};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::convert::TryFrom;

pub fn generate(input: &ValidatedInput) -> TokenStream {
    let rep_enum = generate_rep_enum(&input.rep_ident, &input.versions);
    let conversions = generate_conversions(
        &input.mode,
        &input.domain_ident,
        &input.rep_ident,
        &input.versions,
    );
    let transparent_serde = if input.transparent {
        generate_transparent_serde(&input.mode, &input.domain_ident, &input.rep_ident)
    } else {
        quote! {}
    };

    quote! {
        #rep_enum
        #conversions
        #transparent_serde
    }
}

fn generate_rep_enum(rep_name: &syn::Ident, version_types: &[syn::Path]) -> TokenStream {
    let num_versions = version_types.len();
    let current_version =
        u32::try_from(num_versions).expect("too many versions for u32 discriminant");

    let variants = version_types
        .iter()
        .enumerate()
        .map(|(idx, ty)| {
            let variant_name = format_ident!("V{}", idx + 1);
            let version_str = (idx + 1).to_string();
            quote! {
                #[serde(rename = #version_str)]
                #variant_name(#ty)
            }
        });

    let version_match_arms = (0..num_versions)
        .map(|idx| {
            let variant_name = format_ident!("V{}", idx + 1);
            let version_num =
                u32::try_from(idx + 1).expect("too many versions for u32 discriminant");
            quote! {
                Self::#variant_name(_) => #version_num
            }
        });

    let from_impls = version_types
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
        });

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

        #(#from_impls)*
    }
}

fn generate_conversions(
    mode: &Mode,
    domain_type: &syn::Ident,
    rep_name: &syn::Ident,
    version_types: &[syn::Path],
) -> TokenStream {
    let num_versions = version_types.len();

    let rep_to_domain = match mode {
        Mode::Infallible => {
            let variant_conversions = (0..num_versions)
                .map(|idx| {
                    let variant_name = format_ident!("V{}", idx + 1);
                    let chain = build_infallible_chain(domain_type, version_types, idx);

                    quote! {
                        #rep_name::#variant_name(v) => {
                            #chain
                        }
                    }
                });

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
        Mode::Fallible { error } => {
            let variant_conversions = (0..num_versions)
                .map(|idx| {
                    let variant_name = format_ident!("V{}", idx + 1);
                    let chain = build_fallible_chain(domain_type, version_types, idx);

                    quote! {
                        #rep_name::#variant_name(v) => {
                            #chain
                        }
                    }
                });

            quote! {
                impl core::convert::TryFrom<#rep_name> for #domain_type {
                    type Error = #error;

                    fn try_from(rep: #rep_name) -> Result<Self, Self::Error> {
                        match rep {
                            #(#variant_conversions),*
                        }
                    }
                }
            }
        }
    };

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
) -> TokenStream {
    let serialize_impl = quote! {
        impl serde::Serialize for #domain_type {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> core::result::Result<__S::Ok, __S::Error>
            where
                __S: serde::Serializer,
            {
                #rep_name::from(self).serialize(__serializer)
            }
        }
    };

    let deserialize_impl = match mode {
        Mode::Infallible => {
            quote! {
                impl<'de> serde::Deserialize<'de> for #domain_type {
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> core::result::Result<Self, __D::Error>
                    where
                        __D: serde::Deserializer<'de>,
                    {
                        Ok(#rep_name::deserialize(__deserializer)?.into())
                    }
                }
            }
        }
        Mode::Fallible { .. } => {
            quote! {
                impl<'de> serde::Deserialize<'de> for #domain_type {
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> core::result::Result<Self, __D::Error>
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
) -> TokenStream {
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
) -> TokenStream {
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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, parse_str};

    fn validated_input(mode: Mode) -> ValidatedInput {
        ValidatedInput {
            domain_ident: parse_str::<syn::Ident>("Example").unwrap(),
            rep_ident: parse_str::<syn::Ident>("ExampleVersions").unwrap(),
            mode,
            transparent: false,
            versions: vec![parse_quote!(Version1), parse_quote!(Version2)],
        }
    }

    #[test]
    fn generates_infallible_conversions() {
        let input = validated_input(Mode::Infallible);
        let tokens = generate(&input).to_string();
        assert!(tokens.contains("impl From < ExampleVersions > for Example"));
        assert!(tokens.contains("impl From < & Example > for ExampleVersions"));
    }

    #[test]
    fn generates_fallible_conversions() {
        let input = validated_input(Mode::Fallible {
            error: parse_quote!(ExampleError),
        });
        let tokens = generate(&input).to_string();
        assert!(tokens.contains("impl core :: convert :: TryFrom < ExampleVersions > for Example"));
        assert!(tokens.contains("type Error = ExampleError"));
    }

    #[test]
    fn includes_representation_metadata() {
        let input = validated_input(Mode::Infallible);
        let tokens = generate(&input).to_string();
        assert!(tokens.contains("pub enum ExampleVersions"));
        assert!(tokens.contains("pub const CURRENT : u32 = 2"));
    }
}
