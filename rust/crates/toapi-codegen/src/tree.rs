//! The path tree (`api.users.id(123)`) and the call builders
//! (`.get().limit(10)`), generated on top of the request structs.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use toapi_core::Segment;

use crate::{Error, Generator, error, field_ident};

/// A field of a generated struct.
#[derive(Clone)]
pub(crate) struct Field {
    pub ident: Ident,
    /// The field's type without `Option<..>`.
    pub inner: TokenStream,
    /// Whether the field is an `Option<inner>`.
    pub optional: bool,
}

pub(crate) enum Body {
    /// An object body: one setter per field.
    Fields { ty: Ident, fields: Vec<Field> },
    /// Any other body: one `.body(..)` setter.
    Whole { ty: Ident },
}

pub(crate) struct Operation {
    pub path: String,
    pub method: String,
    /// The base name, e.g. `GetUsersById`.
    pub name: String,
    /// Parameter name, field and type of every path parameter, in order.
    pub path_params: Vec<(String, Ident, TokenStream)>,
    /// The query struct and its fields.
    pub query: Option<(Ident, Vec<Field>)>,
    pub body: Option<Body>,
}

#[derive(Default)]
struct Node {
    statics: BTreeMap<String, Node>,
    /// The parameter name, its type, and the subtree.
    param: Option<(String, TokenStream, Box<Node>)>,
    /// Indices into the operations.
    operations: Vec<usize>,
}

/// Methods generated on every node with operations; parameter methods must
/// not be named like them.
const HTTP_METHODS: [&str; 5] = ["get", "post", "put", "patch", "delete"];
/// Methods of the generated `Api`.
const API_METHODS: [&str; 3] = ["new", "from_client", "client"];
/// Methods of the call builders; setters are renamed to avoid them.
const CALL_METHODS: [&str; 3] = ["watch", "into_request", "into_future"];

/// A `String` argument takes `impl Into<String>`, so `"alice"` works.
/// Other types are taken as they are, which keeps integer literals inferable.
fn argument(ty: &TokenStream) -> (TokenStream, TokenStream) {
    if ty.to_string() == "String" {
        (quote!(impl ::core::convert::Into<String>), quote!(.into()))
    } else {
        (ty.clone(), quote!())
    }
}

fn join(prefix: &str, segment: &str) -> String {
    format!("{}/{segment}", prefix.trim_end_matches('/'))
}

impl Generator {
    pub(crate) fn tree(&mut self) -> Result<(), Error> {
        let mut root = Node::default();
        for (index, operation) in self.operations.iter().enumerate() {
            let segments = toapi_core::parse_path(&operation.path).map_err(error)?;
            let mut node = &mut root;
            let mut prefix = String::from("/");
            for segment in segments {
                match segment {
                    Segment::Static(segment) => {
                        prefix = join(&prefix, &segment);
                        node = node.statics.entry(segment).or_default();
                    }
                    Segment::Param { name, .. } => {
                        let ty = operation
                            .path_params
                            .iter()
                            .find(|(param, _, _)| *param == name)
                            .map(|(_, _, ty)| ty.clone())
                            .unwrap_or_else(|| quote!(String));
                        let (existing, existing_ty, _) = node
                            .param
                            .get_or_insert_with(|| (name.clone(), ty.clone(), Box::default()));
                        if *existing != name {
                            return Err(error(format!(
                                "`{}` and `{}` use different parameter names at the same position",
                                join(&prefix, &format!("{{{existing}}}")),
                                join(&prefix, &format!("{{{name}}}")),
                            )));
                        }
                        if existing_ty.to_string() != ty.to_string() {
                            return Err(error(format!(
                                "parameter `{{{name}}}` after `{prefix}` has different types in different operations"
                            )));
                        }
                        prefix = join(&prefix, &format!("{{{name}}}"));
                        node = &mut node.param.as_mut().unwrap().2;
                    }
                }
            }
            node.operations.push(index);
        }

        self.node(&root, "/", &[])?;
        for index in 0..self.operations.len() {
            self.call(index)?;
        }

        let markers: BTreeSet<String> = self
            .operations
            .iter()
            .flat_map(required_setters)
            .map(|setter| setter.to_string())
            .collect();
        let markers = markers.iter().map(|m| {
            let ident: Ident = syn::parse_str(m).unwrap();
            quote!(pub struct #ident;)
        });
        self.items.push(quote! {
            /// Marker types naming the required fields in the call builders'
            /// typestate, e.g. `Unset<required::name>`.
            #[allow(non_camel_case_types)]
            pub mod required {
                #(#markers)*
            }
        });
        Ok(())
    }

    /// Emits the struct of one tree node and, recursively, of its children.
    ///
    /// `params` are the path parameters collected on the way here, as the
    /// node's private fields and their types.
    fn node(
        &mut self,
        node: &Node,
        path: &str,
        params: &[(Ident, TokenStream)],
    ) -> Result<(), Error> {
        let name = toapi_core::route_type_name(path).map_err(error)?;
        let ident = format_ident!("{}", name);
        let is_root = name == "Api";

        let param_fields: Vec<&Ident> = params.iter().map(|(field, _)| field).collect();
        let param_types: Vec<&TokenStream> = params.iter().map(|(_, ty)| ty).collect();
        let cloned_params: Vec<TokenStream> = param_fields
            .iter()
            .map(|field| quote!(self.#field.clone()))
            .collect();

        // static segments: public fields holding the child nodes
        let mut static_fields = Vec::new();
        let mut static_inits = Vec::new();
        let mut seen = BTreeMap::new();
        for (segment, child) in &node.statics {
            let child_path = join(path, segment);
            let field = field_ident(segment);
            if let Some(other) = seen.insert(field.to_string(), child_path.clone()) {
                return Err(error(format!(
                    "`{other}` and `{child_path}` both map to the field `{field}`"
                )));
            }
            let child_ident = format_ident!(
                "{}",
                toapi_core::route_type_name(&child_path).map_err(error)?
            );
            static_fields.push(quote!(pub #field: #child_ident));
            static_inits.push(
                quote!(#field: #child_ident::__new(__client.clone(), #(#param_fields.clone()),*)),
            );
            self.node(child, &child_path, params)?;
        }

        let mut methods = Vec::new();

        // a parameter segment: a method returning the child node
        if let Some((param, ty, child)) = &node.param {
            let child_path = join(path, &format!("{{{param}}}"));
            let method = field_ident(param);
            let method_name = method.to_string();
            if HTTP_METHODS.contains(&method_name.as_str())
                || (is_root && API_METHODS.contains(&method_name.as_str()))
            {
                return Err(error(format!(
                    "the parameter of `{child_path}` can't be named `{param}`: it would clash with the `{method_name}` method"
                )));
            }
            let child_ident = format_ident!(
                "{}",
                toapi_core::route_type_name(&child_path).map_err(error)?
            );
            let (argument_ty, conversion) = argument(ty);
            let doc = format!(" `{child_path}`");
            methods.push(quote! {
                #[doc = #doc]
                pub fn #method(&self, #method: #argument_ty) -> #child_ident {
                    #child_ident::__new(self.__client.clone(), #(#cloned_params,)* #method #conversion)
                }
            });
            let mut child_params = params.to_vec();
            child_params.push((
                format_ident!("__{}", toapi_core::field_name(param)),
                ty.clone(),
            ));
            self.node(child, &child_path, &child_params)?;
        }

        // operations: a method per HTTP method returning the call builder
        for &index in &node.operations {
            let operation = &self.operations[index];
            let method = format_ident!("{}", operation.method);
            let call = format_ident!("{}", toapi_core::call_type_name(&operation.name));
            // spelled out (rather than defaulted) so compile errors show every
            // required field that is still unset
            let unset = required_setters(operation)
                .into_iter()
                .map(|marker| quote!(::toapi_client::state::Unset<required::#marker>));
            let doc = format!(" `{} {}`", operation.method.to_uppercase(), operation.path);
            methods.push(quote! {
                #[doc = #doc]
                pub fn #method(&self) -> #call<#(#unset),*> {
                    #call::__new(self.__client.clone(), #(#cloned_params),*)
                }
            });
        }

        let api_methods = is_root.then(|| {
            quote! {
                /// A client for the API at `base_url`, e.g. `http://localhost:3000/api`.
                ///
                /// Must be called within a tokio runtime, see `toapi_client::Client::new`.
                pub fn new(base_url: impl ::core::convert::Into<String>) -> Self {
                    Self::from_client(::toapi_client::Client::new(base_url))
                }

                pub fn from_client(client: ::toapi_client::Client) -> Self {
                    Self::__new(client)
                }

                /// The underlying client, e.g. to run request structs or to
                /// invalidate tags.
                pub fn client(&self) -> &::toapi_client::Client {
                    &self.__client
                }
            }
        });
        let doc = if is_root {
            " The API's path tree. Static path segments are fields, parameters are methods."
                .to_string()
        } else {
            format!(" `{path}`")
        };

        self.define(
            &name,
            path,
            quote! {
                #[doc = #doc]
                #[derive(Clone)]
                pub struct #ident {
                    __client: ::toapi_client::Client,
                    #(#param_fields: #param_types,)*
                    #(#static_fields,)*
                }

                // path parameters are cloned into child nodes and calls without
                // knowing whether their type is `Copy`
                #[allow(clippy::clone_on_copy)]
                impl #ident {
                    fn __new(__client: ::toapi_client::Client, #(#param_fields: #param_types),*) -> Self {
                        Self {
                            #(#static_inits,)*
                            #(#param_fields,)*
                            __client,
                        }
                    }

                    #api_methods
                    #(#methods)*
                }
            },
        )
    }

    /// Emits the call builder of one operation.
    fn call(&mut self, index: usize) -> Result<(), Error> {
        let operation = &self.operations[index];
        let call_name = toapi_core::call_type_name(&operation.name);
        let call = format_ident!("{}", call_name);
        let request = format_ident!("{}", toapi_core::request_type_name(&operation.name));

        let path_fields: Vec<&Ident> = operation
            .path_params
            .iter()
            .map(|(_, field, _)| field)
            .collect();
        let path_stores: Vec<Ident> = path_fields
            .iter()
            .map(|field| format_ident!("path_{}", field.to_string().trim_start_matches("r#")))
            .collect();
        let path_types: Vec<&TokenStream> =
            operation.path_params.iter().map(|(_, _, ty)| ty).collect();

        let setters = setters(operation);
        let stores: Vec<&Ident> = setters.iter().map(|s| &s.store).collect();
        let store_types: Vec<&TokenStream> = setters.iter().map(|s| &s.inner).collect();

        // one type parameter per required setter
        let required: Vec<&Setter> = setters.iter().filter(|s| s.required).collect();
        let generics: Vec<Ident> = (0..required.len()).map(|i| format_ident!("S{i}")).collect();
        let unset: Vec<TokenStream> = required
            .iter()
            .map(|s| {
                let marker = &s.name;
                quote!(::toapi_client::state::Unset<required::#marker>)
            })
            .collect();

        let mut setter_impls = Vec::new();
        for setter in &setters {
            let name = &setter.name;
            let store = &setter.store;
            let (argument_ty, conversion) = argument(&setter.inner);
            match required.iter().position(|r| r.store == setter.store) {
                Some(position) => {
                    let impl_generics: Vec<&Ident> = generics
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != position)
                        .map(|(_, g)| g)
                        .collect();
                    let before: Vec<TokenStream> = generics
                        .iter()
                        .enumerate()
                        .map(|(i, g)| {
                            if i == position {
                                unset[i].clone()
                            } else {
                                quote!(#g)
                            }
                        })
                        .collect();
                    let after: Vec<TokenStream> = generics
                        .iter()
                        .enumerate()
                        .map(|(i, g)| {
                            if i == position {
                                quote!(::toapi_client::state::Set)
                            } else {
                                quote!(#g)
                            }
                        })
                        .collect();
                    let others: Vec<&&Ident> = stores.iter().filter(|s| ***s != *store).collect();
                    setter_impls.push(quote! {
                        impl<#(#impl_generics),*> #call<#(#before),*> {
                            pub fn #name(self, value: #argument_ty) -> #call<#(#after),*> {
                                #call {
                                    __client: self.__client,
                                    #(#path_stores: self.#path_stores,)*
                                    #(#others: self.#others,)*
                                    #store: Some(value #conversion),
                                    __state: ::core::marker::PhantomData,
                                }
                            }
                        }
                    });
                }
                None => setter_impls.push(quote! {
                    impl<#(#generics),*> #call<#(#generics),*> {
                        pub fn #name(mut self, value: #argument_ty) -> Self {
                            self.#store = Some(value #conversion);
                            self
                        }
                    }
                }),
            }
        }

        // `.query(..)` and `.body(..)` take a whole query or body struct and
        // set all of its fields at once
        let mut whole_setter = |method: &str, ty: &Ident, members: Vec<(&Setter, &Ident)>| {
            let method = format_ident!("{}", method);
            let fields: Vec<&Ident> = members.iter().map(|(_, field)| *field).collect();
            let bindings: Vec<Ident> = members
                .iter()
                .map(|(setter, _)| format_ident!("__{}", setter.store))
                .collect();
            let inits = members.iter().zip(&bindings).map(|((setter, _), binding)| {
                let store = &setter.store;
                if setter.required {
                    quote!(#store: Some(#binding))
                } else {
                    quote!(#store: #binding)
                }
            });
            let others = stores
                .iter()
                .filter(|store| !members.iter().any(|(setter, _)| setter.store == ***store));
            let after = required.iter().zip(&generics).map(|(setter, g)| {
                if members
                    .iter()
                    .any(|(member, _)| member.store == setter.store)
                {
                    quote!(::toapi_client::state::Set)
                } else {
                    quote!(#g)
                }
            });
            setter_impls.push(quote! {
                impl<#(#generics),*> #call<#(#generics),*> {
                    pub fn #method(self, value: #ty) -> #call<#(#after),*> {
                        let #ty { #(#fields: #bindings),* } = value;
                        #call {
                            __client: self.__client,
                            #(#path_stores: self.#path_stores,)*
                            #(#others: self.#others,)*
                            #(#inits,)*
                            __state: ::core::marker::PhantomData,
                        }
                    }
                }
            });
        };
        if let Some((ty, _)) = &operation.query {
            let members = setters
                .iter()
                .filter_map(|s| s.query_field.as_ref().map(|field| (s, field)))
                .collect();
            whole_setter("query", ty, members);
        }
        if let Some(Body::Fields { ty, .. }) = &operation.body {
            let members = setters
                .iter()
                .filter_map(|s| s.body_field.as_ref().map(|field| (s, field)))
                .collect();
            whole_setter("body", ty, members);
        }

        // building the request struct; required stores are `Some` per the typestate
        let value = |setter: &Setter| {
            let store = &setter.store;
            if setter.required {
                quote!(self.#store.unwrap())
            } else {
                quote!(self.#store)
            }
        };
        let query_init = operation.query.as_ref().map(|(ty, fields)| {
            let inits = fields.iter().map(|field| {
                let ident = &field.ident;
                let setter = setters
                    .iter()
                    .find(|s| s.query_field.as_ref() == Some(ident))
                    .unwrap();
                let value = value(setter);
                quote!(#ident: #value)
            });
            quote!(query: #ty { #(#inits,)* },)
        });
        let body_init = operation.body.as_ref().map(|body| match body {
            Body::Fields { ty, fields } => {
                let inits = fields.iter().map(|field| {
                    let ident = &field.ident;
                    let setter = setters
                        .iter()
                        .find(|s| s.body_field.as_ref() == Some(ident))
                        .unwrap();
                    let value = value(setter);
                    quote!(#ident: #value)
                });
                quote!(body: #ty { #(#inits,)* },)
            }
            Body::Whole { .. } => quote!(body: self.body.unwrap(),),
        });

        let bounds = quote!(#(#generics: ::toapi_client::state::IsSet),*);
        let call_ty = if toapi_core::is_query_method(&operation.method) {
            quote!(::toapi_client::QueryCall<#request>)
        } else {
            quote!(::toapi_client::MutationCall<#request>)
        };
        let watch = toapi_core::is_query_method(&operation.method).then(|| {
            quote! {
                /// A stream of responses: the current one, and a new one every
                /// time the response is invalidated.
                pub fn watch(
                    self,
                ) -> impl ::toapi_client::__private::Stream<
                    Item = Result<
                        ::std::sync::Arc<<#request as ::toapi_client::Request>::Response>,
                        ::toapi_client::Error,
                    >,
                > + ::toapi_client::__private::MaybeSend + 'static {
                    let client = self.__client.clone();
                    client.call(self.into_request()).watch()
                }
            }
        });

        let doc = format!(
            " Builder for `{} {}`. Await it to send the request.",
            operation.method.to_uppercase(),
            operation.path
        );
        let origin = format!(
            "{} {} call",
            operation.method.to_uppercase(),
            operation.path
        );
        self.define(
            &call_name,
            &origin,
            quote! {
                #[doc = #doc]
                #[must_use = "the request is only sent once the call is awaited"]
                pub struct #call<#(#generics),*> {
                    __client: ::toapi_client::Client,
                    #(#path_stores: #path_types,)*
                    #(#stores: Option<#store_types>,)*
                    __state: ::core::marker::PhantomData<fn() -> (#(#generics,)*)>,
                }

                impl #call<#(#unset),*> {
                    fn __new(__client: ::toapi_client::Client, #(#path_stores: #path_types),*) -> Self {
                        Self {
                            __client,
                            #(#path_stores,)*
                            #(#stores: None,)*
                            __state: ::core::marker::PhantomData,
                        }
                    }
                }

                #(#setter_impls)*

                impl<#bounds> #call<#(#generics),*> {
                    /// The request this call sends.
                    pub fn into_request(self) -> #request {
                        #request {
                            #(#path_fields: self.#path_stores,)*
                            #query_init
                            #body_init
                        }
                    }

                    #watch
                }

                impl<#bounds> ::core::future::IntoFuture for #call<#(#generics),*> {
                    type Output = <#call_ty as ::core::future::IntoFuture>::Output;
                    type IntoFuture = <#call_ty as ::core::future::IntoFuture>::IntoFuture;

                    fn into_future(self) -> Self::IntoFuture {
                        let client = self.__client.clone();
                        ::core::future::IntoFuture::into_future(client.call(self.into_request()))
                    }
                }
            },
        )
    }
}

/// A setter of a call builder, for a query parameter, a body field or the
/// whole body.
struct Setter {
    /// The setter method, which is also the name of its `required` marker.
    name: Ident,
    /// The builder's private field.
    store: Ident,
    /// The value type (without `Option`).
    inner: TokenStream,
    required: bool,
    query_field: Option<Ident>,
    body_field: Option<Ident>,
}

fn setters(operation: &Operation) -> Vec<Setter> {
    let mut setters = Vec::new();
    let mut taken: BTreeSet<String> = CALL_METHODS.iter().map(|m| m.to_string()).collect();
    // the whole-struct setters, see `Generator::call`
    if operation.query.is_some() {
        taken.insert("query".into());
    }
    if let Some(Body::Fields { .. }) = &operation.body {
        taken.insert("body".into());
    }
    if let Some((_, fields)) = &operation.query {
        for field in fields {
            let plain = field.ident.to_string();
            setters.push(Setter {
                name: name(&field.ident, "query", &mut taken),
                store: format_ident!("query_{}", plain.trim_start_matches("r#")),
                inner: field.inner.clone(),
                required: !field.optional,
                query_field: Some(field.ident.clone()),
                body_field: None,
            });
        }
    }
    match &operation.body {
        Some(Body::Fields { fields, .. }) => {
            for field in fields {
                let plain = field.ident.to_string();
                setters.push(Setter {
                    name: name(&field.ident, "body", &mut taken),
                    store: format_ident!("body_{}", plain.trim_start_matches("r#")),
                    inner: field.inner.clone(),
                    required: !field.optional,
                    query_field: None,
                    body_field: Some(field.ident.clone()),
                });
            }
        }
        Some(Body::Whole { ty }) => setters.push(Setter {
            name: name(&format_ident!("body"), "request", &mut taken),
            store: format_ident!("body"),
            inner: quote!(#ty),
            required: true,
            query_field: None,
            body_field: None,
        }),
        None => {}
    }
    setters
}

/// The setter name for a field: the field's own name, or `{prefix}_{name}`
/// when that is taken by a builder method or another setter.
fn name(field: &Ident, prefix: &str, taken: &mut BTreeSet<String>) -> Ident {
    let plain = field.to_string();
    let plain = plain.trim_start_matches("r#");
    if taken.insert(plain.to_string()) {
        field.clone()
    } else {
        let prefixed = format!("{prefix}_{plain}");
        taken.insert(prefixed.clone());
        format_ident!("{}", prefixed)
    }
}

fn required_setters(operation: &Operation) -> Vec<Ident> {
    setters(operation)
        .into_iter()
        .filter(|s| s.required)
        .map(|s| s.name)
        .collect()
}
