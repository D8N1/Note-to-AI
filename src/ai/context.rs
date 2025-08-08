use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub embedding: Option<Vec<f32>>,
    pub chunk_index: usize,
    pub source: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct RetrievalResult {
    pub document: Document,
    pub similarity_score: f32,
    pub relevance_score: f32,
    pub context_position: usize,
}

#[derive(Debug, Clone)]
pub struct ContextQuery {
    pub query: String,
    pub query_embedding: Option<Vec<f32>>,
    pub filters: HashMap<String, String>,
    pub max_results: usize,
    pub min_similarity: f32,
    pub context_window: usize,
    pub include_metadata: bool,
}

#[derive(Debug, Clone)]
pub struct ContextWindow {
    pub total_tokens: usize,
    pub available_tokens: usize,
    pub reserved_tokens: usize, // For system prompt, etc.
}

#[derive(Debug)]
pub struct ContextBuilder {
    documents: Arc<RwLock<HashMap<String, Document>>>,
    embeddings_cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    context_templates: Arc<RwLock<HashMap<String, String>>>,
}

impl ContextBuilder {
    pub fn new() -> Self {
        let builder = Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            embeddings_cache: Arc::new(RwLock::new(HashMap::new())),
            context_templates: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // TODO: Initialize default templates in a better way
        // For now, just return the builder without async initialization
        
        builder
    }

    async fn init_default_templates(&self) {
        let mut templates = self.context_templates.write().await;
        
        templates.insert("default".to_string(), 
            "Based on the following context, please answer the question:\n\nContext:\n{context}\n\nQuestion: {query}\n\nAnswer:".to_string());
        
        templates.insert("summarization".to_string(),
            "Please summarize the following information:\n\n{context}\n\nSummary:".to_string());
        
        templates.insert("qa".to_string(),
            "Context Information:\n{context}\n\nUser Question: {query}\n\nProvide a detailed answer based on the context:".to_string());
        
        templates.insert("reasoning".to_string(),
            "Given the following information:\n\n{context}\n\nReasoning Task: {query}\n\nPlease think step by step:".to_string());
    }

    /// Add documents to the knowledge base
    pub async fn add_documents(&self, docs: Vec<Document>) -> Result<()> {
        let mut documents = self.documents.write().await;
        
        for doc in docs {
            if let Some(embedding) = &doc.embedding {
                let mut cache = self.embeddings_cache.write().await;
                cache.insert(doc.id.clone(), embedding.clone());
            }
            documents.insert(doc.id.clone(), doc);
        }
        
        Ok(())
    }

    /// Compute cosine similarity between two vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm_a * norm_b)
    }

    /// Retrieve relevant documents based on query
    pub async fn retrieve_documents(&self, query: &ContextQuery) -> Result<Vec<RetrievalResult>> {
        let documents = self.documents.read().await;
        let embeddings_cache = self.embeddings_cache.read().await;
        
        let mut results = Vec::new();
        
        for (doc_id, document) in documents.iter() {
            // Apply filters
            let mut passes_filter = true;
            for (key, value) in &query.filters {
                if let Some(doc_value) = document.metadata.get(key) {
                    if doc_value != value {
                        passes_filter = false;
                        break;
                    }
                } else {
                    passes_filter = false;
                    break;
                }
            }
            
            if !passes_filter {
                continue;
            }
            
            // Calculate similarity
            let similarity_score = if let (Some(query_emb), Some(doc_emb)) = 
                (&query.query_embedding, embeddings_cache.get(doc_id)) {
                self.cosine_similarity(query_emb, doc_emb)
            } else {
                // Fallback to text-based similarity (simple keyword matching)
                self.text_similarity(&query.query, &document.content)
            };
            
            if similarity_score >= query.min_similarity {
                // Calculate relevance score (combines similarity with recency and metadata)
                let relevance_score = self.calculate_relevance_score(
                    similarity_score, 
                    document, 
                    &query.query
                );
                
                results.push(RetrievalResult {
                    document: document.clone(),
                    similarity_score,
                    relevance_score,
                    context_position: 0, // Will be set during context building
                });
            }
        }
        
        // Sort by relevance score
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        // Limit results
        results.truncate(query.max_results);
        
        Ok(results)
    }

    /// Simple text-based similarity fallback
    fn text_similarity(&self, query: &str, content: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        let query_words: std::collections::HashSet<&str> = query_lower
            .split_whitespace()
            .collect();
        
        let content_words: std::collections::HashSet<&str> = content_lower
            .split_whitespace()
            .collect();
        
        let intersection: std::collections::HashSet<_> = query_words
            .intersection(&content_words)
            .collect();
        
        if query_words.is_empty() {
            return 0.0;
        }
        
        intersection.len() as f32 / query_words.len() as f32
    }

    /// Calculate relevance score considering multiple factors
    fn calculate_relevance_score(&self, similarity: f32, document: &Document, query: &str) -> f32 {
        let mut score = similarity * 0.7; // Base similarity weight
        
        // Recency boost (more recent documents get slight preference)
        let now = chrono::Utc::now();
        let age_hours = (now - document.timestamp).num_hours() as f32;
        let recency_factor = 1.0 / (1.0 + age_hours / 168.0); // Decay over weeks
        score += recency_factor * 0.1;
        
        // Length penalty for very short or very long documents
        let ideal_length = 500.0;
        let length_factor = 1.0 - ((document.content.len() as f32 - ideal_length).abs() / ideal_length).min(1.0);
        score += length_factor * 0.1;
        
        // Metadata boost for high-priority documents
        if let Some(priority) = document.metadata.get("priority") {
            if let Ok(priority_val) = priority.parse::<f32>() {
                score += (priority_val / 10.0) * 0.1;
            }
        }
        
        score.min(1.0)
    }

    /// Build context string from retrieved documents
    pub async fn build_context(
        &self, 
        query: &ContextQuery, 
        window: &ContextWindow,
        template_name: Option<&str>
    ) -> Result<String> {
        let results = self.retrieve_documents(query).await?;
        
        if results.is_empty() {
            return Ok("No relevant context found.".to_string());
        }
        
        // Build context within token limits
        let mut context_parts = Vec::new();
        let mut used_tokens = 0;
        let max_context_tokens = window.available_tokens - window.reserved_tokens;
        
        for (i, result) in results.iter().enumerate() {
            let doc_content = if query.include_metadata {
                format!(
                    "[Source: {}]\n{}\n[Metadata: {:?}]\n",
                    result.document.source,
                    result.document.content,
                    result.document.metadata
                )
            } else {
                format!("[Source: {}]\n{}\n", result.document.source, result.document.content)
            };
            
            // Rough token estimation (4 chars ≈ 1 token)
            let estimated_tokens = doc_content.len() / 4;
            
            if used_tokens + estimated_tokens > max_context_tokens {
                // Try to fit partial content
                let remaining_chars = (max_context_tokens - used_tokens) * 4;
                if remaining_chars > 100 { // Minimum useful content
                    let truncated = format!(
                        "[Source: {}]\n{}...\n",
                        result.document.source,
                        &result.document.content[..remaining_chars.min(result.document.content.len())]
                    );
                    context_parts.push(truncated);
                }
                break;
            }
            
            context_parts.push(doc_content);
            used_tokens += estimated_tokens;
        }
        
        let context_content = context_parts.join("\n---\n");
        
        // Apply template
        let template_name = template_name.unwrap_or("default");
        let templates = self.context_templates.read().await;
        let template = templates.get(template_name)
            .unwrap_or(templates.get("default").unwrap());
        
        let final_context = template
            .replace("{context}", &context_content)
            .replace("{query}", &query.query);
        
        Ok(final_context)
    }

    /// Add custom context template
    pub async fn add_template(&self, name: String, template: String) {
        let mut templates = self.context_templates.write().await;
        templates.insert(name, template);
    }

    /// Get context statistics
    pub async fn get_stats(&self) -> HashMap<String, usize> {
        let documents = self.documents.read().await;
        let embeddings = self.embeddings_cache.read().await;
        
        let mut stats = HashMap::new();
        stats.insert("total_documents".to_string(), documents.len());
        stats.insert("documents_with_embeddings".to_string(), embeddings.len());
        
        // Count by source
        let mut source_counts = HashMap::new();
        for doc in documents.values() {
            *source_counts.entry(doc.source.clone()).or_insert(0) += 1;
        }
        
        for (source, count) in source_counts {
            stats.insert(format!("source_{}", source), count);
        }
        
        stats
    }

    /// Clear all documents and cache
    pub async fn clear(&self) {
        let mut documents = self.documents.write().await;
        let mut embeddings = self.embeddings_cache.write().await;
        
        documents.clear();
        embeddings.clear();
    }

    /// Remove documents by filter
    pub async fn remove_documents(&self, filters: HashMap<String, String>) -> Result<usize> {
        let mut documents = self.documents.write().await;
        let mut embeddings = self.embeddings_cache.write().await;
        
        let mut removed_count = 0;
        let mut to_remove = Vec::new();
        
        for (doc_id, document) in documents.iter() {
            let mut matches = true;
            for (key, value) in &filters {
                if document.metadata.get(key) != Some(value) {
                    matches = false;
                    break;
                }
            }
            
            if matches {
                to_remove.push(doc_id.clone());
            }
        }
        
        for doc_id in to_remove {
            documents.remove(&doc_id);
            embeddings.remove(&doc_id);
            removed_count += 1;
        }
        
        Ok(removed_count)
    }
}