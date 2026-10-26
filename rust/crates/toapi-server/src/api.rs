use std::{future::Future, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::Request,
    handler::Handler,
    http::{Method, request::Parts},
    middleware,
    routing::{self, MethodRouter},
};
use toapi_core::{INVALIDATIONS_ROUTE, OPENAPI_ROUTE};

use crate::{
    cache::{Cache, PubSub},
    handler::{DocumentedHandler, MutationKind, QueryKind},
    invalidation::{FilterFactory, StreamConfig, TagFilter, invalidation_stream, tag_middleware},
    openapi::{DocumentedOperation, Generators, OperationDoc, build_document, path_parameters},
};

type DocFn = fn(&mut Generators) -> OperationDoc;

/// The handlers of one route, built with [`get`], [`post`], [`put`],
/// [`patch`] and [`delete`] and chained like axum's `MethodRouter`:
/// `get(list_users).post(create_user)`.
pub struct MethodRoutes<S> {
    router: MethodRouter<S>,
    operations: Vec<(Method, DocFn)>,
}

macro_rules! method_fns {
    ($($name:ident => $method:ident, $kind:ty;)*) => {
        $(
            #[doc = concat!("Routes `", stringify!($method), "` requests to `handler`.")]
            pub fn $name<H, T, A, S>(handler: H) -> MethodRoutes<S>
            where
                H: Handler<T, S> + DocumentedHandler<A, $kind>,
                T: 'static,
                S: Clone + Send + Sync + 'static,
            {
                MethodRoutes::new().$name(handler)
            }
        )*

        impl<S: Clone + Send + Sync + 'static> MethodRoutes<S> {
            fn new() -> Self {
                MethodRoutes {
                    router: MethodRouter::new(),
                    operations: Vec::new(),
                }
            }

            $(
                #[doc = concat!("Routes `", stringify!($method), "` requests to `handler`.")]
                pub fn $name<H, T, A>(mut self, handler: H) -> Self
                where
                    H: Handler<T, S> + DocumentedHandler<A, $kind>,
                    T: 'static,
                {
                    assert!(
                        !self.operations.iter().any(|(m, _)| *m == Method::$method),
                        concat!("route already has a ", stringify!($method), " handler"),
                    );
                    self.router = self.router.$name(handler);
                    self.operations
                        .push((Method::$method, <H as DocumentedHandler<A, $kind>>::document));
                    self
                }
            )*
        }
    };
}

method_fns! {
    get => GET, QueryKind;
    post => POST, MutationKind;
    put => PUT, MutationKind;
    patch => PATCH, MutationKind;
    delete => DELETE, MutationKind;
}

struct RouteEntry<S> {
    path: String,
    routes: MethodRoutes<S>,
}

/// The API definition: routes plus the invalidation stream configuration.
///
/// It turns into an axum router with [`Api::into_router`] and into an
/// OpenAPI document with [`Api::openapi`]. Both come from the same handler
/// signatures, so the document can't drift from the implementation.
pub struct Api<S> {
    title: String,
    version: String,
    routes: Vec<RouteEntry<S>>,
    cache: Arc<dyn Cache>,
    throttle: Duration,
    keepalive: Duration,
    filter: Option<FilterFactory>,
    hash_secret: Option<String>,
}

impl<S: Clone + Send + Sync + 'static> Api<S> {
    /// An API without routes. `title` and `version` describe it in the
    /// OpenAPI document.
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Api {
            title: title.into(),
            version: version.into(),
            routes: Vec::new(),
            cache: Arc::new(PubSub::new()),
            throttle: Duration::from_millis(500),
            keepalive: Duration::from_secs(10),
            filter: None,
            hash_secret: None,
        }
    }

    /// Adds a route. `path` uses axum's syntax (`/users/{id}`, `/files/{*rest}`),
    /// which is also the path in the OpenAPI document.
    ///
    /// # Panics
    ///
    /// If the path is invalid, already registered, or its placeholders don't
    /// match the handlers' `Path<T>` extractors.
    pub fn route(mut self, path: &str, routes: MethodRoutes<S>) -> Self {
        let template_params = toapi_core::path_params(path).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            !self.routes.iter().any(|r| r.path == path),
            "route `{path}` is registered twice"
        );
        let mut generators = Generators::new();
        for (method, document) in &routes.operations {
            let doc = document(&mut generators);
            let mut provided: Vec<String> = doc
                .path
                .as_ref()
                .map(|schema| {
                    path_parameters(schema, &template_params)
                        .into_iter()
                        .map(|(name, _)| name)
                        .collect()
                })
                .unwrap_or_default();
            let mut expected = template_params.clone();
            provided.sort();
            expected.sort();
            assert!(
                provided == expected,
                "{method} {path}: the route has the parameters {expected:?} but the handler's Path extractor provides {provided:?}"
            );
        }
        self.routes.push(RouteEntry {
            path: path.to_string(),
            routes,
        });
        self
    }

    /// Replaces the default [`PubSub`] cache, e.g. with one backed by Redis.
    /// Keep a clone of the `Arc` to invalidate tags outside of requests,
    /// e.g. from a background job:
    ///
    /// ```ignore
    /// let cache = Arc::new(PubSub::new());
    /// let api = Api::new("Users", "1.0.0").cache(cache.clone());
    /// // later
    /// cache.invalidate(vec!["users".into()], None);
    /// ```
    pub fn cache<C: Cache>(mut self, cache: Arc<C>) -> Self {
        self.cache = cache;
        self
    }

    /// Hashes tags before they leave the server, so clients can match them
    /// without learning their content (`user:123:email`).
    pub fn hash_secret(mut self, secret: impl Into<String>) -> Self {
        self.hash_secret = Some(secret.into());
        self
    }

    /// How long the invalidation stream batches tags after sending a batch.
    /// Defaults to 500ms.
    pub fn throttle(mut self, throttle: Duration) -> Self {
        self.throttle = throttle;
        self
    }

    /// Interval of the empty keepalive lines. Defaults to 10s.
    pub fn keepalive(mut self, keepalive: Duration) -> Self {
        self.keepalive = keepalive;
        self
    }

    /// Decides per stream request which tags its client may see, e.g. only
    /// the tags of the signed in user.
    pub fn invalidation_filter<F, Fut, P>(mut self, filter: F) -> Self
    where
        F: Fn(Parts) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = P> + Send + 'static,
        P: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.filter = Some(Arc::new(move |parts| {
            let fut = filter(parts);
            Box::pin(async move { Arc::new(fut.await) as TagFilter })
        }));
        self
    }

    /// The OpenAPI 3.1 document of the API.
    pub fn openapi(&self) -> serde_json::Value {
        let mut generators = Generators::new();
        let mut operations = Vec::new();
        for entry in &self.routes {
            for (method, document) in &entry.routes.operations {
                operations.push(DocumentedOperation {
                    path: entry.path.clone(),
                    method: method.clone(),
                    doc: document(&mut generators),
                });
            }
        }
        build_document(&self.title, &self.version, operations, &mut generators)
    }

    /// The axum router serving the routes, the OpenAPI document at
    /// `/__tapi/openapi.json` and the invalidation stream at
    /// `/__tapi/invalidations`.
    pub fn into_router(self) -> Router<S> {
        let openapi = Arc::new(self.openapi());
        let config = Arc::new(StreamConfig {
            cache: self.cache,
            throttle: self.throttle,
            keepalive: self.keepalive,
            filter: self.filter,
            hash_secret: self.hash_secret,
        });

        let mut router = Router::new();
        for entry in self.routes {
            router = router.route(&entry.path, entry.routes.router);
        }
        let stream_config = config.clone();
        router
            .route(
                OPENAPI_ROUTE,
                routing::get(move || async move { Json(openapi.as_ref().clone()) }),
            )
            .route(
                INVALIDATIONS_ROUTE,
                routing::get(move |req: Request| invalidation_stream(stream_config.clone(), req)),
            )
            .layer(middleware::from_fn_with_state(config, tag_middleware))
    }
}
