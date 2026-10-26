//! Request macros for the Toapi client.
//!
//! ```ignore
//! client.query(get!("/users/{id}", 123));
//! client.query(get!("/users/{id}?include_posts={posts}", id = 123, posts = Some(true)));
//! client.query(get!("/users?limit&offset", limit = 10, offset = 0));
//! client.mutate(post!("/users/{id}/posts", id = 123, { title: "Hello", draft: false }));
//! ```
//!
//! The macros don't read the OpenAPI document. They derive the generated type
//! names from the method and the route template through `toapi-core` (the
//! same function the codegen uses) and expand to a struct literal:
//!
//! ```ignore
//! GetUsersById {
//!     id: into_field(123),
//!     query: GetUsersByIdQuery { include_posts: into_field(Some(true)) },
//! }
//! ```
//!
//! so the generated types must be in scope at the call site.
//!
//! Placeholder values are bound in this order:
//! 1. a named argument with the placeholder's name (`id = 123`)
//! 2. the next positional argument
//! 3. a local variable with the placeholder's name, as with `format!`
//!
//! `?key` is short for `?key={key}`.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Expr, Ident, LitStr, Token, braced,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

/// Builds a GET request. See the crate docs for the template syntax.
#[proc_macro]
pub fn get(input: TokenStream) -> TokenStream {
    expand("GET", input)
}

/// Builds a DELETE request. See the crate docs for the template syntax.
#[proc_macro]
pub fn delete(input: TokenStream) -> TokenStream {
    expand("DELETE", input)
}

/// Builds a POST request; the trailing `{ field: value, .. }` is the body
/// (left out for routes without a request body).
#[proc_macro]
pub fn post(input: TokenStream) -> TokenStream {
    expand("POST", input)
}

/// Builds a PUT request; the trailing `{ field: value, .. }` is the body.
#[proc_macro]
pub fn put(input: TokenStream) -> TokenStream {
    expand("PUT", input)
}

/// Builds a PATCH request; the trailing `{ field: value, .. }` is the body.
#[proc_macro]
pub fn patch(input: TokenStream) -> TokenStream {
    expand("PATCH", input)
}

fn expand(method: &str, input: TokenStream) -> TokenStream {
    let with_body = toapi_core::has_body(method);
    let parser = move |stream: ParseStream| Input::parse(stream, with_body);
    match syn::parse::Parser::parse(parser, input).and_then(|input| input.expand(method)) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct Input {
    template: LitStr,
    named: Vec<(Ident, Expr)>,
    positional: Vec<Expr>,
    body: Option<Punctuated<BodyField, Token![,]>>,
}

struct BodyField {
    name: Ident,
    value: Expr,
}

impl Parse for BodyField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;
        let value = input.parse()?;
        Ok(BodyField { name, value })
    }
}

impl Input {
    fn parse(input: ParseStream, with_body: bool) -> syn::Result<Self> {
        let template: LitStr = input.parse()?;
        let mut named = Vec::new();
        let mut positional = Vec::new();
        let mut body = None;

        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            if body.is_some() {
                return Err(input.error("the request body must be the last argument"));
            }
            if with_body && input.peek(syn::token::Brace) {
                let content;
                braced!(content in input);
                body = Some(content.parse_terminated(BodyField::parse, Token![,])?);
            } else if input.peek(Ident) && input.peek2(Token![=]) && !input.peek2(Token![==]) {
                let name: Ident = input.parse()?;
                input.parse::<Token![=]>()?;
                named.push((name, input.parse()?));
            } else {
                positional.push(input.parse()?);
            }
        }

        Ok(Input {
            template,
            named,
            positional,
            body,
        })
    }

    fn expand(self, method: &str) -> syn::Result<TokenStream2> {
        let span = self.template.span();
        let error = |message: String| syn::Error::new(span, message);
        let template = self.template.value();
        let (path, query) = match template.split_once('?') {
            Some((path, query)) => (path, Some(query)),
            None => (template.as_str(), None),
        };

        let type_name = toapi_core::type_name(method, path).map_err(error)?;

        // (field, placeholder) pairs of the path and the query, in template order
        let path_fields: Vec<(String, String)> = toapi_core::path_params(path)
            .map_err(error)?
            .into_iter()
            .map(|name| (name.clone(), name))
            .collect();
        let query_fields: Option<Vec<(String, String)>> = query
            .map(|query| {
                query
                    .split('&')
                    .filter(|pair| !pair.is_empty())
                    .map(|pair| match pair.split_once('=') {
                        None => Ok((pair.to_string(), pair.to_string())),
                        Some((key, value)) => value
                            .strip_prefix('{')
                            .and_then(|v| v.strip_suffix('}'))
                            .filter(|v| !v.is_empty())
                            .map(|v| (key.to_string(), v.to_string()))
                            .ok_or_else(|| {
                                error(format!(
                                    "query parameter `{pair}` must be `{key}` or `{key}={{placeholder}}`"
                                ))
                            }),
                    })
                    .collect::<syn::Result<_>>()
            })
            .transpose()?;

        let mut binder = Binder::new(self.named, self.positional, span)?;
        let path_inits = path_fields
            .iter()
            .map(|(field, placeholder)| binder.field_init(field, placeholder))
            .collect::<syn::Result<Vec<_>>>()?;
        let query_init = query_fields
            .map(|fields| {
                let inits = fields
                    .iter()
                    .map(|(field, placeholder)| binder.field_init(field, placeholder))
                    .collect::<syn::Result<Vec<_>>>()?;
                let query_type = Ident::new(&toapi_core::query_type_name(&type_name), span);
                syn::Result::Ok(quote_spanned!(span=> query: #query_type { #(#inits,)* },))
            })
            .transpose()?;
        binder.finish()?;

        let body_init = self.body.map(|fields| {
            let body_type = Ident::new(&toapi_core::request_body_type_name(&type_name), span);
            let inits = fields.into_iter().map(|BodyField { name, value }| {
                quote!(#name: ::toapi_client::__private::into_field(#value))
            });
            quote_spanned!(span=> body: #body_type { #(#inits,)* },)
        });

        let type_ident = Ident::new(&type_name, span);
        Ok(quote_spanned! {span=>
            #type_ident {
                #(#path_inits,)*
                #query_init
                #body_init
            }
        })
    }
}

/// Resolves placeholder values from named arguments, positional arguments and
/// local variables.
struct Binder {
    named: Vec<(Ident, Option<Expr>)>,
    positional: std::vec::IntoIter<Expr>,
    span: Span,
}

impl Binder {
    fn new(named: Vec<(Ident, Expr)>, positional: Vec<Expr>, span: Span) -> syn::Result<Self> {
        for (i, (name, _)) in named.iter().enumerate() {
            if named[..i].iter().any(|(other, _)| other == name) {
                return Err(syn::Error::new(
                    name.span(),
                    format!("`{name}` is bound twice"),
                ));
            }
        }
        Ok(Binder {
            named: named
                .into_iter()
                .map(|(name, expr)| (name, Some(expr)))
                .collect(),
            positional: positional.into_iter(),
            span,
        })
    }

    fn field_init(&mut self, field: &str, placeholder: &str) -> syn::Result<TokenStream2> {
        let field = field_ident(field, self.span);
        let value = self.value(placeholder)?;
        Ok(quote!(#field: ::toapi_client::__private::into_field(#value)))
    }

    fn value(&mut self, placeholder: &str) -> syn::Result<TokenStream2> {
        if let Some((_, expr)) = self.named.iter_mut().find(|(name, _)| name == placeholder) {
            let expr = expr.take().ok_or_else(|| {
                syn::Error::new(
                    self.span,
                    format!("placeholder `{placeholder}` is used twice"),
                )
            })?;
            return Ok(quote!(#expr));
        }
        if let Some(expr) = self.positional.next() {
            return Ok(quote!(#expr));
        }
        // capture a local variable, like `format!("{id}")`
        syn::parse_str::<Ident>(placeholder)
            .map(|_| {
                let ident = Ident::new(placeholder, self.span);
                quote!(#ident)
            })
            .map_err(|_| {
                syn::Error::new(
                    self.span,
                    format!("no value for placeholder `{placeholder}`: pass it as `{placeholder} = ..` or as a positional argument"),
                )
            })
    }

    fn finish(self) -> syn::Result<()> {
        if let Some((name, _)) = self.named.iter().find(|(_, expr)| expr.is_some()) {
            return Err(syn::Error::new(
                name.span(),
                format!("the route template has no placeholder `{name}`"),
            ));
        }
        if let Some(extra) = self.positional.as_slice().first() {
            return Err(syn::Error::new_spanned(
                extra,
                "too many positional arguments for the route template",
            ));
        }
        Ok(())
    }
}

fn field_ident(name: &str, span: Span) -> Ident {
    let field = toapi_core::field_name(name);
    if toapi_core::is_keyword(&field) {
        Ident::new_raw(&field, span)
    } else {
        format_ident!("{}", field, span = span)
    }
}
