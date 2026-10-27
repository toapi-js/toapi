//! The example API. `api()` is the single definition: it serves requests and
//! produces `openapi.json`, from which `users-client` is generated.

use std::{
    collections::BTreeMap,
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use axum::{
    Json,
    extract::{FromRequestParts, Path, Query, State},
    http::{header, request::Parts},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use toapi_cache::InMemoryCache;
use toapi_server::{
    Api, Auth, Cached, Generators, HttpError, Invalidates, OperationDoc, OperationInput, get,
    public,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Member,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewUser {
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserPath {
    pub id: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListUsersQuery {
    /// Maximum number of users to return.
    pub limit: Option<u32>,
}

#[derive(Clone, Default)]
pub struct AppState {
    users: Arc<Mutex<BTreeMap<u64, User>>>,
    next_id: Arc<AtomicU64>,
}

/// The signed in user, from `Authorization: Bearer <name>`.
#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
}

impl<S: Send + Sync> FromRequestParts<S> for Session {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(|name| Session {
                name: name.to_string(),
            })
            .ok_or_else(HttpError::unauthorized)
    }
}

impl OperationInput for Session {
    fn document(op: &mut OperationDoc, _: &mut Generators) {
        op.responses
            .insert("401".into(), json!({ "description": "Unauthorized" }));
    }
}

async fn list_users(
    State(state): State<AppState>,
    Query(query): Query<ListUsersQuery>,
) -> Cached<Vec<User>> {
    let users = state.users.lock().unwrap();
    let limit = query.limit.map_or(usize::MAX, |limit| limit as usize);
    Cached::new(users.values().take(limit).cloned().collect()).tags(["users"])
}

async fn get_user(
    State(state): State<AppState>,
    Path(UserPath { id }): Path<UserPath>,
) -> Result<Cached<User>, HttpError> {
    let users = state.users.lock().unwrap();
    let user = users.get(&id).cloned().ok_or_else(HttpError::not_found)?;
    Ok(Cached::new(user).tags([format!("user:{id}")]))
}

/// The `authorize` of the mutations: the `Session` extractor rejects requests
/// without one (and documents the `401`).
async fn signed_in(session: Session) -> Result<Session, Infallible> {
    Ok(session)
}

async fn create_user(
    Auth(_session): Auth<Session>,
    State(state): State<AppState>,
    Json(new_user): Json<NewUser>,
) -> Invalidates<User> {
    let id = state.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    let user = User {
        id,
        name: new_user.name,
        role: new_user.role,
    };
    state.users.lock().unwrap().insert(id, user.clone());
    Invalidates::new(user).tags(["users"])
}

async fn update_user(
    State(state): State<AppState>,
    Path(UserPath { id }): Path<UserPath>,
    Json(new_user): Json<NewUser>,
) -> Result<Invalidates<User>, HttpError> {
    let mut users = state.users.lock().unwrap();
    let user = users.get_mut(&id).ok_or_else(HttpError::not_found)?;
    user.name = new_user.name;
    user.role = new_user.role;
    Ok(Invalidates::new(user.clone()).tags(["users".to_string(), format!("user:{id}")]))
}

async fn delete_user(
    State(state): State<AppState>,
    Path(UserPath { id }): Path<UserPath>,
) -> Result<Invalidates<()>, HttpError> {
    state
        .users
        .lock()
        .unwrap()
        .remove(&id)
        .ok_or_else(HttpError::not_found)?;
    Ok(Invalidates::new(()).tags(["users".to_string(), format!("user:{id}")]))
}

pub fn api() -> Api<AppState> {
    Api::new("Users", "1.0.0")
        .cache(Arc::new(InMemoryCache::new()))
        .route(
            "/users",
            get(public, list_users).post(signed_in, create_user),
        )
        .route(
            "/users/{id}",
            get(public, get_user)
                .put(signed_in, update_user)
                .delete(signed_in, delete_user),
        )
}
