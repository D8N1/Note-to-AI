pub mod duckdb_store;
pub mod lance_store;
pub mod hybrid_engine;

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// Re-exports for convenience
pub use hybrid_engine::HybridStorageEngine;
pub use duckdb_store::DuckDBStore;
pub use lance_store::LanceStore;

/// Unified storage interface for the vault system
#[async_trait::async_trait]
pub trait StorageEngine: Send + Sync {
    /// Initialize the storage engine
    async fn initialize(&self) -> Result<()>;
    
    /// Store document metadata
    async fn store_document_metadata(&self, metadata: &DocumentMetadata) -> Result<()>;
    
    /// Store document embeddings
    async fn store_document_embeddings(&self, doc_id: &str, embeddings: &DocumentEmbeddings) -> Result<()>;
    
    /// Store block-level embeddings
    async fn store_block_embeddings(&self, doc_id: &str, blocks: &[BlockEmbedding]) -> Result<()>;
    
    /// Search documents by semantic similarity
    async fn semantic_search(&self, query_vector: &[f32], limit: usize, threshold: f32) -> Result<Vec<SearchResult>>;
    
    /// Full-text search in document content
    async fn text_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
    
    /// Get document by path
    async fn get_document(&self, path: &Path) -> Result<Option<DocumentRecord>>;
    
    /// Get documents by tag
    async fn get_documents_by_tag(&self, tag: &str) -> Result<Vec<DocumentRecord>>;
    
    /// Get recently modified documents
    async fn get_recent_documents(&self, limit: usize) -> Result<Vec<DocumentRecord>>;
    
    /// Update document metadata
    async fn update_document_metadata(&self, path: &Path, metadata: &DocumentMetadata) -> Result<()>;
    
    /// Remove document and all associated data
    async fn remove_document(&self, path: &Path) -> Result<()>;
    
    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats>;
    
    /// Optimize storage (vacuum, compaction, etc.)
    async fn optimize(&self) -> Result<()>;
    
    /// Backup storage to specified path
    async fn backup(&self, backup_path: &Path) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub path: PathBuf,
    pub title: String,
    pub content_hash: String,
    pub size: u64,
    pub word_count: usize,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub file_type: FileType,
    pub language: Option<String>,
    pub custom_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentEmbeddings {
    pub document_vector: Vec<f32>,
    pub model_name: String,
    pub embedding_dimension: usize,
    pub created_at: DateTime<Utc>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEmbedding {
    pub block_id: String,
    pub block_type: BlockType,
    pub content: String,
    pub vector: Vec<f32>,
    pub start_pos: usize,
    pub end_pos: usize,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    Markdown,
    Text,
    Image,
    Audio,
    Video,
    Document,
    Code,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockType {
    Paragraph,
    Heading(u8),
    CodeBlock(Option<String>),
    Quote,
    List,
    Table,
    Callout(String),
    Math,
    Embed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: DocumentRecord,
    pub score: f32,
    pub match_type: MatchType,
    pub matched_content: Option<String>,
    pub matched_blocks: Vec<MatchedBlock>,
    pub context: SearchContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub metadata: DocumentMetadata,
    pub snippet: Option<String>,
    pub highlight: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    Semantic,
    FullText,
    Tag,
    Title,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedBlock {
    pub block_id: String,
    pub block_type: BlockType,
    pub content: String,
    pub score: f32,
    pub highlight: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    pub surrounding_content: Option<String>,
    pub related_documents: Vec<String>,
    pub related_tags: Vec<String>,
    pub backlinks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_documents: usize,
    pub total_embeddings: usize,
    pub total_blocks: usize,
    pub storage_size_bytes: u64,
    pub embedding_size_bytes: u64,
    pub metadata_size_bytes: u64,
    pub last_optimized: Option<DateTime<Utc>>,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub avg_search_latency_ms: f64,
    pub avg_indexing_time_ms: f64,
    pub cache_hit_rate: f64,
    pub total_queries: u64,
    pub total_documents_indexed: u64,
}

/// Configuration for storage engines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub base_path: PathBuf,
    pub duckdb_config: DuckDBConfig,
    pub lance_config: LanceConfig,
    pub cache_config: CacheConfig,
    pub performance_config: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuckDBConfig {
    pub database_path: PathBuf,
    pub memory_limit: Option<usize>,
    pub thread_count: Option<usize>,
    pub enable_parquet_cache: bool,
    pub max_cache_size_mb: usize,
    pub wal_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanceConfig {
    pub dataset_path: PathBuf,
    pub vector_dimension: usize,
    pub index_type: IndexType,
    pub num_partitions: Option<usize>,
    pub num_sub_quantizers: Option<usize>,
    pub max_iterations: usize,
    pub enable_compression: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    IVF,     // Inverted File Index
    HNSW,    // Hierarchical Navigable Small World
    Flat,    // Brute force (for small datasets)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enable_memory_cache: bool,
    pub max_cache_entries: usize,
    pub cache_ttl_seconds: u64,
    pub enable_disk_cache: bool,
    pub disk_cache_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub batch_size: usize,
    pub max_concurrent_operations: usize,
    pub enable_async_indexing: bool,
    pub background_optimization_interval_seconds: u64,
    pub enable_query_profiling: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("./storage"),
            duckdb_config: DuckDBConfig::default(),
            lance_config: LanceConfig::default(),
            cache_config: CacheConfig::default(),
            performance_config: PerformanceConfig::default(),
        }
    }
}

impl Default for DuckDBConfig {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("./storage/metadata.duckdb"),
            memory_limit: Some(1024 * 1024 * 1024), // 1GB
            thread_count: None, // Use system default
            enable_parquet_cache: true,
            max_cache_size_mb: 512,
            wal_mode: true,
        }
    }
}

impl Default for LanceConfig {
    fn default() -> Self {
        Self {
            dataset_path: PathBuf::from("./storage/vectors.lance"),
            vector_dimension: 384, // MiniLM default
            index_type: IndexType::IVF,
            num_partitions: Some(256),
            num_sub_quantizers: Some(16),
            max_iterations: 50,
            enable_compression: true,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_memory_cache: true,
            max_cache_entries: 10000,
            cache_ttl_seconds: 3600, // 1 hour
            enable_disk_cache: true,
            disk_cache_size_mb: 256,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            max_concurrent_operations: 8,
            enable_async_indexing: true,
            background_optimization_interval_seconds: 3600, // 1 hour
            enable_query_profiling: false,
        }
    }
}

/// Query builder for complex searches
pub struct QueryBuilder {
    query_text: Option<String>,
    query_vector: Option<Vec<f32>>,
    tags: Vec<String>,
    file_types: Vec<FileType>,
    date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    limit: usize,
    offset: usize,
    similarity_threshold: f32,
    boost_recent: bool,
    boost_tags: bool,
    include_content: bool,
    include_context: bool,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            query_text: None,
            query_vector: None,
            tags: Vec::new(),
            file_types: Vec::new(),
            date_range: None,
            limit: 10,
            offset: 0,
            similarity_threshold: 0.7,
            boost_recent: true,
            boost_tags: true,
            include_content: true,
            include_context: false,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.query_text = Some(text.into());
        self
    }

    pub fn vector(mut self, vector: Vec<f32>) -> Self {
        self.query_vector = Some(vector);
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn file_types(mut self, file_types: Vec<FileType>) -> Self {
        self.file_types = file_types;
        self
    }

    pub fn date_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.date_range = Some((start, end));
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold;
        self
    }

    pub fn boost_recent(mut self, boost: bool) -> Self {
        self.boost_recent = boost;
        self
    }

    pub fn boost_tags(mut self, boost: bool) -> Self {
        self.boost_tags = boost;
        self
    }

    pub fn include_content(mut self, include: bool) -> Self {
        self.include_content = include;
        self
    }

    pub fn include_context(mut self, include: bool) -> Self {
        self.include_context = include;
        self
    }

    /// Execute the query using the provided storage engine
    pub async fn execute(self, engine: &dyn StorageEngine) -> Result<Vec<SearchResult>> {
        // This will be implemented by the hybrid engine to coordinate
        // between DuckDB (text search) and Lance (vector search)
        
        if let Some(vector) = &self.query_vector {
            engine.semantic_search(vector, self.limit, self.similarity_threshold).await
        } else if let Some(text) = &self.query_text {
            engine.text_search(text, self.limit).await
        } else {
            Ok(Vec::new())
        }
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch operations for efficient bulk processing
pub struct BatchOperations {
    documents: Vec<DocumentMetadata>,
    embeddings: Vec<(String, DocumentEmbeddings)>,
    block_embeddings: Vec<(String, Vec<BlockEmbedding>)>,
}

impl BatchOperations {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            embeddings: Vec::new(),
            block_embeddings: Vec::new(),
        }
    }

    pub fn add_document(&mut self, metadata: DocumentMetadata) -> &mut Self {
        self.documents.push(metadata);
        self
    }

    pub fn add_embeddings(&mut self, doc_id: String, embeddings: DocumentEmbeddings) -> &mut Self {
        self.embeddings.push((doc_id, embeddings));
        self
    }

    pub fn add_block_embeddings(&mut self, doc_id: String, blocks: Vec<BlockEmbedding>) -> &mut Self {
        self.block_embeddings.push((doc_id, blocks));
        self
    }

    /// Execute all batch operations
    pub async fn execute(self, engine: &dyn StorageEngine) -> Result<BatchResult> {
        let mut results = BatchResult::default();

        // Process documents
        for doc in self.documents {
            match engine.store_document_metadata(&doc).await {
                Ok(_) => results.documents_processed += 1,
                Err(e) => results.errors.push(format!("Document {}: {}", doc.path.display(), e)),
            }
        }

        // Process embeddings
        for (doc_id, embeddings) in self.embeddings {
            match engine.store_document_embeddings(&doc_id, &embeddings).await {
                Ok(_) => results.embeddings_processed += 1,
                Err(e) => results.errors.push(format!("Embeddings {}: {}", doc_id, e)),
            }
        }

        // Process block embeddings
        for (doc_id, blocks) in self.block_embeddings {
            match engine.store_block_embeddings(&doc_id, &blocks).await {
                Ok(_) => results.block_embeddings_processed += blocks.len(),
                Err(e) => results.errors.push(format!("Block embeddings {}: {}", doc_id, e)),
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Default)]
pub struct BatchResult {
    pub documents_processed: usize,
    pub embeddings_processed: usize,
    pub block_embeddings_processed: usize,
    pub errors: Vec<String>,
}

/// Error types for storage operations
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),
    
    #[error("Vector index error: {0}")]
    VectorIndex(String),
    
    #[error("Document not found: {path}")]
    DocumentNotFound { path: PathBuf },
    
    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidVectorDimension { expected: usize, actual: usize },
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}