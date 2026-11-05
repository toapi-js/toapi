//! The `generate!` macro, re-exported by `toapi-client`.

use std::path::PathBuf;

use proc_macro::TokenStream;
use quote::quote;
use syn::LitStr;

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
#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    let path = syn::parse_macro_input!(input as LitStr);
    match expand(&path) {
        Ok(tokens) => tokens.into(),
        Err(message) => syn::Error::new(path.span(), message)
            .to_compile_error()
            .into(),
    }
}

fn expand(path: &LitStr) -> Result<proc_macro2::TokenStream, String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR is not set; build with cargo".to_string())?;
    let full_path = PathBuf::from(manifest_dir).join(path.value());
    let spec = std::fs::read_to_string(&full_path)
        .map_err(|e| format!("could not read {}: {e}", full_path.display()))?;
    let spec: serde_json::Value = serde_json::from_str(&spec)
        .map_err(|e| format!("{} is not valid JSON: {e}", full_path.display()))?;
    let code = toapi_codegen::generate_tokens(&spec).map_err(|e| e.to_string())?;

    // makes cargo rebuild the crate when the document changes
    let tracked = LitStr::new(&full_path.to_string_lossy(), path.span());
    Ok(quote! {
        const _: &[u8] = include_bytes!(#tracked);
        #code
    })
}
