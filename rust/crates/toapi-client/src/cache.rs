use std::{
    collections::{HashMap, HashSet},
    time::SystemTime,
};

use bytes::Bytes;

struct Entry {
    body: Bytes,
    tags: Vec<String>,
    expires_at: Option<SystemTime>,
}

/// Query responses by URL, with the tags they were sent with.
#[derive(Default)]
pub(crate) struct Cache {
    entries: HashMap<String, Entry>,
    /// Number of active watch streams per URL.
    watchers: HashMap<String, usize>,
    /// Incremented on every invalidation. A response whose request started
    /// before an invalidation may already be stale, so it isn't cached.
    generation: u64,
}

impl Cache {
    pub fn get(&self, url: &str) -> Option<Bytes> {
        let entry = self.entries.get(url)?;
        let fresh = entry
            .expires_at
            .is_none_or(|expires_at| expires_at > SystemTime::now());
        fresh.then(|| entry.body.clone())
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Caches a response unless an invalidation happened since `generation`.
    /// Responses without tags and without an expiry are never invalidated, so
    /// they aren't cached either.
    pub fn insert(
        &mut self,
        generation: u64,
        url: String,
        body: Bytes,
        tags: Vec<String>,
        expires_at: Option<SystemTime>,
    ) {
        if generation != self.generation || (tags.is_empty() && expires_at.is_none()) {
            return;
        }
        self.entries.insert(
            url,
            Entry {
                body,
                tags,
                expires_at,
            },
        );
    }

    /// Drops every entry carrying one of `tags` and returns their URLs.
    pub fn invalidate_tags(&mut self, tags: &[String]) -> Vec<String> {
        self.generation += 1;
        let tags: HashSet<&str> = tags.iter().map(String::as_str).collect();
        let urls: Vec<String> = self
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

    pub fn invalidate_all(&mut self) {
        self.generation += 1;
        self.entries.clear();
    }

    pub fn is_watched(&self, url: &str) -> bool {
        self.watchers.contains_key(url)
    }

    pub fn watched(&self) -> Vec<String> {
        self.watchers.keys().cloned().collect()
    }

    pub fn watch(&mut self, url: &str) {
        *self.watchers.entry(url.to_string()).or_default() += 1;
    }

    pub fn unwatch(&mut self, url: &str) {
        if let Some(count) = self.watchers.get_mut(url) {
            *count -= 1;
            if *count == 0 {
                self.watchers.remove(url);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn invalidates_by_tag() {
        let mut cache = Cache::default();
        cache.insert(
            0,
            "/users".into(),
            Bytes::from("[]"),
            tags(&["users"]),
            None,
        );
        cache.insert(
            0,
            "/users/1".into(),
            Bytes::from("{}"),
            tags(&["user:1"]),
            None,
        );

        assert_eq!(
            cache.invalidate_tags(&tags(&["users"])),
            vec!["/users".to_string()]
        );
        assert!(cache.get("/users").is_none());
        assert!(cache.get("/users/1").is_some());
    }

    #[test]
    fn skips_responses_older_than_an_invalidation() {
        let mut cache = Cache::default();
        let generation = cache.generation();
        cache.invalidate_tags(&tags(&["users"]));
        cache.insert(
            generation,
            "/users".into(),
            Bytes::from("[]"),
            tags(&["users"]),
            None,
        );
        assert!(cache.get("/users").is_none());
    }

    #[test]
    fn skips_untagged_responses() {
        let mut cache = Cache::default();
        cache.insert(0, "/health".into(), Bytes::from("ok"), vec![], None);
        assert!(cache.get("/health").is_none());
    }
}
