//! The `generate!` macro, re-exported by `toapi-client`.

use std::path::PathBuf;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Ident, LitStr, Path, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

/// Generates the client for an OpenAPI document at compile time: the `Api`
/// path tree, the call builders and the request, query and body types.
///
/// ```ignore
/// toapi_client::generate!("../users-server/openapi.json");
///
/// let api = Api::new("http://localhost:3000/api");
/// let user = api.users.id(1).get().await?;
/// ```
///
/// The path is relative to the crate's `Cargo.toml`. The crate is rebuilt
/// when the document changes.
///
/// `derive(..)` adds derives to every generated struct and enum:
///
/// ```ignore
/// toapi_client::generate!("../users-server/openapi.json", derive(Hash, Eq));
/// ```
#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    let Input { path, derives } = syn::parse_macro_input!(input as Input);
    let options = derives
        .into_iter()
        .fold(toapi_codegen::Options::new(), toapi_codegen::Options::derive);
    match expand(&path, &options) {
        Ok(tokens) => tokens.into(),
        Err(message) => syn::Error::new(path.span(), message)
            .to_compile_error()
            .into(),
    }
}

/// `"path/to/openapi.json"` with an optional `, derive(A, b::C)`.
struct Input {
    path: LitStr,
    derives: Vec<Path>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;
        let mut derives = Vec::new();
        if input.parse::<Option<Token![,]>>()?.is_some() && !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword != "derive" {
                return Err(syn::Error::new(keyword.span(), "expected `derive(..)`"));
            }
            let content;
            syn::parenthesized!(content in input);
            derives = Punctuated::<Path, Token![,]>::parse_terminated(&content)?
                .into_iter()
                .collect();
            input.parse::<Option<Token![,]>>()?;
        }
        Ok(Input { path, derives })
    }
}

fn expand(
    path: &LitStr,
    options: &toapi_codegen::Options,
) -> Result<proc_macro2::TokenStream, String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR is not set; build with cargo".to_string())?;
    let full_path = PathBuf::from(manifest_dir).join(path.value());
    let spec = std::fs::read_to_string(&full_path)
        .map_err(|e| format!("could not read {}: {e}", full_path.display()))?;
    let spec: serde_json::Value = serde_json::from_str(&spec)
        .map_err(|e| format!("{} is not valid JSON: {e}", full_path.display()))?;
    let code = toapi_codegen::generate_tokens_with(&spec, options).map_err(|e| e.to_string())?;

    // makes cargo rebuild the crate when the document changes
    let tracked = LitStr::new(&full_path.to_string_lossy(), path.span());
    Ok(quote! {
        const _: &[u8] = include_bytes!(#tracked);
        #code
    })
}
