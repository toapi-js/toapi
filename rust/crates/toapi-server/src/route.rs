use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    extract::{OriginalUri, Request},
    handler::Handler,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};

use crate::{
    auth::{Authorize, Authorized},
    cache::{Cache, CacheEntry, Lookup},
    response::{TagInfo, TagKind, now_ms},
};

/// What a route needs from the `Api`, put into the request extensions by the
/// tag middleware (routes are built before the `Api` they're added to).
#[derive(Clone)]
pub(crate) struct ServerContext {
    pub cache: Arc<dyn Cache>,
    pub default_ttl: Duration,
}

/// Serves one route method, like the TypeScript request handler:
///
/// 1. `authorize`; on `Err` its response is returned and nothing else runs
/// 2. for queries, the cache: a stored response is returned as is
/// 3. the handler
/// 4. for queries, a successful response that set tags or a TTL is stored
pub(crate) async fn serve<Au, AuArgs, H, T, S>(
    authorize: Au,
    handler: H,
    state: S,
    req: Request,
    query: bool,
) -> Response
where
    Au: Authorize<AuArgs, S>,
    H: Handler<T, S>,
    S: Clone + Send + Sync + 'static,
{
    let (mut parts, body) = req.into_parts();
    let Some(context) = parts.extensions.get::<ServerContext>().cloned() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "toapi route used outside of a toapi router",
        )
            .into_response();
    };

    let auth = match authorize.authorize(&mut parts, &state).await {
        Ok(auth) => auth,
        Err(res) => return res,
    };
    parts.extensions.insert(Authorized(auth));

    let lookup = if query {
        let key = cache_key(&parts);
        match context.cache.get(&key).await {
            Lookup::Hit(entry) => return entry.into_response(),
            Lookup::Miss(token) => Some((key, token)),
        }
    } else {
        None
    };

    let res = handler.call(Request::from_parts(parts, body), state).await;
    match lookup {
        Some((key, token)) => store(&context, key, token, res).await,
        None => res,
    }
}

/// The request's URL as the client sent it, including the query string and
/// the prefix of a router the API is nested in.
fn cache_key(parts: &Parts) -> String {
    let uri = parts
        .extensions
        .get::<OriginalUri>()
        .map_or(&parts.uri, |original| &original.0);
    format!("GET {uri}")
}

/// Stores a successful response that explicitly set tags or a TTL.
async fn store(context: &ServerContext, key: String, token: u64, res: Response) -> Response {
    let Some(info) = res
        .extensions()
        .get::<TagInfo>()
        .filter(|info| info.kind == TagKind::Cache)
        .filter(|info| !info.tags.is_empty() || info.expires_at.is_some())
        .cloned()
    else {
        return res;
    };
    if !res.status().is_success() {
        return res;
    }

    let (parts, body) = res.into_parts();
    let Ok(body) = axum::body::to_bytes(body, usize::MAX).await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to read the response body",
        )
            .into_response();
    };
    let ttl = match info.expires_at {
        Some(expires_at) => Duration::from_millis(expires_at.saturating_sub(now_ms())),
        None => context.default_ttl,
    };
    let entry = CacheEntry {
        body: body.clone(),
        tags: info.tags,
        expires_at: info.expires_at,
    };
    context.cache.set(key, entry, ttl, token).await;
    Response::from_parts(parts, Body::from(body))
}
