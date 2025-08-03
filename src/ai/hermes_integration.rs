use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use anyhow::{Result, anyhow};
use crate::ai::model_switcher::{ModelSwitcher, TaskContext};
use crate::ai::context::{ContextBuilder, ContextQuery, ContextWindow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesMessage {
    pub role: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesRequest {
    pub model: String,
    pub messages: Vec<HermesMessage>,
    pub temperature: f32,
    pub max_tokens: usize,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub stop: Option<Vec<String>>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<HermesChoice>,
    pub usage: HermesUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesChoice {
    pub index: usize,
    pub message: HermesMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct HermesConfig {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    pub timeout_seconds: u64,
    pub max_retries: usize,
    pub retry_delay_ms: u64,
}

#[derive(Debug)]
pub struct ConversationContext {
    pub messages: Vec<HermesMessage>,
    pub total_tokens: usize,
    pub max_context_length: usize,
    pub preserve_system_message: bool,
}

impl ConversationContext {
    pub fn new(max_length: usize) -> Self {
        Self {
            messages: Vec::new(),
            total_tokens: 0,
            max_context_length: max_length,
            preserve_system_message: true,
        }
    }

    pub fn add_message(&mut self, message: HermesMessage) {
        // Rough token estimation
        let estimated_tokens = message.content.len() / 4;
        
        self.messages.push(message);
        self.total_tokens += estimated_tokens;
        
        // Trim context if needed
        self.trim_if_needed();
    }

    fn trim_if_needed(&mut self) {
        while self.total_tokens > self.max_context_length && self.messages.len() > 1 {
            // Find first non-system message to remove
            let mut remove_index = 0;
            if self.preserve_system_message && !self.messages.is_empty() && self.messages[0].role == "system" {
                remove_index = 1;
            }
            
            if remove_index < self.messages.len() {
                let removed = self.messages.remove(remove_index);
                let removed_tokens = removed.content.len() / 4;
                self.total_tokens = self.total_tokens.saturating_sub(removed_tokens);
            } else {
                break;
            }
        }
    }
}

pub struct HermesIntegration {
    config: HermesConfig,
    client: Client,
    model_switcher: Arc<ModelSwitcher>,
    context_builder: Arc<ContextBuilder>,
    conversations: Arc<RwLock<std::collections::HashMap<String, ConversationContext>>>,
}

impl HermesIntegration {
    pub fn new(
        config: HermesConfig,
        model_switcher: Arc<ModelSwitcher>,
        context_builder: Arc<ContextBuilder>,
    ) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            model_switcher,
            context_builder,
            conversations: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Initialize a new conversation
    pub async fn create_conversation(&self, conversation_id: String, system_prompt: Option<String>) -> Result<()> {
        let mut conversations = self.conversations.write().await;
        
        let mut context = ConversationContext::new(8192); // Default context length
        
        if let Some(prompt) = system_prompt {
            context.add_message(HermesMessage {
                role: "system".to_string(),
                content: prompt,
                metadata: None,
            });
        }
        
        conversations.insert(conversation_id, context);
        Ok(())
    }

    /// Send a message and get response with RAG context
    pub async fn chat_with_rag(
        &self,
        conversation_id: &str,
        user_message: &str,
        rag_query: Option<ContextQuery>,
        task_context: Option<TaskContext>,
    ) -> Result<HermesResponse> {
        // Get or create conversation
        let mut conversations = self.conversations.write().await;
        let conversation = conversations.get_mut(conversation_id)
            .ok_or_else(|| anyhow!("Conversation {} not found", conversation_id))?;

        // Build RAG context if query provided
        let enhanced_message = if let Some(query) = rag_query {
            let window = ContextWindow {
                total_tokens: conversation.max_context_length,
                available_tokens: conversation.max_context_length - conversation.total_tokens,
                reserved_tokens: 200, // Reserve for completion
            };

            let context = self.context_builder.build_context(&query, &window, Some("qa")).await?;
            format!("{}\n\nUser Message: {}", context, user_message)
        } else {
            user_message.to_string()
        };

        // Add user message to conversation
        conversation.add_message(HermesMessage {
            role: "user".to_string(),
            content: enhanced_message,
            metadata: None,
        });

        // Select optimal model
        let model_name = if let Some(task_ctx) = task_context {
            self.model_switcher.select_model(&task_ctx).await?
        } else {
            self.model_switcher.get_current_model().await
                .unwrap_or_else(|| self.config.default_model.clone())
        };

        // Get model config for request parameters
        let model_config = self.model_switcher.get_model_config(&model_name).await?;

        // Build request
        let request = HermesRequest {
            model: model_name.clone(),
            messages: conversation.messages.clone(),
            temperature: model_config.temperature,
            max_tokens: model_config.max_tokens,
            top_p: 0.9,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            stop: None,
            stream: false,
        };

        // Send request with retries
        let start_time = std::time::Instant::now();
        let response = self.send_request_with_retry(&request).await?;
        let latency = start_time.elapsed().as_millis() as u64;

        // Calculate cost (rough estimation)
        let cost = response.usage.total_tokens as f64 * model_config.cost_per_token;

        // Record metrics
        self.model_switcher.record_metrics(&model_name, latency, cost, true).await;

        // Add assistant response to conversation
        if let Some(choice) = response.choices.first() {
            conversation.add_message(choice.message.clone());
        }

        drop(conversations); // Release lock

        Ok(response)
    }

    /// Send a simple chat message without RAG
    pub async fn chat(
        &self,
        conversation_id: &str,
        user_message: &str,
        task_context: Option<TaskContext>,
    ) -> Result<HermesResponse> {
        self.chat_with_rag(conversation_id, user_message, None, task_context).await
    }

    /// Send request with retry logic
    async fn send_request_with_retry(&self, request: &HermesRequest) -> Result<HermesResponse> {
        let mut last_error = None;
        
        for attempt in 0..=self.config.max_retries {
            match self.send_request(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            self.config.retry_delay_ms * (attempt as u64 + 1)
                        )).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }

    /// Send HTTP request to Hermes API
    async fn send_request(&self, request: &HermesRequest) -> Result<HermesResponse> {
        let url = format!("{}/v1/chat/completions", self.config.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("API request failed: {}", error_text));
        }

        let hermes_response: HermesResponse = response.json().await?;
        Ok(hermes_response)
    }

    /// Generate embeddings for text (if supported by model)
    pub async fn generate_embedding(&self, text: &str, model: Option<&str>) -> Result<Vec<f32>> {
        let model_name = model.unwrap_or("text-embedding-ada-002");
        let url = format!("{}/v1/embeddings", self.config.base_url);
        
        let request = serde_json::json!({
            "model": model_name,
            "input": text
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Embedding request failed"));
        }

        let embedding_response: serde_json::Value = response.json().await?;
        
        let embedding = embedding_response["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| anyhow!("Invalid embedding response"))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    /// Stream chat response (for real-time applications)
    pub async fn chat_stream(
        &self,
        conversation_id: &str,
        user_message: &str,
        callback: impl Fn(String) -> Result<()> + Send + Sync,
    ) -> Result<()> {
        // Implementation for streaming responses
        // This would use Server-Sent Events or WebSocket connection
        // For brevity, returning not implemented
        Err(anyhow!("Streaming not implemented in this example"))
    }

    /// Get conversation history
    pub async fn get_conversation(&self, conversation_id: &str) -> Result<Vec<HermesMessage>> {
        let conversations = self.conversations.read().await;
        let conversation = conversations.get(conversation_id)
            .ok_or_else(|| anyhow!("Conversation {} not found", conversation_id))?;
        
        Ok(conversation.messages.clone())
    }

    /// Clear conversation history
    pub async fn clear_conversation(&self, conversation_id: &str) -> Result<()> {
        let mut conversations = self.conversations.write().await;
        
        if let Some(conversation) = conversations.get_mut(conversation_id) {
            // Keep system message if it exists
            if !conversation.messages.is_empty() && conversation.messages[0].role == "system" {
                let system_msg = conversation.messages[0].clone();
                conversation.messages.clear();
                conversation.messages.push(system_msg);
                conversation.total_tokens = system_msg.content.len() / 4;
            } else {
                conversation.messages.clear();
                conversation.total_tokens = 0;
            }
        }
        
        Ok(())
    }

    /// Get conversation statistics
    pub async fn get_conversation_stats(&self, conversation_id: &str) -> Result<std::collections::HashMap<String, usize>> {
        let conversations = self.conversations.read().await;
        let conversation = conversations.get(conversation_id)
            .ok_or_else(|| anyhow!("Conversation {} not found", conversation_id))?;
        
        let mut stats = std::collections::HashMap::new();
        stats.insert("total_messages".to_string(), conversation.messages.len());
        stats.insert("total_tokens".to_string(), conversation.total_tokens);
        stats.insert("max_context_length".to_string(), conversation.max_context_length);
        
        // Count by role
        let mut role_counts = std::collections::HashMap::new();
        for message in &conversation.messages {
            *role_counts.entry(message.role.clone()).or_insert(0) += 1;
        }
        
        for (role, count) in role_counts {
            stats.insert(format!("messages_{}", role), count);
        }
        
        Ok(stats)
    }

    /// Batch process multiple messages
    pub async fn batch_chat(
        &self,
        requests: Vec<(String, String, Option<ContextQuery>)>, // (conversation_id, message, rag_query)
    ) -> Result<Vec<Result<HermesResponse>>> {
        let mut results = Vec::new();
        
        // Process in parallel with semaphore to limit concurrency
        let semaphore = Arc::new(tokio::sync::Semaphore::new(5)); // Max 5 concurrent requests
        let mut handles = Vec::new();
        
        for (conv_id, message, rag_query) in requests {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let self_clone = self.clone(); // Assuming Clone is implemented
            
            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold permit for duration of request
                self_clone.chat_with_rag(&conv_id, &message, rag_query, None).await
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            results.push(handle.await.unwrap());
        }
        
        Ok(results)
    }
}

// Helper trait for cloning (you might need to implement this)
impl Clone for HermesIntegration {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: self.client.clone(),
            model_switcher: Arc::clone(&self.model_switcher),
            context_builder: Arc::clone(&self.context_builder),
            conversations: Arc::clone(&self.conversations),
        }
    }
}