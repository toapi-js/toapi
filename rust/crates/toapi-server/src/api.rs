use std::{
    any::{TypeId, type_name},
    future::Future,
    sync::Arc,
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{Request, State},
    handler::Handler,
    http::{Method, request::Parts},
    middleware,
    routing::{self, MethodRouter},
};
use toapi_core::{Cache, INVALIDATIONS_ROUTE, OPENAPI_ROUTE};

use crate::{
    auth::Authorize,
    cache::PubSub,
    handler::{DocumentedHandler, MutationKind, QueryKind},
    invalidation::{FilterFactory, StreamConfig, TagFilter, invalidation_stream, tag_middleware},
    openapi::{DocumentedOperation, Generators, OperationDoc, build_document, path_parameters},
    route::serve,
};

/// How long a tagged response without a TTL stays in the cache, like the
/// TypeScript server's `DEFAULT_TTL`.
const DEFAULT_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 14);

type DocFn = fn(&mut Generators) -> OperationDoc;

struct Operation {
    method: Method,
    document: DocFn,
    /// The type the route's `authorize` returns, for checking `Auth<A>`.
    auth: (TypeId, &'static str),
}

/// The OpenAPI operation of a route method: what `authorize` and the handler
/// contribute.
fn document<Au, AuArgs, H, A, K, S>(generators: &mut Generators) -> OperationDoc
where
    Au: Authorize<AuArgs, S>,
    H: DocumentedHandler<A, K>,
{
    let mut op = OperationDoc::default();
    Au::document(&mut op, generators);
    H::document(&mut op, generators);
    op
}

/// The handlers of one route, built with [`get`], [`post`], [`put`],
/// [`patch`] and [`delete`] and chained like axum's `MethodRouter`:
/// `get(public, list_users).post(signed_in, create_user)`.
///
/// Each takes the route's [`Authorize`] first, like the `authorize` of a
/// TypeScript handler, then the handler.
pub struct MethodRoutes<S> {
    router: MethodRouter<S>,
    operations: Vec<Operation>,
}

macro_rules! method_fns {
    ($($name:ident => $method:ident, $kind:ty, $query:literal;)*) => {
        $(
            #[doc = concat!("Routes `", stringify!($method), "` requests through `authorize` to `handler`.")]
            pub fn $name<Au, AuArgs, H, T, A, S>(authorize: Au, handler: H) -> MethodRoutes<S>
            where
                Au: Authorize<AuArgs, S>,
                AuArgs: 'static,
                H: Handler<T, S> + DocumentedHandler<A, $kind>,
                T: 'static,
                S: Clone + Send + Sync + 'static,
            {
                MethodRoutes::new().$name(authorize, handler)
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
                #[doc = concat!("Routes `", stringify!($method), "` requests through `authorize` to `handler`.")]
                pub fn $name<Au, AuArgs, H, T, A>(mut self, authorize: Au, handler: H) -> Self
                where
                    Au: Authorize<AuArgs, S>,
                    AuArgs: 'static,
                    H: Handler<T, S> + DocumentedHandler<A, $kind>,
                    T: 'static,
                {
                    assert!(
                        !self.operations.iter().any(|op| op.method == Method::$method),
                        concat!("route already has a ", stringify!($method), " handler"),
                    );
                    let route = move |State(state): State<S>, req: Request| {
                        serve::<Au, AuArgs, H, T, S>(authorize.clone(), handler.clone(), state, req, $query)
                    };
                    self.router = self.router.$name(route);
                    self.operations.push(Operation {
                        method: Method::$method,
                        document: document::<Au, AuArgs, H, A, $kind, S>,
                        auth: (TypeId::of::<Au::Auth>(), type_name::<Au::Auth>()),
                    });
                    self
                }
            )*
        }
    };
}

method_fns! {
    get => GET, QueryKind, true;
    post => POST, MutationKind, false;
    put => PUT, MutationKind, false;
    patch => PATCH, MutationKind, false;
    delete => DELETE, MutationKind, false;
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
    default_ttl: Duration,
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
            default_ttl: DEFAULT_TTL,
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
    /// If the path is invalid, already registered, its placeholders don't
    /// match the handlers' `Path<T>` extractors, or a handler's `Auth<A>`
    /// isn't what the route's `authorize` returns.
    pub fn route(mut self, path: &str, routes: MethodRoutes<S>) -> Self {
        let template_params = toapi_core::path_params(path).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            !self.routes.iter().any(|r| r.path == path),
            "route `{path}` is registered twice"
        );
        let mut generators = Generators::new();
        for Operation {
            method,
            document,
            auth,
        } in &routes.operations
        {
            let doc = document(&mut generators);
            if let Some((expected, name)) = doc.auth {
                assert!(
                    expected == auth.0,
                    "{method} {path}: the handler extracts Auth<{name}>, but its authorize returns {}",
                    auth.1
                );
            }
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

    /// Replaces the default [`PubSub`] cache, which caches nothing, e.g. with
    /// `toapi_cache::InMemoryCache` or one backed by Redis.
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

    /// How long the cache keeps a tagged response that has no TTL of its own.
    /// Defaults to 14 days, like the TypeScript server's `defaultTTL`.
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
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
            for operation in &entry.routes.operations {
                operations.push(DocumentedOperation {
                    path: entry.path.clone(),
                    method: operation.method.clone(),
                    doc: (operation.document)(&mut generators),
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
            default_ttl: self.default_ttl,
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
