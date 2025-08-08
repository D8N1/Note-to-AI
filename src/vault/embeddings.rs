use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::logger::Logger;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModel {
    pub name: String,
    pub model_type: ModelType,
    pub dimensions: usize,
    pub max_length: usize,
    pub model_path: std::path::PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    SentenceTransformer,
    Bert,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector {
    pub text: String,
    pub vector: Vec<f32>,
    pub model_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub block_embeddings: Option<Vec<BlockEmbedding>>, // Added missing field
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEmbedding {
    pub block_id: String,
    pub content: String,
    pub vector: Vec<f32>,
    pub start_pos: usize,
    pub end_pos: usize,
}

pub struct Embeddings {
    models: Arc<RwLock<HashMap<String, EmbeddingModel>>>,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    logger: Logger,
}

impl Embeddings {
    pub fn new() -> Result<Self> {
        Ok(Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            logger: Logger::new("Embeddings"),
        })
    }

    pub async fn add_model(&self, model: EmbeddingModel) -> Result<()> {
        let mut models = self.models.write().await;
        let model_name = model.name.clone();
        models.insert(model_name.clone(), model);
        self.logger.info(&format!("Added embedding model: {}", model_name));
        Ok(())
    }

    pub async fn embed_text(&self, text: &str, model_name: &str) -> Result<Vec<f32>> {
        // Check cache first
        let cache_key = format!("{}:{}", model_name, text);
        {
            let cache = self.cache.read().await;
            if let Some(embedding) = cache.get(&cache_key) {
                return Ok(embedding.clone());
            }
        }

        // TODO: Implement actual embedding generation
        // For now, return a dummy embedding
        let embedding = self.generate_dummy_embedding(text, model_name).await?;
        
        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, embedding.clone());
        }
        
        Ok(embedding)
    }

    async fn generate_dummy_embedding(&self, text: &str, model_name: &str) -> Result<Vec<f32>> {
        // Generate a simple hash-based embedding for testing
        let mut embedding = Vec::new();
        let text_bytes = text.as_bytes();
        
        for i in 0..384 { // Standard embedding size
            let byte_index = i % text_bytes.len();
            let byte_value = if text_bytes.is_empty() { 0 } else { text_bytes[byte_index] };
            let hash_component = (byte_value as f32 * (i as f32 + 1.0)) % 1.0;
            embedding.push(hash_component);
        }
        
        self.logger.info(&format!("Generated dummy embedding for '{}' using model '{}'", text, model_name));
        Ok(embedding)
    }

    pub async fn batch_embed(&self, texts: Vec<String>, model_name: &str) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        
        for text in texts {
            let embedding = self.embed_text(&text, model_name).await?;
            embeddings.push(embedding);
        }
        
        Ok(embeddings)
    }

    pub async fn similarity(&self, embedding1: &[f32], embedding2: &[f32]) -> Result<f32> {
        if embedding1.len() != embedding2.len() {
            return Err(anyhow::anyhow!("Embedding dimensions don't match"));
        }
        
        // Calculate cosine similarity
        let mut dot_product = 0.0;
        let mut norm1 = 0.0;
        let mut norm2 = 0.0;
        
        for (a, b) in embedding1.iter().zip(embedding2.iter()) {
            dot_product += a * b;
            norm1 += a * a;
            norm2 += b * b;
        }
        
        let similarity = if norm1 > 0.0 && norm2 > 0.0 {
            dot_product / (norm1.sqrt() * norm2.sqrt())
        } else {
            0.0
        };
        
        Ok(similarity)
    }

    pub async fn find_similar(&self, query_embedding: &[f32], embeddings: &[Vec<f32>], top_k: usize) -> Result<Vec<(usize, f32)>> {
        let mut similarities = Vec::new();
        
        for (i, embedding) in embeddings.iter().enumerate() {
            let similarity = self.similarity(query_embedding, embedding).await?;
            similarities.push((i, similarity));
        }
        
        // Sort by similarity (descending) and take top_k
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(top_k);
        
        Ok(similarities)
    }

    pub async fn load_model(&self, model_path: &Path) -> Result<EmbeddingModel> {
        // TODO: Implement model loading from safetensors
        // For now, return a dummy model
        let model = EmbeddingModel {
            name: "dummy-model".to_string(),
            model_type: ModelType::SentenceTransformer,
            dimensions: 384,
            max_length: 512,
            model_path: model_path.to_path_buf(),
            created_at: chrono::Utc::now(),
        };
        
        self.logger.info(&format!("Loaded dummy embedding model from {:?}", model_path));
        Ok(model)
    }

    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        self.logger.info("Cleared embedding cache");
        Ok(())
    }

    pub async fn get_cache_stats(&self) -> Result<HashMap<String, usize>> {
        let cache = self.cache.read().await;
        let mut stats = HashMap::new();
        stats.insert("total_embeddings".to_string(), cache.len());
        stats.insert("cache_size_bytes".to_string(), cache.values().map(|v| v.len() * 4).sum());
        Ok(stats)
    }
}