use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{client::Inner, rt::BoxFuture};

/// Called with the URLs of invalidated cache entries, like the callbacks of
/// the TypeScript client's `PubSub`. A watch stream's callback refetches its
/// URL when it is among them.
pub(crate) type Callback =
    Arc<dyn Fn(Arc<Inner>, Arc<HashSet<String>>) -> BoxFuture<'static, ()> + Send + Sync>;

#[derive(Default)]
pub(crate) struct PubSub {
    subscriptions: HashMap<u64, Callback>,
    next_id: u64,
}

impl PubSub {
    pub fn subscribe(&mut self, callback: Callback) -> u64 {
        self.next_id += 1;
        self.subscriptions.insert(self.next_id, callback);
        self.next_id
    }

    pub fn unsubscribe(&mut self, id: u64) {
        self.subscriptions.remove(&id);
    }

    pub fn callbacks(&self) -> Vec<Callback> {
        self.subscriptions.values().cloned().collect()
    }
}
