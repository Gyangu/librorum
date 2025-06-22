//! LRU Cache Implementation

use crate::vdfs::{VDFSResult, CacheKey, CacheValue};
use crate::vdfs::cache::{LocalCache, CacheEntry};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

/// Simple LRU cache implementation
pub struct LRUCache {
    capacity: usize,
    entries: RwLock<HashMap<CacheKey, CacheEntry>>,
    access_order: RwLock<Vec<CacheKey>>,
}

impl LRUCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: RwLock::new(HashMap::new()),
            access_order: RwLock::new(Vec::new()),
        }
    }
    
    fn evict_if_needed(&self) {
        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        
        while entries.len() > self.capacity && !access_order.is_empty() {
            let oldest_key = access_order.remove(0);
            entries.remove(&oldest_key);
        }
    }
    
    fn update_access_order(&self, key: &CacheKey) {
        let mut access_order = self.access_order.write().unwrap();
        
        // Remove key if it exists
        access_order.retain(|k| k != key);
        
        // Add to end (most recently used)
        access_order.push(key.clone());
    }
}

#[async_trait]
impl LocalCache for LRUCache {
    async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        let mut entries = self.entries.write().unwrap();
        
        if let Some(entry) = entries.get_mut(key) {
            entry.access();
            self.update_access_order(key);
            Some(entry.value.clone())
        } else {
            None
        }
    }
    
    async fn put(&self, key: CacheKey, value: CacheValue) -> VDFSResult<()> {
        let entry = CacheEntry::new(key.clone(), value);
        
        {
            let mut entries = self.entries.write().unwrap();
            entries.insert(key.clone(), entry);
        }
        
        self.update_access_order(&key);
        self.evict_if_needed();
        
        Ok(())
    }
    
    async fn invalidate(&self, key: &CacheKey) -> VDFSResult<()> {
        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        
        entries.remove(key);
        access_order.retain(|k| k != key);
        
        Ok(())
    }
    
    async fn clear(&self) -> VDFSResult<()> {
        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        
        entries.clear();
        access_order.clear();
        
        Ok(())
    }
    
    async fn size(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }
    
    async fn capacity(&self) -> usize {
        self.capacity
    }
}