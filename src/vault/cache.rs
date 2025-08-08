// src/vault/cache.rs - Smart caching layer
use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Cache {
    data: Arc<RwLock<HashMap<String, CachedItem>>>,
    max_size: usize,
}

#[derive(Debug, Clone)]
struct CachedItem {
    data: Vec<u8>,
    created_at: chrono::DateTime<chrono::Utc>,
    access_count: usize,
}

impl Cache {
    pub fn new(max_size: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }
    
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut cache = self.data.write().await;
        if let Some(item) = cache.get_mut(key) {
            item.access_count += 1;
            Some(item.data.clone())
        } else {
            None
        }
    }
    
    pub async fn set(&self, key: String, data: Vec<u8>) {
        let mut cache = self.data.write().await;
        
        // Simple LRU eviction if over capacity
        if cache.len() >= self.max_size {
            if let Some(lru_key) = cache
                .iter()
                .min_by_key(|(_, item)| item.access_count)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&lru_key);
            }
        }
        
        cache.insert(key, CachedItem {
            data,
            created_at: chrono::Utc::now(),
            access_count: 1,
        });
    }
}

// Smart caching (stub)