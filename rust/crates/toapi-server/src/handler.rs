use std::future::Future;

use axum::response::IntoResponse;
use schemars::JsonSchema;

use crate::{
    openapi::{Generators, OperationDoc, OperationInput},
    response::{Cached, Invalidates},
};

/// Marker for GET handlers.
pub struct QueryKind;
/// Marker for POST/PUT/PATCH/DELETE handlers.
pub struct MutationKind;

/// The return type of a handler of kind `K`: [`Cached<T>`] for queries and
/// [`Invalidates<T>`] for mutations, optionally wrapped in a `Result`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid return type for this handler",
    note = "GET handlers return `Cached<T>`, POST/PUT/PATCH/DELETE handlers return `Invalidates<T>` (optionally wrapped in `Result<_, E: IntoResponse>`)"
)]
pub trait HandlerOutput<K> {
    fn document(op: &mut OperationDoc, generators: &mut Generators);
}

impl<T: JsonSchema> HandlerOutput<QueryKind> for Cached<T> {
    fn document(op: &mut OperationDoc, generators: &mut Generators) {
        op.response = Some(generators.components.subschema_for::<T>());
    }
}

impl<T: JsonSchema> HandlerOutput<MutationKind> for Invalidates<T> {
    fn document(op: &mut OperationDoc, generators: &mut Generators) {
        op.response = Some(generators.components.subschema_for::<T>());
    }
}

impl<K, T: HandlerOutput<K>, E: IntoResponse> HandlerOutput<K> for Result<T, E> {
    fn document(op: &mut OperationDoc, generators: &mut Generators) {
        T::document(op, generators)
    }
}

/// An async handler function whose arguments and return type describe an
/// OpenAPI operation. Implemented for async functions of up to 12 arguments.
pub trait DocumentedHandler<Args, K> {
    fn document(op: &mut OperationDoc, generators: &mut Generators);
}

macro_rules! impl_documented_handler {
    ($($arg:ident),*) => {
        impl<F, Fut, K, $($arg,)*> DocumentedHandler<($($arg,)*), K> for F
        where
            F: FnOnce($($arg),*) -> Fut,
            Fut: Future,
            Fut::Output: HandlerOutput<K>,
            $($arg: OperationInput,)*
        {
            fn document(op: &mut OperationDoc, generators: &mut Generators) {
                $(<$arg as OperationInput>::document(op, generators);)*
                <Fut::Output as HandlerOutput<K>>::document(op, generators);
            }
        }
    };
}

impl_documented_handler!();
impl_documented_handler!(A1);
impl_documented_handler!(A1, A2);
impl_documented_handler!(A1, A2, A3);
impl_documented_handler!(A1, A2, A3, A4);
impl_documented_handler!(A1, A2, A3, A4, A5);
impl_documented_handler!(A1, A2, A3, A4, A5, A6);
impl_documented_handler!(A1, A2, A3, A4, A5, A6, A7);
impl_documented_handler!(A1, A2, A3, A4, A5, A6, A7, A8);
impl_documented_handler!(A1, A2, A3, A4, A5, A6, A7, A8, A9);
impl_documented_handler!(A1, A2, A3, A4, A5, A6, A7, A8, A9, A10);
impl_documented_handler!(A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11);
impl_documented_handler!(A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12);
