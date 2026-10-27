use std::{any::TypeId, collections::BTreeMap};

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, Method, Uri},
};
use schemars::{JsonSchema, Schema, SchemaGenerator, generate::SchemaSettings};
use serde_json::{Map, Value, json};

use crate::cache::RequestCache;

/// Schema generators used while building the OpenAPI document.
pub struct Generators {
    /// For request and response bodies. Named types become
    /// `#/components/schemas/..` references.
    pub components: SchemaGenerator,
    /// For path and query parameters, which are always inlined.
    pub inline: SchemaGenerator,
}

impl Generators {
    pub(crate) fn new() -> Self {
        Generators {
            components: SchemaSettings::draft2020_12()
                .with(|s| s.definitions_path = "/components/schemas/".into())
                .into_generator(),
            inline: SchemaSettings::draft2020_12()
                .with(|s| s.inline_subschemas = true)
                .into_generator(),
        }
    }
}

/// What the extractors and the return type of a handler contribute to its
/// OpenAPI operation.
#[derive(Debug, Default, Clone)]
pub struct OperationDoc {
    pub path: Option<Schema>,
    pub query: Option<Schema>,
    pub body: Option<Schema>,
    pub response: Option<Schema>,
    /// Additional responses keyed by status code, e.g. `"401"`.
    pub responses: BTreeMap<String, Value>,
    /// The type an [`Auth`](crate::Auth) extractor expects, checked against
    /// the route's `authorize` when the route is added.
    pub auth: Option<(TypeId, &'static str)>,
}

/// Implemented by every extractor a handler takes.
///
/// The provided implementations cover axum's `Path`, `Query`, `Json`,
/// `State`, `Extension`, `HeaderMap`, `Method`, `Uri` and [`RequestCache`].
/// Custom extractors such as an auth session implement it too, usually with
/// the default (empty) body or by adding a `401` response:
///
/// ```ignore
/// impl OperationInput for Session {
///     fn document(op: &mut OperationDoc, _: &mut Generators) {
///         op.responses.insert("401".into(), json!({ "description": "Unauthorized" }));
///     }
/// }
/// ```
pub trait OperationInput {
    fn document(op: &mut OperationDoc, generators: &mut Generators) {
        let _ = (op, generators);
    }
}

impl<T: JsonSchema> OperationInput for Path<T> {
    fn document(op: &mut OperationDoc, generators: &mut Generators) {
        op.path = Some(generators.inline.subschema_for::<T>());
    }
}

impl<T: JsonSchema> OperationInput for Query<T> {
    fn document(op: &mut OperationDoc, generators: &mut Generators) {
        op.query = Some(generators.inline.subschema_for::<T>());
    }
}

impl<T: JsonSchema> OperationInput for Json<T> {
    fn document(op: &mut OperationDoc, generators: &mut Generators) {
        op.body = Some(generators.components.subschema_for::<T>());
    }
}

impl<S> OperationInput for State<S> {}
impl<T> OperationInput for Extension<T> {}
impl OperationInput for HeaderMap {}
impl OperationInput for Method {}
impl OperationInput for Uri {}
impl OperationInput for RequestCache {}

/// The parameter names a `Path<T>` schema provides, matched against the
/// placeholders of the route template.
///
/// - a struct provides its field names
/// - a tuple provides one value per placeholder, in order
/// - a single value provides the one placeholder
pub(crate) fn path_parameters(schema: &Schema, template_params: &[String]) -> Vec<(String, Value)> {
    let value = schema.as_value();
    if let Some(properties) = value.get("properties").and_then(Value::as_object) {
        return properties
            .iter()
            .map(|(name, schema)| (name.clone(), schema.clone()))
            .collect();
    }
    if let Some(items) = value.get("prefixItems").and_then(Value::as_array) {
        return template_params
            .iter()
            .cloned()
            .zip(items.iter().cloned())
            .collect();
    }
    template_params
        .iter()
        .take(1)
        .map(|name| (name.clone(), value.clone()))
        .collect()
}

pub(crate) struct DocumentedOperation {
    pub path: String,
    pub method: Method,
    pub doc: OperationDoc,
}

pub(crate) fn build_document(
    title: &str,
    version: &str,
    operations: Vec<DocumentedOperation>,
    generators: &mut Generators,
) -> Value {
    let mut paths = Map::new();
    for DocumentedOperation { path, method, doc } in operations {
        let template_params = toapi_core::path_params(&path).expect("route was validated");
        let mut parameters = Vec::new();
        if let Some(schema) = &doc.path {
            for (name, schema) in path_parameters(schema, &template_params) {
                parameters.push(json!({
                    "name": name,
                    "in": "path",
                    "required": true,
                    "schema": schema,
                }));
            }
        }
        if let Some(schema) = &doc.query {
            let value = schema.as_value();
            let required: Vec<&str> = value
                .get("required")
                .and_then(Value::as_array)
                .map(|r| r.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            if let Some(properties) = value.get("properties").and_then(Value::as_object) {
                for (name, schema) in properties {
                    parameters.push(json!({
                        "name": name,
                        "in": "query",
                        "required": required.contains(&name.as_str()),
                        "schema": schema,
                    }));
                }
            }
        }

        let mut operation = Map::new();
        if !parameters.is_empty() {
            operation.insert("parameters".into(), Value::Array(parameters));
        }
        if let Some(schema) = &doc.body {
            operation.insert(
                "requestBody".into(),
                json!({
                    "required": true,
                    "content": { "application/json": { "schema": schema } },
                }),
            );
        }
        let mut responses: Map<String, Value> = doc.responses.into_iter().collect();
        let mut ok = json!({ "description": "OK" });
        if let Some(schema) = &doc.response {
            ok["content"] = json!({ "application/json": { "schema": schema } });
        }
        responses.insert("200".into(), ok);
        operation.insert("responses".into(), Value::Object(responses));

        paths
            .entry(path)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .unwrap()
            .insert(
                method.as_str().to_ascii_lowercase(),
                Value::Object(operation),
            );
    }

    let schemas = generators.components.take_definitions(true);
    let mut document = json!({
        "openapi": "3.1.0",
        "info": { "title": title, "version": version },
        "paths": paths,
    });
    if !schemas.is_empty() {
        document["components"] = json!({ "schemas": schemas });
    }
    document
}
