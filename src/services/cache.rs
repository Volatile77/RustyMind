use crate::config::CacheConfig;
use crate::models::{CacheStats, ChatMessage};
use anyhow::Result;
use moka::future::Cache;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CacheService {
    cache: Cache<String, String>,
    stats: Arc<RwLock<CacheMetrics>>,
    config: CacheConfig,
}

#[derive(Debug, Default)]
struct CacheMetrics {
    hits: u64,
    misses: u64,
}

impl CacheService {
    pub fn new(config: CacheConfig) -> Self {
        let max_capacity = config.max_size_mb * 1024 * 1024; // Convert MB to bytes
        let ttl = Duration::from_secs(config.ttl_seconds);

        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();

        Self {
            cache,
            stats: Arc::new(RwLock::new(CacheMetrics::default())),
            config,
        }
    }

    /// Generate cache key from messages and model
    pub fn generate_key(messages: &[ChatMessage], model: &str) -> String {
        let content: String = messages
            .iter()
            .map(|m| format!("{}:{}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("||");

        let input = format!("{}::{}", model, content);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get cached response
    pub async fn get(&self, key: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        match self.cache.get(key).await {
            Some(value) => {
                let mut stats = self.stats.write().await;
                stats.hits += 1;
                tracing::debug!("✅ Cache hit for key: {}", &key[..8]);
                Some(value)
            }
            None => {
                let mut stats = self.stats.write().await;
                stats.misses += 1;
                tracing::debug!("❌ Cache miss for key: {}", &key[..8]);
                None
            }
        }
    }

    /// Set cached response
    pub async fn set(&self, key: String, value: String) {
        if !self.config.enabled {
            return;
        }

        self.cache.insert(key.clone(), value).await;
        tracing::debug!("💾 Cached response for key: {}", &key[..8]);
    }

    /// Check if key exists
    pub async fn contains(&self, key: &str) -> bool {
        if !self.config.enabled {
            return false;
        }
        self.cache.get(key).await.is_some()
    }

    /// Clear cache
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        let mut stats = self.stats.write().await;
        stats.hits = 0;
        stats.misses = 0;
        tracing::info!("🧹 Cache cleared");
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let metrics = self.stats.read().await;
        let total_requests = metrics.hits + metrics.misses;
        let hit_rate = if total_requests > 0 {
            metrics.hits as f64 / total_requests as f64
        } else {
            0.0
        };
        let miss_rate = if total_requests > 0 {
            metrics.misses as f64 / total_requests as f64
        } else {
            0.0
        };

        // Estimate memory usage (Moka doesn't provide exact memory usage)
        let entry_count = self.cache.entry_count();
        let estimated_size_mb = (entry_count as f64 * 5.0) / 1024.0; // Rough estimate: 5KB per entry
        let memory_usage_percent = if self.config.max_size_mb > 0 {
            (estimated_size_mb / self.config.max_size_mb as f64) * 100.0
        } else {
            0.0
        };

        CacheStats {
            total_entries: entry_count,
            total_size_mb: estimated_size_mb,
            hit_rate,
            miss_rate,
            memory_usage_percent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_service() {
        let config = CacheConfig {
            max_size_mb: 10,
            ttl_seconds: 60,
            enabled: true,
        };

        let cache = CacheService::new(config);
        let key = "test_key";

        // Test miss
        assert!(cache.get(key).await.is_none());

        // Test set and hit
        cache.set(key.to_string(), "test_value".to_string()).await;
        assert_eq!(cache.get(key).await, Some("test_value".to_string()));

        // Test stats
        let stats = cache.stats().await;
        assert_eq!(stats.hit_rate, 0.5); // 1 hit, 1 miss
    }
}
