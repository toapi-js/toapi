use std::{
    any::Any,
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use dashmap::DashMap;
use futures::future::{BoxFuture, Shared};

use crate::Error;

/// A parsed response. Stored type-erased because one cache holds the
/// responses of every route; readers downcast it to the route's response type.
pub(crate) type Value = Arc<dyn Any + Send + Sync>;

/// The result of a request, shared by everyone asking for the same URL while
/// the entry lives. It resolves once, so the response is parsed only once.
pub(crate) type SharedResult = Shared<BoxFuture<'static, Result<Value, Error>>>;

struct Entry {
    /// Identifies this entry, so a request that finishes after its entry was
    /// invalidated (and maybe replaced) doesn't touch the newer entry.
    id: u64,
    result: SharedResult,
    /// Empty until the response arrived.
    tags: HashSet<String>,
}

/// Requests by URL, stored as soon as they are made (like the TypeScript
/// client), so concurrent requests for the same URL share one request.
///
/// The cache is read far more often than written, so it's a sharded
/// concurrent map: reads only take a read lock on one shard and don't block
/// each other.
#[derive(Default)]
pub(crate) struct Cache {
    entries: DashMap<String, Entry>,
    next_id: AtomicU64,
}

impl Cache {
    pub fn get(&self, url: &str) -> Option<SharedResult> {
        self.entries.get(url).map(|entry| entry.result.clone())
    }

    /// The cached request for `url`, or the one `start` returns for a new
    /// entry with the given id. Checking and inserting is atomic, so
    /// concurrent callers share one request.
    pub fn get_or_insert_with(
        &self,
        url: &str,
        start: impl FnOnce(u64) -> SharedResult,
    ) -> SharedResult {
        // the common case: a read lock only
        if let Some(result) = self.get(url) {
            return result;
        }
        self.entries
            .entry(url.to_string())
            .or_insert_with(|| {
                let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
                Entry {
                    id,
                    result: start(id),
                    tags: HashSet::new(),
                }
            })
            .result
            .clone()
    }

    /// Records the tags of a response, if its entry is still current.
    pub fn set_tags(&self, url: &str, id: u64, tags: HashSet<String>) {
        if let Some(mut entry) = self.entries.get_mut(url).filter(|e| e.id == id) {
            entry.tags = tags;
        }
    }

    /// Removes an entry if it is still the one with `id`.
    pub fn remove(&self, url: &str, id: u64) -> bool {
        self.entries.remove_if(url, |_, e| e.id == id).is_some()
    }

    /// Removes every entry carrying one of `tags` and returns their URLs.
    pub fn invalidate_tags(&self, tags: &HashSet<String>) -> HashSet<String> {
        self.remove_where(|entry| !entry.tags.is_disjoint(tags))
    }

    /// Removes every entry and returns their URLs.
    pub fn invalidate_all(&self) -> HashSet<String> {
        self.remove_where(|_| true)
    }

    fn remove_where(&self, matches: impl Fn(&Entry) -> bool) -> HashSet<String> {
        let mut removed = HashSet::new();
        self.entries.retain(|url, entry| {
            if matches(entry) {
                removed.insert(url.clone());
                false
            } else {
                true
            }
        });
        removed
    }
}

#[cfg(test)]
mod tests {
    use futures::FutureExt;

    use super::*;

    fn ready(value: &'static str) -> SharedResult {
        futures::future::ready(Ok(Arc::new(value) as Value))
            .boxed()
            .shared()
    }

    fn tags(tags: &[&str]) -> HashSet<String> {
        tags.iter().map(|t| t.to_string()).collect()
    }

    /// Inserts an entry and returns its id.
    fn insert(cache: &Cache, url: &str, value: &'static str) -> u64 {
        let mut id = 0;
        drop(cache.get_or_insert_with(url, |new_id| {
            id = new_id;
            ready(value)
        }));
        id
    }

    #[test]
    fn shares_entries() {
        let cache = Cache::default();
        let id = insert(&cache, "/users", "[]");
        assert_ne!(id, 0);
        // an existing entry is returned without starting a new request
        drop(cache.get_or_insert_with("/users", |_| panic!("started a second request")));
    }

    #[test]
    fn invalidates_by_tag() {
        let cache = Cache::default();
        let a = insert(&cache, "/users", "[]");
        let b = insert(&cache, "/users/1", "{}");
        cache.set_tags("/users", a, tags(&["users"]));
        cache.set_tags("/users/1", b, tags(&["user:1"]));

        let urls = cache.invalidate_tags(&tags(&["users"]));
        assert_eq!(urls, HashSet::from(["/users".to_string()]));
        assert!(cache.get("/users").is_none());
        assert!(cache.get("/users/1").is_some());
    }

    #[test]
    fn late_responses_leave_newer_entries_alone() {
        let cache = Cache::default();
        let old = insert(&cache, "/users", "old");
        cache.invalidate_all();
        insert(&cache, "/users", "new");

        // the old request finishes after the invalidation
        cache.set_tags("/users", old, tags(&["users"]));
        assert!(!cache.remove("/users", old));
        assert!(cache.invalidate_tags(&tags(&["users"])).is_empty());
        assert!(cache.get("/users").is_some());
    }
}
