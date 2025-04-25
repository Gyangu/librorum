# Librorum - 基于 VDFS 的文件管理器

一个基于 VDFS（虚拟分布式文件系统）的跨平台文件管理器，支持多节点之间的文件元数据同步和文件传输。

## 项目结构

- `core/` - VDFS 核心实现
  - 文件系统操作
  - 元数据管理
  - 节点同步
  - gRPC 服务
- `cli/` - 命令行工具
  - 节点管理
  - 文件操作
  - 文件传输

## RoadMap

- [x] 项目基础架构搭建
  - [x] 目录结构规划
  - [x] 依赖配置管理
  - [x] 基础文档编写
- [ ] 核心功能开发
  - [ ] 文件系统基础操作
  - [ ] 元数据管理系统
  - [ ] 节点间通信协议
  - [ ] P2P 文件传输
- [ ] CLI 工具开发
  - [ ] 基础命令实现
  - [ ] 节点管理功能
  - [ ] 文件操作接口
- [ ] 前端开发
  - [ ] UI 设计
  - [ ] 核心功能对接
  - [ ] 用户交互优化
- [ ] 测试与优化
  - [ ] 单元测试覆盖
  - [ ] 性能测试
  - [ ] 压力测试
- [ ] 文档完善
  - [ ] API 文档
  - [ ] 使用手册
  - [ ] 开发指南

## 功能特点

- 分布式文件系统
  - 多节点支持
  - 文件元数据同步
  - 文件传输
- 文件操作
  - 创建、删除、移动、复制
  - 文件信息查询
  - 目录浏览
- 节点管理
  - 节点状态监控
  - 自动同步
  - P2P 传输

## 系统要求

### 后端 (Rust)
- Rust 1.70 或更高版本
- Tonic
- Tokio
- SQLite
- 其他依赖项见 `core/Cargo.toml`

### 前端 (Swift)
- macOS 12.0 或更高版本
- Xcode 14.0 或更高版本
- Swift 5.7 或更高版本

## 快速开始

1. 克隆仓库
2. 编译项目
   ```bash
   cargo build
   ```
3. 启动节点
   ```bash
   # 启动第一个节点
   cargo run --bin librorum-cli -- start --node-config examples/node1.toml --cluster-config examples/cluster.toml
   
   # 启动第二个节点
   cargo run --bin librorum-cli -- start --node-config examples/node2.toml --cluster-config examples/cluster.toml
   ```

## 使用示例

1. 列出文件
   ```bash
   cargo run --bin librorum-cli -- list --path / --node node1
   ```

2. 创建文件
   ```bash
   cargo run --bin librorum-cli -- create --path /test.txt --type file --node node1
   ```

3. 传输文件
   ```bash
   cargo run --bin librorum-cli -- drop --path /test.txt --source-node node1 --target-node node2
   ```

## 配置文件

### 节点配置 (node.toml)
```toml
id = "node1"
name = "电脑A"
host = "127.0.0.1"
port = 50051
root_dir = "./data/node1"
max_file_size = 1073741824  # 1GB
chunk_size = 1048576       # 1MB
workers = 4
```

### 集群配置 (cluster.toml)
```toml
sync_interval = 60  # 60秒
p2p_enabled = true

[[nodes]]
id = "node1"
name = "电脑A"
host = "127.0.0.1"
port = 50051

[[nodes]]
id = "node2"
name = "手机B"
host = "127.0.0.1"
port = 50052
```

## 开发指南

- 后端开发：参见 `core/README.md`
- 前端开发：参见 `client/README.md`

## 许可证

MIT 许可证
