pub mod cache;
pub mod crdt;
pub mod embeddings;
pub mod indexer;
pub mod parser;
pub mod search;

use crate::Result;
use std::path::PathBuf;

pub struct Vault {
    path: PathBuf,
    indexer: indexer::Indexer,
    parser: parser::Parser,
    cache: cache::Cache,
    search: search::Search,
}

impl Vault {
    pub fn new(path: PathBuf) -> Result<Self> {
        let indexer = indexer::Indexer::new(path.clone())?;
        let parser = parser::Parser::new()?;
        let cache = cache::Cache::new()?;
        let search = search::Search::new()?;
        
        Ok(Self {
            path,
            indexer,
            parser,
            cache,
            search,
        })
    }
    
    pub async fn index(&mut self) -> Result<()> {
        self.indexer.index_all().await
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<search::SearchResult>> {
        self.search.search(query).await
    }
}
