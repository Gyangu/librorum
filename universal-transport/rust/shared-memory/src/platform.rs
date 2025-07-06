//! Platform-specific implementations for shared memory

use crate::{SharedMemoryError, Result};

/// Platform capabilities
pub struct PlatformCapabilities {
    /// Maximum shared memory region size
    pub max_region_size: usize,
    /// Supports named regions
    pub supports_named_regions: bool,
    /// Supports anonymous regions
    pub supports_anonymous_regions: bool,
    /// Default page size
    pub page_size: usize,
}

impl PlatformCapabilities {
    /// Get platform capabilities
    pub fn get() -> Self {
        #[cfg(unix)]
        {
            Self::unix_capabilities()
        }
        #[cfg(windows)]
        {
            Self::windows_capabilities()
        }
        #[cfg(not(any(unix, windows)))]
        {
            Self::default_capabilities()
        }
    }
    
    #[cfg(unix)]
    fn unix_capabilities() -> Self {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
        
        Self {
            max_region_size: 1024 * 1024 * 1024, // 1GB default
            supports_named_regions: true,
            supports_anonymous_regions: true,
            page_size,
        }
    }
    
    #[cfg(windows)]
    fn windows_capabilities() -> Self {
        use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};
        
        let mut sys_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
        unsafe { GetSystemInfo(&mut sys_info) };
        
        Self {
            max_region_size: 2 * 1024 * 1024 * 1024, // 2GB on Windows
            supports_named_regions: true,
            supports_anonymous_regions: true,
            page_size: sys_info.dwPageSize as usize,
        }
    }
    
    #[cfg(not(any(unix, windows)))]
    fn default_capabilities() -> Self {
        Self {
            max_region_size: 64 * 1024 * 1024, // 64MB conservative default
            supports_named_regions: false,
            supports_anonymous_regions: false,
            page_size: 4096,
        }
    }
}

/// Platform-specific utilities
pub struct PlatformUtils;

impl PlatformUtils {
    /// Get current process ID
    pub fn get_process_id() -> u32 {
        #[cfg(unix)]
        {
            unsafe { libc::getpid() as u32 }
        }
        #[cfg(windows)]
        {
            unsafe { winapi::um::processthreadsapi::GetCurrentProcessId() }
        }
        #[cfg(not(any(unix, windows)))]
        {
            std::process::id()
        }
    }
    
    /// Get number of CPU cores
    pub fn get_cpu_count() -> usize {
        num_cpus::get()
    }
    
    /// Check if running with elevated privileges
    pub fn is_elevated() -> bool {
        #[cfg(unix)]
        {
            unsafe { libc::geteuid() == 0 }
        }
        #[cfg(windows)]
        {
            // Simplified check - in practice, you'd want to check the token
            false
        }
        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    }
    
    /// Generate a unique region name
    pub fn generate_region_name(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        
        let process_id = Self::get_process_id();
        
        format!("{}_{}_{}_{}", prefix, process_id, timestamp, uuid::Uuid::new_v4().simple())
    }
    
    /// Align size to page boundary
    pub fn align_to_page_size(size: usize) -> usize {
        let capabilities = PlatformCapabilities::get();
        let page_size = capabilities.page_size;
        
        // Round up to nearest page size
        (size + page_size - 1) & !(page_size - 1)
    }
    
    /// Check if shared memory is supported
    pub fn is_shared_memory_supported() -> bool {
        let capabilities = PlatformCapabilities::get();
        capabilities.supports_named_regions || capabilities.supports_anonymous_regions
    }
    
    /// Get optimal buffer size for the platform
    pub fn get_optimal_buffer_size() -> usize {
        let capabilities = PlatformCapabilities::get();
        
        // Use a multiple of page size
        let base_size = 64 * 1024; // 64KB base
        let aligned_size = Self::align_to_page_size(base_size);
        
        std::cmp::min(aligned_size, capabilities.max_region_size / 4)
    }
}

/// Platform-specific optimizations
pub struct PlatformOptimizations;

impl PlatformOptimizations {
    /// Set memory advice for better performance
    pub fn optimize_memory_access(ptr: *mut u8, size: usize) -> Result<()> {
        #[cfg(unix)]
        {
            unsafe {
                nix::sys::mman::madvise(
                    ptr as *mut std::ffi::c_void,
                    size,
                    nix::sys::mman::MmapAdvise::MADV_WILLNEED
                ).map_err(|e| SharedMemoryError::Platform(format!("madvise failed: {}", e)))?;
            }
        }
        
        #[cfg(windows)]
        {
            // Windows doesn't have direct equivalent, but we can prefault pages
            let _ = (ptr, size); // Suppress unused variable warning
        }
        
        Ok(())
    }
    
    /// Hint that memory will be accessed sequentially
    pub fn hint_sequential_access(ptr: *mut u8, size: usize) -> Result<()> {
        #[cfg(unix)]
        {
            unsafe {
                nix::sys::mman::madvise(
                    ptr as *mut std::ffi::c_void,
                    size,
                    nix::sys::mman::MmapAdvise::MADV_SEQUENTIAL
                ).map_err(|e| SharedMemoryError::Platform(format!("madvise failed: {}", e)))?;
            }
        }
        
        #[cfg(windows)]
        {
            let _ = (ptr, size);
        }
        
        Ok(())
    }
    
    /// Hint that memory will be accessed randomly
    pub fn hint_random_access(ptr: *mut u8, size: usize) -> Result<()> {
        #[cfg(unix)]
        {
            unsafe {
                nix::sys::mman::madvise(
                    ptr as *mut std::ffi::c_void,
                    size,
                    nix::sys::mman::MmapAdvise::MADV_RANDOM
                ).map_err(|e| SharedMemoryError::Platform(format!("madvise failed: {}", e)))?;
            }
        }
        
        #[cfg(windows)]
        {
            let _ = (ptr, size);
        }
        
        Ok(())
    }
    
    /// Lock pages in memory to prevent swapping
    pub fn lock_memory(ptr: *mut u8, size: usize) -> Result<()> {
        #[cfg(unix)]
        {
            unsafe {
                nix::sys::mman::mlock(ptr as *mut std::ffi::c_void, size)
                    .map_err(|e| SharedMemoryError::Platform(format!("mlock failed: {}", e)))?;
            }
        }
        
        #[cfg(windows)]
        {
            unsafe {
                let result = winapi::um::memoryapi::VirtualLock(ptr as *mut std::ffi::c_void, size);
                if result == 0 {
                    return Err(SharedMemoryError::Platform("VirtualLock failed".to_string()));
                }
            }
        }
        
        Ok(())
    }
    
    /// Unlock previously locked memory
    pub fn unlock_memory(ptr: *mut u8, size: usize) -> Result<()> {
        #[cfg(unix)]
        {
            unsafe {
                nix::sys::mman::munlock(ptr as *mut std::ffi::c_void, size)
                    .map_err(|e| SharedMemoryError::Platform(format!("munlock failed: {}", e)))?;
            }
        }
        
        #[cfg(windows)]
        {
            unsafe {
                let result = winapi::um::memoryapi::VirtualUnlock(ptr as *mut std::ffi::c_void, size);
                if result == 0 {
                    return Err(SharedMemoryError::Platform("VirtualUnlock failed".to_string()));
                }
            }
        }
        
        Ok(())
    }
}

/// Memory barrier utilities for cross-platform synchronization
pub struct MemoryBarriers;

impl MemoryBarriers {
    /// Full memory barrier
    pub fn full_barrier() {
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    }
    
    /// Acquire barrier
    pub fn acquire_barrier() {
        std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
    }
    
    /// Release barrier
    pub fn release_barrier() {
        std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
    }
    
    /// Compiler barrier (prevents reordering)
    pub fn compiler_barrier() {
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_capabilities() {
        let capabilities = PlatformCapabilities::get();
        assert!(capabilities.page_size > 0);
        assert!(capabilities.max_region_size > 0);
    }

    #[test]
    fn test_platform_utils() {
        assert!(PlatformUtils::get_process_id() > 0);
        assert!(PlatformUtils::get_cpu_count() > 0);
        
        let region_name = PlatformUtils::generate_region_name("test");
        assert!(region_name.starts_with("test_"));
        
        let aligned = PlatformUtils::align_to_page_size(1000);
        let capabilities = PlatformCapabilities::get();
        assert_eq!(aligned % capabilities.page_size, 0);
        assert!(aligned >= 1000);
    }

    #[test]
    fn test_shared_memory_support() {
        // Should be true on most platforms
        let supported = PlatformUtils::is_shared_memory_supported();
        
        #[cfg(any(unix, windows))]
        assert!(supported);
        
        #[cfg(not(any(unix, windows)))]
        let _ = supported; // May or may not be supported
    }

    #[test]
    fn test_optimal_buffer_size() {
        let size = PlatformUtils::get_optimal_buffer_size();
        assert!(size > 0);
        
        let capabilities = PlatformCapabilities::get();
        assert_eq!(size % capabilities.page_size, 0);
    }
}