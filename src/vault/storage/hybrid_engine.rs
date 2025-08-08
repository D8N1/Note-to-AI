use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, Context};
use tokio::sync::RwLock;
use tracing::{info, debug, error, instrument};

use super::{
    StorageEngine, DuckDBStore, LanceStore, StorageConfig,
    DocumentMetadata, DocumentEmbeddings, BlockEmbedding,
    SearchResult, DocumentRecord, StorageStats, MatchType,
    QueryBuilder, BatchOperations
};

/// Hybrid storage engine that coordinates DuckDB (metadata/text) and Lance (vectors)
pub struct HybridStorageEngine {
    duckdb: Arc<DuckDBStore>,
    lance: Arc<LanceStore>,
    config: StorageConfig,
    stats: Arc<RwLock<RuntimeStats>>,
}

#[derive(Debug, Default)]
struct RuntimeStats {
    queries_executed: u64,
    hybrid_searches: u64,
    documents_indexed: u64,
    avg_query_time_ms: f64,
    cache_hits: u64,
    cache_misses: u64,
}

impl HybridStorageEngine {
    /// Create a new hybrid storage engine
    pub async fn new(config: StorageConfig) -> Result<Self> {
        info!("Initializing hybrid storage engine");
        
        // Create storage directories
        tokio::fs::create_dir_all(&config.base_path).await?;
        tokio::fs::create_dir_all(&config.duckdb_config.database_path.parent().unwrap_or(&config.base_path)).await?;
        tokio::fs::create_dir_all(&config.lance_config.dataset_path.parent().unwrap_or(&config.base_path)).await?;
        
        // Initialize DuckDB store
        let duckdb = Arc::new(DuckDBStore::new(config.duckdb_config.clone()).await?);
        
        // Initialize Lance store
        let lance = Arc::new(LanceStore::new(config.lance_config.clone()).await?);
        
        let engine = Self {
            duckdb,
            lance,
            config,
            stats: Arc::new(RwLock::new(RuntimeStats::default())),
        };
        
        info!("Hybrid storage engine initialized successfully");
        Ok(engine)
    }
    
    /// Perform a hybrid search combining semantic and text search
    #[instrument(skip(self, query_vector, query_text))]
    pub async fn hybrid_search(
        &self,
        query_vector: Option<&[f32]>,
        query_text: Option<&str>,
        limit: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<SearchResult>> {
        let start_time = std::time::Instant::now();
        
        debug!("Executing hybrid search with limit={}, threshold={}", limit, similarity_threshold);
        
        let mut semantic_results = Vec::new();
        let mut text_results = Vec::new();
        
        // Execute semantic search if vector provided
        if let Some(vector) = query_vector {
            match self.lance.semantic_search(vector, limit * 2, similarity_threshold).await {
                Ok(results) => {
                    semantic_results = results;
                    debug!("Semantic search returned {} results", semantic_results.len());
                }
                Err(e) => error!("Semantic search failed: {}", e),
            }
        }
        
        // Execute text search if text provided
        if let Some(text) = query_text {
            match self.duckdb.text_search(text, limit * 2).await {
                Ok(results) => {
                    text_results = results;
                    debug!("Text search returned {} results", text_results.len());
                }
                Err(e) => error!("Text search failed: {}", e),
            }
        }
        
        // Merge and rank results
        let merged_results = self.merge_search_results(
            semantic_results,
            text_results,
            limit,
            query_vector.is_some() && query_text.is_some(),
        ).await?;
        
        // Update stats
        let query_time = start_time.elapsed().as_millis() as f64;
        self.update_query_stats(query_time, true).await;
        
        info!("Hybrid search completed in {:.2}ms with {} results", query_time, merged_results.len());
        Ok(merged_results)
    }
    
    /// Merge search results from different sources with intelligent ranking
    async fn merge_search_results(
        &self,
        mut semantic_results: Vec<SearchResult>,
        mut text_results: Vec<SearchResult>,
        limit: usize,
        is_hybrid: bool,
    ) -> Result<Vec<SearchResult>> {
        use std::collections::HashMap;
        
        let mut result_map: HashMap<String, SearchResult> = HashMap::new();
        let hybrid_boost = if is_hybrid { 1.2 } else { 1.0 };
        
        // Process semantic results
        for mut result in semantic_results {
            let doc_path = result.document.metadata.path.to_string_lossy().to_string();
            result.score *= 1.0; // Base semantic score
            if is_hybrid {
                result.match_type = MatchType::Hybrid;
            }
            result_map.insert(doc_path, result);
        }
        
        // Process text results and merge with semantic
        for mut result in text_results {
            let doc_path = result.document.metadata.path.to_string_lossy().to_string();
            
            if let Some(existing) = result_map.get_mut(&doc_path) {
                // Combine scores for documents found in both searches
                existing.score = (existing.score + result.score * 0.8) * hybrid_boost;
                existing.match_type = MatchType::Hybrid;
                
                // Merge matched content
                if let Some(text_content) = result.matched_content {
                    if existing.matched_content.is_none() {
                        existing.matched_content = Some(text_content);
                    }
                }
            } else {
                // Add text-only results
                result.score *= 0.8; // Slightly lower weight for text-only
                if is_hybrid {
                    result.match_type = MatchType::Hybrid;
                }
                result_map.insert(doc_path, result);
            }
        }
        
        // Convert to sorted vector
        let mut results: Vec<SearchResult> = result_map.into_values().collect();
        
        // Apply recency boost
        let now = chrono::Utc::now();
        for result in &mut results {
            let age_days = (now - result.document.metadata.modified_at).num_days();
            if age_days <= 7 {
                result.score *= 1.1; // Recent boost
            } else if age_days <= 30 {
                result.score *= 1.05; // Mild recent boost
            }
        }
        
        // Sort by score and limit
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        
        Ok(results)
    }
    
    /// Get comprehensive analytics about the vault
    pub async fn get_analytics(&self) -> Result<VaultAnalytics> {
        let duckdb_stats = self.duckdb.get_stats().await?;
        let lance_stats = self.lance.get_stats().await?;
        let runtime_stats = self.stats.read().await;
        
        Ok(VaultAnalytics {
            total_documents: duckdb_stats.total_documents,
            total_embeddings: lance_stats.total_embeddings,
            total_queries: runtime_stats.queries_executed,
            avg_query_time_ms: runtime_stats.avg_query_time_ms,
            cache_hit_rate: if runtime_stats.cache_hits + runtime_stats.cache_misses > 0 {
                runtime_stats.cache_hits as f64 / (runtime_stats.cache_hits + runtime_stats.cache_misses) as f64
            } else {
                0.0
            },
            storage_breakdown: StorageBreakdown {
                metadata_size_mb: duckdb_stats.metadata_size_bytes as f64 / (1024.0 * 1024.0),
                vector_size_mb: lance_stats.embedding_size_bytes as f64 / (1024.0 * 1024.0),
                total_size_mb: (duckdb_stats.storage_size_bytes + lance_stats.storage_size_bytes) as f64 / (1024.0 * 1024.0),
            },
            top_tags: self.get_top_tags(10).await?,
            recent_activity: self.get_recent_activity(50).await?,
        })
    }
    
    /// Get most frequently used tags
    async fn get_top_tags(&self, limit: usize) -> Result<Vec<TagStats>> {
        self.duckdb.get_top_tags(limit).await
    }
    
    /// Get recent document activity
    async fn get_recent_activity(&self, limit: usize) -> Result<Vec<ActivityRecord>> {
        self.duckdb.get_recent_activity(limit).await
    }
    
    /// Update query statistics
    async fn update_query_stats(&self, query_time_ms: f64, was_hybrid: bool) {
        let mut stats = self.stats.write().await;
        stats.queries_executed += 1;
        if was_hybrid {
            stats.hybrid_searches += 1;
        }
        
        // Update running average
        let total_queries = stats.queries_executed as f64;
        stats.avg_query_time_ms = (stats.avg_query_time_ms * (total_queries - 1.0) + query_time_ms) / total_queries;
    }
    
    /// Optimize both storage systems
    #[instrument(skip(self))]
    pub async fn optimize_all(&self) -> Result<OptimizationReport> {
        info!("Starting comprehensive storage optimization");
        let start_time = std::time::Instant::now();
        
        // Run optimizations in parallel
        let (duckdb_result, lance_result) = tokio::join!(
            self.duckdb.optimize(),
            self.lance.optimize()
        );
        
        let optimization_time = start_time.elapsed();
        
        let report = OptimizationReport {
            duration_ms: optimization_time.as_millis() as u64,
            duckdb_optimized: duckdb_result.is_ok(),
            lance_optimized: lance_result.is_ok(),
            errors: {
                let mut errors = Vec::new();
                if let Err(e) = duckdb_result {
                    errors.push(format!("DuckDB optimization failed: {}", e));
                }
                if let Err(e) = lance_result {
                    errors.push(format!("Lance backup failed: {}", e));
                }
                errors
            },
        };
        
        info!("Backup completed in {:?}", backup_time);
        Ok(report)
    }
    
    /// Calculate total backup size
    async fn calculate_backup_size(&self, duckdb_path: &Path, lance_path: &Path) -> Result<u64> {
        let mut total_size = 0u64;
        
        // Calculate DuckDB backup size
        if let Ok(entries) = tokio::fs::read_dir(duckdb_path).await {
            let mut entries = entries;
            while let Some(entry) = entries.next_entry().await? {
                if let Ok(metadata) = entry.metadata().await {
                    total_size += metadata.len();
                }
            }
        }
        
        // Calculate Lance backup size
        if let Ok(entries) = tokio::fs::read_dir(lance_path).await {
            let mut entries = entries;
            while let Some(entry) = entries.next_entry().await? {
                if let Ok(metadata) = entry.metadata().await {
                    total_size += metadata.len();
                }
            }
        }
        
        Ok(total_size)
    }
}

#[async_trait::async_trait]
impl StorageEngine for HybridStorageEngine {
    async fn initialize(&self) -> Result<()> {
        info!("Initializing hybrid storage engine components");
        
        // Initialize both storage systems in parallel
        let (duckdb_result, lance_result) = tokio::join!(
            self.duckdb.initialize(),
            self.lance.initialize()
        );
        
        duckdb_result.context("Failed to initialize DuckDB")?;
        lance_result.context("Failed to initialize Lance")?;
        
        info!("Hybrid storage engine initialization completed");
        Ok(())
    }
    
    async fn store_document_metadata(&self, metadata: &DocumentMetadata) -> Result<()> {
        self.duckdb.store_document_metadata(metadata).await?;
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.documents_indexed += 1;
        
        Ok(())
    }
    
    async fn store_document_embeddings(&self, doc_id: &str, embeddings: &DocumentEmbeddings) -> Result<()> {
        self.lance.store_document_embeddings(doc_id, embeddings).await
    }
    
    async fn store_block_embeddings(&self, doc_id: &str, blocks: &[BlockEmbedding]) -> Result<()> {
        self.lance.store_block_embeddings(doc_id, blocks).await
    }
    
    async fn semantic_search(&self, query_vector: &[f32], limit: usize, threshold: f32) -> Result<Vec<SearchResult>> {
        let start_time = std::time::Instant::now();
        let results = self.lance.semantic_search(query_vector, limit, threshold).await?;
        
        // Enrich results with metadata from DuckDB
        let enriched_results = self.enrich_search_results(results).await?;
        
        let query_time = start_time.elapsed().as_millis() as f64;
        self.update_query_stats(query_time, false).await;
        
        Ok(enriched_results)
    }
    
    async fn text_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let start_time = std::time::Instant::now();
        let results = self.duckdb.text_search(query, limit).await?;
        
        let query_time = start_time.elapsed().as_millis() as f64;
        self.update_query_stats(query_time, false).await;
        
        Ok(results)
    }
    
    async fn get_document(&self, path: &Path) -> Result<Option<DocumentRecord>> {
        self.duckdb.get_document(path).await
    }
    
    async fn get_documents_by_tag(&self, tag: &str) -> Result<Vec<DocumentRecord>> {
        self.duckdb.get_documents_by_tag(tag).await
    }
    
    async fn get_recent_documents(&self, limit: usize) -> Result<Vec<DocumentRecord>> {
        self.duckdb.get_recent_documents(limit).await
    }
    
    async fn update_document_metadata(&self, path: &Path, metadata: &DocumentMetadata) -> Result<()> {
        self.duckdb.update_document_metadata(path, metadata).await
    }
    
    async fn remove_document(&self, path: &Path) -> Result<()> {
        // Remove from both systems in parallel
        let (duckdb_result, lance_result) = tokio::join!(
            self.duckdb.remove_document(path),
            self.lance.remove_document(path)
        );
        
        duckdb_result.context("Failed to remove document from DuckDB")?;
        lance_result.context("Failed to remove document from Lance")?;
        
        Ok(())
    }
    
    async fn get_stats(&self) -> Result<StorageStats> {
        let (duckdb_stats, lance_stats) = tokio::join!(
            self.duckdb.get_stats(),
            self.lance.get_stats()
        );
        
        let duckdb_stats = duckdb_stats?;
        let lance_stats = lance_stats?;
        let runtime_stats = self.stats.read().await;
        
        Ok(StorageStats {
            total_documents: duckdb_stats.total_documents,
            total_embeddings: lance_stats.total_embeddings,
            total_blocks: lance_stats.total_blocks,
            storage_size_bytes: duckdb_stats.storage_size_bytes + lance_stats.storage_size_bytes,
            embedding_size_bytes: lance_stats.embedding_size_bytes,
            metadata_size_bytes: duckdb_stats.metadata_size_bytes,
            last_optimized: duckdb_stats.last_optimized.max(lance_stats.last_optimized),
            performance_metrics: super::PerformanceMetrics {
                avg_search_latency_ms: runtime_stats.avg_query_time_ms,
                avg_indexing_time_ms: 0.0, // TODO: Track indexing time
                cache_hit_rate: if runtime_stats.cache_hits + runtime_stats.cache_misses > 0 {
                    runtime_stats.cache_hits as f64 / (runtime_stats.cache_hits + runtime_stats.cache_misses) as f64
                } else {
                    0.0
                },
                total_queries: runtime_stats.queries_executed,
                total_documents_indexed: runtime_stats.documents_indexed,
            },
        })
    }
    
    async fn optimize(&self) -> Result<()> {
        let report = self.optimize_all().await?;
        if !report.errors.is_empty() {
            return Err(anyhow::anyhow!("Optimization errors: {}", report.errors.join("; ")));
        }
        Ok(())
    }
    
    async fn backup(&self, backup_path: &Path) -> Result<()> {
        let report = self.backup_all(backup_path).await?;
        if !report.errors.is_empty() {
            return Err(anyhow::anyhow!("Backup errors: {}", report.errors.join("; ")));
        }
        Ok(())
    }
}

impl HybridStorageEngine {
    /// Enrich search results with additional metadata
    async fn enrich_search_results(&self, results: Vec<SearchResult>) -> Result<Vec<SearchResult>> {
        // For semantic search results, we might want to add more context from DuckDB
        // This is where we could add related documents, backlinks, etc.
        Ok(results) // For now, return as-is
    }
}

/// Query builder specifically for hybrid searches
pub struct HybridQueryBuilder<'a> {
    engine: &'a HybridStorageEngine,
    query_text: Option<String>,
    query_vector: Option<Vec<f32>>,
    tags: Vec<String>,
    limit: usize,
    similarity_threshold: f32,
    boost_recent: bool,
    boost_tags: bool,
}

impl<'a> HybridQueryBuilder<'a> {
    pub fn new(engine: &'a HybridStorageEngine) -> Self {
        Self {
            engine,
            query_text: None,
            query_vector: None,
            tags: Vec::new(),
            limit: 10,
            similarity_threshold: 0.7,
            boost_recent: true,
            boost_tags: true,
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
    
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
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
    
    /// Execute the hybrid search
    pub async fn execute(self) -> Result<Vec<SearchResult>> {
        self.engine.hybrid_search(
            self.query_vector.as_deref(),
            self.query_text.as_deref(),
            self.limit,
            self.similarity_threshold,
        ).await
    }
}

// Additional types for analytics and reporting

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct VaultAnalytics {
    pub total_documents: usize,
    pub total_embeddings: usize,
    pub total_queries: u64,
    pub avg_query_time_ms: f64,
    pub cache_hit_rate: f64,
    pub storage_breakdown: StorageBreakdown,
    pub top_tags: Vec<TagStats>,
    pub recent_activity: Vec<ActivityRecord>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StorageBreakdown {
    pub metadata_size_mb: f64,
    pub vector_size_mb: f64,
    pub total_size_mb: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TagStats {
    pub tag: String,
    pub count: usize,
    pub avg_score: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ActivityRecord {
    pub document_path: std::path::PathBuf,
    pub activity_type: ActivityType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ActivityType {
    Created,
    Modified,
    Accessed,
    Indexed,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OptimizationReport {
    pub duration_ms: u64,
    pub duckdb_optimized: bool,
    pub lance_optimized: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BackupReport {
    pub duration_ms: u64,
    pub duckdb_backed_up: bool,
    pub lance_backed_up: bool,
    pub backup_path: std::path::PathBuf,
    pub total_size_bytes: u64,
    pub errors: Vec<String>,
} optimization failed: {}", e));
                }
                errors
            },
        };
        
        info!("Storage optimization completed in {:?}", optimization_time);
        Ok(report)
    }
    
    /// Create a query builder for complex searches
    pub fn query(&self) -> HybridQueryBuilder {
        HybridQueryBuilder::new(self)
    }
    
    /// Backup both storage systems
    pub async fn backup_all(&self, backup_path: &Path) -> Result<BackupReport> {
        info!("Starting comprehensive backup to {}", backup_path.display());
        let start_time = std::time::Instant::now();
        
        // Create backup directories
        let duckdb_backup_path = backup_path.join("duckdb");
        let lance_backup_path = backup_path.join("lance");
        tokio::fs::create_dir_all(&duckdb_backup_path).await?;
        tokio::fs::create_dir_all(&lance_backup_path).await?;
        
        // Run backups in parallel
        let (duckdb_result, lance_result) = tokio::join!(
            self.duckdb.backup(&duckdb_backup_path),
            self.lance.backup(&lance_backup_path)
        );
        
        let backup_time = start_time.elapsed();
        
        let report = BackupReport {
            duration_ms: backup_time.as_millis() as u64,
            duckdb_backed_up: duckdb_result.is_ok(),
            lance_backed_up: lance_result.is_ok(),
            backup_path: backup_path.to_path_buf(),
            total_size_bytes: self.calculate_backup_size(&duckdb_backup_path, &lance_backup_path).await?,
            errors: {
                let mut errors = Vec::new();
                if let Err(e) = duckdb_result {
                    errors.push(format!("DuckDB backup failed: {}", e));
                }
                if let Err(e) = lance_result {
                    errors.push(format!("Lance