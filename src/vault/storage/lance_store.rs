use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, Context, bail};
use serde_json;
use tracing::{info, debug, error, instrument, warn};
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use lance::{Dataset, Error as LanceError};
use lance::dataset::{WriteParams, WriteMode};
use arrow::array::{Float32Array, StringArray, Int64Array, RecordBatch};
use arrow::datatypes::{Schema, Field, DataType};
use datafusion::prelude::*;

use super::{
    StorageEngine, DocumentMetadata, DocumentEmbeddings, BlockEmbedding,
    SearchResult, DocumentRecord, StorageStats, MatchType, SearchContext,
    LanceConfig, IndexType
};

/// Lance-based vector storage for document and block embeddings
pub struct LanceStore {
    config: LanceConfig,
    document_dataset: Arc<RwLock<Option<Dataset>>>,
    block_dataset: Arc<RwLock<Option<Dataset>>>,
    schema_cache: Arc<RwLock<SchemaCache>>,
}

#[derive(Debug, Default)]
struct SchemaCache {
    document_schema: Option<Schema>,
    block_schema: Option<Schema>,
}

impl LanceStore {
    /// Create a new Lance vector store
    pub async fn new(config: LanceConfig) -> Result<Self> {
        info!("Initializing Lance vector store at {}", config.dataset_path.display());
        
        // Create directory if it doesn't exist
        if let Some(parent) = config.dataset_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let store = Self {
            config,
            document_dataset: Arc::new(RwLock::new(None)),
            block_dataset: Arc::new(RwLock::new(None)),
            schema_cache: Arc::new(RwLock::new(SchemaCache::default())),
        };
        
        info!("Lance vector store initialized");
        Ok(store)
    }
    
    /// Initialize Lance datasets and schemas
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing Lance datasets and schemas");
        
        // Initialize document embeddings dataset
        self.initialize_document_dataset().await?;
        
        // Initialize block embeddings dataset
        self.initialize_block_dataset().await?;
        
        info!("Lance datasets initialized successfully");
        Ok(())
    }
    
    /// Initialize the document embeddings dataset
    async fn initialize_document_dataset(&self) -> Result<()> {
        let doc_path = self.config.dataset_path.join("documents");
        
        // Define schema for document embeddings
        let schema = Schema::new(vec![
            Field::new("document_id", DataType::Utf8, false),
            Field::new("document_path", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("embedding", DataType::List(
                Arc::new(Field::new("item", DataType::Float32, true))
            ), false),
            Field::new("model_name", DataType::Utf8, false),
            Field::new("embedding_dimension", DataType::Int64, false),
            Field::new("created_at", DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None), false),
            Field::new("checksum", DataType::Utf8, false),
            Field::new("metadata", DataType::Utf8, true), // JSON metadata
        ]);
        
        // Cache schema
        {
            let mut cache = self.schema_cache.write().await;
            cache.document_schema = Some(schema.clone());
        }
        
        // Create or open dataset
        let dataset = if doc_path.exists() {
            debug!("Opening existing document dataset");
            Dataset::open(&doc_path.to_string_lossy())
                .await
                .context("Failed to open document dataset")?
        } else {
            debug!("Creating new document dataset");
            // Create empty dataset with schema
            let empty_batch = RecordBatch::new_empty(Arc::new(schema));
            Dataset::write(
                vec![empty_batch],
                &doc_path.to_string_lossy(),
                Some(WriteParams {
                    mode: WriteMode::Create,
                    ..Default::default()
                })
            ).await.context("Failed to create document dataset")?
        };
        
        // Store dataset reference
        {
            let mut dataset_lock = self.document_dataset.write().await;
            *dataset_lock = Some(dataset);
        }
        
        debug!("Document dataset initialized");
        Ok(())
    }
    
    /// Initialize the block embeddings dataset
    async fn initialize_block_dataset(&self) -> Result<()> {
        let block_path = self.config.dataset_path.join("blocks");
        
        // Define schema for block embeddings
        let schema = Schema::new(vec![
            Field::new("block_id", DataType::Utf8, false),
            Field::new("document_id", DataType::Utf8, false),
            Field::new("document_path", DataType::Utf8, false),
            Field::new("block_type", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("embedding", DataType::List(
                Arc::new(Field::new("item", DataType::Float32, true))
            ), false),
            Field::new("start_pos", DataType::Int64, false),
            Field::new("end_pos", DataType::Int64, false),
            Field::new("created_at", DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None), false),
            Field::new("metadata", DataType::Utf8, true),
        ]);
        
        // Cache schema
        {
            let mut cache = self.schema_cache.write().await;
            cache.block_schema = Some(schema.clone());
        }
        
        // Create or open dataset
        let dataset = if block_path.exists() {
            debug!("Opening existing block dataset");
            Dataset::open(&block_path.to_string_lossy())
                .await
                .context("Failed to open block dataset")?
        } else {
            debug!("Creating new block dataset");
            let empty_batch = RecordBatch::new_empty(Arc::new(schema));
            Dataset::write(
                vec![empty_batch],
                &block_path.to_string_lossy(),
                Some(WriteParams {
                    mode: WriteMode::Create,
                    ..Default::default()
                })
            ).await.context("Failed to create block dataset")?
        };
        
        // Store dataset reference
        {
            let mut dataset_lock = self.block_dataset.write().await;
            *dataset_lock = Some(dataset);
        }
        
        debug!("Block dataset initialized");
        Ok(())
    }
    
    /// Create vector index for fast similarity search
    pub async fn create_vector_index(&self, dataset_type: DatasetType) -> Result<()> {
        info!("Creating vector index for {:?} dataset", dataset_type);
        
        let dataset = match dataset_type {
            DatasetType::Document => {
                let dataset_lock = self.document_dataset.read().await;
                dataset_lock.as_ref()
                    .context("Document dataset not initialized")?
                    .clone()
            },
            DatasetType::Block => {
                let dataset_lock = self.block_dataset.read().await;
                dataset_lock.as_ref()
                    .context("Block dataset not initialized")?
                    .clone()
            },
        };
        
        // Create index based on configuration
        let index_params = match self.config.index_type {
            IndexType::IVF => {
                let mut params = HashMap::new();
                params.insert("num_partitions".to_string(), 
                    self.config.num_partitions.unwrap_or(256).to_string());
                params.insert("num_sub_quantizers".to_string(), 
                    self.config.num_sub_quantizers.unwrap_or(16).to_string());
                params
            },
            IndexType::HNSW => {
                let mut params = HashMap::new();
                params.insert("max_connections".to_string(), "16".to_string());
                params.insert("ef_construction".to_string(), "200".to_string());
                params
            },
            IndexType::Flat => HashMap::new(), // No parameters needed for flat index
        };
        
        // Build the index
        dataset.create_index(
            &["embedding"], 
            lance::index::IndexType::Vector,
            None, // Use default index name
            &index_params,
            true, // Replace existing index
        ).await.context("Failed to create vector index")?;
        
        info!("Vector index created successfully");
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum DatasetType {
    Document,
    Block,
}

#[async_trait::async_trait]
impl StorageEngine for LanceStore {
    async fn initialize(&self) -> Result<()> {
        self.initialize().await
    }
    
    async fn store_document_metadata(&self, _metadata: &DocumentMetadata) -> Result<()> {
        // Lance doesn't store document metadata - that's DuckDB's job
        Ok(())
    }
    
    #[instrument(skip(self, embeddings))]
    async fn store_document_embeddings(&self, doc_id: &str, embeddings: &DocumentEmbeddings) -> Result<()> {
        debug!("Storing document embeddings for {}", doc_id);
        
        // Validate embedding dimension
        if embeddings.document_vector.len() != self.config.vector_dimension {
            bail!(
                "Invalid vector dimension: expected {}, got {}",
                self.config.vector_dimension,
                embeddings.document_vector.len()
            );
        }
        
        let dataset_lock = self.document_dataset.read().await;
        let dataset = dataset_lock.as_ref()
            .context("Document dataset not initialized")?;
        
        // Prepare data for insertion
        let document_ids = StringArray::from(vec![doc_id]);
        let document_paths = StringArray::from(vec![doc_id]); // Using doc_id as path for now
        let titles = StringArray::from(vec![""]); // Will be filled from DuckDB if needed
        
        // Convert embedding vector to Arrow format
        let embedding_values = Float32Array::from(embeddings.document_vector.clone());
        let embedding_offsets = arrow::buffer::OffsetBuffer::new(vec![0, embeddings.document_vector.len() as i32].into());
        let embedding_field = Arc::new(Field::new("item", DataType::Float32, true));
        let embedding_list = arrow::array::ListArray::new(
            embedding_field,
            embedding_offsets,
            Arc::new(embedding_values),
            None,
        );
        
        let model_names = StringArray::from(vec![embeddings.model_name.as_str()]);
        let dimensions = Int64Array::from(vec![embeddings.embedding_dimension as i64]);
        let timestamps = arrow::array::TimestampMicrosecondArray::from(vec![
            embeddings.created_at.timestamp_micros()
        ]);
        let checksums = StringArray::from(vec![embeddings.checksum.as_str()]);
        let metadata_json = StringArray::from(vec![serde_json::to_string(&serde_json::json!({
            "model_name": embeddings.model_name,
            "dimension": embeddings.embedding_dimension
        }))?]);
        
        // Get schema from cache
        let schema = {
            let cache = self.schema_cache.read().await;
            cache.document_schema.as_ref()
                .context("Document schema not cached")?
                .clone()
        };
        
        // Create record batch
        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(document_ids),
                Arc::new(document_paths),
                Arc::new(titles),
                Arc::new(embedding_list),
                Arc::new(model_names),
                Arc::new(dimensions),
                Arc::new(timestamps),
                Arc::new(checksums),
                Arc::new(metadata_json),
            ],
        ).context("Failed to create record batch")?;
        
        // Write to dataset
        dataset.write(
            vec![batch],
            Some(WriteParams {
                mode: WriteMode::Append,
                ..Default::default()
            })
        ).await.context("Failed to write embeddings to dataset")?;
        
        debug!("Document embeddings stored successfully");
        Ok(())
    }
    
    #[instrument(skip(self, blocks))]
    async fn store_block_embeddings(&self, doc_id: &str, blocks: &[BlockEmbedding]) -> Result<()> {
        if blocks.is_empty() {
            return Ok(());
        }
        
        debug!("Storing {} block embeddings for {}", blocks.len(), doc_id);
        
        let dataset_lock = self.block_dataset.read().await;
        let dataset = dataset_lock.as_ref()
            .context("Block dataset not initialized")?;
        
        // Prepare arrays for all blocks
        let mut block_ids = Vec::new();
        let mut document_ids = Vec::new();
        let mut document_paths = Vec::new();
        let mut block_types = Vec::new();
        let mut contents = Vec::new();
        let mut embedding_data = Vec::new();
        let mut start_positions = Vec::new();
        let mut end_positions = Vec::new();
        let mut timestamps = Vec::new();
        let mut metadata_jsons = Vec::new();
        
        for block in blocks {
            // Validate embedding dimension
            if block.vector.len() != self.config.vector_dimension {
                warn!(
                    "Block embedding dimension mismatch: expected {}, got {}. Skipping block {}",
                    self.config.vector_dimension,
                    block.vector.len(),
                    block.block_id
                );
                continue;
            }
            
            block_ids.push(block.block_id.clone());
            document_ids.push(doc_id.to_string());
            document_paths.push(doc_id.to_string()); // Using doc_id as path
            block_types.push(serde_json::to_string(&block.block_type)?);
            contents.push(block.content.clone());
            embedding_data.extend(block.vector.clone());
            start_positions.push(block.start_pos as i64);
            end_positions.push(block.end_pos as i64);
            timestamps.push(block.created_at.timestamp_micros());
            metadata_jsons.push(serde_json::to_string(&serde_json::json!({
                "block_type": block.block_type,
                "content_length": block.content.len()
            }))?);
        }
        
        if block_ids.is_empty() {
            warn!("No valid block embeddings to store after dimension validation");
            return Ok(());
        }
        
        // Create Arrow arrays
        let block_ids_array = StringArray::from(block_ids);
        let document_ids_array = StringArray::from(document_ids);
        let document_paths_array = StringArray::from(document_paths);
        let block_types_array = StringArray::from(block_types);
        let contents_array = StringArray::from(contents);
        let start_positions_array = Int64Array::from(start_positions);
        let end_positions_array = Int64Array::from(end_positions);
        let timestamps_array = arrow::array::TimestampMicrosecondArray::from(timestamps);
        let metadata_array = StringArray::from(metadata_jsons);
        
        // Create embeddings list array
        let embedding_values = Float32Array::from(embedding_data);
        let vector_dim = self.config.vector_dimension;
        let num_blocks = blocks.len();
        let mut offsets = Vec::with_capacity(num_blocks + 1);
        for i in 0..=num_blocks {
            offsets.push((i * vector_dim) as i32);
        }
        let embedding_offsets = arrow::buffer::OffsetBuffer::new(offsets.into());
        let embedding_field = Arc::new(Field::new("item", DataType::Float32, true));
        let embeddings_list = arrow::array::ListArray::new(
            embedding_field,
            embedding_offsets,
            Arc::new(embedding_values),
            None,
        );
        
        // Get schema from cache
        let schema = {
            let cache = self.schema_cache.read().await;
            cache.block_schema.as_ref()
                .context("Block schema not cached")?
                .clone()
        };
        
        // Create record batch
        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(block_ids_array),
                Arc::new(document_ids_array),
                Arc::new(document_paths_array),
                Arc::new(block_types_array),
                Arc::new(contents_array),
                Arc::new(embeddings_list),
                Arc::new(start_positions_array),
                Arc::new(end_positions_array),
                Arc::new(timestamps_array),
                Arc::new(metadata_array),
            ],
        ).context("Failed to create block embeddings batch")?;
        
        // Write to dataset
        dataset.write(
            vec![batch],
            Some(WriteParams {
                mode: WriteMode::Append,
                ..Default::default()
            })
        ).await.context("Failed to write block embeddings to dataset")?;
        
        debug!("Block embeddings stored successfully");
        Ok(())
    }
    
    #[instrument(skip(self, query_vector))]
    async fn semantic_search(&self, query_vector: &[f32], limit: usize, threshold: f32) -> Result<Vec<SearchResult>> {
        debug!("Executing semantic search with limit={}, threshold={}", limit, threshold);
        
        // Validate query vector dimension
        if query_vector.len() != self.config.vector_dimension {
            bail!(
                "Invalid query vector dimension: expected {}, got {}",
                self.config.vector_dimension,
                query_vector.len()
            );
        }
        
        let start_time = std::time::Instant::now();
        
        // Search in document embeddings first
        let document_results = self.search_documents(query_vector, limit, threshold).await?;
        
        // Search in block embeddings for more granular results
        let block_results = self.search_blocks(query_vector, limit * 2, threshold).await?;
        
        // Combine and rank results
        let combined_results = self.combine_search_results(document_results, block_results, limit).await?;
        
        let search_time = start_time.elapsed();
        debug!("Semantic search completed in {:?} with {} results", search_time, combined_results.len());
        
        Ok(combined_results)
    }
    
    async fn text_search(&self, _query: &str, _limit: usize) -> Result<Vec<SearchResult>> {
        // Lance doesn't do text search - that's DuckDB's job
        Ok(Vec::new())
    }
    
    async fn get_document(&self, _path: &Path) -> Result<Option<DocumentRecord>> {
        // Lance doesn't store document metadata - that's DuckDB's job
        Ok(None)
    }
    
    async fn get_documents_by_tag(&self, _tag: &str) -> Result<Vec<DocumentRecord>> {
        // Lance doesn't store tags - that's DuckDB's job
        Ok(Vec::new())
    }
    
    async fn get_recent_documents(&self, _limit: usize) -> Result<Vec<DocumentRecord>> {
        // Lance doesn't store document metadata - that's DuckDB's job
        Ok(Vec::new())
    }
    
    async fn update_document_metadata(&self, _path: &Path, _metadata: &DocumentMetadata) -> Result<()> {
        // Lance doesn't store document metadata - that's DuckDB's job
        Ok(())
    }
    
    async fn remove_document(&self, path: &Path) -> Result<()> {
        let doc_id = path.to_string_lossy();
        debug!("Removing document embeddings for {}", doc_id);
        
        // Remove from document dataset
        {
            let dataset_lock = self.document_dataset.read().await;
            if let Some(dataset) = dataset_lock.as_ref() {
                // Lance doesn't support direct deletion, so we'd need to rewrite
                // For now, we'll mark as deleted in metadata or use a tombstone approach
                warn!("Document deletion from Lance not fully implemented - requires dataset rewrite");
            }
        }
        
        // Remove from block dataset
        {
            let dataset_lock = self.block_dataset.read().await;
            if let Some(dataset) = dataset_lock.as_ref() {
                warn!("Block deletion from Lance not fully implemented - requires dataset rewrite");
            }
        }
        
        Ok(())
    }
    
    async fn get_stats(&self) -> Result<StorageStats> {
        let mut total_documents = 0;
        let mut total_blocks = 0;
        let mut storage_size = 0;
        
        // Get document dataset stats
        {
            let dataset_lock = self.document_dataset.read().await;
            if let Some(dataset) = dataset_lock.as_ref() {
                let fragments = dataset.get_fragments().await?;
                total_documents = fragments.len();
                
                // Estimate storage size
                for fragment in fragments {
                    if let Ok(metadata) = fragment.metadata().await {
                        storage_size += metadata.physical_rows.unwrap_or(0) as u64 * 
                            (self.config.vector_dimension * 4 + 200) as u64; // Rough estimate
                    }
                }
            }
        }
        
        // Get block dataset stats
        {
            let dataset_lock = self.block_dataset.read().await;
            if let Some(dataset) = dataset_lock.as_ref() {
                let fragments = dataset.get_fragments().await?;
                for fragment in fragments {
                    if let Ok(metadata) = fragment.metadata().await {
                        total_blocks += metadata.physical_rows.unwrap_or(0);
                    }
                }
            }
        }
        
        Ok(StorageStats {
            total_documents,
            total_embeddings: total_documents,
            total_blocks: total_blocks as usize,
            storage_size_bytes: storage_size,
            embedding_size_bytes: storage_size,
            metadata_size_bytes: 0, // Lance doesn't store metadata
            last_optimized: None,
            performance_metrics: super::PerformanceMetrics {
                avg_search_latency_ms: 0.0, // TODO: Track this
                avg_indexing_time_ms: 0.0,
                cache_hit_rate: 0.0,
                total_queries: 0,
                total_documents_indexed: total_documents as u64,
            },
        })
    }
    
    async fn optimize(&self) -> Result<()> {
        info!("Optimizing Lance datasets");
        
        // Compact datasets to remove tombstones and optimize storage
        if let Some(dataset) = self.document_dataset.read().await.as_ref() {
            dataset.optimize().await
                .context("Failed to optimize document dataset")?;
        }
        
        if let Some(dataset) = self.block_dataset.read().await.as_ref() {
            dataset.optimize().await
                .context("Failed to optimize block dataset")?;
        }
        
        info!("Lance optimization completed");
        Ok(())
    }
    
    async fn backup(&self, backup_path: &Path) -> Result<()> {
        info!("Backing up Lance datasets to {}", backup_path.display());
        
        let doc_backup_path = backup_path.join("documents");
        let block_backup_path = backup_path.join("blocks");
        
        // Create backup directories
        tokio::fs::create_dir_all(&doc_backup_path).await?;
        tokio::fs::create_dir_all(&block_backup_path).await?;
        
        // Copy document dataset
        let doc_source = self.config.dataset_path.join("documents");
        if doc_source.exists() {
            copy_dir_all(&doc_source, &doc_backup_path).await?;
        }
        
        // Copy block dataset
        let block_source = self.config.dataset_path.join("blocks");
        if block_source.exists() {
            copy_dir_all(&block_source, &block_backup_path).await?;
        }
        
        info!("Lance backup completed");
        Ok(())
    }
}

impl LanceStore {
    /// Search document embeddings
    async fn search_documents(&self, query_vector: &[f32], limit: usize, threshold: f32) -> Result<Vec<SearchResult>> {
        let dataset_lock = self.document_dataset.read().await;
        let dataset = dataset_lock.as_ref()
            .context("Document dataset not initialized")?;
        
        // Perform vector search
        let results = dataset.scan()
            .nearest("embedding", query_vector, limit)?
            .distance_threshold(threshold)
            .execute()
            .await?;
        
        let mut search_results = Vec::new();
        
        // Convert results to SearchResult format
        let batches = results.try_collect::<Vec<_>>().await?;
        for batch in batches {
            let document_ids = batch.column(0).as_any().downcast_ref::<StringArray>()
                .context("Failed to cast document_id column")?;
            let distances = batch.column_by_name("_distance")
                .and_then(|col| col.as_any().downcast_ref::<Float32Array>());
            
            for (i, doc_id) in document_ids.iter().enumerate() {
                if let Some(doc_id) = doc_id {
                    let score = if let Some(distances) = distances {
                        1.0 - distances.value(i) // Convert distance to similarity
                    } else {
                        0.5 // Default score if distance not available
                    };
                    
                    search_results.push(SearchResult {
                        document: DocumentRecord {
                            metadata: DocumentMetadata {
                                path: PathBuf::from(doc_id),
                                title: doc_id.to_string(), // Will be enriched by hybrid engine
                                content_hash: String::new(),
                                size: 0,
                                word_count: 0,
                                created_at: Utc::now(),
                                modified_at: Utc::now(),
                                indexed_at: Utc::now(),
                                tags: Vec::new(),
                                links: Vec::new(),
                                file_type: super::FileType::Unknown,
                                language: None,
                                custom_fields: HashMap::new(),
                            },
                            snippet: None,
                            highlight: None,
                        },
                        score,
                        match_type: MatchType::Semantic,
                        matched_content: None,
                        matched_blocks: Vec::new(),
                        context: SearchContext {
                            surrounding_content: None,
                            related_documents: Vec::new(),
                            related_tags: Vec::new(),
                            backlinks: Vec::new(),
                        },
                    });
                }
            }
        }
        
        Ok(search_results)
    }
    
    /// Search block embeddings
    async fn search_blocks(&self, query_vector: &[f32], limit: usize, threshold: f32) -> Result<Vec<SearchResult>> {
        let dataset_lock = self.block_dataset.read().await;
        let dataset = dataset_lock.as_ref()
            .context("Block dataset not initialized")?;
        
        // Perform vector search
        let results = dataset.scan()
            .nearest("embedding", query_vector, limit)?
            .distance_threshold(threshold)
            .execute()
            .await?;
        
        let mut search_results = Vec::new();
        
        // Convert results to SearchResult format
        let batches = results.try_collect::<Vec<_>>().await?;
        for batch in batches {
            let document_ids = batch.column(1).as_any().downcast_ref::<StringArray>()
                .context("Failed to cast document_id column")?;
            let block_ids = batch.column(0).as_any().downcast_ref::<StringArray>()
                .context("Failed to cast block_id column")?;
            let contents = batch.column(4).as_any().downcast_ref::<StringArray>()
                .context("Failed to cast content column")?;
            let distances = batch.column_by_name("_distance")
                .and_then(|col| col.as_any().downcast_ref::<Float32Array>());
            
            for i in 0..batch.num_rows() {
                if let (Some(doc_id), Some(block_id), Some(content)) = (
                    document_ids.value(i),
                    block_ids.value(i),
                    contents.value(i)
                ) {
                    let score = if let Some(distances) = distances {
                        1.0 - distances.value(i) // Convert distance to similarity
                    } else {
                        0.5
                    };
                    
                    search_results.push(SearchResult {
                        document: DocumentRecord {
                            metadata: DocumentMetadata {
                                path: PathBuf::from(doc_id),
                                title: doc_id.to_string(),
                                content_hash: String::new(),
                                size: 0,
                                word_count: 0,
                                created_at: Utc::now(),
                                modified_at: Utc::now(),
                                indexed_at: Utc::now(),
                                tags: Vec::new(),
                                links: Vec::new(),
                                file_type: super::FileType::Unknown,
                                language: None,
                                custom_fields: HashMap::new(),
                            },
                            snippet: Some(content.to_string()),
                            highlight: None,
                        },
                        score,
                        match_type: MatchType::Semantic,
                        matched_content: Some(content.to_string()),
                        matched_blocks: vec![super::MatchedBlock {
                            block_id: block_id.to_string(),
                            block_type: super::BlockType::Paragraph, // Default
                            content: content.to_string(),
                            score,
                            highlight: None,
                        }],
                        context: SearchContext {
                            surrounding_content: None,
                            related_documents: Vec::new(),
                            related_tags: Vec::new(),
                            backlinks: Vec::new(),
                        },
                    });
                }
            }
        }
        
        Ok(search_results)
    }
    
    /// Combine document and block search results
    async fn combine_search_results(
        &self,
        document_results: Vec<SearchResult>,
        block_results: Vec<SearchResult>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut combined = HashMap::new();
        
        // Add document-level results
        for result in document_results {
            let doc_path = result.document.metadata.path.clone();
            combined.insert(doc_path, result);
        }
        
        // Merge block-level results
        for result in block_results {
            let doc_path = result.document.metadata.path.clone();
            
            if let Some(existing) = combined.get_mut(&doc_path) {
                // Boost score for documents with matching blocks
                existing.score = (existing.score + result.score * 0.8).max(existing.score);
                existing.matched_blocks.extend(result.matched_blocks);
            } else {
                // Add block-only results with slightly lower weight
                let mut block_result = result;
                block_result.score *= 0.9;
                combined.insert(doc_path, block_result);
            }
        }
        
        // Convert to sorted vector
        let mut results: Vec<SearchResult> = combined.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        
        Ok(results)
    }
    
    /// Get embedding statistics
    pub async fn get_embedding_stats(&self) -> Result<EmbeddingStats> {
        let mut doc_count = 0;
        let mut block_count = 0;
        let mut total_vectors = 0;
        
        // Document dataset stats
        if let Some(dataset) = self.document_dataset.read().await.as_ref() {
            let fragments = dataset.get_fragments().await?;
            doc_count = fragments.len();
            total_vectors += doc_count;
        }
        
        // Block dataset stats
        if let Some(dataset) = self.block_dataset.read().await.as_ref() {
            let fragments = dataset.get_fragments().await?;
            for fragment in fragments {
                if let Ok(metadata) = fragment.metadata().await {
                    block_count += metadata.physical_rows.unwrap_or(0) as usize;
                }
            }
            total_vectors += block_count;
        }
        
        Ok(EmbeddingStats {
            document_embeddings: doc_count,
            block_embeddings: block_count,
            total_vectors,
            vector_dimension: self.config.vector_dimension,
            index_type: self.config.index_type.clone(),
        })
    }
}

/// Statistics about stored embeddings
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingStats {
    pub document_embeddings: usize,
    pub block_embeddings: usize,
    pub total_vectors: usize,
    pub vector_dimension: usize,
    pub index_type: IndexType,
}

/// Utility function to recursively copy directories
async fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    use tokio::fs;
    
    fs::create_dir_all(dst).await?;
    
    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let ty = entry.file_type().await?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path).await?;
        } else {
            fs::copy(&src_path, &dst_path).await?;
        }
    }
    
    Ok(())
}