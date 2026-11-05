use bloom_core::{
    ByHash, Identify, State, futures_util::task::SpawnExt, use_async_memo, use_effect, use_memo,
    use_spawner, use_state,
};
use futures::stream::Stream;
use futures::{
    StreamExt,
    lock::{Mutex, MutexLockFuture},
};
use std::pin::Pin;
use std::sync::Arc;
use toapi_client::{Error, Query, QueryCall, Request};

use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Subscription<R, S>(u64, Mutex<Pin<Box<S>>>)
where
    S: Stream<Item = Result<Arc<R>, Error>>;

impl<R, S> Identify for Subscription<R, S>
where
    S: Stream<Item = Result<Arc<R>, Error>>,
{
    fn id(&self) -> u64 {
        self.0
    }
}

impl<R, S> Subscription<R, S>
where
    S: Stream<Item = Result<Arc<R>, Error>>,
{
    fn new(stream: S) -> Self
    where
        S: Stream<Item = Result<Arc<R>, Error>> + 'static,
    {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Subscription(id, Mutex::new(Box::pin(stream)))
    }

    fn lock(&self) -> MutexLockFuture<'_, Pin<Box<S>>> {
        self.1.lock()
    }
}

pub async fn use_query<R>(query: QueryCall<R>) -> Arc<Result<Arc<R::Response>, ByHash<Error>>>
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
            .map_err(ByHash)
    })
    .await;

    let state: State<Result<Arc<R::Response>, ByHash<Error>>> = use_state(|| initial);

    let spawner = use_spawner();
    use_effect(
        (stream, state.clone(), spawner),
        |(stream, state, spawner)| {
            spawner.spawn(async move {
                let mut lock = stream.lock().await;
                while let Some(result) = lock.next().await {
                    state.set(result.map_err(ByHash));
                }
            });

            || {}
        },
    );

    state.into()
}
