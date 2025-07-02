//! Consistency Management

use crate::vdfs::{VDFSResult, VDFSError, VirtualPath, FileId, ChunkId};
use crate::vdfs::metadata::{MetadataManager, FileInfo, ChunkMetadata};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;

/// Consistency checker for metadata validation
#[async_trait]
pub trait ConsistencyChecker: Send + Sync {
    async fn check_file_consistency(&self, file_info: &FileInfo) -> VDFSResult<Vec<ConsistencyIssue>>;
    async fn check_chunk_consistency(&self, chunk_metadata: &ChunkMetadata) -> VDFSResult<Vec<ConsistencyIssue>>;
    async fn check_cross_references(&self, files: &HashMap<VirtualPath, FileInfo>) -> VDFSResult<Vec<ConsistencyIssue>>;
}

/// Types of consistency issues
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsistencyIssue {
    /// Missing chunk metadata
    MissingChunkMetadata { file_path: VirtualPath, chunk_id: ChunkId },
    /// Orphaned chunk metadata
    OrphanedChunkMetadata { chunk_id: ChunkId },
    /// File size mismatch
    FileSizeMismatch { file_path: VirtualPath, expected_size: u64, actual_size: u64 },
    /// Broken chunk chain
    BrokenChunkChain { file_path: VirtualPath, missing_chunk_index: usize },
    /// Invalid file ID mapping
    InvalidFileIdMapping { file_id: FileId, expected_path: VirtualPath, actual_path: Option<VirtualPath> },
    /// Checksum mismatch
    ChecksumMismatch { chunk_id: ChunkId, expected: String, actual: String },
    /// Duplicate file IDs
    DuplicateFileId { file_id: FileId, paths: Vec<VirtualPath> },
    /// Invalid replica information
    InvalidReplicaInfo { chunk_id: ChunkId, invalid_nodes: Vec<String> },
}

/// Default consistency checker implementation
pub struct DefaultConsistencyChecker;

#[async_trait]
impl ConsistencyChecker for DefaultConsistencyChecker {
    async fn check_file_consistency(&self, file_info: &FileInfo) -> VDFSResult<Vec<ConsistencyIssue>> {
        let mut issues = Vec::new();
        
        // Check if total chunk size matches file size
        let total_chunk_size: u64 = file_info.chunks.iter().map(|c| c.size as u64).sum();
        if total_chunk_size != file_info.metadata.size {
            issues.push(ConsistencyIssue::FileSizeMismatch {
                file_path: VirtualPath::new("unknown"), // Will be filled by caller
                expected_size: file_info.metadata.size,
                actual_size: total_chunk_size,
            });
        }
        
        // Check for chunk chain integrity
        for (index, chunk) in file_info.chunks.iter().enumerate() {
            if chunk.size == 0 && index < file_info.chunks.len() - 1 {
                issues.push(ConsistencyIssue::BrokenChunkChain {
                    file_path: VirtualPath::new("unknown"), // Will be filled by caller
                    missing_chunk_index: index,
                });
            }
        }
        
        Ok(issues)
    }
    
    async fn check_chunk_consistency(&self, chunk_metadata: &ChunkMetadata) -> VDFSResult<Vec<ConsistencyIssue>> {
        let mut issues = Vec::new();
        
        // Check if checksum is valid (non-empty and proper format)
        if chunk_metadata.checksum.is_empty() || chunk_metadata.checksum.len() != 64 {
            issues.push(ConsistencyIssue::ChecksumMismatch {
                chunk_id: chunk_metadata.id,
                expected: "64-character hex string".to_string(),
                actual: chunk_metadata.checksum.clone(),
            });
        }
        
        // Check replica information validity
        let invalid_nodes: Vec<String> = chunk_metadata.replicas.iter()
            .filter(|node| node.is_empty() || !node.starts_with("node-"))
            .cloned()
            .collect();
        
        if !invalid_nodes.is_empty() {
            issues.push(ConsistencyIssue::InvalidReplicaInfo {
                chunk_id: chunk_metadata.id,
                invalid_nodes,
            });
        }
        
        Ok(issues)
    }
    
    async fn check_cross_references(&self, files: &HashMap<VirtualPath, FileInfo>) -> VDFSResult<Vec<ConsistencyIssue>> {
        let mut issues = Vec::new();
        let mut file_id_map: HashMap<FileId, Vec<VirtualPath>> = HashMap::new();
        let mut all_chunk_ids: HashSet<ChunkId> = HashSet::new();
        let mut referenced_chunks: HashSet<ChunkId> = HashSet::new();
        
        // Collect all file IDs and chunk references
        for (path, info) in files {
            file_id_map.entry(info.metadata.id)
                .or_insert_with(Vec::new)
                .push(path.clone());
            
            for chunk in &info.chunks {
                all_chunk_ids.insert(chunk.id);
                referenced_chunks.insert(chunk.id);
            }
        }
        
        // Check for duplicate file IDs
        for (file_id, paths) in file_id_map {
            if paths.len() > 1 {
                issues.push(ConsistencyIssue::DuplicateFileId { file_id, paths });
            }
        }
        
        // Find orphaned chunks (chunks that exist but aren't referenced by any file)
        for chunk_id in &all_chunk_ids {
            if !referenced_chunks.contains(chunk_id) {
                issues.push(ConsistencyIssue::OrphanedChunkMetadata { chunk_id: *chunk_id });
            }
        }
        
        Ok(issues)
    }
}

/// Consistency manager for metadata
pub struct ConsistencyManager {
    metadata_manager: Arc<dyn MetadataManager>,
    checker: Box<dyn ConsistencyChecker>,
}

impl ConsistencyManager {
    pub fn new(metadata_manager: Arc<dyn MetadataManager>) -> Self {
        Self {
            metadata_manager,
            checker: Box::new(DefaultConsistencyChecker),
        }
    }
    
    pub fn with_checker(
        metadata_manager: Arc<dyn MetadataManager>,
        checker: Box<dyn ConsistencyChecker>,
    ) -> Self {
        Self {
            metadata_manager,
            checker,
        }
    }
    
    /// Check consistency of entire metadata store
    pub async fn check_consistency(&self) -> VDFSResult<Vec<VirtualPath>> {
        let mut inconsistent_paths = Vec::new();
        let issues = self.check_all_issues().await?;
        
        for issue in issues {
            match issue {
                ConsistencyIssue::MissingChunkMetadata { file_path, .. } |
                ConsistencyIssue::FileSizeMismatch { file_path, .. } |
                ConsistencyIssue::BrokenChunkChain { file_path, .. } => {
                    if !inconsistent_paths.contains(&file_path) {
                        inconsistent_paths.push(file_path);
                    }
                },
                ConsistencyIssue::DuplicateFileId { paths, .. } => {
                    for path in paths {
                        if !inconsistent_paths.contains(&path) {
                            inconsistent_paths.push(path);
                        }
                    }
                },
                ConsistencyIssue::InvalidFileIdMapping { expected_path, .. } => {
                    if !inconsistent_paths.contains(&expected_path) {
                        inconsistent_paths.push(expected_path);
                    }
                },
                _ => {
                    // For system-wide issues like orphaned chunks, create a special marker
                    let marker_path = VirtualPath::new(format!("system_consistency_issue_{:?}", issue));
                    if !inconsistent_paths.contains(&marker_path) {
                        inconsistent_paths.push(marker_path);
                    }
                }
            }
        }
        
        Ok(inconsistent_paths)
    }
    
    /// Get detailed consistency issues
    pub async fn check_all_issues(&self) -> VDFSResult<Vec<ConsistencyIssue>> {
        let mut all_issues = Vec::new();
        
        // Use the metadata manager's verify_consistency method
        let inconsistent_files = self.metadata_manager.verify_consistency().await?;
        
        // For each inconsistent file, get detailed issues
        for path in inconsistent_files {
            if path.as_str().starts_with("orphaned_chunk:") {
                // Handle orphaned chunk
                if let Some(chunk_id_str) = path.as_str().strip_prefix("orphaned_chunk:") {
                    // Parse chunk ID - simplified approach
                    let mut chunk_id = [0u8; 32];
                    if chunk_id_str.len() >= 32 {
                        // This is a simplified parsing - in real implementation would need proper parsing
                        all_issues.push(ConsistencyIssue::OrphanedChunkMetadata { 
                            chunk_id 
                        });
                    }
                }
            } else {
                // Check file-specific issues
                if let Ok(file_info) = self.metadata_manager.get_file_info(&path).await {
                    let mut file_issues = self.checker.check_file_consistency(&file_info).await?;
                    
                    // Update file path in issues
                    for issue in &mut file_issues {
                        match issue {
                            ConsistencyIssue::FileSizeMismatch { file_path, .. } |
                            ConsistencyIssue::BrokenChunkChain { file_path, .. } => {
                                *file_path = path.clone();
                            },
                            _ => {}
                        }
                    }
                    
                    all_issues.extend(file_issues);
                    
                    // Check chunk consistency
                    for chunk in &file_info.chunks {
                        let chunk_issues = self.checker.check_chunk_consistency(chunk).await?;
                        all_issues.extend(chunk_issues);
                    }
                }
            }
        }
        
        Ok(all_issues)
    }
    
    /// Repair inconsistent metadata for a specific path
    pub async fn repair(&self, path: &VirtualPath) -> VDFSResult<()> {
        // Use the metadata manager's repair functionality
        self.metadata_manager.repair_metadata(path).await?;
        
        // Perform additional repair operations if needed
        if let Ok(file_info) = self.metadata_manager.get_file_info(path).await {
            // Verify file size consistency after repair
            let total_chunk_size: u64 = file_info.chunks.iter().map(|c| c.size as u64).sum();
            
            if total_chunk_size != file_info.metadata.size {
                // Update file metadata with correct size
                let mut corrected_info = file_info.clone();
                corrected_info.metadata.size = total_chunk_size;
                corrected_info.metadata.modified = SystemTime::now();
                
                self.metadata_manager.set_file_info(path, corrected_info).await?;
            }
        }
        
        Ok(())
    }
    
    /// Repair all detected consistency issues
    pub async fn repair_all(&self) -> VDFSResult<usize> {
        let issues = self.check_all_issues().await?;
        let mut repaired_count = 0;
        
        // Group issues by path for efficient repair
        let mut path_issues: HashMap<VirtualPath, Vec<ConsistencyIssue>> = HashMap::new();
        
        for issue in issues {
            match &issue {
                ConsistencyIssue::MissingChunkMetadata { file_path, .. } |
                ConsistencyIssue::FileSizeMismatch { file_path, .. } |
                ConsistencyIssue::BrokenChunkChain { file_path, .. } => {
                    path_issues.entry(file_path.clone())
                        .or_insert_with(Vec::new)
                        .push(issue);
                },
                ConsistencyIssue::DuplicateFileId { paths, .. } => {
                    for path in paths {
                        path_issues.entry(path.clone())
                            .or_insert_with(Vec::new)
                            .push(issue.clone());
                    }
                },
                _ => {
                    // Handle system-wide issues
                    self.repair_system_issue(&issue).await?;
                    repaired_count += 1;
                }
            }
        }
        
        // Repair path-specific issues
        for (path, _issues) in path_issues {
            self.repair(&path).await?;
            repaired_count += 1;
        }
        
        Ok(repaired_count)
    }
    
    /// Repair system-wide consistency issues
    async fn repair_system_issue(&self, issue: &ConsistencyIssue) -> VDFSResult<()> {
        match issue {
            ConsistencyIssue::OrphanedChunkMetadata { chunk_id } => {
                // Remove orphaned chunk metadata
                // Note: This assumes the metadata manager has a method to remove chunk metadata
                // In practice, you might need to extend the MetadataManager trait
                
                // For now, we'll simulate removal by creating a marker path
                let marker_path = VirtualPath::new(format!("orphaned_chunk:{:?}", chunk_id));
                self.metadata_manager.repair_metadata(&marker_path).await?;
            },
            _ => {
                // Other system issues can be handled here
            }
        }
        
        Ok(())
    }
    
    /// Force rebuild of all metadata indexes
    pub async fn rebuild_indexes(&self) -> VDFSResult<()> {
        self.metadata_manager.rebuild_index().await
    }
    
    /// Check consistency of a specific file
    pub async fn check_file(&self, path: &VirtualPath) -> VDFSResult<Vec<ConsistencyIssue>> {
        let file_info = self.metadata_manager.get_file_info(path).await?;
        let mut issues = self.checker.check_file_consistency(&file_info).await?;
        
        // Update file path in issues
        for issue in &mut issues {
            match issue {
                ConsistencyIssue::FileSizeMismatch { file_path, .. } |
                ConsistencyIssue::BrokenChunkChain { file_path, .. } => {
                    *file_path = path.clone();
                },
                _ => {}
            }
        }
        
        // Check individual chunks
        for chunk in &file_info.chunks {
            let chunk_issues = self.checker.check_chunk_consistency(chunk).await?;
            issues.extend(chunk_issues);
        }
        
        Ok(issues)
    }
}