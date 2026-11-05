use bloom_core::{
    ByHash, Identify, State, futures_util::task::SpawnExt, use_async_memo, use_effect, use_memo,
    use_spawner, use_state,
};
use futures::StreamExt;
use std::sync::Arc;
use toapi_client::{Query, QueryCall, Request};

use crate::{Error, subscription::Subscription};

pub async fn use_query<R>(query: QueryCall<R>) -> Arc<Result<Arc<R::Response>, Arc<Error>>>
where
    R: Request<Kind = Query>,
    R::Response: Identify + Send + Sync + 'static,
{
    let stream = use_memo(ByHash(query), |ByHash(query)| {
        Subscription::new(query.watch())
    });

    let initial = use_async_memo(stream.clone(), |stream| async move {
        let mut lock = stream.lock().await;
        lock.next()
            .await
            .expect("Toapi Subscription failed to return initial value")
            .map_err(|error| Arc::new(error.into()))
    })
    .await;

    let state: State<Result<Arc<R::Response>, Arc<Error>>> = use_state(|| initial);

    let spawner = use_spawner();
    use_effect(
        (stream, state.clone(), spawner),
        |(stream, state, spawner)| {
            let err_state = state.clone();

            if let Err(error) = spawner.spawn(async move {
                let mut lock = stream.lock().await;
                while let Some(result) = lock.next().await {
                    state.set(result.map_err(|error| Arc::new(error.into())));
                }
            }) {
                err_state.set(Err(Arc::new(error.into())))
            }

            || {}
        },
    );

    state.into()
}
