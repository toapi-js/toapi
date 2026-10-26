//! The users API client, generated from `users-server/openapi.json`: the
//! `Api` path tree, call builders and request structs.
//!
//! Calls only compile when they match the API:
//!
//! ```compile_fail
//! # async fn f(api: users_client::Api) {
//! // a required body field (`role`) is missing
//! api.users.post().name("Ada").await;
//! # }
//! ```
//!
//! ```compile_fail
//! # async fn f(api: users_client::Api) {
//! // `{id}` is a u64
//! api.users.id("ada").get().await;
//! # }
//! ```
//!
//! ```compile_fail
//! # async fn f(api: users_client::Api) {
//! // there is no `POST /users/{id}`
//! api.users.id(1).post();
//! # }
//! ```
//!
//! ```compile_fail
//! # fn f(api: users_client::Api) {
//! // only GET requests can be watched
//! api.users.id(1).delete().watch();
//! # }
//! ```
//!
//! ```compile_fail
//! # fn f(api: users_client::Api) {
//! // `Client::call` returns a `MutationCall` for mutations, which has no `watch`
//! api.client().call(users_client::DeleteUsersByIdRequest { id: 1 }).watch();
//! # }
//! ```
//!
//! ```
//! # async fn f(api: users_client::Api) {
//! use users_client::Role;
//! api.users.post().name("Ada").role(Role::Admin).await;
//! api.users.id(1).get().await;
//! api.users.get().limit(10).await;
//! api.users.get().query(users_client::GetUsersQuery { limit: Some(10) }).await;
//! // a whole body sets all required fields
//! let ada = users_client::NewUser { name: "Ada".into(), role: Role::Admin };
//! api.users.post().body(ada).await;
//! api.client().call(users_client::GetUsersByIdRequest { id: 1 }).watch();
//! # }
//! ```

include!(concat!(env!("OUT_DIR"), "/toapi_client.rs"));
