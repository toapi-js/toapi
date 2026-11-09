use bloom_core::{
    ByHash, Identify, State, use_async_memo, use_effect, use_memo, use_spawner, use_state,
};
use futures::StreamExt;
use std::sync::Arc;
use toapi_client::{IntoQueryCall, Request};

use crate::{Error, subscription::Subscription};

type Value<T> = Result<Arc<T>, Error>;

/// Accepts a generated GET builder (`client.users.get()`) or a [`QueryCall`]
/// from [`Client::call`](toapi_client::Client::call).
///
/// [`QueryCall`]: toapi_client::QueryCall
pub async fn use_query<C>(query: C) -> Value<<C::Request as Request>::Response>
where
    C: IntoQueryCall,
    <C::Request as Request>::Response: Identify + Send + Sync + 'static,
{
    let query = query.into_query_call();
    let stream = use_memo(ByHash(query), |ByHash(query)| {
        Subscription::new(query.watch()).into()
    });

    let initial = use_async_memo(stream.clone(), |stream| async move {
        let mut lock = stream.lock().await;
        lock.next()
            .await
            .expect("Toapi Subscription failed to return initial value")
            .map_err(|error| error.into())
            .into()
    })
    .await;

    let state: State<Value<<C::Request as Request>::Response>> = use_state(|| initial);

    let spawner = use_spawner();
    use_effect(
        (stream, state.clone(), spawner),
        |(stream, state, spawner)| {
            let err_state = state.clone();

            if let Err(error) = spawner.spawn(async move {
                let mut lock = stream.lock().await;
                while let Some(result) = lock.next().await {
                    state.set(result.map_err(|error| error.into()));
                }
            }) {
                err_state.set(Err(error.into()))
            }

            || {}
        },
    );

    (*state).clone().into()
}
