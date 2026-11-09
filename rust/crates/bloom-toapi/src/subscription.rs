use bloom_core::Identify;
use futures::lock::{Mutex, MutexLockFuture};
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use toapi_client::Error;
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub(crate) struct Subscription<R, S>(u64, Mutex<Pin<Box<S>>>)
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
    pub(crate) fn new(stream: S) -> Self
    where
        S: Stream<Item = Result<Arc<R>, Error>> + 'static,
    {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Subscription(id, Mutex::new(Box::pin(stream)))
    }

    pub(crate) fn lock(&self) -> MutexLockFuture<'_, Pin<Box<S>>> {
        self.1.lock()
    }
}
