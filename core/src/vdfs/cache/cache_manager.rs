//! Cache Manager Implementation

use crate::vdfs::{VDFSResult, CacheKey, CacheValue};
use crate::vdfs::cache::{LocalCache, DistributedCache, CachePolicy};

/// Main cache manager
pub struct CacheManager {
    local_cache: Box<dyn LocalCache>,
    distributed_cache: Option<Box<dyn DistributedCache>>,
    policy: CachePolicy,
}

impl CacheManager {
    pub fn new(
        local_cache: Box<dyn LocalCache>,
        distributed_cache: Option<Box<dyn DistributedCache>>,
        policy: CachePolicy,
    ) -> Self {
        Self {
            local_cache,
            distributed_cache,
            policy,
        }
    }
    
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // Try local cache first
        if let Some(value) = self.local_cache.get(key).await {
            return Some(value);
        }
        
        // Try distributed cache if available
        if let Some(distributed) = &self.distributed_cache {
            if let Ok(Some(value)) = distributed.get(key).await {
                // Cache locally for future use
                let _ = self.local_cache.put(key.clone(), value.clone()).await;
                return Some(value);
            }
        }
        
        None
    }
    
    pub async fn put(&self, key: CacheKey, value: CacheValue) -> VDFSResult<()> {
        // Store in local cache
        self.local_cache.put(key.clone(), value.clone()).await?;
        
        // Store in distributed cache if available
        if let Some(distributed) = &self.distributed_cache {
            distributed.put(key, value).await?;
        }
        
        Ok(())
    }
    
    pub async fn invalidate(&self, key: &CacheKey) -> VDFSResult<()> {
        self.local_cache.invalidate(key).await?;
        
        if let Some(distributed) = &self.distributed_cache {
            distributed.invalidate(key).await?;
        }
        
        Ok(())
    }
    
    pub async fn clear(&self) -> VDFSResult<()> {
        self.local_cache.clear().await?;
        Ok(())
    }
}