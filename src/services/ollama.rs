use crate::config::OllamaConfig;
use crate::models::{ChatMessage, OllamaRequest, OllamaResponse};
use anyhow::{anyhow, Result};
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use std::pin::Pin;
use std::time::Duration;

#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    config: OllamaConfig,
}

impl OllamaClient {
    pub fn new(config: OllamaConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Send a chat completion request (non-streaming)
    pub async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        model: &str,
        system_prompt: &str,
        stream: bool,
    ) -> Result<String> {
        let mut all_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        }];
        all_messages.extend_from_slice(messages);

        let request = OllamaRequest {
            model: model.to_string(),
            messages: all_messages,
            stream,
            keep_alive: Some(self.config.keep_alive.clone()),
        };

        let url = format!("{}/api/chat", self.config.api_url);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send request to Ollama: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Ollama API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Ollama response: {}", e))?;

        Ok(ollama_response
            .message
            .map(|m| m.content)
            .unwrap_or_default())
    }

    /// Send a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        messages: &[ChatMessage],
        model: &str,
        system_prompt: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<OllamaResponse>> + Send>>> {
        let mut all_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        }];
        all_messages.extend_from_slice(messages);

        let request = OllamaRequest {
            model: model.to_string(),
            messages: all_messages,
            stream: true,
            keep_alive: Some(self.config.keep_alive.clone()),
        };

        let url = format!("{}/api/chat", self.config.api_url);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send request to Ollama: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Ollama API error: {}",
                response.status()
            ));
        }

        let stream = response.bytes_stream().map(move |result| {
            result
                .map_err(|e| anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    let text = String::from_utf8(bytes.to_vec())
                        .map_err(|e| anyhow!("UTF-8 error: {}", e))?;

                    // Parse each line as JSON
                    for line in text.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<OllamaResponse>(line) {
                            Ok(response) => return Ok(response),
                            Err(e) => {
                                tracing::warn!("Failed to parse line: {} - {}", line, e);
                            }
                        }
                    }

                    // If no valid response found, return an error
                    Err(anyhow!("No valid response in chunk"))
                })
        });

        Ok(Box::pin(stream))
    }

    /// Check if Ollama is available
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.config.api_url);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_client() -> OllamaClient {
        let config = OllamaConfig {
            api_url: "http://localhost:11434".to_string(),
            model: "llama2".to_string(),
            system_prompt: "You are a helpful assistant.".to_string(),
            keep_alive: "15m".to_string(),
            timeout_seconds: 300,
        };

        OllamaClient::new(config)
    }

    #[tokio::test]
    #[ignore] // Run only when Ollama is available
    async fn test_health_check() {
        let client = create_test_client();
        let result = client.health_check().await;
        assert!(result.is_ok());
    }
}
