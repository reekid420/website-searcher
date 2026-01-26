use crate::models::SearchResult;
use crate::monitoring::get_metrics;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, instrument, warn};

/// Minimum cache size (default)
pub const MIN_CACHE_SIZE: usize = 3;
/// Maximum cache size
pub const MAX_CACHE_SIZE: usize = 20;
/// Default TTL for cache entries (12 hours)
pub const DEFAULT_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// A single cached search entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheEntry {
    /// The search query
    pub query: String,
    /// The search results
    pub results: Vec<SearchResult>,
    /// Unix timestamp when the search was performed
    pub timestamp: u64,
    /// Time-to-live for this entry in seconds (default 12 hours)
    #[serde(default = "default_ttl_seconds")]
    pub ttl: u64,
}

/// Default TTL in seconds (12 hours)
fn default_ttl_seconds() -> u64 {
    DEFAULT_TTL.as_secs()
}

impl CacheEntry {
    /// Check if this cache entry has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Check if the entry is older than its TTL
        let expired = now.saturating_sub(self.timestamp) > self.ttl;

        if expired {
            debug!(
                query = %self.query,
                age_seconds = now.saturating_sub(self.timestamp),
                ttl_seconds = self.ttl,
                "Cache entry expired"
            );
        }

        expired
    }

    /// Get the age of this entry (seconds since creation)
    pub fn age(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now.saturating_sub(self.timestamp)
    }

    /// Get the remaining TTL (seconds until expiration)
    pub fn remaining_ttl(&self) -> u64 {
        if self.is_expired() {
            0
        } else {
            let age = self.age();
            self.ttl.saturating_sub(age)
        }
    }
}

/// Search result cache with LRU-like behavior
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchCache {
    /// Cached entries, ordered from oldest to newest
    entries: Vec<CacheEntry>,
    /// Maximum number of entries to store
    max_size: usize,
}

impl SearchCache {
    /// Create a new empty cache with the specified max size
    pub fn new(max_size: usize) -> Self {
        let max_size = max_size.clamp(MIN_CACHE_SIZE, MAX_CACHE_SIZE);
        Self {
            entries: Vec::new(),
            max_size,
        }
    }

    /// Create a new cache with default size (3)
    pub fn with_default_size() -> Self {
        Self::new(MIN_CACHE_SIZE)
    }

    /// Get the current number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the current max size
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Set the max size (clamped to 3-20)
    pub fn set_max_size(&mut self, size: usize) {
        self.max_size = size.clamp(MIN_CACHE_SIZE, MAX_CACHE_SIZE);
        // Evict entries if we now exceed the new max
        while self.entries.len() > self.max_size {
            self.entries.remove(0);
        }
    }

    /// Get cached results for a query (case-insensitive match)
    /// Returns None if entry is expired
    #[instrument(skip(self), fields(query = %query))]
    pub fn get(&self, query: &str) -> Option<&CacheEntry> {
        let query_lower = query.to_lowercase();

        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| e.query.to_lowercase() == query_lower && !e.is_expired())
        {
            debug!(
                query = %query,
                result_count = entry.results.len(),
                age_seconds = entry.age(),
                "Cache hit"
            );
            get_metrics().record_cache_hit();
            Some(entry)
        } else {
            debug!(query = %query, "Cache miss");
            get_metrics().record_cache_miss();
            None
        }
    }

    /// Add a search to the cache
    /// If the query already exists, it's updated and moved to the end (most recent)
    pub fn add(&mut self, query: String, results: Vec<SearchResult>) {
        self.add_with_ttl(query, results, DEFAULT_TTL);
    }

    /// Add a search to the cache with custom TTL
    /// If the query already exists, it's updated and moved to the end (most recent)
    #[instrument(skip(self, results), fields(query = %query, result_count = results.len()))]
    pub fn add_with_ttl(&mut self, query: String, results: Vec<SearchResult>, ttl: Duration) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        info!(
            query = %query,
            result_count = results.len(),
            ttl_seconds = ttl.as_secs(),
            "Adding entry to cache"
        );

        // Remove existing entry for this query (case-insensitive)
        let query_lower = query.to_lowercase();
        self.entries
            .retain(|e| e.query.to_lowercase() != query_lower);

        // Add new entry at the end
        self.entries.push(CacheEntry {
            query,
            results,
            timestamp,
            ttl: ttl.as_secs(),
        });

        // Evict oldest if we exceed max size
        while self.entries.len() > self.max_size {
            self.entries.remove(0);
        }
    }

    /// Remove a specific entry by query
    pub fn remove(&mut self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        let before = self.entries.len();
        self.entries
            .retain(|e| e.query.to_lowercase() != query_lower);
        self.entries.len() < before
    }

    /// Clear all cached entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get all entries (oldest first)
    pub fn entries(&self) -> &[CacheEntry] {
        &self.entries
    }

    /// Get mutable access to all entries (for testing)
    #[cfg(test)]
    pub fn entries_mut(&mut self) -> &mut Vec<CacheEntry> {
        &mut self.entries
    }

    /// Get entries in reverse order (newest first)
    pub fn entries_newest_first(&self) -> impl Iterator<Item = &CacheEntry> {
        self.entries.iter().rev()
    }

    /// Remove all expired entries from the cache
    pub fn cleanup_expired(&mut self) {
        self.entries.retain(|e| !e.is_expired());
    }

    /// Get the number of expired entries (without removing them)
    pub fn expired_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_expired()).count()
    }

    /// Load cache from a JSON file
    pub async fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut cache: SearchCache = serde_json::from_str(&content)?;
        // Clean up expired entries on load
        cache.cleanup_expired();
        Ok(cache)
    }

    /// Save cache to a JSON file
    pub async fn save_to_file(&self, path: &Path) -> anyhow::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Load cache from file synchronously
    pub fn load_from_file_sync(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut cache: SearchCache = serde_json::from_str(&content)?;
        // Clean up expired entries on load
        cache.cleanup_expired();
        Ok(cache)
    }

    /// Save cache to file synchronously
    pub fn save_to_file_sync(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(site: &str, title: &str) -> SearchResult {
        SearchResult {
            site: site.to_string(),
            title: title.to_string(),
            url: format!("https://example.com/{}", title.replace(' ', "-")),
        }
    }

    #[test]
    fn cache_new_with_default_size() {
        let cache = SearchCache::with_default_size();
        assert_eq!(cache.max_size(), MIN_CACHE_SIZE);
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_new_clamps_size() {
        let too_small = SearchCache::new(1);
        assert_eq!(too_small.max_size(), MIN_CACHE_SIZE);

        let too_large = SearchCache::new(100);
        assert_eq!(too_large.max_size(), MAX_CACHE_SIZE);

        let just_right = SearchCache::new(10);
        assert_eq!(just_right.max_size(), 10);
    }

    #[test]
    fn cache_add_and_get() {
        let mut cache = SearchCache::with_default_size();
        let results = vec![make_result("fitgirl", "Elden Ring")];

        cache.add("elden ring".to_string(), results.clone());

        let entry = cache.get("elden ring").expect("should find entry");
        assert_eq!(entry.query, "elden ring");
        assert_eq!(entry.results, results);
    }

    #[test]
    fn cache_get_is_case_insensitive() {
        let mut cache = SearchCache::with_default_size();
        cache.add(
            "Elden Ring".to_string(),
            vec![make_result("fitgirl", "Elden Ring")],
        );

        assert!(cache.get("elden ring").is_some());
        assert!(cache.get("ELDEN RING").is_some());
        assert!(cache.get("Elden Ring").is_some());
    }

    #[test]
    fn cache_evicts_oldest_when_full() {
        let mut cache = SearchCache::new(3);

        cache.add("query1".to_string(), vec![]);
        cache.add("query2".to_string(), vec![]);
        cache.add("query3".to_string(), vec![]);

        assert_eq!(cache.len(), 3);
        assert!(cache.get("query1").is_some());

        // Add a 4th entry, should evict query1
        cache.add("query4".to_string(), vec![]);

        assert_eq!(cache.len(), 3);
        assert!(cache.get("query1").is_none());
        assert!(cache.get("query2").is_some());
        assert!(cache.get("query3").is_some());
        assert!(cache.get("query4").is_some());
    }

    #[test]
    fn cache_update_moves_to_end() {
        let mut cache = SearchCache::new(3);

        cache.add("query1".to_string(), vec![]);
        cache.add("query2".to_string(), vec![]);
        cache.add("query3".to_string(), vec![]);

        // Re-add query1, should move to end
        cache.add("query1".to_string(), vec![make_result("dodi", "Game")]);

        let entries: Vec<_> = cache.entries_newest_first().collect();
        assert_eq!(entries[0].query, "query1");
        assert_eq!(entries[0].results.len(), 1);
    }

    #[test]
    fn cache_clear_removes_all() {
        let mut cache = SearchCache::with_default_size();
        cache.add("query1".to_string(), vec![]);
        cache.add("query2".to_string(), vec![]);

        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_remove_specific_entry() {
        let mut cache = SearchCache::with_default_size();
        cache.add("query1".to_string(), vec![]);
        cache.add("query2".to_string(), vec![]);

        assert!(cache.remove("query1"));
        assert!(cache.get("query1").is_none());
        assert!(cache.get("query2").is_some());

        // Removing non-existent returns false
        assert!(!cache.remove("nonexistent"));
    }

    #[test]
    fn cache_set_max_size_evicts_if_needed() {
        let mut cache = SearchCache::new(5);
        for i in 1..=5 {
            cache.add(format!("query{}", i), vec![]);
        }
        assert_eq!(cache.len(), 5);

        cache.set_max_size(3);
        assert_eq!(cache.max_size(), 3);
        assert_eq!(cache.len(), 3);

        // First two should be evicted
        assert!(cache.get("query1").is_none());
        assert!(cache.get("query2").is_none());
        assert!(cache.get("query3").is_some());
    }

    #[test]
    fn cache_serialization_roundtrip() {
        let mut cache = SearchCache::new(5);
        cache.add(
            "elden ring".to_string(),
            vec![make_result("fitgirl", "Elden Ring")],
        );
        cache.add("baldurs gate 3".to_string(), vec![]);

        let json = serde_json::to_string(&cache).unwrap();
        let restored: SearchCache = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        assert!(restored.get("elden ring").is_some());
        assert!(restored.get("baldurs gate 3").is_some());
    }

    #[test]
    fn cache_entries_newest_first() {
        let mut cache = SearchCache::with_default_size();
        cache.add("first".to_string(), vec![]);
        cache.add("second".to_string(), vec![]);
        cache.add("third".to_string(), vec![]);

        let queries: Vec<_> = cache.entries_newest_first().map(|e| &e.query).collect();
        assert_eq!(queries, vec!["third", "second", "first"]);
    }

    #[test]
    fn cache_entry_ttl_and_expiration() {
        let mut cache = SearchCache::with_default_size();

        // Add entry with 1 second TTL
        cache.add_with_ttl("test".to_string(), vec![], Duration::from_secs(1));

        // Should be found immediately
        assert!(cache.get("test").is_some());

        // Simulate time passing (manually set timestamp in the past)
        if let Some(entry) = cache.entries_mut().last_mut() {
            entry.timestamp = 0; // Set to epoch
        }

        // Now it should be expired
        assert!(cache.get("test").is_none());
    }

    #[test]
    fn cache_cleanup_expired() {
        let mut cache = SearchCache::new(5);

        // Add entries with different TTLs
        cache.add_with_ttl(
            "fresh".to_string(),
            vec![],
            Duration::from_secs(12 * 60 * 60),
        );
        cache.add_with_ttl("old".to_string(), vec![], Duration::from_secs(1));

        // Simulate time passing for the old entry
        for entry in cache.entries_mut().iter_mut() {
            if entry.query == "old" {
                entry.timestamp = 0;
            }
        }

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.expired_count(), 1);

        // Cleanup should remove expired entries
        cache.cleanup_expired();

        assert_eq!(cache.len(), 1);
        assert!(cache.get("fresh").is_some());
        assert!(cache.get("old").is_none());
    }

    #[test]
    fn cache_entry_age_and_remaining_ttl() {
        let entry = CacheEntry {
            query: "test".to_string(),
            results: vec![],
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 3600, // 1 hour ago
            ttl: DEFAULT_TTL.as_secs(),
        };

        // Age should be approximately 1 hour
        assert!(entry.age() >= 3590 && entry.age() <= 3610);

        // Remaining TTL should be approximately 11 hours
        let remaining = entry.remaining_ttl();
        assert!((11 * 60 * 60 - 10..=11 * 60 * 60 + 10).contains(&remaining));
    }

    #[test]
    fn cache_loads_and_cleans_expired() {
        let mut cache = SearchCache::new(5);
        cache.add_with_ttl(
            "valid".to_string(),
            vec![],
            Duration::from_secs(12 * 60 * 60),
        );
        cache.add_with_ttl("expired".to_string(), vec![], Duration::from_secs(1));

        // Manually expire one entry
        for entry in cache.entries_mut().iter_mut() {
            if entry.query == "expired" {
                entry.timestamp = 0;
            }
        }

        // Serialize and deserialize
        let json = serde_json::to_string(&cache).unwrap();
        let mut loaded: SearchCache = serde_json::from_str(&json).unwrap();

        // Manually cleanup expired entries (simulating what load_from_file does)
        loaded.cleanup_expired();

        // Expired entry should be automatically cleaned
        assert_eq!(loaded.len(), 1);
        assert!(loaded.get("valid").is_some());
        assert!(loaded.get("expired").is_none());
    }
}
