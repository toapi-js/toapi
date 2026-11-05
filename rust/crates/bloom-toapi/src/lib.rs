use bloom_core::{use_effect, use_memo, use_state};
use futures::{StreamExt, lock::Mutex};
use toapi_client::{Query, QueryCall, Request};

pub async fn use_query<R>(query: QueryCall<R>)
where
    R: Request<Kind = Query>,
    R::Response: Send + Sync + 'static,
{
    let stream = use_memo(query, |query| Mutex::new(Box::pin(query.watch())));

    let state = use_state(async move || {
        let mut lock = stream.lock().await;
        lock.next().await
    });

    use_effect(stream, |stream| {
        let lock = stream.lock().await;
        while let Some(result) = stream.lock
    });
}
