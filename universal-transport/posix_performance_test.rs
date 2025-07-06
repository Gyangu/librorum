#!/usr/bin/env rust-script

//! POSIX共享内存实际性能测试
//! 测量真实的Swift↔Rust通信速度

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::thread;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};

extern "C" {
    fn mmap(addr: *mut std::ffi::c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut std::ffi::c_void;
    fn munmap(addr: *mut std::ffi::c_void, len: usize) -> i32;
}

const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const MAP_SHARED: i32 = 0x1;
const MAP_FAILED: *mut std::ffi::c_void = (-1isize) as *mut std::ffi::c_void;

#[repr(C)]
struct PerfTestHeader {
    magic: u32,
    message_id: u64,
    timestamp: u64,
    payload_size: u32,
    // 20字节头部
}

impl PerfTestHeader {
    const SIZE: usize = 20;
    const MAGIC: u32 = 0x50455246; // "PERF"
    
    fn new(message_id: u64, payload_size: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            message_id,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            payload_size,
        }
    }
}

#[repr(C)]
struct PerfControl {
    rust_write_pos: AtomicU64,
    rust_read_pos: AtomicU64,
    swift_write_pos: AtomicU64,
    swift_read_pos: AtomicU64,
    rust_msg_count: AtomicU64,
    swift_msg_count: AtomicU64,
    test_running: AtomicU32,
    swift_connected: AtomicU32,
}

impl PerfControl {
    const SIZE: usize = 64;
    
    fn init(&self) {
        self.rust_write_pos.store(0, Ordering::SeqCst);
        self.rust_read_pos.store(0, Ordering::SeqCst);
        self.swift_write_pos.store(0, Ordering::SeqCst);
        self.swift_read_pos.store(0, Ordering::SeqCst);
        self.rust_msg_count.store(0, Ordering::SeqCst);
        self.swift_msg_count.store(0, Ordering::SeqCst);
        self.test_running.store(1, Ordering::SeqCst);
        self.swift_connected.store(0, Ordering::SeqCst);
    }
}

struct PosixPerfTest {
    file_path: String,
    size: usize,
    ptr: *mut std::ffi::c_void,
}

impl PosixPerfTest {
    fn new(file_path: &str, size: usize) -> Result<Self, String> {
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
        
        // 初始化控制块
        let control = unsafe { &*(ptr as *const PerfControl) };
        control.init();
        
        // 清零内存
        unsafe { ptr::write_bytes(ptr as *mut u8, 0, size); }
        control.init();
        
        Ok(Self {
            file_path: file_path.to_string(),
            size,
            ptr,
        })
    }
    
    fn get_control(&self) -> &PerfControl {
        unsafe { &*(self.ptr as *const PerfControl) }
    }
    
    fn get_rust_data_ptr(&self) -> *mut u8 {
        unsafe { self.ptr.add(PerfControl::SIZE) as *mut u8 }
    }
    
    fn get_swift_data_ptr(&self) -> *mut u8 {
        let rust_area_size = (self.size - PerfControl::SIZE) / 2;
        unsafe { self.ptr.add(PerfControl::SIZE + rust_area_size) as *mut u8 }
    }
    
    fn get_area_size(&self) -> usize {
        (self.size - PerfControl::SIZE) / 2
    }
    
    fn write_rust_message(&self, payload: &[u8]) -> Result<u64, String> {
        let control = self.get_control();
        let data_ptr = self.get_rust_data_ptr();
        let area_size = self.get_area_size();
        let total_size = PerfTestHeader::SIZE + payload.len();
        
        if total_size > area_size {
            return Err("Message too large".to_string());
        }
        
        let write_pos = control.rust_write_pos.load(Ordering::Acquire) % area_size as u64;
        let message_id = control.rust_msg_count.fetch_add(1, Ordering::SeqCst);
        
        let header = PerfTestHeader::new(message_id, payload.len() as u32);
        
        unsafe {
            ptr::copy_nonoverlapping(
                &header as *const _ as *const u8,
                data_ptr.add(write_pos as usize),
                PerfTestHeader::SIZE,
            );
            
            if !payload.is_empty() {
                ptr::copy_nonoverlapping(
                    payload.as_ptr(),
                    data_ptr.add(write_pos as usize + PerfTestHeader::SIZE),
                    payload.len(),
                );
            }
        }
        
        control.rust_write_pos.store(
            (write_pos + total_size as u64) % area_size as u64, 
            Ordering::Release
        );
        
        Ok(message_id)
    }
    
    fn read_swift_message(&self) -> Result<Option<(u64, Vec<u8>)>, String> {
        let control = self.get_control();
        let data_ptr = self.get_swift_data_ptr();
        let area_size = self.get_area_size();
        
        let read_pos = control.swift_read_pos.load(Ordering::Acquire);
        let write_pos = control.swift_write_pos.load(Ordering::Acquire);
        
        if read_pos == write_pos {
            return Ok(None);
        }
        
        let read_offset = (read_pos % area_size as u64) as usize;
        
        let header = unsafe { 
            ptr::read_unaligned(data_ptr.add(read_offset) as *const PerfTestHeader) 
        };
        
        if header.magic != PerfTestHeader::MAGIC {
            return Err(format!("Invalid magic: 0x{:x}", header.magic));
        }
        
        let payload = if header.payload_size > 0 {
            let mut payload = vec![0u8; header.payload_size as usize];
            unsafe {
                ptr::copy_nonoverlapping(
                    data_ptr.add(read_offset + PerfTestHeader::SIZE),
                    payload.as_mut_ptr(),
                    header.payload_size as usize,
                );
            }
            payload
        } else {
            Vec::new()
        };
        
        let total_size = PerfTestHeader::SIZE + header.payload_size as usize;
        control.swift_read_pos.store(
            (read_pos + total_size as u64) % area_size as u64,
            Ordering::Release
        );
        
        Ok(Some((header.message_id, payload)))
    }
    
    fn run_performance_test(&self, test_duration_secs: u64, message_size: usize) -> Result<(), String> {
        println!("🚀 POSIX共享内存性能测试");
        println!("========================");
        println!("测试时长: {}秒", test_duration_secs);
        println!("消息大小: {}字节", message_size);
        println!("等待Swift客户端连接...");
        
        let control = self.get_control();
        let test_payload = vec![0x42u8; message_size];
        
        // 等待Swift连接
        while control.swift_connected.load(Ordering::Acquire) == 0 {
            thread::sleep(Duration::from_millis(100));
        }
        
        println!("✅ Swift客户端已连接，开始性能测试...");
        
        let start_time = Instant::now();
        let mut rust_sent = 0u64;
        let mut swift_received = 0u64;
        let mut last_report = Instant::now();
        
        while start_time.elapsed().as_secs() < test_duration_secs {
            // 高频发送消息
            for _ in 0..100 {
                if self.write_rust_message(&test_payload).is_ok() {
                    rust_sent += 1;
                }
            }
            
            // 读取Swift消息
            let mut read_count = 0;
            while let Ok(Some((msg_id, _))) = self.read_swift_message() {
                swift_received += 1;
                read_count += 1;
                if read_count > 100 { break; } // 避免阻塞发送
            }
            
            // 每秒报告一次
            if last_report.elapsed().as_secs() >= 1 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let rust_rate = rust_sent as f64 / elapsed;
                let swift_rate = swift_received as f64 / elapsed;
                let rust_bandwidth = (rust_sent as f64 * message_size as f64) / elapsed / 1024.0 / 1024.0;
                let swift_bandwidth = (swift_received as f64 * message_size as f64) / elapsed / 1024.0 / 1024.0;
                
                println!("📊 [{:.1}s] Rust发送: {} msg ({:.0} msg/s, {:.1} MB/s), Swift接收: {} msg ({:.0} msg/s, {:.1} MB/s)",
                    elapsed, rust_sent, rust_rate, rust_bandwidth, swift_received, swift_rate, swift_bandwidth);
                
                last_report = Instant::now();
            }
            
            // 微小延迟避免100% CPU
            thread::sleep(Duration::from_micros(1));
        }
        
        control.test_running.store(0, Ordering::Release);
        
        let final_elapsed = start_time.elapsed().as_secs_f64();
        
        println!("\n🎯 最终性能测试结果:");
        println!("==================");
        println!("测试时长: {:.2}秒", final_elapsed);
        println!("消息大小: {}字节", message_size);
        println!("");
        
        println!("Rust→Swift通信:");
        println!("  发送消息: {}", rust_sent);
        println!("  消息速率: {:.0} msg/s", rust_sent as f64 / final_elapsed);
        println!("  数据速率: {:.2} MB/s", (rust_sent as f64 * message_size as f64) / final_elapsed / 1024.0 / 1024.0);
        
        println!("Swift→Rust通信:");
        println!("  接收消息: {}", swift_received);
        println!("  消息速率: {:.0} msg/s", swift_received as f64 / final_elapsed);
        println!("  数据速率: {:.2} MB/s", (swift_received as f64 * message_size as f64) / final_elapsed / 1024.0 / 1024.0);
        
        println!("双向总计:");
        let total_messages = rust_sent + swift_received;
        let total_bytes = total_messages as f64 * message_size as f64;
        println!("  总消息数: {}", total_messages);
        println!("  总数据量: {:.2} MB", total_bytes / 1024.0 / 1024.0);
        println!("  平均速率: {:.0} msg/s", total_messages as f64 / final_elapsed);
        println!("  平均带宽: {:.2} MB/s", total_bytes / final_elapsed / 1024.0 / 1024.0);
        
        // 计算延迟估算
        if swift_received > 0 {
            let avg_latency_us = (final_elapsed * 1_000_000.0) / swift_received as f64;
            println!("  平均延迟: {:.2} μs", avg_latency_us);
        }
        
        Ok(())
    }
}

impl Drop for PosixPerfTest {
    fn drop(&mut self) {
        unsafe { munmap(self.ptr, self.size); }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌟 POSIX共享内存 - 实际性能测试");
    println!("==============================");
    println!();
    
    let shared_file = "/tmp/posix_perf_test.dat";
    let shared_size = 16 * 1024 * 1024; // 16MB
    let message_size = 1024; // 1KB消息
    let test_duration = 10; // 10秒测试
    
    let tester = PosixPerfTest::new(shared_file, shared_size)?;
    
    println!("📋 测试配置:");
    println!("  共享内存文件: {}", shared_file);
    println!("  共享内存大小: {} MB", shared_size / 1024 / 1024);
    println!("  每个进程区域: {} MB", shared_size / 2 / 1024 / 1024);
    println!("  测试消息大小: {} 字节", message_size);
    println!("  测试持续时间: {} 秒", test_duration);
    println!();
    println!("请在另一个终端运行Swift客户端:");
    println!("  swift posix_performance_client.swift");
    println!();
    
    tester.run_performance_test(test_duration, message_size)?;
    
    Ok(())
}