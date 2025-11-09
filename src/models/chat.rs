use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default = "default_true")]
    pub stream: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub use_cache: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueResponse {
    pub request_id: String,
    pub status: QueueStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatus {
    pub queue_position: usize,
    pub queue_length: usize,
    pub estimated_wait_time: u64,
    pub is_processing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueStatusResponse {
    pub request_id: String,
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<QueueStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub total_entries: u64,
    pub total_size_mb: f64,
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub memory_usage_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchStats {
    pub total_requests: u64,
    pub cached_responses: u64,
    pub deduplicated_requests: u64,
    pub batches_processed: u64,
    pub average_batch_size: f64,
    pub cache_hit_rate: u32,
    pub deduplication_rate: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStats {
    pub timestamp: String,
    pub response_cache: CacheStats,
    pub conversation_cache: CacheStats,
    pub batch_processor: BatchStats,
    pub queue_length: usize,
    pub is_processing: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheAction {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionResponse {
    pub success: bool,
    pub message: String,
}

// Ollama API types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaResponse {
    pub message: Option<ChatMessage>,
    #[serde(default)]
    pub done: bool,
}

fn default_true() -> bool {
    true
}
