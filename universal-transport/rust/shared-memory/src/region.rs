//! Shared memory region management

use crate::{SharedMemoryError, Result, RingBuffer};
use std::ptr::NonNull;
use std::sync::Arc;

/// Shared memory region handle
pub struct SharedMemoryRegion {
    /// Region name/identifier
    pub name: String,
    /// Region size in bytes
    pub size: usize,
    /// Memory pointer
    ptr: NonNull<u8>,
    /// Platform-specific handle
    platform_handle: PlatformHandle,
    /// Whether this process created the region
    is_creator: bool,
}

/// Platform-specific handle types
#[derive(Debug)]
pub enum PlatformHandle {
    #[cfg(unix)]
    Unix { fd: i32 },
    #[cfg(windows)]
    Windows { handle: *mut std::ffi::c_void },
}

impl SharedMemoryRegion {
    /// Create a new shared memory region
    pub fn create(name: impl Into<String>, size: usize) -> Result<Self> {
        let name = name.into();
        validate_region_name(&name)?;
        validate_region_size(size)?;
        
        let (ptr, platform_handle) = create_platform_region(&name, size)?;
        
        Ok(Self {
            name,
            size,
            ptr,
            platform_handle,
            is_creator: true,
        })
    }
    
    /// Open an existing shared memory region
    pub fn open(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        validate_region_name(&name)?;
        
        let (ptr, size, platform_handle) = open_platform_region(&name)?;
        
        Ok(Self {
            name,
            size,
            ptr,
            platform_handle,
            is_creator: false,
        })
    }
    
    /// Get a slice view of the memory
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }
    
    /// Get a mutable slice view of the memory
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }
    
    /// Get raw pointer to the memory
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }
    
    /// Get raw mutable pointer to the memory
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }
    
    /// Initialize the region with a ring buffer
    pub fn initialize_ring_buffer(&mut self, buffer_size: usize) -> Result<&mut RingBuffer> {
        if buffer_size + std::mem::size_of::<RingBuffer>() > self.size {
            return Err(SharedMemoryError::InvalidSize {
                size: buffer_size,
                min: 0,
                max: self.size - std::mem::size_of::<RingBuffer>(),
            });
        }
        
        let ring_buffer_ptr = self.as_mut_ptr() as *mut RingBuffer;
        
        unsafe {
            // Initialize ring buffer header
            std::ptr::write(ring_buffer_ptr, RingBuffer::new(buffer_size as u64));
            Ok(&mut *ring_buffer_ptr)
        }
    }
    
    /// Get ring buffer from initialized region
    pub fn get_ring_buffer(&self) -> Result<&RingBuffer> {
        if self.size < std::mem::size_of::<RingBuffer>() {
            return Err(SharedMemoryError::Protocol(
                "Region too small for ring buffer".to_string()
            ));
        }
        
        let ring_buffer_ptr = self.as_ptr() as *const RingBuffer;
        
        unsafe {
            Ok(&*ring_buffer_ptr)
        }
    }
    
    /// Get mutable ring buffer from initialized region
    pub fn get_ring_buffer_mut(&mut self) -> Result<&mut RingBuffer> {
        if self.size < std::mem::size_of::<RingBuffer>() {
            return Err(SharedMemoryError::Protocol(
                "Region too small for ring buffer".to_string()
            ));
        }
        
        let ring_buffer_ptr = self.as_mut_ptr() as *mut RingBuffer;
        
        unsafe {
            Ok(&mut *ring_buffer_ptr)
        }
    }
    
    /// Get the data buffer within the ring buffer
    pub fn get_data_buffer(&self) -> Result<&[u8]> {
        let ring_buffer = self.get_ring_buffer()?;
        let capacity = ring_buffer.capacity.load(std::sync::atomic::Ordering::Acquire) as usize;
        
        let data_ptr = unsafe {
            self.as_ptr().add(std::mem::size_of::<RingBuffer>())
        };
        
        Ok(unsafe { std::slice::from_raw_parts(data_ptr, capacity) })
    }
    
    /// Get mutable data buffer within the ring buffer
    pub fn get_data_buffer_mut(&mut self) -> Result<&mut [u8]> {
        let ring_buffer = self.get_ring_buffer()?;
        let capacity = ring_buffer.capacity.load(std::sync::atomic::Ordering::Acquire) as usize;
        
        let data_ptr = unsafe {
            self.as_mut_ptr().add(std::mem::size_of::<RingBuffer>())
        };
        
        Ok(unsafe { std::slice::from_raw_parts_mut(data_ptr, capacity) })
    }
}

impl Drop for SharedMemoryRegion {
    fn drop(&mut self) {
        // Platform-specific cleanup
        let _ = cleanup_platform_region(&self.platform_handle, &self.name, self.is_creator);
    }
}

// Safety: SharedMemoryRegion can be sent between threads
unsafe impl Send for SharedMemoryRegion {}
// Safety: SharedMemoryRegion can be shared between threads with proper synchronization
unsafe impl Sync for SharedMemoryRegion {}

/// Shared memory region manager for handling multiple regions
pub struct SharedMemoryManager {
    regions: std::collections::HashMap<String, Arc<SharedMemoryRegion>>,
}

impl SharedMemoryManager {
    /// Create a new shared memory manager
    pub fn new() -> Self {
        Self {
            regions: std::collections::HashMap::new(),
        }
    }
    
    /// Create or get a shared memory region
    pub fn get_or_create_region(&mut self, name: impl Into<String>, size: usize) -> Result<Arc<SharedMemoryRegion>> {
        let name = name.into();
        
        if let Some(region) = self.regions.get(&name) {
            return Ok(Arc::clone(region));
        }
        
        // Try to open existing region first
        let region = match SharedMemoryRegion::open(&name) {
            Ok(region) => region,
            Err(_) => SharedMemoryRegion::create(&name, size)?,
        };
        
        let region_arc = Arc::new(region);
        self.regions.insert(name.clone(), Arc::clone(&region_arc));
        
        Ok(region_arc)
    }
    
    /// Remove a region from management
    pub fn remove_region(&mut self, name: &str) -> Option<Arc<SharedMemoryRegion>> {
        self.regions.remove(name)
    }
    
    /// List all managed regions
    pub fn list_regions(&self) -> Vec<String> {
        self.regions.keys().cloned().collect()
    }
    
    /// Get a region by name
    pub fn get_region(&self, name: &str) -> Option<Arc<SharedMemoryRegion>> {
        self.regions.get(name).cloned()
    }
}

impl Default for SharedMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

// Platform-specific implementations

#[cfg(unix)]
mod unix_impl {
    use super::*;
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::AsRawFd;
    
    pub fn create_platform_region(name: &str, size: usize) -> Result<(NonNull<u8>, PlatformHandle)> {
        let c_name = CString::new(name).map_err(|_| {
            SharedMemoryError::Platform("Invalid region name".to_string())
        })?;
        
        // Create shared memory object
        let fd = unsafe {
            nix::sys::mman::shm_open(
                c_name.as_c_str(),
                nix::fcntl::OFlag::O_CREAT | nix::fcntl::OFlag::O_RDWR,
                nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR
            )
        }.map_err(|e| SharedMemoryError::from_platform_error(e as i32, "shm_open failed"))?;
        
        let raw_fd = fd.as_raw_fd();
        
        // Set size
        unsafe {
            nix::unistd::ftruncate(&fd, size as i64)
        }.map_err(|e| SharedMemoryError::from_platform_error(e as i32, "ftruncate failed"))?;
        
        // Map memory
        let ptr = unsafe {
            nix::sys::mman::mmap(
                None,
                std::num::NonZeroUsize::new(size).unwrap(),
                nix::sys::mman::ProtFlags::PROT_READ | nix::sys::mman::ProtFlags::PROT_WRITE,
                nix::sys::mman::MapFlags::MAP_SHARED,
                Some(&fd),
                0
            )
        }.map_err(|e| SharedMemoryError::from_platform_error(e as i32, "mmap failed"))?;
        
        let non_null_ptr = NonNull::new(ptr as *mut u8)
            .ok_or_else(|| SharedMemoryError::MappingFailed("mmap returned null".to_string()))?;
        
        Ok((non_null_ptr, PlatformHandle::Unix { fd: raw_fd }))
    }
    
    pub fn open_platform_region(name: &str) -> Result<(NonNull<u8>, usize, PlatformHandle)> {
        let c_name = CString::new(name).map_err(|_| {
            SharedMemoryError::Platform("Invalid region name".to_string())
        })?;
        
        // Open existing shared memory object
        let fd = unsafe {
            nix::sys::mman::shm_open(
                c_name.as_c_str(),
                nix::fcntl::OFlag::O_RDWR,
                nix::sys::stat::Mode::empty()
            )
        }.map_err(|e| SharedMemoryError::from_platform_error(e as i32, "shm_open failed"))?;
        
        let raw_fd = fd.as_raw_fd();
        
        // Get size
        let stat = unsafe {
            nix::sys::stat::fstat(raw_fd)
        }.map_err(|e| SharedMemoryError::from_platform_error(e as i32, "fstat failed"))?;
        
        let size = stat.st_size as usize;
        
        // Map memory
        let ptr = unsafe {
            nix::sys::mman::mmap(
                None,
                std::num::NonZeroUsize::new(size).unwrap(),
                nix::sys::mman::ProtFlags::PROT_READ | nix::sys::mman::ProtFlags::PROT_WRITE,
                nix::sys::mman::MapFlags::MAP_SHARED,
                Some(&fd),
                0
            )
        }.map_err(|e| SharedMemoryError::from_platform_error(e as i32, "mmap failed"))?;
        
        let non_null_ptr = NonNull::new(ptr as *mut u8)
            .ok_or_else(|| SharedMemoryError::MappingFailed("mmap returned null".to_string()))?;
        
        Ok((non_null_ptr, size, PlatformHandle::Unix { fd: raw_fd }))
    }
    
    pub fn cleanup_platform_region(handle: &PlatformHandle, name: &str, is_creator: bool) -> Result<()> {
        if let PlatformHandle::Unix { fd } = handle {
            // Unmap memory and close file descriptor
            let _ = unsafe { nix::unistd::close(*fd) };
            
            // If we created the region, unlink it
            if is_creator {
                let c_name = CString::new(name).map_err(|_| {
                    SharedMemoryError::Platform("Invalid region name".to_string())
                })?;
                let _ = unsafe { nix::sys::mman::shm_unlink(c_name.as_c_str()) };
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::ffi::CString;
    use winapi::um::winbase::{CreateFileMappingA, OpenFileMappingA};
    use winapi::um::memoryapi::{MapViewOfFile, UnmapViewOfFile};
    use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
    use winapi::um::winnt::{PAGE_READWRITE, FILE_MAP_ALL_ACCESS};
    
    pub fn create_platform_region(name: &str, size: usize) -> Result<(NonNull<u8>, PlatformHandle)> {
        let c_name = CString::new(name).map_err(|_| {
            SharedMemoryError::Platform("Invalid region name".to_string())
        })?;
        
        let handle = unsafe {
            CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                std::ptr::null_mut(),
                PAGE_READWRITE,
                (size >> 32) as u32,
                (size & 0xFFFFFFFF) as u32,
                c_name.as_ptr()
            )
        };
        
        if handle.is_null() {
            return Err(SharedMemoryError::Platform("CreateFileMapping failed".to_string()));
        }
        
        let ptr = unsafe {
            MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size)
        };
        
        if ptr.is_null() {
            unsafe { CloseHandle(handle) };
            return Err(SharedMemoryError::MappingFailed("MapViewOfFile failed".to_string()));
        }
        
        let non_null_ptr = NonNull::new(ptr as *mut u8)
            .ok_or_else(|| SharedMemoryError::MappingFailed("MapViewOfFile returned null".to_string()))?;
        
        Ok((non_null_ptr, PlatformHandle::Windows { handle }))
    }
    
    pub fn open_platform_region(name: &str) -> Result<(NonNull<u8>, usize, PlatformHandle)> {
        let c_name = CString::new(name).map_err(|_| {
            SharedMemoryError::Platform("Invalid region name".to_string())
        })?;
        
        let handle = unsafe {
            OpenFileMappingA(FILE_MAP_ALL_ACCESS, 0, c_name.as_ptr())
        };
        
        if handle.is_null() {
            return Err(SharedMemoryError::RegionNotFound(name.to_string()));
        }
        
        // For Windows, we need to map the entire object to get its size
        let ptr = unsafe {
            MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, 0)
        };
        
        if ptr.is_null() {
            unsafe { CloseHandle(handle) };
            return Err(SharedMemoryError::MappingFailed("MapViewOfFile failed".to_string()));
        }
        
        // Query the size using VirtualQuery
        let mut mbi = unsafe { std::mem::zeroed() };
        let size = unsafe {
            winapi::um::memoryapi::VirtualQuery(ptr, &mut mbi, std::mem::size_of_val(&mbi))
        };
        
        if size == 0 {
            unsafe { 
                UnmapViewOfFile(ptr);
                CloseHandle(handle);
            };
            return Err(SharedMemoryError::Platform("VirtualQuery failed".to_string()));
        }
        
        let non_null_ptr = NonNull::new(ptr as *mut u8)
            .ok_or_else(|| SharedMemoryError::MappingFailed("MapViewOfFile returned null".to_string()))?;
        
        Ok((non_null_ptr, mbi.RegionSize, PlatformHandle::Windows { handle }))
    }
    
    pub fn cleanup_platform_region(handle: &PlatformHandle, _name: &str, _is_creator: bool) -> Result<()> {
        if let PlatformHandle::Windows { handle } = handle {
            unsafe {
                CloseHandle(*handle);
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
use unix_impl::*;
#[cfg(windows)]
use windows_impl::*;

/// Validate region name
fn validate_region_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 255 {
        return Err(SharedMemoryError::Platform("Invalid region name length".to_string()));
    }
    
    // Check for invalid characters
    if name.contains('\0') {
        return Err(SharedMemoryError::Platform("Region name contains null byte".to_string()));
    }
    
    Ok(())
}

/// Validate region size
fn validate_region_size(size: usize) -> Result<()> {
    const MIN_SIZE: usize = 4096; // 4KB
    const MAX_SIZE: usize = 1024 * 1024 * 1024; // 1GB
    
    if size < MIN_SIZE || size > MAX_SIZE {
        return Err(SharedMemoryError::InvalidSize {
            size,
            min: MIN_SIZE,
            max: MAX_SIZE,
        });
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_creation() {
        let region = SharedMemoryRegion::create("test_region", 4096);
        assert!(region.is_ok());
        
        let region = region.unwrap();
        assert_eq!(region.name, "test_region");
        assert_eq!(region.size, 4096);
    }

    #[test]
    fn test_region_validation() {
        // Test invalid name
        assert!(SharedMemoryRegion::create("", 4096).is_err());
        assert!(SharedMemoryRegion::create("test\0name", 4096).is_err());
        
        // Test invalid size
        assert!(SharedMemoryRegion::create("test", 100).is_err());
        assert!(SharedMemoryRegion::create("test", usize::MAX).is_err());
    }

    #[test]
    fn test_ring_buffer_initialization() {
        let mut region = SharedMemoryRegion::create("test_ring", 8192).unwrap();
        let buffer_size = 4096;
        
        let ring_buffer = region.initialize_ring_buffer(buffer_size).unwrap();
        assert_eq!(ring_buffer.capacity.load(std::sync::atomic::Ordering::Acquire), buffer_size as u64);
        assert!(ring_buffer.is_empty());
        assert!(!ring_buffer.is_full());
    }
}