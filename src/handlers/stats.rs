use crate::models::{ActionResponse, CacheAction, SystemStats};
use crate::services::{BatchProcessor, CacheService, QueueService};
use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use std::sync::Arc;

pub struct StatsState {
    pub response_cache: CacheService,
    pub conversation_cache: CacheService,
    pub batch_processor: BatchProcessor,
    pub queue: Arc<QueueService>,
}

/// Get system statistics
pub async fn get_stats(
    State(state): State<Arc<StatsState>>,
) -> Result<Json<SystemStats>, StatusCode> {
    let response_cache_stats = state.response_cache.stats().await;
    let conversation_cache_stats = state.conversation_cache.stats().await;
    let batch_stats = state.batch_processor.stats().await;
    let (queue_length, is_processing) = state.queue.get_queue_info().await;

    let stats = SystemStats {
        timestamp: Utc::now().to_rfc3339(),
        response_cache: response_cache_stats,
        conversation_cache: conversation_cache_stats,
        batch_processor: batch_stats,
        queue_length,
        is_processing,
    };

    Ok(Json(stats))
}

/// Manage cache (clear, warm, etc.)
pub async fn manage_cache(
    State(state): State<Arc<StatsState>>,
    Json(action): Json<CacheAction>,
) -> Result<Json<ActionResponse>, StatusCode> {
    match action.action.as_str() {
        "clear" => {
            state.response_cache.clear().await;
            state.conversation_cache.clear().await;
            Ok(Json(ActionResponse {
                success: true,
                message: "All caches cleared".to_string(),
            }))
        }
        "clear_response_cache" => {
            state.response_cache.clear().await;
            Ok(Json(ActionResponse {
                success: true,
                message: "Response cache cleared".to_string(),
            }))
        }
        "clear_conversation_cache" => {
            state.conversation_cache.clear().await;
            Ok(Json(ActionResponse {
                success: true,
                message: "Conversation cache cleared".to_string(),
            }))
        }
        "warm_model" => {
            // Extract model from data if provided
            let model = action
                .data
                .and_then(|d| d.get("model").and_then(|m| m.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "deepseek-r1:8b".to_string());

            match state.batch_processor.warm_model(&model).await {
                Ok(_) => Ok(Json(ActionResponse {
                    success: true,
                    message: format!("Model {} warmed", model),
                })),
                Err(e) => {
                    tracing::error!("Failed to warm model: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Health check endpoint
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now().to_rfc3339(),
    }))
}
