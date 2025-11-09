mod config;
mod handlers;
mod models;
mod services;
mod utils;

use crate::config::Config;
use crate::handlers::{
    chat::{chat_optimized, AppState},
    queue::{cancel_request, enqueue_request, get_queue_status},
    stats::{get_stats, health, manage_cache, StatsState},
};
use crate::services::{BatchProcessor, CacheService, OllamaClient, QueueService};
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,chatbot_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::load()?;
    tracing::info!("Configuration loaded successfully");

    // Initialize services
    let response_cache = CacheService::new(config.cache.clone());
    let conversation_cache = CacheService::new(config.conversation_cache.clone());
    let ollama_client = OllamaClient::new(config.ollama.clone());
    let queue_service = Arc::new(QueueService::new(config.queue.clone()));

    // Check Ollama connectivity
    match ollama_client.health_check().await {
        Ok(true) => tracing::info!("✅ Ollama is available at {}", config.ollama.api_url),
        Ok(false) | Err(_) => {
            tracing::warn!("⚠️  Ollama may not be available at {}", config.ollama.api_url);
        }
    }

    // Initialize batch processor
    let batch_processor = BatchProcessor::new(
        response_cache.clone(),
        ollama_client.clone(),
        config.batch.clone(),
    );

    // Warm model on startup
    tracing::info!("🔥 Warming model...");
    if let Err(e) = batch_processor.warm_model(&config.ollama.model).await {
        tracing::warn!("Failed to warm model: {}", e);
    }

    // Create shared state for chat handler
    let app_state = Arc::new(AppState {
        cache: response_cache.clone(),
        conversation_cache: conversation_cache.clone(),
        ollama: ollama_client,
        model: config.ollama.model.clone(),
        system_prompt: config.ollama.system_prompt.clone(),
    });

    // Create shared state for stats handler
    let stats_state = Arc::new(StatsState {
        response_cache,
        conversation_cache,
        batch_processor,
        queue: queue_service.clone(),
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health))
        // Chat endpoints
        .route("/api/chat-optimized", post(chat_optimized))
        .with_state(app_state)
        // Queue endpoints
        .route("/api/chat-queue", post(enqueue_request))
        .route("/api/chat-queue", get(get_queue_status))
        .route("/api/chat-queue", delete(cancel_request))
        .with_state(queue_service)
        // Stats endpoints
        .route("/api/cache-stats", get(get_stats))
        .route("/api/cache-stats", post(manage_cache))
        .with_state(stats_state)
        // Add CORS
        .layer(cors);

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🚀 Server listening on http://{}", addr);
    tracing::info!("📊 Endpoints:");
    tracing::info!("  - POST   /api/chat-optimized");
    tracing::info!("  - POST   /api/chat-queue");
    tracing::info!("  - GET    /api/chat-queue");
    tracing::info!("  - DELETE /api/chat-queue");
    tracing::info!("  - GET    /api/cache-stats");
    tracing::info!("  - POST   /api/cache-stats");
    tracing::info!("  - GET    /health");

    axum::serve(listener, app).await?;

    Ok(())
}
`