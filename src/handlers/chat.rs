use crate::models::{ChatRequest, ChatResponse, StreamChunk};
use crate::services::{CacheService, OllamaClient};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    Json,
};
use futures::stream::{Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;

pub struct AppState {
    pub cache: CacheService,
    pub conversation_cache: CacheService,
    pub ollama: OllamaClient,
    pub model: String,
    pub system_prompt: String,
}

/// Handle optimized chat request with caching
pub async fn chat_optimized(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<Response, StatusCode> {
    let model = request.model.as_ref().unwrap_or(&state.model);
    let system_prompt = request
        .system_prompt
        .as_ref()
        .unwrap_or(&state.system_prompt);

    // Check cache first
    if request.use_cache {
        let cache_key = CacheService::generate_key(&request.messages, model);

        if let Some(cached) = state.cache.get(&cache_key).await {
            tracing::info!("✅ Serving from cache");

            if request.stream {
                // Stream cached response
                let stream = stream_cached_response(cached, None);
                return Ok(Sse::new(stream).into_response());
            } else {
                let response = ChatResponse {
                    message: crate::models::ChatMessage {
                        role: "assistant".to_string(),
                        content: cached,
                    },
                    cached: Some(true),
                };
                return Ok(Json(response).into_response());
            }
        }
    }

    // Cache miss - fetch from Ollama
    if request.stream {
        match state
            .ollama
            .chat_completion_stream(&request.messages, model, system_prompt)
            .await
        {
            Ok(ollama_stream) => {
                let cache = state.cache.clone();
                let cache_key = CacheService::generate_key(&request.messages, model);
                let use_cache = request.use_cache;

                let stream = stream_ollama_response(ollama_stream, cache, cache_key, use_cache);
                Ok(Sse::new(stream).into_response())
            }
            Err(e) => {
                tracing::error!("Ollama streaming error: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        match state
            .ollama
            .chat_completion(&request.messages, model, system_prompt, false)
            .await
        {
            Ok(content) => {
                // Cache the response
                if request.use_cache {
                    let cache_key = CacheService::generate_key(&request.messages, model);
                    state.cache.set(cache_key, content.clone()).await;
                }

                let response = ChatResponse {
                    message: crate::models::ChatMessage {
                        role: "assistant".to_string(),
                        content,
                    },
                    cached: Some(false),
                };
                Ok(Json(response).into_response())
            }
            Err(e) => {
                tracing::error!("Ollama error: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Stream cached response word by word for smooth UX
fn stream_cached_response(
    content: String,
    request_id: Option<String>,
) -> impl Stream<Item = Result<axum::response::sse::Event, Infallible>> {
    let words: Vec<String> = content.split_whitespace().map(|s| s.to_string()).collect();
    let total = words.len();
    let request_id_clone = request_id.clone();

    futures::stream::iter(words.into_iter().enumerate().map(move |(i, word)| {
        let chunk = StreamChunk {
            content: Some(if i < total - 1 {
                format!("{} ", word)
            } else {
                word
            }),
            done: false,
            request_id: request_id_clone.clone(),
            cached: Some(true),
            error: None,
        };

        let json = serde_json::to_string(&chunk).unwrap();
        Ok(axum::response::sse::Event::default().data(json))
    }))
    .chain(futures::stream::once(async move {
        let chunk = StreamChunk {
            content: None,
            done: true,
            request_id,
            cached: Some(true),
            error: None,
        };

        let json = serde_json::to_string(&chunk).unwrap();
        Ok(axum::response::sse::Event::default().data(json))
    }))
}

/// Stream Ollama response and cache it
fn stream_ollama_response(
    mut ollama_stream: std::pin::Pin<
        Box<dyn Stream<Item = anyhow::Result<crate::models::OllamaResponse>> + Send>,
    >,
    cache: CacheService,
    cache_key: String,
    use_cache: bool,
) -> impl Stream<Item = Result<axum::response::sse::Event, Infallible>> {
    let accumulated = Arc::new(tokio::sync::Mutex::new(String::new()));

    async_stream::stream! {
        while let Some(result) = ollama_stream.next().await {
            match result {
                Ok(ollama_response) => {
                    if let Some(message) = &ollama_response.message {
                        // Accumulate content
                        let mut acc = accumulated.lock().await;
                        acc.push_str(&message.content);

                        let chunk = StreamChunk {
                            content: Some(message.content.clone()),
                            done: false,
                            request_id: None,
                            cached: Some(false),
                            error: None,
                        };

                        let json = serde_json::to_string(&chunk).unwrap();
                        yield Ok(axum::response::sse::Event::default().data(json));
                    }

                    if ollama_response.done {
                        // Cache the complete response
                        if use_cache {
                            let acc = accumulated.lock().await;
                            cache.set(cache_key.clone(), acc.clone()).await;
                            tracing::info!("💾 Cached streaming response");
                        }

                        let chunk = StreamChunk {
                            content: None,
                            done: true,
                            request_id: None,
                            cached: Some(false),
                            error: None,
                        };

                        let json = serde_json::to_string(&chunk).unwrap();
                        yield Ok(axum::response::sse::Event::default().data(json));
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                    let chunk = StreamChunk {
                        content: None,
                        done: true,
                        request_id: None,
                        cached: None,
                        error: Some(e.to_string()),
                    };

                    let json = serde_json::to_string(&chunk).unwrap();
                    yield Ok(axum::response::sse::Event::default().data(json));
                    break;
                }
            }
        }
    }
}
