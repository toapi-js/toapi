//! Typestate markers of the generated call builders.
//!
//! A call builder such as `PutUsersByIdCall<Unset<required::name>, Set>` has
//! one type parameter per required query parameter or body field. Setting the
//! field turns its `Unset<..>` into `Set`, and the call can only be awaited
//! once every parameter is `Set`.

use std::marker::PhantomData;

/// A required field that has been set.
pub struct Set;

/// A required field `F` that hasn't been set yet. `F` is a marker type named
/// after the field, so compile errors name the missing field.
pub struct Unset<F>(PhantomData<F>);

/// Implemented by [`Set`] only.
#[diagnostic::on_unimplemented(
    message = "a required field is not set: `{Self}`",
    label = "set every required query parameter and body field before sending the request"
)]
pub trait IsSet {}

impl IsSet for Set {}
