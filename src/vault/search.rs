use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use rusqlite::{Connection, params};
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::vault::parser::{ParsedDocument, BlockType};
use crate::vault::embeddings::EmbeddingVector;
use crate::logger::Logger;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: SearchDocument,
    pub score: f32,
    pub match_type: MatchType,
    pub matched_content: String,
    pub context: SearchContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocument {
    pub path: PathBuf,
    pub title: String,
    pub snippet: String,
    pub tags: Vec<String>,
    pub modified: u64,
    pub word_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    Semantic,      // Vector similarity
    Exact,         // Exact text match
    Fuzzy,         // Fuzzy text match
    Tag,           // Tag match
    Title,         // Title match
    Link,          // Backlink match
    Hybrid,        // Combination of multiple types
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    pub matched_blocks: Vec<MatchedBlock>,
    pub surrounding_context: String,
    pub backlinks: Vec<String>,
    pub related_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedBlock {
    pub block_type: BlockType,
    pub content: String,
    pub score: f32,
    pub start_pos: usize,
    pub end_pos: usize,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    pub filters: SearchFilters,
    pub options: SearchOptions,
}

#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub tags: Vec<String>,
    pub paths: Vec<PathBuf>,
    pub file_types: Vec<String>,
    pub date_range: Option<(u64, u64)>,
    pub min_words: Option<usize>,
    pub max_words: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub limit: usize,
    pub similarity_threshold: f32,
    pub include_context: bool,
    pub context_window: usize,
    pub boost_recent: bool,
    pub boost_tags: bool,
    pub boost_titles: bool,
    pub hybrid_search: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            similarity_threshold: 0.7,
            include_context: true,
            context_window: 100,
            boost_recent: true,
            boost_tags: true,
            boost_titles: true,
            hybrid_search: true,
        }
    }
}

pub struct VectorSearchEngine {
    db_path: PathBuf,
    index: Arc<RwLock<VectorIndex>>,
    logger: Logger,
}

#[derive(Debug)]
struct VectorIndex {
    documents: HashMap<String, IndexedDocument>,
    embeddings: HashMap<String, Vec<f32>>,
    block_embeddings: HashMap<String, Vec<BlockEmbedding>>,
    tag_index: HashMap<String, HashSet<String>>,
    title_index: HashMap<String, String>,
    link_graph: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Clone)]
struct IndexedDocument {
    pub path: PathBuf,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub modified: u64,
    pub word_count: usize,
    pub blocks: Vec<IndexedBlock>,
}

#[derive(Debug, Clone)]
struct IndexedBlock {
    pub block_type: BlockType,
    pub content: String,
    pub start_pos: usize,
    pub end_pos: usize,
    pub embedding_id: String,
}

#[derive(Debug, Clone)]
struct BlockEmbedding {
    pub block_id: String,
    pub embedding: Vec<f32>,
    pub content: String,
    pub block_type: BlockType,
}

impl VectorSearchEngine {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let index = VectorIndex {
            documents: HashMap::new(),
            embeddings: HashMap::new(),
            block_embeddings: HashMap::new(),
            tag_index: HashMap::new(),
            title_index: HashMap::new(),
            link_graph: HashMap::new(),
        };

        Ok(Self {
            db_path,
            index: Arc::new(RwLock::new(index)),
            logger: Logger::new("VectorSearchEngine"),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        self.create_search_tables().await?;
        self.load_index_from_db().await?;
        self.logger.info("Vector search engine initialized");
        Ok(())
    }

    async fn create_search_tables(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;

        // Document embeddings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS document_embeddings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_path TEXT UNIQUE NOT NULL,
                embedding BLOB NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Block embeddings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS block_embeddings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_path TEXT NOT NULL,
                block_id TEXT UNIQUE NOT NULL,
                block_type TEXT NOT NULL,
                content TEXT NOT NULL,
                embedding BLOB NOT NULL,
                start_pos INTEGER NOT NULL,
                end_pos INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Search index for fast text queries
        conn.execute(
            "CREATE TABLE IF NOT EXISTS search_index (
                document_path TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL,
                modified INTEGER NOT NULL,
                word_count INTEGER NOT NULL
            )",
            [],
        )?;

        // Create FTS5 table for full-text search
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
                title, content, tags, content=search_index, content_rowid=rowid
            )",
            [],
        )?;

        // Indexes
        conn.execute("CREATE INDEX IF NOT EXISTS idx_doc_embeddings_path ON document_embeddings(document_path)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_block_embeddings_doc ON block_embeddings(document_path)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_search_tags ON search_index(tags)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_search_modified ON search_index(modified)", [])?;

        Ok(())
    }

    pub async fn index_document(&self, document: &ParsedDocument, embedding: &EmbeddingVector) -> Result<()> {
        let doc_id = document.path.to_string_lossy().to_string();
        
        // Store document embedding
        self.store_document_embedding(&doc_id, &embedding.vector).await?;
        
        // Store block embeddings (if available)
        if let Some(block_embeddings) = &embedding.block_embeddings {
            self.store_block_embeddings(&doc_id, block_embeddings).await?;
        }

        // Update in-memory index
        let mut index = self.index.write().await;
        
        let indexed_doc = IndexedDocument {
            path: document.path.clone(),
            title: document.title.clone(),
            content: document.content.clone(),
            tags: document.tags.clone(),
            modified: document.metadata.last_parsed.timestamp() as u64,
            word_count: document.metadata.word_count,
            blocks: document.blocks.iter().enumerate().map(|(i, block)| {
                IndexedBlock {
                    block_type: block.block_type.clone(),
                    content: block.content.clone(),
                    start_pos: block.position.start,
                    end_pos: block.position.end,
                    embedding_id: format!("{}_{}", doc_id, i),
                }
            }).collect(),
        };

        index.documents.insert(doc_id.clone(), indexed_doc);
        index.embeddings.insert(doc_id.clone(), embedding.vector.clone());

        // Update auxiliary indexes
        for tag in &document.tags {
            index.tag_index.entry(tag.clone())
                .or_insert_with(HashSet::new)
                .insert(doc_id.clone());
        }

        index.title_index.insert(document.title.clone(), doc_id.clone());

        // Update link graph
        for link in &document.links {
            index.link_graph.entry(link.target.clone())
                .or_insert_with(HashSet::new)
                .insert(doc_id.clone());
        }

        // Update search index
        self.update_search_index(document).await?;

        self.logger.debug(&format!("Indexed document: {}", document.path.display()));
        Ok(())
    }

    async fn store_document_embedding(&self, doc_id: &str, embedding: &[f32]) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let embedding_bytes = self.serialize_embedding(embedding)?;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO document_embeddings (document_path, embedding, updated_at)
             VALUES (?1, ?2, ?3)",
            params![doc_id, embedding_bytes, now],
        )?;

        Ok(())
    }

    async fn store_block_embeddings(&self, doc_id: &str, block_embeddings: &[crate::vault::embeddings::BlockEmbedding]) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let now = chrono::Utc::now().timestamp();

        // Clear existing block embeddings for this document
        conn.execute(
            "DELETE FROM block_embeddings WHERE document_path = ?1",
            params![doc_id],
        )?;

        // Insert new block embeddings
        for (i, block_emb) in block_embeddings.iter().enumerate() {
            let block_id = format!("{}_{}", doc_id, i);
            let embedding_bytes = self.serialize_embedding(&block_emb.vector)?;

            conn.execute(
                "INSERT INTO block_embeddings 
                 (document_path, block_id, block_type, content, embedding, start_pos, end_pos, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    doc_id,
                    block_id,
                    serde_json::to_string(&BlockType::Paragraph)?, // Default, should be passed properly
                    block_emb.content.clone(),
                    embedding_bytes,
                    0, // start_pos - should be passed from block
                    0, // end_pos - should be passed from block
                    now
                ],
            )?;
        }

        Ok(())
    }

    async fn update_search_index(&self, document: &ParsedDocument) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        let tags_json = serde_json::to_string(&document.tags)?;
        
        conn.execute(
            "INSERT OR REPLACE INTO search_index 
             (document_path, title, content, tags, modified, word_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                document.path.to_string_lossy(),
                document.title,
                document.plain_text,
                tags_json,
                document.metadata.last_parsed.timestamp(),
                document.metadata.word_count
            ],
        )?;

        // Update FTS index
        conn.execute(
            "INSERT OR REPLACE INTO search_fts (title, content, tags)
             VALUES (?1, ?2, ?3)",
            params![
                document.title,
                document.plain_text,
                tags_json
            ],
        )?;

        Ok(())
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        if query.options.hybrid_search {
            // Combine multiple search strategies
            let semantic_results = self.semantic_search(&query.text, &query.options).await?;
            let text_results = self.text_search(&query.text, &query.options).await?;
            let tag_results = self.tag_search(&query.filters.tags, &query.options).await?;

            results = self.merge_search_results(semantic_results, text_results, tag_results, &query.options)?;
        } else {
            // Use primary search method
            results = self.semantic_search(&query.text, &query.options).await?;
        }

        // Apply filters
        results = self.apply_filters(results, &query.filters)?;

        // Sort and limit results
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(query.options.limit);

        Ok(results)
    }

    async fn semantic_search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        // This would use the embeddings engine to get query embedding
        // For now, we'll simulate with a placeholder
        let query_embedding = vec![0.0; 384]; // Placeholder - should come from embeddings engine

        let index = self.index.read().await;
        let mut results = Vec::new();

        for (doc_id, doc_embedding) in &index.embeddings {
            let similarity = self.cosine_similarity(&query_embedding, doc_embedding);
            
            if similarity >= options.similarity_threshold {
                if let Some(doc) = index.documents.get(doc_id) {
                    let search_doc = SearchDocument {
                        path: doc.path.clone(),
                        title: doc.title.clone(),
                        snippet: self.generate_snippet(&doc.content, query, 200),
                        tags: doc.tags.clone(),
                        modified: doc.modified,
                        word_count: doc.word_count,
                    };

                    let context = if options.include_context {
                        self.build_search_context(doc, query, &index).await?
                    } else {
                        SearchContext {
                            matched_blocks: Vec::new(),
                            surrounding_context: String::new(),
                            backlinks: Vec::new(),
                            related_tags: Vec::new(),
                        }
                    };

                    results.push(SearchResult {
                        document: search_doc,
                        score: similarity,
                        match_type: MatchType::Semantic,
                        matched_content: query.to_string(),
                        context,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn text_search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        let conn = Connection::open(&self.db_path)?;
        
        let mut stmt = conn.prepare(
            "SELECT document_path, title, content, tags, modified, word_count, 
                    bm25(search_fts) as score
             FROM search_fts 
             WHERE search_fts MATCH ?1
             ORDER BY score
             LIMIT ?2"
        )?;

        let rows = stmt.query_map(params![query, options.limit], |row| {
            let path: String = row.get(0)?;
            let title: String = row.get(1)?;
            let content: String = row.get(2)?;
            let tags_json: String = row.get(3)?;
            let modified: i64 = row.get(4)?;
            let word_count: i64 = row.get(5)?;
            let score: f64 = row.get(6)?;

            let tags: Vec<String> = serde_json::from_str(&tags_json)
                .unwrap_or_default();

            Ok(SearchResult {
                document: SearchDocument {
                    path: PathBuf::from(path),
                    title: title.clone(),
                    snippet: self.generate_snippet(&content, query, 200),
                    tags,
                    modified: modified as u64,
                    word_count: word_count as usize,
                },
                score: score as f32,
                match_type: MatchType::Exact,
                matched_content: query.to_string(),
                context: SearchContext {
                    matched_blocks: Vec::new(),
                    surrounding_context: String::new(),
                    backlinks: Vec::new(),
                    related_tags: Vec::new(),
                },
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    async fn tag_search(&self, tags: &[String], options: &SearchOptions) -> Result<Vec<SearchResult>> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        let index = self.index.read().await;
        let mut results = Vec::new();

        for (tag, doc_ids) in &index.tag_index {
            if tags.iter().any(|t| tag.contains(t) || t.contains(tag)) {
                for doc_id in doc_ids {
                    if let Some(doc) = index.documents.get(doc_id) {
                        let search_doc = SearchDocument {
                            path: doc.path.clone(),
                            title: doc.title.clone(),
                            snippet: self.generate_snippet(&doc.content, &tags.join(" "), 200),
                            tags: doc.tags.clone(),
                            modified: doc.modified,
                            word_count: doc.word_count,
                        };

                        results.push(SearchResult {
                            document: search_doc,
                            score: 0.8, // High score for tag matches
                            match_type: MatchType::Tag,
                            matched_content: tag.clone(),
                            context: SearchContext {
                                matched_blocks: Vec::new(),
                                surrounding_context: String::new(),
                                backlinks: Vec::new(),
                                related_tags: Vec::new(),
                            },
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    fn merge_search_results(
        &self,
        semantic: Vec<SearchResult>,
        text: Vec<SearchResult>,
        tag: Vec<SearchResult>,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let mut merged = HashMap::new();

        // Add semantic results
        for result in semantic {
            let key = result.document.path.clone();
            merged.insert(key, result);
        }

        // Boost text results
        for mut result in text {
            let key = result.document.path.clone();
            if let Some(existing) = merged.get_mut(&key) {
                existing.score = (existing.score + result.score * 0.7).max(existing.score);
                existing.match_type = MatchType::Hybrid;
            } else {
                result.score *= 0.7;
                merged.insert(key, result);
            }
        }

        // Boost tag results
        for mut result in tag {
            let key = result.document.path.clone();
            if let Some(existing) = merged.get_mut(&key) {
                existing.score = (existing.score + result.score * 0.5).max(existing.score);
                existing.match_type = MatchType::Hybrid;
            } else {
                result.score *= 0.5;
                merged.insert(key, result);
            }
        }

        Ok(merged.into_values().collect())
    }

    fn apply_filters(&self, mut results: Vec<SearchResult>, filters: &SearchFilters) -> Result<Vec<SearchResult>> {
        results.retain(|result| {
            // Filter by tags
            if !filters.tags.is_empty() {
                let has_tag = filters.tags.iter().any(|filter_tag| {
                    result.document.tags.iter().any(|doc_tag| doc_tag.contains(filter_tag))
                });
                if !has_tag {
                    return false;
                }
            }

            // Filter by paths
            if !filters.paths.is_empty() {
                let matches_path = filters.paths.iter().any(|filter_path| {
                    result.document.path.starts_with(filter_path)
                });
                if !matches_path {
                    return false;
                }
            }

            // Filter by date range
            if let Some((start, end)) = filters.date_range {
                if result.document.modified < start || result.document.modified > end {
                    return false;
                }
            }

            // Filter by word count
            if let Some(min_words) = filters.min_words {
                if result.document.word_count < min_words {
                    return false;
                }
            }

            if let Some(max_words) = filters.max_words {
                if result.document.word_count > max_words {
                    return false;
                }
            }

            true
        });

        Ok(results)
    }

    async fn build_search_context(&self, doc: &IndexedDocument, query: &str, index: &VectorIndex) -> Result<SearchContext> {
        let mut matched_blocks = Vec::new();
        
        // Find blocks that match the query
        for block in &doc.blocks {
            if block.content.to_lowercase().contains(&query.to_lowercase()) {
                matched_blocks.push(MatchedBlock {
                    block_type: block.block_type.clone(),
                    content: block.content.clone(),
                    score: 0.8, // Simple text match score
                    start_pos: block.start_pos,
                    end_pos: block.end_pos,
                });
            }
        }

        // Find backlinks
        let doc_path = doc.path.to_string_lossy().to_string();
        let mut backlinks = Vec::new();
        for (link_target, linking_docs) in &index.link_graph {
            if link_target.contains(&doc.title) || link_target.contains(&doc_path) {
                for linking_doc in linking_docs {
                    if let Some(linking_document) = index.documents.get(linking_doc) {
                        backlinks.push(linking_document.title.clone());
                    }
                }
            }
        }

        // Find related tags
        let mut related_tags = HashSet::new();
        for tag in &doc.tags {
            if let Some(tagged_docs) = index.tag_index.get(tag) {
                for tagged_doc in tagged_docs {
                    if let Some(tagged_document) = index.documents.get(tagged_doc) {
                        for other_tag in &tagged_document.tags {
                            if other_tag != tag {
                                related_tags.insert(other_tag.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(SearchContext {
            matched_blocks,
            surrounding_context: self.generate_snippet(&doc.content, query, 300),
            backlinks,
            related_tags: related_tags.into_iter().collect(),
        })
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

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

    fn serialize_embedding(&self, embedding: &[f32]) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(embedding.len() * 4);
        for &value in embedding {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Ok(bytes)
    }

    fn deserialize_embedding(&self, bytes: &[u8]) -> Result<Vec<f32>> {
        if bytes.len() % 4 != 0 {
            return Err(anyhow::anyhow!("Invalid embedding byte length"));
        }

        let mut embedding = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.chunks_exact(4) {
            let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            embedding.push(value);
        }
        Ok(embedding)
    }

    async fn load_index_from_db(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let mut index = self.index.write().await;

        // Load documents from search index
        let mut stmt = conn.prepare(
            "SELECT document_path, title, content, tags, modified, word_count FROM search_index"
        )?;

        let rows = stmt.query_map([], |row| {
            let path: String = row.get(0)?;
            let title: String = row.get(1)?;
            let content: String = row.get(2)?;
            let tags_json: String = row.get(3)?;
            let modified: i64 = row.get(4)?;
            let word_count: i64 = row.get(5)?;

            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            Ok((path, title, content, tags, modified as u64, word_count as usize))
        })?;

        for row in rows {
            let (path_str, title, content, tags, modified, word_count) = row?;
            let path = PathBuf::from(&path_str);

            let indexed_doc = IndexedDocument {
                path: path.clone(),
                title: title.clone(),
                content,
                tags: tags.clone(),
                modified,
                word_count,
                blocks: Vec::new(), // Will be populated separately if needed
            };

            index.documents.insert(path_str.clone(), indexed_doc);
            index.title_index.insert(title, path_str.clone());

            for tag in tags {
                index.tag_index.entry(tag)
                    .or_insert_with(HashSet::new)
                    .insert(path_str.clone());
            }
        }

        self.logger.info(&format!("Loaded {} documents into search index", index.documents.len()));
        Ok(())
    }

    pub async fn remove_document(&self, path: &PathBuf) -> Result<()> {
        let doc_id = path.to_string_lossy().to_string();
        
        // Remove from database
        let conn = Connection::open(&self.db_path)?;
        conn.execute("DELETE FROM document_embeddings WHERE document_path = ?1", params![doc_id])?;
        conn.execute("DELETE FROM block_embeddings WHERE document_path = ?1", params![doc_id])?;
        conn.execute("DELETE FROM search_index WHERE document_path = ?1", params![doc_id])?;
        conn.execute("DELETE FROM search_fts WHERE rowid IN (SELECT rowid FROM search_index WHERE document_path = ?1)", params![doc_id])?;

        // Remove from in-memory index
        let mut index = self.index.write().await;
        if let Some(doc) = index.documents.remove(&doc_id) {
            index.embeddings.remove(&doc_id);
            index.title_index.remove(&doc.title);
            
            for tag in &doc.tags {
                if let Some(tag_docs) = index.tag_index.get_mut(tag) {
                    tag_docs.remove(&doc_id);
                    if tag_docs.is_empty() {
                        index.tag_index.remove(tag);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn get_stats(&self) -> Result<SearchStats> {
        let index = self.index.read().await;
        
        Ok(SearchStats {
            total_documents: index.documents.len(),
            total_embeddings: index.embeddings.len(),
            total_tags: index.tag_index.len(),
            total_links: index.link_graph.len(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct SearchStats {
    pub total_documents: usize,
    pub total_embeddings: usize,
    pub total_tags: usize,
    pub total_links: usize,
}
