use crate::config::BatchConfig;
use crate::models::{BatchStats, ChatMessage};
use crate::services::{CacheService, OllamaClient};
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

#[derive(Clone)]
pub struct BatchProcessor {
    cache: CacheService,
    ollama: OllamaClient,
    config: BatchConfig,
    stats: Arc<RwLock<BatchMetrics>>,
}

#[derive(Debug, Default)]
struct BatchMetrics {
    total_requests: u64,
    cached_responses: u64,
    deduplicated_requests: u64,
    batches_processed: u64,
    total_batch_size: u64,
}

impl BatchProcessor {
    pub fn new(cache: CacheService, ollama: OllamaClient, config: BatchConfig) -> Self {
        Self {
            cache,
            ollama,
            config,
            stats: Arc::new(RwLock::new(BatchMetrics::default())),
        }
    }

    /// Process a single request with caching and batching
    pub async fn process(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
        system_prompt: &str,
        _priority: i32, // Can be used for priority queuing in future
    ) -> Result<String> {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        drop(stats);

        // Check cache first
        let cache_key = CacheService::generate_key(&messages, model);

        if let Some(cached) = self.cache.get(&cache_key).await {
            let mut stats = self.stats.write().await;
            stats.cached_responses += 1;
            tracing::info!("✅ Serving from cache");
            return Ok(cached);
        }

        // TODO: Implement actual batching logic with buffer
        // For now, process immediately
        let response = self
            .ollama
            .chat_completion(&messages, model, system_prompt, false)
            .await?;

        // Cache the response
        self.cache.set(cache_key, response.clone()).await;

        let mut stats = self.stats.write().await;
        stats.batches_processed += 1;
        stats.total_batch_size += 1;

        Ok(response)
    }

    /// Get batch processor statistics
    pub async fn stats(&self) -> BatchStats {
        let metrics = self.stats.read().await;
        let average_batch_size = if metrics.batches_processed > 0 {
            metrics.total_batch_size as f64 / metrics.batches_processed as f64
        } else {
            0.0
        };

        let cache_hit_rate = if metrics.total_requests > 0 {
            ((metrics.cached_responses as f64 / metrics.total_requests as f64) * 100.0) as u32
        } else {
            0
        };

        let deduplication_rate = if metrics.total_requests > 0 {
            ((metrics.deduplicated_requests as f64 / metrics.total_requests as f64) * 100.0) as u32
        } else {
            0
        };

        BatchStats {
            total_requests: metrics.total_requests,
            cached_responses: metrics.cached_responses,
            deduplicated_requests: metrics.deduplicated_requests,
            batches_processed: metrics.batches_processed,
            average_batch_size,
            cache_hit_rate,
            deduplication_rate,
        }
    }

    /// Warm the model by sending a test request
    pub async fn warm_model(&self, model: &str) -> Result<()> {
        tracing::info!("🔥 Warming model: {}", model);

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        }];

        self.ollama
            .chat_completion(&messages, model, "You are a helpful assistant.", false)
            .await?;

        tracing::info!("✅ Model warmed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CacheConfig, OllamaConfig};

    #[tokio::test]
    async fn test_batch_processor_stats() {
        let cache_config = CacheConfig {
            max_size_mb: 10,
            ttl_seconds: 60,
            enabled: true,
        };

        let ollama_config = OllamaConfig {
            api_url: "http://localhost:11434".to_string(),
            model: "test".to_string(),
            system_prompt: "test".to_string(),
            keep_alive: "15m".to_string(),
            timeout_seconds: 300,
        };

        let batch_config = BatchConfig {
            max_batch_size: 3,
            batch_timeout_ms: 2000,
            enable_deduplication: true,
        };

        let cache = CacheService::new(cache_config);
        let ollama = OllamaClient::new(ollama_config);
        let processor = BatchProcessor::new(cache, ollama, batch_config);

        let stats = processor.stats().await;
        assert_eq!(stats.total_requests, 0);
    }
}
