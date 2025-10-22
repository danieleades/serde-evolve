use darling::{FromDeriveInput, FromMeta};
use syn::DeriveInput;

#[derive(Debug)]
pub struct ParsedInput {
    pub ident: syn::Ident,
    pub representation: Option<syn::Ident>,
    pub mode: Option<String>,
    pub error: Option<syn::Path>,
    pub transparent: bool,
    pub versions: Vec<syn::Path>,
}

pub fn parse_input(input: &DeriveInput) -> darling::Result<ParsedInput> {
    let receiver = VersionedReceiver::from_derive_input(input)?;

    Ok(ParsedInput {
        ident: receiver.ident,
        representation: receiver.rep,
        mode: receiver.mode,
        error: receiver.error,
        transparent: receiver.transparent.unwrap_or(false),
        versions: receiver.chain.0,
    })
}

// `darling` expands the `default` attribute into an `if let / else` block that triggers
// Clippy's option-if-let-else lint; suppress it locally so callers do not need to.
#[allow(clippy::option_if_let_else)]
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(versioned), supports(struct_any))]
struct VersionedReceiver {
    pub(crate) ident: syn::Ident,

    /// Name of the generated representation enum (defaults to {Type}Versions)
    #[darling(default)]
    pub(crate) rep: Option<syn::Ident>,

    /// Mode: "infallible" or "fallible" (defaults to "fallible")
    #[darling(default)]
    pub(crate) mode: Option<String>,

    /// Error type for fallible mode
    #[darling(default)]
    pub(crate) error: Option<syn::Path>,

    /// Enable transparent serde support (serialize/deserialize domain type directly)
    #[darling(default)]
    pub(crate) transparent: Option<bool>,

    /// Chain of version types
    pub(crate) chain: ChainList,
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::{ToTokens, format_ident};
    use syn::parse_quote;

    #[test]
    fn parses_metadata_from_input() {
        let input: DeriveInput = parse_quote! {
            #[derive(Versioned)]
            #[versioned(
                chain(Version1, Version2),
                rep = "CustomRep",
                mode = "fallible",
                error = "MyError",
                transparent = true
            )]
            struct Example;
        };

        let parsed = parse_input(&input).expect("expected parse success");
        assert_eq!(parsed.ident, format_ident!("Example"));
        assert_eq!(parsed.representation, Some(format_ident!("CustomRep")));
        assert_eq!(parsed.mode.as_deref(), Some("fallible"));
        assert_eq!(
            parsed.error.unwrap().to_token_stream().to_string(),
            "MyError"
        );
        assert!(parsed.transparent);
        assert_eq!(parsed.versions.len(), 2);
    }
}
