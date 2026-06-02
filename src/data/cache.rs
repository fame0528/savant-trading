//! TTL-based in-memory cache with LRU eviction.
//!
//! Each insight module (on-chain, funding, sentiment, RSS, liquidation)
//! gets its own cache instance. Data is fetched once per TTL window and
//! served from cache on subsequent requests.
//!
//! Graceful degradation: if the API fails, serve stale cached data
//! regardless of TTL.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A cached entry with its timestamp.
#[derive(Debug, Clone)]
struct CacheEntry<V: Clone> {
    value: V,
    cached_at: Instant,
    ttl: Duration,
}

/// TTL-based cache with LRU eviction.
#[derive(Debug)]
pub struct TtlCache<V: Clone> {
    entries: HashMap<String, CacheEntry<V>>,
    max_entries: usize,
}

impl<V: Clone> TtlCache<V> {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
        }
    }

    /// Get a cached value if it exists and hasn't expired.
    pub fn get(&self, key: &str) -> Option<&V> {
        self.entries.get(key).and_then(|entry| {
            if entry.cached_at.elapsed() < entry.ttl {
                Some(&entry.value)
            } else {
                None
            }
        })
    }

    /// Get a cached value even if expired (for graceful degradation).
    /// Returns the stale value if it exists, regardless of TTL.
    pub fn get_stale(&self, key: &str) -> Option<&V> {
        self.entries.get(key).map(|entry| &entry.value)
    }

    /// Insert or update a cached value with a specific TTL.
    pub fn insert(&mut self, key: String, value: V, ttl: Duration) {
        // Evict oldest entry if at capacity
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
            if let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.cached_at)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&oldest_key);
            }
        }

        self.entries.insert(
            key,
            CacheEntry {
                value,
                cached_at: Instant::now(),
                ttl,
            },
        );
    }

    /// Check if a key exists and is fresh (not expired).
    pub fn is_fresh(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Number of entries in the cache (including expired).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove all expired entries.
    pub fn evict_expired(&mut self) {
        self.entries
            .retain(|_, entry| entry.cached_at.elapsed() < entry.ttl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_hit_and_miss() {
        let mut cache = TtlCache::new(100);
        cache.insert("key1".into(), "value1".into(), Duration::from_secs(60));

        assert_eq!(cache.get("key1"), Some(&"value1".to_string()));
        assert_eq!(cache.get("key2"), None);
    }

    #[test]
    fn cache_expired() {
        let mut cache = TtlCache::new(100);
        cache.insert(
            "key1".into(),
            "value1".into(),
            Duration::from_millis(0), // expires immediately
        );

        // Wait a tick
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.get("key1"), None); // expired
        assert_eq!(cache.get_stale("key1"), Some(&"value1".to_string())); // stale still available
    }

    #[test]
    fn cache_lru_eviction() {
        let mut cache = TtlCache::new(2);
        cache.insert("a".into(), 1, Duration::from_secs(60));
        cache.insert("b".into(), 2, Duration::from_secs(60));
        cache.insert("c".into(), 3, Duration::from_secs(60)); // evicts oldest (a)

        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("b"), Some(&2));
        assert_eq!(cache.get("c"), Some(&3));
    }

    #[test]
    fn cache_update_existing() {
        let mut cache = TtlCache::new(100);
        cache.insert("key1".into(), "old".into(), Duration::from_secs(60));
        cache.insert("key1".into(), "new".into(), Duration::from_secs(60));

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("key1"), Some(&"new".to_string()));
    }

    #[test]
    fn cache_evict_expired() {
        let mut cache = TtlCache::new(100);
        cache.insert("fresh".into(), 1, Duration::from_secs(60));
        cache.insert("stale".into(), 2, Duration::from_millis(0));

        std::thread::sleep(Duration::from_millis(10));
        cache.evict_expired();

        assert_eq!(cache.len(), 1);
        assert!(cache.is_fresh("fresh"));
        assert!(!cache.is_fresh("stale"));
    }
}
