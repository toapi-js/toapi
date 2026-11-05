use std::{
    any::{TypeId, type_name},
    convert::Infallible,
    future::Future,
};

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};

use crate::openapi::{Generators, OperationDoc, OperationInput};

/// Decides whether a request may reach a route's handler, like the
/// `authorize` of a TypeScript handler. It runs before the response cache is
/// consulted, so cached responses only go to authorized requests.
///
/// Implemented for async functions whose arguments are extractors that don't
/// read the body (`FromRequestParts`) and that return `Result<A, E>`:
///
/// ```ignore
/// async fn signed_in(session: Session) -> Result<User, HttpError> {
///     session.user().ok_or_else(HttpError::unauthorized)
/// }
/// ```
///
/// On `Err`, the error is the response. On `Ok`, the handler can read the
/// value with the [`Auth`] extractor. [`public`] lets every request through.
pub trait Authorize<Args, S>: Clone + Send + Sync + 'static {
    type Auth: Clone + Send + Sync + 'static;

    fn authorize<'a>(
        self,
        parts: &'a mut Parts,
        state: &'a S,
    ) -> impl Future<Output = Result<Self::Auth, Response>> + Send + 'a;

    /// Adds what the extractors contribute to the OpenAPI operation, e.g. a
    /// `401` response.
    fn document(op: &mut OperationDoc, generators: &mut Generators);
}

macro_rules! impl_authorize {
    ($($arg:ident),*) => {
        impl<F, Fut, A, E, S, $($arg,)*> Authorize<($($arg,)*), S> for F
        where
            F: FnOnce($($arg),*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Result<A, E>> + Send,
            A: Clone + Send + Sync + 'static,
            E: IntoResponse,
            S: Send + Sync,
            $($arg: FromRequestParts<S> + OperationInput + Send,)*
        {
            type Auth = A;

            #[allow(non_snake_case, unused_variables)]
            fn authorize<'a>(
                self,
                parts: &'a mut Parts,
                state: &'a S,
            ) -> impl Future<Output = Result<A, Response>> + Send + 'a {
                async move {
                    $(
                        let $arg = $arg::from_request_parts(parts, state)
                            .await
                            .map_err(IntoResponse::into_response)?;
                    )*
                    self($($arg),*).await.map_err(IntoResponse::into_response)
                }
            }

            #[allow(unused_variables)]
            fn document(op: &mut OperationDoc, generators: &mut Generators) {
                $(<$arg as OperationInput>::document(op, generators);)*
            }
        }
    };
}

impl_authorize!();
impl_authorize!(A1);
impl_authorize!(A1, A2);
impl_authorize!(A1, A2, A3);
impl_authorize!(A1, A2, A3, A4);
impl_authorize!(A1, A2, A3, A4, A5);
impl_authorize!(A1, A2, A3, A4, A5, A6);
impl_authorize!(A1, A2, A3, A4, A5, A6, A7);
impl_authorize!(A1, A2, A3, A4, A5, A6, A7, A8);

/// The [`Authorize`] of public routes: lets every request through.
///
/// ```ignore
/// .route("/users", get(public, list_users))
/// ```
pub async fn public() -> Result<(), Infallible> {
    Ok(())
}

/// What the route's [`Authorize`] returned, as an extractor:
///
/// ```ignore
/// async fn create_user(Auth(user): Auth<User>, Json(body): Json<NewUser>) -> Invalidates<User>
/// ```
///
/// `A` must be what the route's `authorize` returns; `Api::route` checks
/// that when the route is added.
pub struct Auth<A>(pub A);

/// The result of the route's [`Authorize`], in the request extensions.
#[derive(Clone)]
pub(crate) struct Authorized<A>(pub A);

impl<A, S> FromRequestParts<S> for Auth<A>
where
    A: Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Authorized<A>>()
            .map(|authorized| Auth(authorized.0.clone()))
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!(
                        "Auth<{}> used in a route whose authorize returns something else",
                        type_name::<A>()
                    ),
                )
            })
    }
}

impl<A: 'static> OperationInput for Auth<A> {
    fn document(op: &mut OperationDoc, _: &mut Generators) {
        op.auth = Some((TypeId::of::<A>(), type_name::<A>()));
    }
}
