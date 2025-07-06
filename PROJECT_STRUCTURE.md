# Librorum 项目结构

## 📁 主要目录结构

```
librorum/
├── 📋 README.md                 # 项目主要说明
├── 📋 CLAUDE.md                 # Claude Code 使用指南
├── 📋 PROJECT_STRUCTURE.md      # 本文件 - 项目结构说明
├── ⚙️  Cargo.toml               # Rust workspace 配置
├── ⚙️  librorum.toml            # 应用配置文件
│
├── 🏗️  shared/                  # 共享库（gRPC、协议、工具）
│   ├── src/
│   │   ├── config.rs           # 配置管理
│   │   ├── transport/          # Data Portal 传输协议
│   │   └── proto/             # gRPC 协议定义
│   └── Cargo.toml
│
├── 🔧 core/                     # 核心守护进程
│   ├── src/
│   │   ├── main.rs            # 守护进程入口
│   │   ├── node_manager/      # 节点管理和服务发现
│   │   ├── vdfs/             # 虚拟分布式文件系统
│   │   └── proto/            # 协议生成代码
│   └── Cargo.toml
│
├── 💻 cli/                     # 命令行客户端
│   ├── src/
│   │   ├── main.rs           # CLI 入口点
│   │   └── lib.rs            # CLI 功能库
│   └── Cargo.toml
│
├── 📱 client/                  # Swift 客户端应用
│   ├── librorum.xcodeproj/   # Xcode 项目
│   ├── librorum/             # 主要源码
│   │   ├── Models/           # 数据模型
│   │   ├── Views/            # SwiftUI 界面
│   │   ├── Services/         # 服务层
│   │   ├── Transport/        # 传输层
│   │   └── Resources/        # 资源（包含后端二进制）
│   └── librorumTests/        # 测试代码
│
├── 🧪 examples/               # 示例和演示代码
│   ├── mdns_test/           # mDNS 服务发现测试
│   ├── tklog_test/          # 日志系统测试
│   └── tracing_test/        # 追踪系统测试
│
├── 🌍 universal-transport/    # Data Portal 传输协议实现
│   ├── rust/                # Rust 实现
│   ├── swift/               # Swift 实现
│   └── examples/            # 跨语言示例
│
├── 📖 docs/                   # 项目文档
│   ├── ACTUAL_TEST_RESULTS.md
│   ├── PERFORMANCE_OPTIMIZATION_TODO.md
│   ├── PROJECT_TODO.md
│   └── ...                  # 其他技术文档
│
├── 🔧 scripts/               # 构建和测试脚本
│   ├── comprehensive_ipc_test.sh
│   ├── test_*.sh
│   └── ...                  # 其他脚本
│
├── 🗑️  temp_files/           # 临时文件和测试日志
│   ├── debug_test.txt
│   ├── *.log
│   └── ...                  # 临时测试文件
│
└── 🎨 temp_icon/             # 应用图标资源
    └── App Icons/
```

## 🏗️ 架构概述

### 核心组件
1. **Shared Library** (`shared/`) - 通用组件和协议定义
2. **Core Daemon** (`core/`) - 分布式文件系统核心服务
3. **CLI Client** (`cli/`) - 命令行界面
4. **Swift Client** (`client/`) - 跨平台图形界面应用

### 关键特性
- **Data Portal**: 零拷贝高性能传输协议
- **mDNS**: 自动服务发现
- **VDFS**: 虚拟分布式文件系统
- **Cross-platform**: 支持 macOS 和 iOS

## 🚀 快速开始

```bash
# 构建所有 Rust 组件
cargo build --all --release

# 运行核心守护进程
./target/release/librorum-core --daemon

# 运行 CLI 客户端
./target/release/librorum-cli status

# 打开 Swift 应用
open client/librorum.xcodeproj
```

## 📝 开发指南

详细的开发指南请参阅：
- 📋 `CLAUDE.md` - 代码开发说明
- 📖 `docs/PROJECT_TODO.md` - 待办事项
- 📖 `docs/PERFORMANCE_OPTIMIZATION_TODO.md` - 性能优化计划

## 🔗 Git 仓库结构

### 双仓库架构
此项目由 **两个独立的 Git 仓库** 组成：

1. **主仓库**: `https://github.com/Gyangu/librorum.git`
   - 路径：`/Users/gy/librorum/`
   - 包含：分布式文件系统核心组件 (shared, core, cli, client)

2. **传输协议仓库**: `https://github.com/Gyangu/universal-transport.git`
   - 路径：`/Users/gy/librorum/universal-transport/`
   - 包含：Data Portal 高性能传输协议实现
   - **独立仓库**，非 git submodule

### 开发工作流
```bash
# 主仓库操作
git add . && git commit -m "更新核心功能"
git push origin main

# 传输协议仓库操作（需要先切换目录）
cd universal-transport/
git add . && git commit -m "更新传输协议"
git push origin main
cd ..
```

⚠️ **重要提醒**：提交前务必确认当前所在仓库！

## 🧹 项目整理说明

此目录结构已于 2025-01-06 整理，将原本混乱的文件重新组织为：
- ✅ 移除了重复的目录和文件
- ✅ 将文档集中到 `docs/` 目录
- ✅ 将脚本集中到 `scripts/` 目录  
- ✅ 将临时文件移动到 `temp_files/` 目录
- ✅ 统一了项目结构命名规范
- ✅ 明确了双仓库的Git管理结构