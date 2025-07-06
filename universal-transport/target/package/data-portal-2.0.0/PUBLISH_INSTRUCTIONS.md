# 🚀 Data Portal 发布指南

## 📦 发布到 crates.io

### 1. 账户设置
首先需要在 [crates.io](https://crates.io/) 创建账户并获取 API token：

```bash
# 登录 crates.io (需要先注册账户)
cargo login [YOUR_API_TOKEN]
```

### 2. 发布包
```bash
# 确认当前目录
cd /Users/gy/librorum/universal-transport

# 最终检查
cargo test
cargo package

# 发布！
cargo publish
```

### 3. 使用发布的包
发布后，其他用户可以这样使用：

```toml
# Cargo.toml
[dependencies]
data-portal = "2.0.0"
```

```rust
use data_portal::{PortalServer, SharedMemoryTransport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = PortalServer::new("127.0.0.1:9090")?;
    server.start_shared_memory().await?;
    Ok(())
}
```

## 📱 Swift Package

Swift 用户可以直接通过 git URL 使用：

```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/Gyangu/data-portal", from: "2.0.0")
]
```

或在 Xcode 中：
1. File > Add Package Dependencies
2. 输入: `https://github.com/Gyangu/data-portal`

## 🔄 更新版本

更新版本时：

```bash
# 1. 更新版本号
vi Cargo.toml  # 修改 version = "2.0.1"

# 2. 更新 Swift Package
vi swift/Package.swift  # 如果需要

# 3. 提交更改
git add -A
git commit -m "chore: bump version to 2.0.1"
git tag v2.0.1
git push origin main --tags

# 4. 发布新版本
cargo publish
```

## 📊 包信息

- **包名**: `data-portal`
- **当前版本**: `2.0.0`
- **许可证**: MIT
- **仓库**: `https://github.com/Gyangu/data-portal`
- **文档**: 自动生成在 [docs.rs](https://docs.rs/data-portal)

## 🌟 特性

- 🚀 **极致性能**: 69.4 GB/s 峰值传输速度
- ⚡ **超低延迟**: 77 ns 纳秒级响应
- 🔄 **零拷贝**: POSIX 共享内存直接操作
- 🌐 **跨语言**: Rust ↔ Swift 完美兼容
- 📱 **跨平台**: macOS, iOS, Linux

## 🎯 使用场景

- 高频交易系统
- 实时游戏引擎  
- AI 推理加速
- 媒体流处理
- 分布式计算