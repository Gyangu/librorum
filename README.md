# Librorum

## 项目概述

Librorum 是一个开源的分布式文件系统，由Rust开发的后端核心和Swift开发的跨平台客户端组成。本项目旨在提供高效、安全的文件存储和管理解决方案，支持macOS和iOS平台。

## 系统架构

### 核心模块 (Rust)

- **守护进程系统**：实现服务的启动、停止、重启和状态管理
- **节点管理**：管理本地节点和远程节点的连接，支持mDNS自动发现
- **日志系统**：支持结构化日志记录、日志轮转和查看
- **配置管理**：处理系统配置的加载、验证和保存

### 客户端 (Swift)

- **跨平台UI**：使用SwiftUI构建的用户界面，支持macOS和iOS
- **数据持久化**：使用SwiftData进行本地数据存储
- **状态管理**：使用Observation框架进行状态管理

## 目录结构

```
librorum/
├── core/                   # Rust核心实现
│   ├── src/                # 源代码目录
│   │   ├── main.rs         # 程序入口
│   │   ├── lib.rs          # 库入口
│   │   ├── daemon.rs       # 守护进程管理
│   │   ├── logger.rs       # 日志系统
│   │   ├── config.rs       # 配置系统
│   │   ├── node_manager/   # 节点管理模块
│   │   └── proto/          # gRPC协议定义
│   └── Cargo.toml          # Rust项目配置
├── client/                 # Swift客户端实现
│   ├── Sources/            # 源代码目录
│   │   ├── LibrorumCore/   # 核心功能模块
│   │   ├── LibrorumUI/     # 共享UI组件
│   │   ├── LibrorumMac/    # macOS应用入口
│   │   └── LibrorumIOS/    # iOS应用入口
│   └── Package.swift       # Swift包配置
├── librorum.toml           # 应用默认配置
└── README.md               # 项目说明文档
```

## 安装要求

- Rust 2024 Edition
- Swift 5.9+
- macOS 14+或iOS 17+（完整功能）

## 构建步骤

### 后端 (Rust)

```bash
# 从项目根目录运行
cargo build --release
```

### 客户端 (Swift)

```bash
# 进入client目录
cd client

# 构建iOS模块
swift build --target LibrorumIOS

# 构建macOS模块
swift build --target LibrorumMac
```

## 使用说明

### 服务管理

```bash
# 初始化配置
./target/release/librorum init

# 启动服务
./target/release/librorum start

# 查看服务状态
./target/release/librorum status

# 停止服务
./target/release/librorum stop

# 查看日志
./target/release/librorum logs
```

### 配置文件

默认配置位于`librorum.toml`，包含以下主要配置项：

```toml
node_prefix = "node"         # 节点名称前缀
bind_host = "0.0.0.0"        # 绑定主机地址
bind_port = 50051            # 绑定端口
log_level = "info"           # 日志级别 
data_dir = "/path/to/data"   # 数据存储目录
heartbeat_interval = 5       # 心跳间隔（秒）
discovery_interval = 10      # 节点发现间隔（秒）
```

## 开发计划

- [ ] 完善macOS客户端兼容性
- [ ] 实现更高级的文件同步功能
- [ ] 添加加密和权限控制
- [ ] 支持更多操作系统平台

## 许可证

本项目采用MIT许可证，详见LICENSE文件。

## 贡献

欢迎提交问题和PR，一起改进这个项目！