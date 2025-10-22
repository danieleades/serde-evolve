use crate::parse::ParsedInput;
use quote::format_ident;

#[derive(Debug, Clone)]
pub struct ValidatedInput {
    pub domain_ident: syn::Ident,
    pub rep_ident: syn::Ident,
    pub mode: Mode,
    pub transparent: bool,
    pub versions: Vec<syn::Path>,
}

#[derive(Debug, Clone)]
pub enum Mode {
    Infallible,
    Fallible { error: syn::Path },
}

pub fn validate(parsed: ParsedInput) -> Result<ValidatedInput, syn::Error> {
    let ParsedInput {
        ident,
        representation,
        mode,
        error,
        transparent,
        versions,
    } = parsed;

    if versions.is_empty() {
        return Err(syn::Error::new_spanned(
            &ident,
            "chain must contain at least one version type",
        ));
    }

    let rep_ident = representation.unwrap_or_else(|| format_ident!("{}Versions", ident));

    let validated_mode = match mode.as_deref().unwrap_or("fallible") {
        "infallible" => Mode::Infallible,
        "fallible" => match error {
            Some(error) => Mode::Fallible { error },
            None => {
                return Err(syn::Error::new_spanned(
                    &ident,
                    "fallible mode requires 'error' attribute",
                ));
            }
        },
        other => {
            return Err(syn::Error::new_spanned(
                &ident,
                format!("invalid mode '{other}', expected 'infallible' or 'fallible'"),
            ));
        }
    };

    Ok(ValidatedInput {
        domain_ident: ident,
        rep_ident,
        mode: validated_mode,
        transparent,
        versions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, parse_str};

    fn base_parsed_input() -> ParsedInput {
        ParsedInput {
            ident: parse_str::<syn::Ident>("Example").unwrap(),
            representation: None,
            mode: None,
            error: Some(parse_quote!(ExampleError)),
            transparent: false,
            versions: vec![parse_quote!(Version1), parse_quote!(Version2)],
        }
    }

    #[test]
    fn infers_defaults_and_representation_name() {
        let parsed = base_parsed_input();
        let validated = validate(parsed).expect("validation should succeed");

        assert_eq!(validated.domain_ident.to_string(), "Example");
        assert_eq!(validated.rep_ident.to_string(), "ExampleVersions");
        assert!(matches!(validated.mode, Mode::Fallible { .. }));
        assert!(!validated.transparent);
        assert_eq!(validated.versions.len(), 2);
    }

    #[test]
    fn errors_when_missing_error_in_fallible_mode() {
        let mut parsed = base_parsed_input();
        parsed.error = None;
        let err = validate(parsed).expect_err("validation should fail");
        assert_eq!(err.to_string(), "fallible mode requires 'error' attribute");
    }

    #[test]
    fn errors_on_empty_version_chain() {
        let mut parsed = base_parsed_input();
        parsed.versions.clear();
        let err = validate(parsed).expect_err("validation should fail");
        assert_eq!(
            err.to_string(),
            "chain must contain at least one version type"
        );
    }
}
