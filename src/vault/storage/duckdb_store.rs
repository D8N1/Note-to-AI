use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::{Result, Context};
use serde_json;
use tracing::{info, debug, error, instrument};
use chrono::{DateTime, Utc};
use duckdb::{Connection, params, Result as DuckResult};

use super::{
    StorageEngine, DocumentMetadata, DocumentEmbeddings, BlockEmbedding,
    SearchResult, DocumentRecord, StorageStats, MatchType, MatchedBlock, SearchContext,
    DuckDBConfig, TagStats, ActivityRecord, ActivityType, FileType
};

/// DuckDB-based storage for document metadata and full-text search
pub struct DuckDBStore {
    config: DuckDBConfig,
    connection: Connection,
}

impl DuckDBStore {
    /// Create a new DuckDB store
    pub async fn new(config: DuckDBConfig) -> Result<Self> {
        info!("Initializing DuckDB store at {}", config.database_path.display());
        
        // Create directory if it doesn't exist
        if let Some(parent) = config.database_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Open connection
        let connection = Connection::open(&config.database_path)
            .context("Failed to open DuckDB connection")?;
        
        let store = Self {
            config,
            connection,
        };
        
        store.configure_duckdb().await?;
        info!("DuckDB store initialized successfully");
        
        Ok(store)
    }
    
    /// Configure DuckDB settings for optimal performance
    async fn configure_duckdb(&self) -> Result<()> {
        debug!("Configuring DuckDB settings");
        
        // Set memory limit
        if let Some(memory_limit) = self.config.memory_limit {
            let memory_mb = memory_limit / (1024 * 1024);
            self.connection.execute(&format!("SET memory_limit = '{}MB'", memory_mb), [])?;
        }
        
        // Set thread count
        if let Some(threads) = self.config.thread_count {
            self.connection.execute(&format!("SET threads = {}", threads), [])?;
        }
        
        // Enable WAL mode for better concurrency
        if self.config.wal_mode {
            self.connection.execute("PRAGMA journal_mode = WAL", [])?;
        }
        
        // Configure parquet cache
        if self.config.enable_parquet_cache {
            self.connection.execute(&format!("SET max_memory = '{}MB'", self.config.max_cache_size_mb), [])?;
        }
        
        // Install and load FTS extension for full-text search
        self.connection.execute("INSTALL fts", [])?;
        self.connection.execute("LOAD fts", [])?;
        
        debug!("DuckDB configuration completed");
        Ok(())
    }
    
    /// Initialize database schema
    pub async fn initialize(&self) -> Result<()> {
        info!("Creating DuckDB schema");
        
        self.create_tables().await?;
        self.create_indexes().await?;
        self.create_fts_indexes().await?;
        self.create_views().await?;
        
        info!("DuckDB schema created successfully");
        Ok(())
    }
    
    /// Create all necessary tables
    async fn create_tables(&self) -> Result<()> {
        // Documents table - main metadata storage
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY,
                path VARCHAR UNIQUE NOT NULL,
                title VARCHAR NOT NULL,
                content_hash VARCHAR NOT NULL,
                size BIGINT NOT NULL,
                word_count INTEGER NOT NULL,
                created_at TIMESTAMP NOT NULL,
                modified_at TIMESTAMP NOT NULL,
                indexed_at TIMESTAMP NOT NULL,
                file_type VARCHAR NOT NULL,
                language VARCHAR,
                custom_fields JSON
            )",
            [],
        )?;
        
        // Tags table - normalized tag storage
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY,
                tag VARCHAR UNIQUE NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Document tags junction table
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS document_tags (
                document_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (document_id, tag_id),
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        // Links table - document relationships
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS links (
                id INTEGER PRIMARY KEY,
                source_document_id INTEGER NOT NULL,
                target_path VARCHAR NOT NULL,
                link_type VARCHAR NOT NULL,
                alias VARCHAR,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (source_document_id) REFERENCES documents(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        // Full-text content table (separate for better performance)
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS document_content (
                document_id INTEGER PRIMARY KEY,
                content TEXT NOT NULL,
                plain_text TEXT NOT NULL,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        // Search analytics table
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS search_analytics (
                id INTEGER PRIMARY KEY,
                query_text VARCHAR,
                query_type VARCHAR NOT NULL,
                results_count INTEGER NOT NULL,
                execution_time_ms DOUBLE NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Document access log for analytics
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS document_access_log (
                id INTEGER PRIMARY KEY,
                document_id INTEGER NOT NULL,
                access_type VARCHAR NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        debug!("All tables created successfully");
        Ok(())
    }
    
    /// Create indexes for optimal query performance
    async fn create_indexes(&self) -> Result<()> {
        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_documents_path ON documents(path)",
            "CREATE INDEX IF NOT EXISTS idx_documents_modified_at ON documents(modified_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_documents_file_type ON documents(file_type)",
            "CREATE INDEX IF NOT EXISTS idx_documents_content_hash ON documents(content_hash)",
            "CREATE INDEX IF NOT EXISTS idx_tags_tag ON tags(tag)",
            "CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_document_id)",
            "CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_path)",
            "CREATE INDEX IF NOT EXISTS idx_search_analytics_timestamp ON search_analytics(timestamp DESC)",
            "CREATE INDEX IF NOT EXISTS idx_access_log_document_timestamp ON document_access_log(document_id, timestamp DESC)",
        ];
        
        for index_sql in indexes {
            self.connection.execute(index_sql, [])?;
        }
        
        debug!("All indexes created successfully");
        Ok(())
    }
    
    /// Create full-text search indexes
    async fn create_fts_indexes(&self) -> Result<()> {
        // Create FTS index for document content
        self.connection.execute(
            "CREATE INDEX IF NOT EXISTS fts_content_idx ON document_content USING FTS(content, plain_text)",
            [],
        )?;
        
        // Create FTS index for document titles
        self.connection.execute(
            "CREATE INDEX IF NOT EXISTS fts_title_idx ON documents USING FTS(title)",
            [],
        )?;
        
        debug!("FTS indexes created successfully");
        Ok(())
    }
    
    /// Create useful views for analytics
    async fn create_views(&self) -> Result<()> {
        // Document statistics view
        self.connection.execute(
            "CREATE OR REPLACE VIEW document_stats AS
            SELECT 
                d.*,
                COUNT(DISTINCT dt.tag_id) as tag_count,
                COUNT(DISTINCT l.id) as outbound_links,
                COUNT(DISTINCT il.id) as inbound_links
            FROM documents d
            LEFT JOIN document_tags dt ON d.id = dt.document_id
            LEFT JOIN links l ON d.id = l.source_document_id
            LEFT JOIN links il ON d.path = il.target_path
            GROUP BY d.id",
            [],
        )?;
        
        // Tag popularity view
        self.connection.execute(
            "CREATE OR REPLACE VIEW tag_popularity AS
            SELECT 
                t.tag,
                COUNT(dt.document_id) as document_count,
                MAX(d.modified_at) as last_used
            FROM tags t
            JOIN document_tags dt ON t.id = dt.tag_id
            JOIN documents d ON dt.document_id = d.id
            GROUP BY t.tag
            ORDER BY document_count DESC",
            [],
        )?;
        
        debug!("Views created successfully");
        Ok(())
    }
}

#[async_trait::async_trait]
impl StorageEngine for DuckDBStore {
    async fn initialize(&self) -> Result<()> {
        self.initialize().await
    }
    
    #[instrument(skip(self, metadata))]
    async fn store_document_metadata(&self, metadata: &DocumentMetadata) -> Result<()> {
        debug!("Storing document metadata for {}", metadata.path.display());
        
        // Start transaction
        let tx = self.connection.transaction()?;
        
        // Insert or update document
        let document_id = tx.query_row(
            "INSERT INTO documents 
             (path, title, content_hash, size, word_count, created_at, modified_at, indexed_at, file_type, language, custom_fields)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (path) DO UPDATE SET
                title = excluded.title,
                content_hash = excluded.content_hash,
                size = excluded.size,
                word_count = excluded.word_count,
                modified_at = excluded.modified_at,
                indexed_at = excluded.indexed_at,
                file_type = excluded.file_type,
                language = excluded.language,
                custom_fields = excluded.custom_fields
             RETURNING id",
            params![
                metadata.path.to_string_lossy(),
                metadata.title,
                metadata.content_hash,
                metadata.size as i64,
                metadata.word_count as i32,
                metadata.created_at,
                metadata.modified_at,
                metadata.indexed_at,
                serde_json::to_string(&metadata.file_type)?,
                metadata.language,
                serde_json::to_string(&metadata.custom_fields)?
            ],
        )?;
        
        let document_id: i64 = document_id;
        
        // Clear existing tags
        tx.execute(
            "DELETE FROM document_tags WHERE document_id = ?",
            params![document_id],
        )?;
        
        // Insert tags
        for tag in &metadata.tags {
            // Insert tag if it doesn't exist
            tx.execute(
                "INSERT INTO tags (tag) VALUES (?) ON CONFLICT (tag) DO NOTHING",
                params![tag],
            )?;
            
            // Get tag ID
            let tag_id: i64 = tx.query_row(
                "SELECT id FROM tags WHERE tag = ?",
                params![tag],
                |row| row.get(0),
            )?;
            
            // Link document to tag
            tx.execute(
                "INSERT INTO document_tags (document_id, tag_id) VALUES (?, ?)",
                params![document_id, tag_id],
            )?;
        }
        
        // Clear existing links
        tx.execute(
            "DELETE FROM links WHERE source_document_id = ?",
            params![document_id],
        )?;
        
        // Insert links
        for link in &metadata.links {
            tx.execute(
                "INSERT INTO links (source_document_id, target_path, link_type, alias)
                 VALUES (?, ?, ?, ?)",
                params![document_id, link, "wikilink", None::<String>],
            )?;
        }
        
        // Log the indexing activity
        tx.execute(
            "INSERT INTO document_access_log (document_id, access_type)
             VALUES (?, 'indexed')",
            params![document_id],
        )?;
        
        tx.commit()?;
        debug!("Document metadata stored successfully");
        Ok(())
    }
    
    async fn store_document_embeddings(&self, _doc_id: &str, _embeddings: &DocumentEmbeddings) -> Result<()> {
        // DuckDB doesn't store embeddings - that's Lance's job
        Ok(())
    }
    
    async fn store_block_embeddings(&self, _doc_id: &str, _blocks: &[BlockEmbedding]) -> Result<()> {
        // DuckDB doesn't store embeddings - that's Lance's job
        Ok(())
    }
    
    async fn semantic_search(&self, _query_vector: &[f32], _limit: usize, _threshold: f32) -> Result<Vec<SearchResult>> {
        // DuckDB doesn't do semantic search - that's Lance's job
        Ok(Vec::new())
    }
    
    #[instrument(skip(self, query))]
    async fn text_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let start_time = std::time::Instant::now();
        debug!("Executing text search for query: {}", query);
        
        let mut results = Vec::new();
        
        // Search in titles and content using FTS
        let mut stmt = self.connection.prepare(
            "SELECT 
                d.id, d.path, d.title, d.content_hash, d.size, d.word_count,
                d.created_at, d.modified_at, d.indexed_at, d.file_type, d.language, d.custom_fields,
                dc.plain_text,
                fts_main_document_content.score as content_score,
                fts_main_documents.score as title_score
            FROM documents d
            LEFT JOIN document_content dc ON d.id = dc.document_id
            LEFT JOIN (
                SELECT rowid, score FROM fts_content_idx WHERE fts_content_idx MATCH ?
            ) fts_main_document_content ON d.id = fts_main_document_content.rowid
            LEFT JOIN (
                SELECT rowid, score FROM fts_title_idx WHERE fts_title_idx MATCH ?
            ) fts_main_documents ON d.id = fts_main_documents.rowid
            WHERE fts_main_document_content.score IS NOT NULL OR fts_main_documents.score IS NOT NULL
            ORDER BY 
                COALESCE(fts_main_documents.score, 0) * 2 + COALESCE(fts_main_document_content.score, 0) DESC
            LIMIT ?",
        )?;
        
        let rows = stmt.query_map(params![query, query, limit], |row| {
            let path: String = row.get(1)?;
            let title: String = row.get(2)?;
            let content_hash: String = row.get(3)?;
            let size: i64 = row.get(4)?;
            let word_count: i32 = row.get(5)?;
            let created_at: DateTime<Utc> = row.get(6)?;
            let modified_at: DateTime<Utc> = row.get(7)?;
            let indexed_at: DateTime<Utc> = row.get(8)?;
            let file_type_str: String = row.get(9)?;
            let language: Option<String> = row.get(10)?;
            let custom_fields_str: String = row.get(11)?;
            let plain_text: Option<String> = row.get(12)?;
            let content_score: Option<f64> = row.get(13)?;
            let title_score: Option<f64> = row.get(14)?;
            
            let file_type: FileType = serde_json::from_str(&file_type_str).unwrap_or(FileType::Unknown);
            let custom_fields: HashMap<String, serde_json::Value> = 
                serde_json::from_str(&custom_fields_str).unwrap_or_default();
            
            let score = (title_score.unwrap_or(0.0) * 2.0 + content_score.unwrap_or(0.0)) as f32;
            
            let snippet = if let Some(text) = plain_text {
                self.generate_snippet(&text, query, 200)
            } else {
                title.clone()
            };
            
            Ok(SearchResult {
                document: DocumentRecord {
                    metadata: DocumentMetadata {
                        path: PathBuf::from(path),
                        title,
                        content_hash,
                        size: size as u64,
                        word_count: word_count as usize,
                        created_at,
                        modified_at,
                        indexed_at,
                        tags: Vec::new(), // Will be filled separately if needed
                        links: Vec::new(), // Will be filled separately if needed
                        file_type,
                        language,
                        custom_fields,
                    },
                    snippet: Some(snippet),
                    highlight: None,
                },
                score,
                match_type: MatchType::FullText,
                matched_content: Some(query.to_string()),
                matched_blocks: Vec::new(),
                context: SearchContext {
                    surrounding_content: None,
                    related_documents: Vec::new(),
                    related_tags: Vec::new(),
                    backlinks: Vec::new(),
                },
            })
        })?;
        
        for row in rows {
            results.push(row?);
        }
        
        let search_time = start_time.elapsed().as_millis() as f64;
        
        // Log search analytics
        self.connection.execute(
            "INSERT INTO search_analytics (query_text, query_type, results_count, execution_time_ms)
             VALUES (?, 'text_search', ?, ?)",
            params![query, results.len() as i32, search_time],
        )?;
        
        debug!("Text search completed in {:.2}ms with {} results", search_time, results.len());
        Ok(results)
    }
    
    async fn get_document(&self, path: &Path) -> Result<Option<DocumentRecord>> {
        let path_str = path.to_string_lossy();
        
        let mut stmt = self.connection.prepare(
            "SELECT d.*, dc.plain_text
             FROM documents d
             LEFT JOIN document_content dc ON d.id = dc.document_id
             WHERE d.path = ?"
        )?;
        
        let result = stmt.query_row(params![path_str], |row| {
            let title: String = row.get("title")?;
            let content_hash: String = row.get("content_hash")?;
            let size: i64 = row.get("size")?;
            let word_count: i32 = row.get("word_count")?;
            let created_at: DateTime<Utc> = row.get("created_at")?;
            let modified_at: DateTime<Utc> = row.get("modified_at")?;
            let indexed_at: DateTime<Utc> = row.get("indexed_at")?;
            let file_type_str: String = row.get("file_type")?;
            let language: Option<String> = row.get("language")?;
            let custom_fields_str: String = row.get("custom_fields")?;
            let plain_text: Option<String> = row.get("plain_text")?;
            
            let file_type: FileType = serde_json::from_str(&file_type_str).unwrap_or(FileType::Unknown);
            let custom_fields: HashMap<String, serde_json::Value> = 
                serde_json::from_str(&custom_fields_str).unwrap_or_default();
            
            Ok(DocumentRecord {
                metadata: DocumentMetadata {
                    path: path.to_path_buf(),
                    title,
                    content_hash,
                    size: size as u64,
                    word_count: word_count as usize,
                    created_at,
                    modified_at,
                    indexed_at,
                    tags: Vec::new(), // TODO: Load tags
                    links: Vec::new(), // TODO: Load links
                    file_type,
                    language,
                    custom_fields,
                },
                snippet: plain_text.map(|text| {
                    if text.len() > 200 {
                        format!("{}...", &text[..200])
                    } else {
                        text
                    }
                }),
                highlight: None,
            })
        });
        
        match result {
            Ok(record) => Ok(Some(record)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    async fn get_documents_by_tag(&self, tag: &str) -> Result<Vec<DocumentRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT d.path, d.title, d.content_hash, d.size, d.word_count,
                    d.created_at, d.modified_at, d.indexed_at, d.file_type, d.language, d.custom_fields
             FROM documents d
             JOIN document_tags dt ON d.id = dt.document_id
             JOIN tags t ON dt.tag_id = t.id
             WHERE t.tag = ?
             ORDER BY d.modified_at DESC"
        )?;
        
        let rows = stmt.query_map(params![tag], |row| {
            let path: String = row.get(0)?;
            let title: String = row.get(1)?;
            let content_hash: String = row.get(2)?;
            let size: i64 = row.get(3)?;
            let word_count: i32 = row.get(4)?;
            let created_at: DateTime<Utc> = row.get(5)?;
            let modified_at: DateTime<Utc> = row.get(6)?;
            let indexed_at: DateTime<Utc> = row.get(7)?;
            let file_type_str: String = row.get(8)?;
            let language: Option<String> = row.get(9)?;
            let custom_fields_str: String = row.get(10)?;
            
            let file_type: FileType = serde_json::from_str(&file_type_str).unwrap_or(FileType::Unknown);
            let custom_fields: HashMap<String, serde_json::Value> = 
                serde_json::from_str(&custom_fields_str).unwrap_or_default();
            
            Ok(DocumentRecord {
                metadata: DocumentMetadata {
                    path: PathBuf::from(path),
                    title,
                    content_hash,
                    size: size as u64,
                    word_count: word_count as usize,
                    created_at,
                    modified_at,
                    indexed_at,
                    tags: vec![tag.to_string()],
                    links: Vec::new(),
                    file_type,
                    language,
                    custom_fields,
                },
                snippet: None,
                highlight: None,
            })
        })?;
        
        let mut documents = Vec::new();
        for row in rows {
            documents.push(row?);
        }
        
        Ok(documents)
    }
    
    async fn get_recent_documents(&self, limit: usize) -> Result<Vec<DocumentRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT d.path, d.title, d.content_hash, d.size, d.word_count,
                    d.created_at, d.modified_at, d.indexed_at, d.file_type, d.language, d.custom_fields
             FROM documents d
             ORDER BY d.modified_at DESC
             LIMIT ?"
        )?;
        
        let rows = stmt.query_map(params![limit], |row| {
            let path: String = row.get(0)?;
            let title: String = row.get(1)?;
            let content_hash: String = row.get(2)?;
            let size: i64 = row.get(3)?;
            let word_count: i32 = row.get(4)?;
            let created_at: DateTime<Utc> = row.get(5)?;
            let modified_at: DateTime<Utc> = row.get(6)?;
            let indexed_at: DateTime<Utc> = row.get(7)?;
            let file_type_str: String = row.get(8)?;
            let language: Option<String> = row.get(9)?;
            let custom_fields_str: String = row.get(10)?;
            
            let file_type: FileType = serde_json::from_str(&file_type_str).unwrap_or(FileType::Unknown);
            let custom_fields: HashMap<String, serde_json::Value> = 
                serde_json::from_str(&custom_fields_str).unwrap_or_default();
            
            Ok(DocumentRecord {
                metadata: DocumentMetadata {
                    path: PathBuf::from(path),
                    title,
                    content_hash,
                    size: size as u64,
                    word_count: word_count as usize,
                    created_at,
                    modified_at,
                    indexed_at,
                    tags: Vec::new(),
                    links: Vec::new(),
                    file_type,
                    language,
                    custom_fields,
                },
                snippet: None,
                highlight: None,
            })
        })?;
        
        let mut documents = Vec::new();
        for row in rows {
            documents.push(row?);
        }
        
        Ok(documents)
    }
    
    async fn update_document_metadata(&self, path: &Path, metadata: &DocumentMetadata) -> Result<()> {
        self.store_document_metadata(metadata).await
    }
    
    async fn remove_document(&self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy();
        
        // DuckDB will cascade delete from related tables due to foreign key constraints
        let deleted = self.connection.execute(
            "DELETE FROM documents WHERE path = ?",
            params![path_str],
        )?;
        
        if deleted > 0 {
            debug!("Removed document {} from DuckDB", path.display());
        }
        
        Ok(())
    }
    
    async fn get_stats(&self) -> Result<StorageStats> {
        // Get basic counts
        let total_documents: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM documents",
            [],
            |row| row.get(0)
        )?;
        
        let total_tags: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM tags",
            [],
            |row| row.get(0)
        )?;
        
        let total_links: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM links",
            [],
            |row| row.get(0)
        )?;
        
        // Get storage size (approximate)
        let storage_size = self.estimate_storage_size().await?;
        
        // Get performance metrics
        let avg_search_time: Option<f64> = self.connection.query_row(
            "SELECT AVG(execution_time_ms) FROM search_analytics WHERE timestamp > datetime('now', '-1 day')",
            [],
            |row| row.get(0)
        ).unwrap_or(None);
        
        let total_queries: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM search_analytics",
            [],
            |row| row.get(0)
        )?;
        
        Ok(StorageStats {
            total_documents: total_documents as usize,
            total_embeddings: 0, // DuckDB doesn't store embeddings
            total_blocks: 0,
            storage_size_bytes: storage_size,
            embedding_size_bytes: 0,
            metadata_size_bytes: storage_size,
            last_optimized: None,
            performance_metrics: super::PerformanceMetrics {
                avg_search_latency_ms: avg_search_time.unwrap_or(0.0),
                avg_indexing_time_ms: 0.0,
                cache_hit_rate: 0.0,
                total_queries: total_queries as u64,
                total_documents_indexed: total_documents as u64,
            },
        })
    }
    
    async fn optimize(&self) -> Result<()> {
        info!("Optimizing DuckDB database");
        
        // Analyze tables for better query planning
        self.connection.execute("ANALYZE", [])?;
        
        // Checkpoint WAL if enabled
        if self.config.wal_mode {
            self.connection.execute("CHECKPOINT", [])?;
        }
        
        // Vacuum to reclaim space
        self.connection.execute("VACUUM", [])?;
        
        info!("DuckDB optimization completed");
        Ok(())
    }
    
    async fn backup(&self, backup_path: &Path) -> Result<()> {
        info!("Backing up DuckDB to {}", backup_path.display());
        
        let backup_file = backup_path.join("metadata.duckdb");
        
        // Use DuckDB's EXPORT DATABASE functionality
        self.connection.execute(
            &format!("EXPORT DATABASE '{}'", backup_file.display()),
            [],
        )?;
        
        info!("DuckDB backup completed");
        Ok(())
    }
}

impl DuckDBStore {
    /// Store document content separately for full-text search
    pub async fn store_document_content(&self, doc_id: i64, content: &str, plain_text: &str) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO document_content (document_id, content, plain_text)
             VALUES (?, ?, ?)",
            params![doc_id, content, plain_text],
        )?;
        
        Ok(())
    }
    
    /// Get document ID by path
    pub async fn get_document_id(&self, path: &Path) -> Result<Option<i64>> {
        let path_str = path.to_string_lossy();
        
        let result = self.connection.query_row(
            "SELECT id FROM documents WHERE path = ?",
            params![path_str],
            |row| row.get(0)
        );
        
        match result {
            Ok(id) => Ok(Some(id)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    /// Get top tags by usage
    pub async fn get_top_tags(&self, limit: usize) -> Result<Vec<TagStats>> {
        let mut stmt = self.connection.prepare(
            "SELECT tag, document_count, last_used 
             FROM tag_popularity 
             ORDER BY document_count DESC 
             LIMIT ?"
        )?;
        
        let rows = stmt.query_map(params![limit], |row| {
            Ok(TagStats {
                tag: row.get(0)?,
                count: row.get::<_, i64>(1)? as usize,
                avg_score: 0.0, // Could calculate this if needed
            })
        })?;
        
        let mut tags = Vec::new();
        for row in rows {
            tags.push(row?);
        }
        
        Ok(tags)
    }
    
    /// Get recent document activity
    pub async fn get_recent_activity(&self, limit: usize) -> Result<Vec<ActivityRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT d.path, dal.access_type, dal.timestamp
             FROM document_access_log dal
             JOIN documents d ON dal.document_id = d.id
             ORDER BY dal.timestamp DESC
             LIMIT ?"
        )?;
        
        let rows = stmt.query_map(params![limit], |row| {
            let path: String = row.get(0)?;
            let access_type_str: String = row.get(1)?;
            let timestamp: DateTime<Utc> = row.get(2)?;
            
            let activity_type = match access_type_str.as_str() {
                "created" => ActivityType::Created,
                "modified" => ActivityType::Modified,
                "accessed" => ActivityType::Accessed,
                "indexed" => ActivityType::Indexed,
                _ => ActivityType::Accessed,
            };
            
            Ok(ActivityRecord {
                document_path: PathBuf::from(path),
                activity_type,
                timestamp,
            })
        })?;
        
        let mut activities = Vec::new();
        for row in rows {
            activities.push(row?);
        }
        
        Ok(activities)
    }
    
    /// Generate a snippet around the query match
    fn generate_snippet(&self, content: &str, query: &str, max_length: usize) -> String {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        if let Some(pos) = content_lower.find(&query_lower) {
            let start = pos.saturating_sub(max_length / 2);
            let end = (pos + query.len() + max_length / 2).min(content.len());
            
            let mut snippet = content[start..end].to_string();
            
            if start > 0 {
                snippet = format!("...{}", snippet);
            }
            if end < content.len() {
                snippet = format!("{}...", snippet);
            }
            
            snippet
        } else {
            content.chars().take(max_length).collect::<String>()
        }
    }
    
    /// Estimate storage size
    async fn estimate_storage_size(&self) -> Result<u64> {
        // Get file size of the database
        if let Ok(metadata) = tokio::fs::metadata(&self.config.database_path).await {
            Ok(metadata.len())
        } else {
            Ok(0)
        }
    }
    
    /// Export data to parquet for analytics
    pub async fn export_to_parquet(&self, output_path: &Path) -> Result<()> {
        info!("Exporting DuckDB data to parquet at {}", output_path.display());
        
        // Create output directory
        tokio::fs::create_dir_all(output_path).await?;
        
        // Export documents table
        self.connection.execute(
            &format!(
                "COPY documents TO '{}/documents.parquet' (FORMAT PARQUET)",
                output_path.display()
            ),
            [],
        )?;
        
        // Export tags and document_tags
        self.connection.execute(
            &format!(
                "COPY (
                    SELECT d.path, d.title, t.tag, dt.document_id, dt.tag_id
                    FROM documents d
                    JOIN document_tags dt ON d.id = dt.document_id
                    JOIN tags t ON dt.tag_id = t.id
                ) TO '{}/document_tags.parquet' (FORMAT PARQUET)",
                output_path.display()
            ),
            [],
        )?;
        
        // Export search analytics
        self.connection.execute(
            &format!(
                "COPY search_analytics TO '{}/search_analytics.parquet' (FORMAT PARQUET)",
                output_path.display()
            ),
            [],
        )?;
        
        info!("Parquet export completed");
        Ok(())
    }
    
    /// Import analytics data for machine learning
    pub async fn get_ml_features(&self) -> Result<Vec<MLFeature>> {
        let mut stmt = self.connection.prepare(
            "SELECT 
                d.word_count,
                d.file_type,
                COUNT(DISTINCT dt.tag_id) as tag_count,
                COUNT(DISTINCT l.id) as link_count,
                EXTRACT(HOUR FROM d.created_at) as created_hour,
                EXTRACT(DOW FROM d.created_at) as created_day_of_week,
                julianday('now') - julianday(d.modified_at) as days_since_modified
             FROM documents d
             LEFT JOIN document_tags dt ON d.id = dt.document_id
             LEFT JOIN links l ON d.id = l.source_document_id
             GROUP BY d.id"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok(MLFeature {
                word_count: row.get::<_, i32>(0)? as f64,
                file_type: row.get(1)?,
                tag_count: row.get::<_, i64>(2)? as f64,
                link_count: row.get::<_, i64>(3)? as f64,
                created_hour: row.get::<_, i32>(4)? as f64,
                created_day_of_week: row.get::<_, i32>(5)? as f64,
                days_since_modified: row.get::<_, f64>(6)?,
            })
        })?;
        
        let mut features = Vec::new();
        for row in rows {
            features.push(row?);
        }
        
        Ok(features)
    }
}

/// ML features extracted from document metadata
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MLFeature {
    pub word_count: f64,
    pub file_type: String,
    pub tag_count: f64,
    pub link_count: f64,
    pub created_hour: f64,
    pub created_day_of_week: f64,
    pub days_since_modified: f64,
}