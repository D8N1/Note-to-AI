use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use blake3::{Hash, Hasher};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tokio::fs as async_fs;
use walkdir::WalkDir;
use rusqlite::{Connection, params};
use crate::logger::Logger;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndex {
    pub path: PathBuf,
    pub hash: String,
    pub size: u64,
    pub modified: u64,
    pub indexed_at: u64,
    pub file_type: FileType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileType {
    Markdown,
    Text,
    Image,
    Audio,
    Video,
    Document,
    Unknown,
}

impl FileType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "md" | "markdown" => FileType::Markdown,
            "txt" | "text" => FileType::Text,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => FileType::Image,
            "mp3" | "wav" | "flac" | "ogg" | "m4a" => FileType::Audio,
            "mp4" | "avi" | "mkv" | "webm" | "mov" => FileType::Video,
            "pdf" | "doc" | "docx" | "rtf" | "odt" => FileType::Document,
            _ => FileType::Unknown,
        }
    }
}

pub struct VaultIndexer {
    db_path: PathBuf,
    vault_path: PathBuf,
    ignore_patterns: HashSet<String>,
    logger: Logger,
}

impl VaultIndexer {
    pub fn new(db_path: PathBuf, vault_path: PathBuf) -> Result<Self> {
        let mut ignore_patterns = HashSet::new();
        ignore_patterns.insert(".git".to_string());
        ignore_patterns.insert(".obsidian".to_string());
        ignore_patterns.insert("node_modules".to_string());
        ignore_patterns.insert(".DS_Store".to_string());
        ignore_patterns.insert("Thumbs.db".to_string());

        Ok(Self {
            db_path,
            vault_path,
            ignore_patterns,
            logger: Logger::new("VaultIndexer"),
        })
    }

    pub fn add_ignore_pattern(&mut self, pattern: String) {
        self.ignore_patterns.insert(pattern);
    }

    pub async fn initialize_db(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .context("Failed to open database connection")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS file_index (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT UNIQUE NOT NULL,
                hash TEXT NOT NULL,
                size INTEGER NOT NULL,
                modified INTEGER NOT NULL,
                indexed_at INTEGER NOT NULL,
                file_type TEXT NOT NULL,
                content_hash TEXT,
                metadata TEXT
            )",
            [],
        ).context("Failed to create file_index table")?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_hash ON file_index(hash)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_type ON file_index(file_type)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_modified ON file_index(modified)",
            [],
        )?;

        self.logger.info("Database initialized successfully");
        Ok(())
    }

    pub async fn full_index(&self) -> Result<IndexStats> {
        self.logger.info("Starting full vault indexing");
        let start_time = std::time::Instant::now();

        let mut stats = IndexStats::default();
        let entries = self.scan_vault_files()?;

        for entry in entries {
            match self.index_file(&entry).await {
                Ok(action) => {
                    match action {
                        IndexAction::Added => stats.added += 1,
                        IndexAction::Updated => stats.updated += 1,
                        IndexAction::Skipped => stats.skipped += 1,
                    }
                }
                Err(e) => {
                    self.logger.error(&format!("Failed to index {}: {}", entry.display(), e));
                    stats.errors += 1;
                }
            }
        }

        // Clean up deleted files
        let deleted = self.clean_deleted_files().await?;
        stats.deleted = deleted;

        let duration = start_time.elapsed();
        self.logger.info(&format!(
            "Full indexing completed in {:?}: {} added, {} updated, {} deleted, {} skipped, {} errors",
            duration, stats.added, stats.updated, stats.deleted, stats.skipped, stats.errors
        ));

        Ok(stats)
    }

    pub async fn incremental_index(&self, paths: Vec<PathBuf>) -> Result<IndexStats> {
        self.logger.info(&format!("Starting incremental indexing of {} files", paths.len()));
        let mut stats = IndexStats::default();

        for path in paths {
            if !path.exists() {
                // File was deleted
                if self.remove_file_from_index(&path).await? {
                    stats.deleted += 1;
                }
                continue;
            }

            match self.index_file(&path).await {
                Ok(action) => {
                    match action {
                        IndexAction::Added => stats.added += 1,
                        IndexAction::Updated => stats.updated += 1,
                        IndexAction::Skipped => stats.skipped += 1,
                    }
                }
                Err(e) => {
                    self.logger.error(&format!("Failed to index {}: {}", path.display(), e));
                    stats.errors += 1;
                }
            }
        }

        self.logger.info(&format!(
            "Incremental indexing completed: {} added, {} updated, {} deleted, {} skipped, {} errors",
            stats.added, stats.updated, stats.deleted, stats.skipped, stats.errors
        ));

        Ok(stats)
    }

    async fn index_file(&self, path: &Path) -> Result<IndexAction> {
        if self.should_ignore_file(path) {
            return Ok(IndexAction::Skipped);
        }

        let metadata = fs::metadata(path)
            .context("Failed to read file metadata")?;

        if metadata.is_dir() {
            return Ok(IndexAction::Skipped);
        }

        let modified = metadata.modified()?
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Check if file needs indexing
        if let Some(existing) = self.get_file_index(path).await? {
            if existing.modified >= modified && existing.size == metadata.len() {
                return Ok(IndexAction::Skipped);
            }
        }

        // Calculate BLAKE3 hash
        let content = async_fs::read(path).await
            .context("Failed to read file content")?;
        
        let hash = self.calculate_blake3_hash(&content);
        
        let file_type = path.extension()
            .and_then(|ext| ext.to_str())
            .map(FileType::from_extension)
            .unwrap_or(FileType::Unknown);

        let file_index = FileIndex {
            path: path.to_path_buf(),
            hash: hash.to_string(),
            size: metadata.len(),
            modified,
            indexed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs(),
            file_type,
        };

        let action = if self.get_file_index(path).await?.is_some() {
            self.update_file_index(&file_index).await?;
            IndexAction::Updated
        } else {
            self.insert_file_index(&file_index).await?;
            IndexAction::Added
        };

        Ok(action)
    }

    fn calculate_blake3_hash(&self, content: &[u8]) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(content);
        hasher.finalize()
    }

    fn scan_vault_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for entry in WalkDir::new(&self.vault_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            if path.is_file() && !self.should_ignore_file(path) {
                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    fn should_ignore_file(&self, path: &Path) -> bool {
        // Check ignore patterns
        for component in path.components() {
            if let Some(name) = component.as_os_str().to_str() {
                if self.ignore_patterns.contains(name) {
                    return true;
                }
            }
        }

        // Check file extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "tmp" | "temp" | "lock" | "swp" | "bak" => return true,
                _ => {}
            }
        }

        // Check filename patterns
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') && name != ".md" {
                return true;
            }
        }

        false
    }

    async fn get_file_index(&self, path: &Path) -> Result<Option<FileIndex>> {
        let conn = Connection::open(&self.db_path)?;
        
        let path_str = path.to_string_lossy();
        let mut stmt = conn.prepare(
            "SELECT path, hash, size, modified, indexed_at, file_type 
             FROM file_index WHERE path = ?1"
        )?;

        let result = stmt.query_row(params![path_str], |row| {
            Ok(FileIndex {
                path: PathBuf::from(row.get::<_, String>(0)?),
                hash: row.get(1)?,
                size: row.get(2)?,
                modified: row.get(3)?,
                indexed_at: row.get(4)?,
                file_type: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(FileType::Unknown),
            })
        });

        match result {
            Ok(index) => Ok(Some(index)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn insert_file_index(&self, index: &FileIndex) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        conn.execute(
            "INSERT INTO file_index (path, hash, size, modified, indexed_at, file_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                index.path.to_string_lossy(),
                index.hash,
                index.size,
                index.modified,
                index.indexed_at,
                serde_json::to_string(&index.file_type)?
            ],
        )?;

        Ok(())
    }

    async fn update_file_index(&self, index: &FileIndex) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        conn.execute(
            "UPDATE file_index 
             SET hash = ?2, size = ?3, modified = ?4, indexed_at = ?5, file_type = ?6
             WHERE path = ?1",
            params![
                index.path.to_string_lossy(),
                index.hash,
                index.size,
                index.modified,
                index.indexed_at,
                serde_json::to_string(&index.file_type)?
            ],
        )?;

        Ok(())
    }

    async fn remove_file_from_index(&self, path: &Path) -> Result<bool> {
        let conn = Connection::open(&self.db_path)?;
        
        let changed = conn.execute(
            "DELETE FROM file_index WHERE path = ?1",
            params![path.to_string_lossy()],
        )?;

        Ok(changed > 0)
    }

    async fn clean_deleted_files(&self) -> Result<usize> {
        let conn = Connection::open(&self.db_path)?;
        
        let mut stmt = conn.prepare("SELECT path FROM file_index")?;
        let paths: Vec<String> = stmt.query_map([], |row| {
            Ok(row.get::<_, String>(0)?)
        })?.collect::<Result<Vec<_>, _>>()?;

        let mut deleted_count = 0;
        for path_str in paths {
            let path = PathBuf::from(&path_str);
            if !path.exists() {
                conn.execute(
                    "DELETE FROM file_index WHERE path = ?1",
                    params![path_str],
                )?;
                deleted_count += 1;
                self.logger.debug(&format!("Removed deleted file from index: {}", path_str));
            }
        }

        Ok(deleted_count)
    }

    pub async fn get_files_by_type(&self, file_type: FileType) -> Result<Vec<FileIndex>> {
        let conn = Connection::open(&self.db_path)?;
        
        let mut stmt = conn.prepare(
            "SELECT path, hash, size, modified, indexed_at, file_type 
             FROM file_index WHERE file_type = ?1 ORDER BY modified DESC"
        )?;

        let file_type_str = serde_json::to_string(&file_type)?;
        let rows = stmt.query_map(params![file_type_str], |row| {
            Ok(FileIndex {
                path: PathBuf::from(row.get::<_, String>(0)?),
                hash: row.get(1)?,
                size: row.get(2)?,
                modified: row.get(3)?,
                indexed_at: row.get(4)?,
                file_type: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(FileType::Unknown),
            })
        })?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }

        Ok(files)
    }

    pub async fn get_recent_files(&self, limit: usize) -> Result<Vec<FileIndex>> {
        let conn = Connection::open(&self.db_path)?;
        
        let mut stmt = conn.prepare(
            "SELECT path, hash, size, modified, indexed_at, file_type 
             FROM file_index ORDER BY modified DESC LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            Ok(FileIndex {
                path: PathBuf::from(row.get::<_, String>(0)?),
                hash: row.get(1)?,
                size: row.get(2)?,
                modified: row.get(3)?,
                indexed_at: row.get(4)?,
                file_type: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(FileType::Unknown),
            })
        })?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }

        Ok(files)
    }

    pub async fn get_stats(&self) -> Result<VaultStats> {
        let conn = Connection::open(&self.db_path)?;
        
        let total_files: i64 = conn.query_row(
            "SELECT COUNT(*) FROM file_index",
            [],
            |row| row.get(0)
        )?;

        let total_size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(size), 0) FROM file_index",
            [],
            |row| row.get(0)
        )?;

        let mut stmt = conn.prepare(
            "SELECT file_type, COUNT(*) FROM file_index GROUP BY file_type"
        )?;

        let mut type_counts = std::collections::HashMap::new();
        let rows = stmt.query_map([], |row| {
            let file_type_str: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((file_type_str, count))
        })?;

        for row in rows {
            let (file_type_str, count) = row?;
            if let Ok(file_type) = serde_json::from_str::<FileType>(&file_type_str) {
                type_counts.insert(file_type, count as usize);
            }
        }

        Ok(VaultStats {
            total_files: total_files as usize,
            total_size: total_size as u64,
            type_counts,
        })
    }
}

#[derive(Debug, Default)]
pub struct IndexStats {
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
    pub skipped: usize,
    pub errors: usize,
}

#[derive(Debug)]
pub struct VaultStats {
    pub total_files: usize,
    pub total_size: u64,
    pub type_counts: std::collections::HashMap<FileType, usize>,
}

#[derive(Debug)]
enum IndexAction {
    Added,
    Updated,
    Skipped,
}

