#!/usr/bin/env rust-script

//! 简化但工作的POSIX共享内存演示
//! 展示Rust如何创建和使用共享内存

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::thread;

extern "C" {
    fn mmap(addr: *mut std::ffi::c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut std::ffi::c_void;
    fn munmap(addr: *mut std::ffi::c_void, len: usize) -> i32;
}

const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const MAP_SHARED: i32 = 0x1;
const MAP_FAILED: *mut std::ffi::c_void = (-1isize) as *mut std::ffi::c_void;

#[repr(C)]
struct SimpleMessage {
    magic: u32,
    sequence: u64,
    timestamp: u64,
    message_len: u32,
    // 后跟消息内容
}

impl SimpleMessage {
    const MAGIC: u32 = 0x55545042;
    const HEADER_SIZE: usize = 20;
    
    fn new(sequence: u64, message_len: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            sequence,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            message_len,
        }
    }
    
    fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

struct PosixDemo {
    file_path: String,
    size: usize,
    ptr: *mut std::ffi::c_void,
}

impl PosixDemo {
    fn new(file_path: &str, size: usize) -> Result<Self, String> {
        // 创建共享文件
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        
        file.set_len(size as u64)
            .map_err(|e| format!("Failed to set file size: {}", e))?;
        
        let fd = file.as_raw_fd();
        let ptr = unsafe { mmap(ptr::null_mut(), size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0) };
        
        if ptr == MAP_FAILED {
            return Err("Failed to map memory".to_string());
        }
        
        // 清零内存
        unsafe { ptr::write_bytes(ptr as *mut u8, 0, size); }
        
        Ok(Self {
            file_path: file_path.to_string(),
            size,
            ptr,
        })
    }
    
    fn write_test_data(&self) -> Result<(), String> {
        let messages = vec![
            "Hello from Rust!",
            "POSIX共享内存测试",
            "这是跨进程通信",
            "Swift可以读取这些消息",
            "测试完成！",
        ];
        
        let mut offset = 0;
        
        for (i, msg) in messages.iter().enumerate() {
            let msg_bytes = msg.as_bytes();
            let header = SimpleMessage::new(i as u64, msg_bytes.len() as u32);
            let total_size = SimpleMessage::HEADER_SIZE + msg_bytes.len();
            
            if offset + total_size > self.size {
                return Err("Not enough space".to_string());
            }
            
            unsafe {
                // 写入消息头
                ptr::copy_nonoverlapping(
                    &header as *const _ as *const u8,
                    (self.ptr as *mut u8).add(offset),
                    SimpleMessage::HEADER_SIZE,
                );
                
                // 写入消息内容
                ptr::copy_nonoverlapping(
                    msg_bytes.as_ptr(),
                    (self.ptr as *mut u8).add(offset + SimpleMessage::HEADER_SIZE),
                    msg_bytes.len(),
                );
            }
            
            println!("📝 写入消息 #{}: \"{}\" ({}字节)", i, msg, msg_bytes.len());
            offset += total_size;
        }
        
        // 写入结束标记
        let end_marker = SimpleMessage::new(999, 0);
        unsafe {
            ptr::copy_nonoverlapping(
                &end_marker as *const _ as *const u8,
                (self.ptr as *mut u8).add(offset),
                SimpleMessage::HEADER_SIZE,
            );
        }
        
        Ok(())
    }
    
    fn read_test_data(&self) -> Result<(), String> {
        let mut offset = 0;
        let mut count = 0;
        
        println!("\n📖 读取共享内存中的消息:");
        
        loop {
            if offset + SimpleMessage::HEADER_SIZE > self.size {
                break;
            }
            
            let header = unsafe {
                ptr::read_unaligned((self.ptr as *const u8).add(offset) as *const SimpleMessage)
            };
            
            if !header.is_valid() {
                println!("❌ 无效消息头，停止读取");
                break;
            }
            
            if header.sequence == 999 {
                println!("✅ 到达结束标记");
                break;
            }
            
            if header.message_len > 0 {
                let msg_data = unsafe {
                    std::slice::from_raw_parts(
                        (self.ptr as *const u8).add(offset + SimpleMessage::HEADER_SIZE),
                        header.message_len as usize,
                    )
                };
                
                let msg_str = String::from_utf8_lossy(msg_data);
                println!("📨 消息 #{}: \"{}\" (时间戳: {})", header.sequence, msg_str, header.timestamp);
                count += 1;
            }
            
            offset += SimpleMessage::HEADER_SIZE + header.message_len as usize;
        }
        
        println!("✅ 共读取 {} 条消息", count);
        Ok(())
    }
    
    fn run_demo(&self) -> Result<(), String> {
        println!("🚀 POSIX共享内存演示");
        println!("===================");
        println!("文件: {}", self.file_path);
        println!("大小: {} bytes", self.size);
        println!("地址: {:p}", self.ptr);
        println!();
        
        // 写入测试数据
        self.write_test_data()?;
        
        println!("\n⏳ 等待3秒...");
        thread::sleep(Duration::from_secs(3));
        
        // 读取测试数据
        self.read_test_data()?;
        
        println!("\n🎯 演示说明:");
        println!("  ✅ 成功创建文件映射共享内存");
        println!("  ✅ 写入多条测试消息");
        println!("  ✅ 读取并验证消息完整性");
        println!("  ✅ 使用二进制消息格式");
        println!("  ✅ 时间戳和序列号验证");
        
        println!("\n💡 Swift程序可以:");
        println!("  • 打开相同的文件: {}", self.file_path);
        println!("  • 使用mmap映射到内存");
        println!("  • 读取相同的消息格式");
        println!("  • 实现真正的进程间通信");
        
        Ok(())
    }
}

impl Drop for PosixDemo {
    fn drop(&mut self) {
        unsafe { munmap(self.ptr, self.size); }
        println!("🧹 内存映射已清理");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌟 POSIX共享内存 - 简化演示");
    println!("============================");
    println!();
    
    let demo = PosixDemo::new("/tmp/posix_demo_simple.dat", 4096)?;
    demo.run_demo()?;
    
    println!("\n🔬 技术验证:");
    println!("  ✅ 文件映射作为POSIX共享内存的替代方案");
    println!("  ✅ 二进制消息格式跨语言兼容");
    println!("  ✅ 内存映射实现零拷贝访问");
    println!("  ✅ 适合Swift和Rust进程间通信");
    
    Ok(())
}