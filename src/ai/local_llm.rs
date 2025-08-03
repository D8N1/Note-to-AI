use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore, mpsc};
use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::llama::{Llama, LlamaConfig, Cache};
use candle_transformers::models::mistral::{Model as MistralModel, Config as MistralConfig};
use candle_transformers::models::phi::{Model as PhiModel, Config as PhiConfig};
use candle_transformers::generation_utils::LogitsProcessor;
use hf_hub::api::tokio::Api;
use tokenizers::Tokenizer;
use crate::logger::Logger;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub max_new_tokens: usize,
    pub temperature: f64,
    pub top_p: f64,
    pub top_k: Option<usize>,
    pub repetition_penalty: f64,
    pub do_sample: bool,
    pub stop_tokens: Vec<String>,
    pub seed: Option<u64>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            max_new_tokens: 512,
            temperature: 0.7,
            top_p: 0.9,
            top_k: Some(50),
            repetition_penalty: 1.1,
            do_sample: true,
            stop_tokens: vec!["</s>".to_string(), "<|end|>".to_string()],
            seed: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_name: String,
    pub model_type: ModelType,
    pub device: String, // "cpu", "cuda", "metal"
    pub dtype: String,  // "f16", "f32", "bf16"
    pub max_sequence_length: usize,
    pub cache_size: usize,
    pub use_flash_attention: bool,
    pub model_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    Llama,
    Mistral,
    Phi,
    CodeLlama,
    Hermes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub prompt: String,
    pub config: GenerationConfig,
    pub context: Option<String>,
    pub system_prompt: Option<String>,
    pub chat_format: bool,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub text: String,
    pub tokens_generated: usize,
    pub generation_time_ms: u64,
    pub tokens_per_second: f64,
    pub stop_reason: StopReason,
    pub model_name: String,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StopReason {
    MaxTokens,
    StopToken,
    EndOfSequence,
    Error(String),
}

pub struct LocalLLM {
    config: ModelConfig,
    model: Arc<RwLock<Option<LoadedModel>>>,
    tokenizer: Arc<RwLock<Option<Tokenizer>>>,
    device: Device,
    semaphore: Arc<Semaphore>,
    logger: Logger,
    generation_cache: Arc<RwLock<HashMap<String, Cache>>>,
}

enum LoadedModel {
    Llama(Llama),
    Mistral(MistralModel),
    Phi(PhiModel),
}

impl LoadedModel {
    async fn forward(&mut self, input_ids: &Tensor, pos: usize) -> Result<Tensor> {
        match self {
            LoadedModel::Llama(model) => {
                model.forward(input_ids, pos).context("Llama forward pass failed")
            }
            LoadedModel::Mistral(model) => {
                model.forward(input_ids, pos).context("Mistral forward pass failed")
            }
            LoadedModel::Phi(model) => {
                model.forward(input_ids, pos).context("Phi forward pass failed")
            }
        }
    }
}

pub struct StreamingResponse {
    receiver: mpsc::Receiver<Result<String>>,
}

impl StreamingResponse {
    pub async fn next(&mut self) -> Option<Result<String>> {
        self.receiver.recv().await
    }
}

impl LocalLLM {
    pub fn new(config: ModelConfig) -> Result<Self> {
        let device = match config.device.as_str() {
            "cuda" => Device::new_cuda(0).context("CUDA device not available")?,
            "metal" => Device::new_metal(0).context("Metal device not available")?,
            _ => Device::Cpu,
        };

        Ok(Self {
            config,
            model: Arc::new(RwLock::new(None)),
            tokenizer: Arc::new(RwLock::new(None)),
            device,
            semaphore: Arc::new(Semaphore::new(1)), // Single concurrent generation for now
            logger: Logger::new("LocalLLM"),
            generation_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        self.logger.info(&format!("Initializing LLM: {} on {}", self.config.model_name, self.config.device));
        
        let start_time = std::time::Instant::now();
        
        // Load tokenizer first
        let tokenizer = self.load_tokenizer().await?;
        *self.tokenizer.write().await = Some(tokenizer);
        
        // Load model
        let model = self.load_model().await?;
        *self.model.write().await = Some(model);
        
        let duration = start_time.elapsed();
        self.logger.info(&format!("Model loaded in {:?}", duration));
        
        Ok(())
    }

    async fn load_tokenizer(&self) -> Result<Tokenizer> {
        self.logger.debug("Loading tokenizer...");
        
        if let Some(local_path) = &self.config.model_path {
            let tokenizer_path = local_path.join("tokenizer.json");
            if tokenizer_path.exists() {
                return Tokenizer::from_file(&tokenizer_path)
                    .context("Failed to load local tokenizer");
            }
        }
        
        // Download from HuggingFace
        let api = Api::new()?;
        let repo = api.model(self.config.model_name.clone());
        
        let tokenizer_path = repo.get("tokenizer.json").await
            .context("Failed to download tokenizer")?;
        
        Tokenizer::from_file(&tokenizer_path)
            .context("Failed to load tokenizer")
    }

    async fn load_model(&self) -> Result<LoadedModel> {
        self.logger.debug("Loading model weights...");
        
        let api = Api::new()?;
        let repo = api.model(self.config.model_name.clone());
        
        // Download config and weights
        let config_path = repo.get("config.json").await?;
        let weights_paths = self.get_model_weights(&repo).await?;
        
        // Load config
        let config_content = std::fs::read_to_string(&config_path)?;
        
        match self.config.model_type {
            ModelType::Llama | ModelType::CodeLlama => {
                let llama_config: LlamaConfig = serde_json::from_str(&config_content)?;
                let model = self.load_llama_model(&weights_paths, &llama_config).await?;
                Ok(LoadedModel::Llama(model))
            }
            ModelType::Mistral => {
                let mistral_config: MistralConfig = serde_json::from_str(&config_content)?;
                let model = self.load_mistral_model(&weights_paths, &mistral_config).await?;
                Ok(LoadedModel::Mistral(model))
            }
            ModelType::Phi => {
                let phi_config: PhiConfig = serde_json::from_str(&config_content)?;
                let model = self.load_phi_model(&weights_paths, &phi_config).await?;
                Ok(LoadedModel::Phi(model))
            }
            ModelType::Hermes => {
                // Hermes is typically based on Llama/Mistral
                let llama_config: LlamaConfig = serde_json::from_str(&config_content)?;
                let model = self.load_llama_model(&weights_paths, &llama_config).await?;
                Ok(LoadedModel::Llama(model))
            }
        }
    }

    async fn get_model_weights(&self, repo: &hf_hub::api::tokio::ApiRepo) -> Result<Vec<PathBuf>> {
        let mut weights_paths = Vec::new();
        
        // Try safetensors first
        if let Ok(path) = repo.get("model.safetensors").await {
            weights_paths.push(path);
            return Ok(weights_paths);
        }
        
        // Try sharded safetensors
        let mut shard_index = 1;
        loop {
            let shard_name = format!("model-{:05}-of-{:05}.safetensors", shard_index, 99999);
            if let Ok(path) = repo.get(&shard_name).await {
                weights_paths.push(path);
                shard_index += 1;
            } else {
                break;
            }
            
            if shard_index > 100 { // Safety limit
                break;
            }
        }
        
        if !weights_paths.is_empty() {
            return Ok(weights_paths);
        }
        
        // Fallback to PyTorch
        if let Ok(path) = repo.get("pytorch_model.bin").await {
            weights_paths.push(path);
            return Ok(weights_paths);
        }
        
        anyhow::bail!("No supported model weights found")
    }

    async fn load_llama_model(&self, weights_paths: &[PathBuf], config: &LlamaConfig) -> Result<Llama> {
        use candle_core::safetensors::load;
        
        let dtype = match self.config.dtype.as_str() {
            "f16" => DType::F16,
            "bf16" => DType::BF16,
            _ => DType::F32,
        };
        
        let mut tensors = HashMap::new();
        
        for weights_path in weights_paths {
            let file_tensors = load(weights_path, &self.device)?;
            tensors.extend(file_tensors);
        }
        
        let var_builder = VarBuilder::from_tensors(tensors, dtype, &self.device);
        
        Llama::load(&var_builder, config)
            .context("Failed to load Llama model")
    }

    async fn load_mistral_model(&self, weights_paths: &[PathBuf], config: &MistralConfig) -> Result<MistralModel> {
        use candle_core::safetensors::load;
        
        let dtype = match self.config.dtype.as_str() {
            "f16" => DType::F16,
            "bf16" => DType::BF16,
            _ => DType::F32,
        };
        
        let mut tensors = HashMap::new();
        
        for weights_path in weights_paths {
            let file_tensors = load(weights_path, &self.device)?;
            tensors.extend(file_tensors);
        }
        
        let var_builder = VarBuilder::from_tensors(tensors, dtype, &self.device);
        
        MistralModel::load(&var_builder, config)
            .context("Failed to load Mistral model")
    }

    async fn load_phi_model(&self, weights_paths: &[PathBuf], config: &PhiConfig) -> Result<PhiModel> {
        use candle_core::safetensors::load;
        
        let dtype = match self.config.dtype.as_str() {
            "f16" => DType::F16,
            "bf16" => DType::BF16,
            _ => DType::F32,
        };
        
        let mut tensors = HashMap::new();
        
        for weights_path in weights_paths {
            let file_tensors = load(weights_path, &self.device)?;
            tensors.extend(file_tensors);
        }
        
        let var_builder = VarBuilder::from_tensors(tensors, dtype, &self.device);
        
        PhiModel::load(&var_builder, config)
            .context("Failed to load Phi model")
    }

    pub async fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse> {
        let _permit = self.semaphore.acquire().await?;
        let start_time = std::time::Instant::now();
        
        self.logger.debug(&format!("Generating response for prompt: {:.50}...", request.prompt));
        
        // Format the prompt
        let formatted_prompt = self.format_prompt(&request)?;
        
        // Tokenize
        let tokens = self.tokenize(&formatted_prompt).await?;
        
        // Generate
        let (generated_tokens, stop_reason) = self.generate_tokens(&tokens, &request.config).await?;
        
        // Decode
        let generated_text = self.decode_tokens(&generated_tokens).await?;
        
        let generation_time = start_time.elapsed();
        let tokens_per_second = generated_tokens.len() as f64 / generation_time.as_secs_f64();
        
        Ok(GenerationResponse {
            text: generated_text,
            tokens_generated: generated_tokens.len(),
            generation_time_ms: generation_time.as_millis() as u64,
            tokens_per_second,
            stop_reason,
            model_name: self.config.model_name.clone(),
            finish_reason: "completed".to_string(),
        })
    }

    pub async fn generate_stream(&self, request: GenerationRequest) -> Result<StreamingResponse> {
        let (sender, receiver) = mpsc::channel(32);
        let self_clone = self.clone_for_streaming().await?;
        
        tokio::spawn(async move {
            if let Err(e) = self_clone.generate_stream_impl(request, sender.clone()).await {
                let _ = sender.send(Err(e)).await;
            }
        });
        
        Ok(StreamingResponse { receiver })
    }

    async fn clone_for_streaming(&self) -> Result<Self> {
        // Create a lightweight clone for streaming
        // In practice, you might want to share the model between instances
        Ok(Self {
            config: self.config.clone(),
            model: self.model.clone(),
            tokenizer: self.tokenizer.clone(),
            device: self.device.clone(),
            semaphore: Arc::new(Semaphore::new(1)),
            logger: Logger::new("LocalLLM-Stream"),
            generation_cache: self.generation_cache.clone(),
        })
    }

    async fn generate_stream_impl(
        &self,
        request: GenerationRequest,
        sender: mpsc::Sender<Result<String>>,
    ) -> Result<()> {
        let _permit = self.semaphore.acquire().await?;
        
        let formatted_prompt = self.format_prompt(&request)?;
        let mut tokens = self.tokenize(&formatted_prompt).await?;
        
        let tokenizer_guard = self.tokenizer.read().await;
        let tokenizer = tokenizer_guard.as_ref().context("Tokenizer not loaded")?;
        
        let mut model_guard = self.model.write().await;
        let model = model_guard.as_mut().context("Model not loaded")?;
        
        let mut logits_processor = LogitsProcessor::new(
            request.config.seed.unwrap_or(299792458),
            Some(request.config.temperature),
            Some(request.config.top_p),
        );
        
        for index in 0..request.config.max_new_tokens {
            let input_tensor = Tensor::new(&tokens[..], &self.device)?
                .unsqueeze(0)?;
            
            let logits = model.forward(&input_tensor, tokens.len() - 1).await?;
            let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;
            
            let next_token = logits_processor.sample(&logits)?;
            tokens.push(next_token);
            
            // Decode the new token
            if let Ok(new_text) = tokenizer.decode(&[next_token], true) {
                if !new_text.is_empty() {
                    if sender.send(Ok(new_text.clone())).await.is_err() {
                        break; // Client disconnected
                    }
                }
            }
            
            // Check for stop tokens
            if let Ok(current_text) = tokenizer.decode(&tokens, true) {
                if request.config.stop_tokens.iter().any(|stop| current_text.ends_with(stop)) {
                    break;
                }
            }
            
            // Check for EOS token
            if next_token == tokenizer.token_to_id("</s>").unwrap_or(u32::MAX) {
                break;
            }
        }
        
        Ok(())
    }

    async fn generate_tokens(&self, prompt_tokens: &[u32], config: &GenerationConfig) -> Result<(Vec<u32>, StopReason)> {
        let mut tokens = prompt_tokens.to_vec();
        let mut generated_tokens = Vec::new();
        
        let tokenizer_guard = self.tokenizer.read().await;
        let tokenizer = tokenizer_guard.as_ref().context("Tokenizer not loaded")?;
        
        let mut model_guard = self.model.write().await;
        let model = model_guard.as_mut().context("Model not loaded")?;
        
        let mut logits_processor = LogitsProcessor::new(
            config.seed.unwrap_or(299792458),
            Some(config.temperature),
            Some(config.top_p),
        );
        
        for _ in 0..config.max_new_tokens {
            let input_tensor = Tensor::new(&tokens[..], &self.device)?
                .unsqueeze(0)?;
            
            let logits = model.forward(&input_tensor, tokens.len() - 1).await?;
            let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;
            
            // Apply repetition penalty
            let logits = if config.repetition_penalty != 1.0 {
                self.apply_repetition_penalty(&logits, &tokens, config.repetition_penalty)?
            } else {
                logits
            };
            
            let next_token = if config.do_sample {
                logits_processor.sample(&logits)?
            } else {
                logits.argmax(0)?.to_scalar::<u32>()?
            };
            
            tokens.push(next_token);
            generated_tokens.push(next_token);
            
            // Check for stop conditions
            if let Ok(current_text) = tokenizer.decode(&tokens, true) {
                if config.stop_tokens.iter().any(|stop| current_text.ends_with(stop)) {
                    return Ok((generated_tokens, StopReason::StopToken));
                }
            }
            
            // Check for EOS token
            if next_token == tokenizer.token_to_id("</s>").unwrap_or(u32::MAX) {
                return Ok((generated_tokens, StopReason::EndOfSequence));
            }
        }
        
        Ok((generated_tokens, StopReason::MaxTokens))
    }

    fn apply_repetition_penalty(&self, logits: &Tensor, tokens: &[u32], penalty: f64) -> Result<Tensor> {
        let mut logits_vec = logits.to_vec1::<f32>()?;
        
        // Count token frequencies in the context
        let mut token_counts = HashMap::new();
        for &token in tokens {
            *token_counts.entry(token).or_insert(0) += 1;
        }
        
        // Apply penalty
        for (token, count) in token_counts {
            if let Some(logit) = logits_vec.get_mut(token as usize) {
                if *logit > 0.0 {
                    *logit /= penalty.powf(count as f64) as f32;
                } else {
                    *logit *= penalty.powf(count as f64) as f32;
                }
            }
        }
        
        Tensor::new(&logits_vec[..], &self.device)
    }

    async fn tokenize(&self, text: &str) -> Result<Vec<u32>> {
        let tokenizer_guard = self.tokenizer.read().await;
        let tokenizer = tokenizer_guard.as_ref().context("Tokenizer not loaded")?;
        
        let encoding = tokenizer.encode(text, false)
            .context("Failed to tokenize text")?;
        
        Ok(encoding.get_ids().to_vec())
    }

    async fn decode_tokens(&self, tokens: &[u32]) -> Result<String> {
        let tokenizer_guard = self.tokenizer.read().await;
        let tokenizer = tokenizer_guard.as_ref().context("Tokenizer not loaded")?;
        
        tokenizer.decode(tokens, true)
            .context("Failed to decode tokens")
    }

    fn format_prompt(&self, request: &GenerationRequest) -> Result<String> {
        if request.chat_format {
            self.format_chat_prompt(request)
        } else {
            Ok(request.prompt.clone())
        }
    }

    fn format_chat_prompt(&self, request: &GenerationRequest) -> Result<String> {
        let mut formatted = String::new();
        
        // Add system prompt if provided
        if let Some(system) = &request.system_prompt {
            formatted.push_str(&format!("<|system|>\n{}\n", system));
        }
        
        // Add context if provided
        if let Some(context) = &request.context {
            formatted.push_str(&format!("<|context|>\n{}\n", context));
        }
        
        // Add user prompt
        formatted.push_str(&format!("<|user|>\n{}\n<|assistant|>\n", request.prompt));
        
        Ok(formatted)
    }

    pub async fn get_model_info(&self) -> Result<ModelInfo> {
        let model_guard = self.model.read().await;
        let is_loaded = model_guard.is_some();
        
        Ok(ModelInfo {
            name: self.config.model_name.clone(),
            model_type: self.config.model_type.clone(),
            device: self.config.device.clone(),
            dtype: self.config.dtype.clone(),
            max_sequence_length: self.config.max_sequence_length,
            is_loaded,
            memory_usage: self.estimate_memory_usage().await?,
        })
    }

    async fn estimate_memory_usage(&self) -> Result<u64> {
        // Rough estimation based on model parameters and dtype
        let base_memory = match self.config.model_type {
            ModelType::Llama => 7_000_000_000, // 7B parameters
            ModelType::Mistral => 7_000_000_000,
            ModelType::Phi => 3_000_000_000,   // 3B parameters
            ModelType::CodeLlama => 7_000_000_000,
            ModelType::Hermes => 7_000_000_000,
        };
        
        let bytes_per_param = match self.config.dtype.as_str() {
            "f16" | "bf16" => 2,
            "f32" => 4,
            _ => 4,
        };
        
        Ok(base_memory * bytes_per_param)
    }

    pub async fn unload_model(&self) -> Result<()> {
        let mut model_guard = self.model.write().await;
        *model_guard = None;
        
        let mut tokenizer_guard = self.tokenizer.write().await;
        *tokenizer_guard = None;
        
        // Clear cache
        let mut cache_guard = self.generation_cache.write().await;
        cache_guard.clear();
        
        self.logger.info("Model unloaded and memory freed");
        Ok(())
    }

    pub async fn warm_up(&self) -> Result<()> {
        self.logger.info("Warming up model...");
        
        let warm_up_request = GenerationRequest {
            prompt: "Hello".to_string(),
            config: GenerationConfig {
                max_new_tokens: 5,
                temperature: 0.7,
                ..Default::default()
            },
            context: None,
            system_prompt: None,
            chat_format: false,
            stream: false,
        };
        
        let _response = self.generate(warm_up_request).await?;
        self.logger.info("Model warm-up completed");
        
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub model_type: ModelType,
    pub device: String,
    pub dtype: String,
    pub max_sequence_length: usize,
    pub is_loaded: bool,
    pub memory_usage: u64,
}

// Factory functions for common models
impl LocalLLM {
    pub fn llama_7b_chat(device: Device) -> Result<Self> {
        let config = ModelConfig {
            model_name: "meta-llama/Llama-2-7b-chat-hf".to_string(),
            model_type: ModelType::Llama,
            device: device.to_string(),
            dtype: "f16".to_string(),
            max_sequence_length: 4096,
            cache_size: 1024,
            use_flash_attention: true,
            model_path: None,
        };
        
        Self::new(config)
    }
    
    pub fn mistral_7b_instruct(device: Device) -> Result<Self> {
        let config = ModelConfig {
            model_name: "mistralai/Mistral-7B-Instruct-v0.1".to_string(),
            model_type: ModelType::Mistral,
            device: device.to_string(),
            dtype: "f16".to_string(),
            max_sequence_length: 8192,
            cache_size: 1024,
            use_flash_attention: true,
            model_path: None,
        };
        
        Self::new(config)
    }
    
    pub fn code_llama_7b(device: Device) -> Result<Self> {
        let config = ModelConfig {
            model_name: "codellama/CodeLlama-7b-Instruct-hf".to_string(),
            model_type: ModelType::CodeLlama,
            device: device.to_string(),
            dtype: "f16".to_string(),
            max_sequence_length: 4096,
            cache_size: 1024,
            use_flash_attention: true,
            model_path: None,
        };
        
        Self::new(config)
    }
    
    pub fn phi_3_mini(device: Device) -> Result<Self> {
        let config = ModelConfig {
            model_name: "microsoft/Phi-3-mini-4k-instruct".to_string(),
            model_type: ModelType::Phi,
            device: device.to_string(),
            dtype: "f16".to_string(),
            max_sequence_length: 4096,
            cache_size: 512,
            use_flash_attention: false,
            model_path: None,
        };
        
        Self::new(config)
    }
}

trait DeviceString {
    fn to_string(&self) -> String;
}

impl DeviceString for Device {
    fn to_string(&self) -> String {
        match self {
            Device::Cpu => "cpu".to_string(),
            Device::Cuda(_) => "cuda".to_string(),
            Device::Metal(_) => "metal".to_string(),
        }
    }
}