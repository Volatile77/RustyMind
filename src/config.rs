use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub ollama: OllamaConfig,
    pub cache: CacheConfig,
    pub conversation_cache: CacheConfig,
    pub queue: QueueConfig,
    pub batch: BatchConfig,
    pub cors: CorsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaConfig {
    pub api_url: String,
    pub model: String,
    pub system_prompt: String,
    pub keep_alive: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    pub max_size_mb: u64,
    pub ttl_seconds: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    pub max_concurrent: usize,
    pub estimated_time_per_request_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub batch_timeout_ms: u64,
    #[serde(default = "default_true")]
    pub enable_deduplication: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
}

fn default_workers() -> usize {
    4
}

fn default_timeout() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenv::dotenv().ok();

        // Try to load config file from multiple locations
        let config_path = std::env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "config.toml".to_string());

        let config = config::Config::builder()
            .add_source(config::File::with_name(&config_path).required(false))
            .add_source(config::File::with_name("config.toml").required(false))
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        Ok(config.try_deserialize()?)
    }
}
