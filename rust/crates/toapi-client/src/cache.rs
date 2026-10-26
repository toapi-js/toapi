use std::{
    any::Any,
    collections::{HashMap, HashSet},
    sync::Arc,
};

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
    tags: Vec<String>,
}

/// Requests by URL, stored as soon as they are made (like the TypeScript
/// client), so concurrent requests for the same URL share one request.
#[derive(Default)]
pub(crate) struct Cache {
    entries: HashMap<String, Entry>,
    next_id: u64,
}

impl Cache {
    pub fn get(&self, url: &str) -> Option<SharedResult> {
        self.entries.get(url).map(|entry| entry.result.clone())
    }

    pub fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn insert(&mut self, url: String, id: u64, result: SharedResult) {
        self.entries.insert(
            url,
            Entry {
                id,
                result,
                tags: Vec::new(),
            },
        );
    }

    /// Records the tags of a response, if its entry is still current.
    pub fn set_tags(&mut self, url: &str, id: u64, tags: Vec<String>) {
        if let Some(entry) = self.entries.get_mut(url).filter(|e| e.id == id) {
            entry.tags = tags;
        }
    }

    /// Removes an entry if it is still the one with `id`.
    pub fn remove(&mut self, url: &str, id: u64) -> bool {
        if self.entries.get(url).is_some_and(|e| e.id == id) {
            self.entries.remove(url);
            true
        } else {
            false
        }
    }

    /// Removes every entry carrying one of `tags` and returns their URLs.
    pub fn invalidate_tags(&mut self, tags: &[String]) -> HashSet<String> {
        let tags: HashSet<&str> = tags.iter().map(String::as_str).collect();
        let urls: HashSet<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.tags.iter().any(|tag| tags.contains(tag.as_str())))
            .map(|(url, _)| url.clone())
            .collect();
        for url in &urls {
            self.entries.remove(url);
        }
        urls
    }

    /// Removes every entry and returns their URLs.
    pub fn invalidate_all(&mut self) -> HashSet<String> {
        self.entries.drain().map(|(url, _)| url).collect()
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

    fn tags(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn invalidates_by_tag() {
        let mut cache = Cache::default();
        let (a, b) = (cache.next_id(), cache.next_id());
        cache.insert("/users".into(), a, ready("[]"));
        cache.insert("/users/1".into(), b, ready("{}"));
        cache.set_tags("/users", a, tags(&["users"]));
        cache.set_tags("/users/1", b, tags(&["user:1"]));

        let urls = cache.invalidate_tags(&tags(&["users"]));
        assert_eq!(urls, HashSet::from(["/users".to_string()]));
        assert!(cache.get("/users").is_none());
        assert!(cache.get("/users/1").is_some());
    }

    #[test]
    fn late_responses_leave_newer_entries_alone() {
        let mut cache = Cache::default();
        let old = cache.next_id();
        cache.insert("/users".into(), old, ready("old"));
        cache.invalidate_all();
        let new = cache.next_id();
        cache.insert("/users".into(), new, ready("new"));

        // the old request finishes after the invalidation
        cache.set_tags("/users", old, tags(&["users"]));
        assert!(!cache.remove("/users", old));
        assert!(cache.invalidate_tags(&tags(&["users"])).is_empty());
        assert!(cache.get("/users").is_some());
    }
}
