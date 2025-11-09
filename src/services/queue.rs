use crate::config::QueueConfig;
use crate::models::{ChatMessage, QueueStatus};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct QueuedRequest {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub system_prompt: String,
    pub timestamp: i64,
}

#[derive(Clone)]
pub struct QueueService {
    queue: Arc<RwLock<VecDeque<QueuedRequest>>>,
    processing: Arc<RwLock<bool>>,
    config: QueueConfig,
}

impl QueueService {
    pub fn new(config: QueueConfig) -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            processing: Arc::new(RwLock::new(false)),
            config,
        }
    }

    /// Enqueue a new request
    pub async fn enqueue(
        &self,
        messages: Vec<ChatMessage>,
        model: String,
        system_prompt: String,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp_millis();

        let request = QueuedRequest {
            id: id.clone(),
            messages,
            model,
            system_prompt,
            timestamp,
        };

        let mut queue = self.queue.write().await;
        queue.push_back(request);

        tracing::debug!("📥 Request {} added to queue (length: {})", id, queue.len());

        id
    }

    /// Get status for a specific request
    pub async fn get_status(&self, request_id: &str) -> Option<QueueStatus> {
        let queue = self.queue.read().await;
        let processing = self.processing.read().await;

        let position = queue.iter().position(|r| r.id == request_id);

        position.map(|pos| {
            let queue_position = pos + 1;
            let queue_length = queue.len();
            let estimated_wait_time = pos as u64 * self.config.estimated_time_per_request_ms;
            let is_processing = *processing && pos == 0;

            QueueStatus {
                queue_position,
                queue_length,
                estimated_wait_time,
                is_processing,
            }
        })
    }

    /// Get general queue info
    pub async fn get_queue_info(&self) -> (usize, bool) {
        let queue = self.queue.read().await;
        let processing = self.processing.read().await;
        (queue.len(), *processing)
    }

    /// Dequeue the next request (internal use)
    pub async fn dequeue(&self) -> Option<QueuedRequest> {
        let mut queue = self.queue.write().await;
        let request = queue.pop_front();

        if request.is_some() {
            tracing::debug!("📤 Request dequeued (remaining: {})", queue.len());
        }

        request
    }

    /// Mark queue as processing
    pub async fn set_processing(&self, is_processing: bool) {
        let mut processing = self.processing.write().await;
        *processing = is_processing;
    }

    /// Check if we can process more requests
    pub async fn can_process(&self) -> bool {
        let processing = self.processing.read().await;
        !*processing
    }

    /// Cancel a request
    pub async fn cancel(&self, request_id: &str) -> bool {
        let mut queue = self.queue.write().await;

        if let Some(pos) = queue.iter().position(|r| r.id == request_id) {
            queue.remove(pos);
            tracing::debug!("❌ Request {} cancelled", request_id);
            return true;
        }

        false
    }

    /// Get queue length
    pub async fn len(&self) -> usize {
        let queue = self.queue.read().await;
        queue.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.read().await;
        queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_service() {
        let config = QueueConfig {
            max_concurrent: 1,
            estimated_time_per_request_ms: 30000,
        };

        let queue = QueueService::new(config);

        // Test enqueue
        let messages = vec![];
        let id = queue
            .enqueue(messages, "model".to_string(), "prompt".to_string())
            .await;

        assert_eq!(queue.len().await, 1);

        // Test get_status
        let status = queue.get_status(&id).await;
        assert!(status.is_some());
        assert_eq!(status.unwrap().queue_position, 1);

        // Test dequeue
        let request = queue.dequeue().await;
        assert!(request.is_some());
        assert_eq!(queue.len().await, 0);
    }
}
