//! Data Portal 库
//! 
//! 高性能跨平台传输协议库，支持POSIX共享内存和TCP网络传输

pub use crate::protocol::*;
pub use crate::transport::*;

pub mod protocol {
    use crc32fast::Hasher;
    
    /// UTP协议固定32字节头部
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct PortalHeader {
        pub magic: u32,       // 0x55545000 ("UTP\0")
        pub version: u8,      // Protocol version
        pub msg_type: u8,     // Message type
        pub flags: u16,       // Control flags
        pub payload_len: u32, // Payload length
        pub sequence: u32,    // Sequence number
        pub timestamp: u64,   // Timestamp
        pub checksum: u32,    // CRC32 checksum
        pub reserved: [u8; 4], // Reserved for future use
    }
    
    impl PortalHeader {
        pub const MAGIC: u32 = 0x55545000;
        pub const SIZE: usize = 32;
        
        pub fn new(msg_type: u8, payload_len: u32, sequence: u32) -> Self {
            let mut header = Self {
                magic: Self::MAGIC,
                version: 2,
                msg_type,
                flags: 0,
                payload_len,
                sequence,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
                checksum: 0,
                reserved: [0; 4],
            };
            
            // Calculate CRC32 checksum
            header.checksum = header.calculate_checksum();
            header
        }
        
        pub fn to_bytes(&self) -> [u8; 32] {
            unsafe { std::mem::transmute(*self) }
        }
        
        pub fn from_bytes(bytes: &[u8; 32]) -> Self {
            unsafe { std::mem::transmute(*bytes) }
        }
        
        fn calculate_checksum(&self) -> u32 {
            let mut hasher = Hasher::new();
            hasher.update(&self.magic.to_le_bytes());
            hasher.update(&[self.version]);
            hasher.update(&[self.msg_type]);
            hasher.update(&self.flags.to_le_bytes());
            hasher.update(&self.payload_len.to_le_bytes());
            hasher.update(&self.sequence.to_le_bytes());
            hasher.update(&self.timestamp.to_le_bytes());
            hasher.finalize()
        }
        
        pub fn verify_checksum(&self) -> bool {
            let mut temp_header = *self;
            temp_header.checksum = 0;
            let expected = temp_header.calculate_checksum();
            self.checksum == expected
        }
    }
}

pub mod transport {
    use std::ptr;
    use std::slice;
    use std::ffi::CString;
    use anyhow::{Result, Context};
    
    /// POSIX共享内存传输层
    pub struct SharedMemoryTransport {
        name: String,
        fd: i32,
        ptr: *mut u8,
        size: usize,
    }
    
    impl SharedMemoryTransport {
        pub fn new(name: &str, size: usize) -> Result<Self> {
            let c_name = CString::new(name).context("Invalid shared memory name")?;
            
            // Create shared memory segment
            let fd = unsafe {
                libc::shm_open(
                    c_name.as_ptr(),
                    libc::O_CREAT | libc::O_RDWR,
                    0o666
                )
            };
            
            if fd == -1 {
                return Err(anyhow::anyhow!("Failed to create shared memory segment"));
            }
            
            // Set size
            unsafe {
                if libc::ftruncate(fd, size as libc::off_t) == -1 {
                    libc::close(fd);
                    return Err(anyhow::anyhow!("Failed to set shared memory size"));
                }
            }
            
            // Map memory
            let ptr = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    size,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    fd,
                    0
                )
            };
            
            if ptr == libc::MAP_FAILED {
                unsafe { libc::close(fd); }
                return Err(anyhow::anyhow!("Failed to map shared memory"));
            }
            
            Ok(Self {
                name: name.to_string(),
                fd,
                ptr: ptr as *mut u8,
                size,
            })
        }
        
        /// 零拷贝写入数据
        pub unsafe fn write_zero_copy(&self, data: &[u8], offset: usize) -> Result<()> {
            if offset + data.len() > self.size {
                return Err(anyhow::anyhow!("Write would exceed shared memory bounds"));
            }
            
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.ptr.add(offset),
                data.len()
            );
            
            Ok(())
        }
        
        /// 零拷贝读取数据
        pub unsafe fn read_zero_copy(&self, offset: usize, len: usize) -> Result<&[u8]> {
            if offset + len > self.size {
                return Err(anyhow::anyhow!("Read would exceed shared memory bounds"));
            }
            
            Ok(slice::from_raw_parts(self.ptr.add(offset), len))
        }
        
        /// 获取原始指针（用于直接内存操作）
        pub fn as_ptr(&self) -> *mut u8 {
            self.ptr
        }
        
        pub fn size(&self) -> usize {
            self.size
        }
    }
    
    impl Drop for SharedMemoryTransport {
        fn drop(&mut self) {
            unsafe {
                libc::munmap(self.ptr as *mut libc::c_void, self.size);
                libc::close(self.fd);
                
                let c_name = CString::new(self.name.clone()).unwrap();
                libc::shm_unlink(c_name.as_ptr());
            }
        }
    }
    
    unsafe impl Send for SharedMemoryTransport {}
    unsafe impl Sync for SharedMemoryTransport {}
}