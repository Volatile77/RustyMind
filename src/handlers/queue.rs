use crate::models::{QueueRequest, QueueResponse, QueueStatusResponse};
use crate::services::QueueService;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct StatusQuery {
    #[serde(rename = "requestId")]
    request_id: Option<String>,
}

/// Add request to queue
pub async fn enqueue_request(
    State(queue): State<Arc<QueueService>>,
    Json(request): Json<QueueRequest>,
) -> Result<Json<QueueResponse>, StatusCode> {
    let model = request
        .model
        .unwrap_or_else(|| "deepseek-r1:8b".to_string());
    let system_prompt = request
        .system_prompt
        .unwrap_or_else(|| "Format all responses in markdown.".to_string());

    let request_id = queue.enqueue(request.messages, model, system_prompt).await;

    // Get initial status
    let status = queue
        .get_status(&request_id)
        .await
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(QueueResponse { request_id, status }))
}

/// Get queue status
pub async fn get_queue_status(
    State(queue): State<Arc<QueueService>>,
    Query(params): Query<StatusQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(request_id) = params.request_id {
        // Get status for specific request
        match queue.get_status(&request_id).await {
            Some(status) => Ok(Json(serde_json::json!(QueueStatusResponse {
                request_id,
                completed: false,
                status: Some(status),
            }))),
            None => Ok(Json(serde_json::json!(QueueStatusResponse {
                request_id,
                completed: true,
                status: None,
            }))),
        }
    } else {
        // Get general queue info
        let (queue_length, is_processing) = queue.get_queue_info().await;
        Ok(Json(serde_json::json!({
            "queue_length": queue_length,
            "is_processing": is_processing,
        })))
    }
}

/// Cancel request
pub async fn cancel_request(
    State(queue): State<Arc<QueueService>>,
    Query(params): Query<StatusQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let request_id = params
        .request_id
        .ok_or(StatusCode::BAD_REQUEST)?;

    let cancelled = queue.cancel(&request_id).await;

    Ok(Json(serde_json::json!({
        "request_id": request_id,
        "cancelled": cancelled,
    })))
}
