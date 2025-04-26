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
    - [x] 基本文件操作接口
    - [ ] 完整的文件系统实现
    - [ ] 文件权限管理
  - [ ] 元数据管理系统
    - [x] 基本元数据结构
    - [ ] 元数据同步机制
    - [ ] 元数据一致性保证
  - [x] 节点间通信协议
    - [x] gRPC 服务接口
    - [x] 节点注册机制
    - [x] 集群信息管理
    - [x] 节点状态更新
    - [x] 节点发现机制
    - [x] 节点心跳检测
    - [x] 集群管理
    - [x] 节点自动重连
  - [ ] P2P 文件传输
    - [x] 基本传输框架
    - [x] 文件传输测试
    - [ ] 实际 P2P 实现
    - [ ] 传输优化
- [ ] CLI 工具开发
  - [ ] 基础命令实现
    - [x] 启动/停止节点
    - [ ] 创建/删除文件
    - [ ] 上传/下载文件
    - [ ] 列出目录内容
    - [ ] 获取文件信息
  - [ ] 节点管理功能
    - [x] 注册节点
    - [ ] 加入/离开集群
    - [x] 获取节点状态
    - [ ] 查看集群信息
  - [ ] 文件传输功能
    - [ ] 文件传输请求
    - [ ] 传输状态监控
    - [ ] 断点续传支持
- [ ] 前端开发
  - [ ] UI 设计
  - [ ] 核心功能对接
  - [ ] 用户交互优化
- [ ] 测试与优化
  - [x] 文件系统操作测试
  - [x] 节点通信测试
  - [x] 集群管理测试
  - [x] 文件传输测试
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

## CLI 功能需求

### 基础命令
1. **启动节点**：`start` - 启动一个 VDFS 节点
   - 参数：`--node-config <FILE>` - 节点配置文件路径
   - 参数：`--cluster-config <FILE>` - 集群配置文件路径
   - 选项：`--daemon` - 作为守护进程运行
   
2. **停止节点**：`stop` - 停止运行中的 VDFS 节点
   - 参数：`--node-id <ID>` - 要停止的节点 ID
   - 选项：`--force` - 强制停止，不等待任务完成

3. **状态**：`status` - 显示节点状态
   - 参数：`--node-id <ID>` - 要查询的节点 ID
   - 选项：`--json` - 以 JSON 格式输出

### 文件操作
1. **列出文件**：`list` - 列出目录内容
   - 参数：`--path <PATH>` - 要列出的目录路径
   - 参数：`--node <ID>` - 节点 ID
   - 选项：`--recursive` - 递归列出子目录

2. **创建**：`create` - 创建文件或目录
   - 参数：`--path <PATH>` - 要创建的路径
   - 参数：`--type <TYPE>` - 文件类型（file 或 directory）
   - 参数：`--node <ID>` - 节点 ID
   
3. **删除**：`delete` - 删除文件或目录
   - 参数：`--path <PATH>` - 要删除的路径
   - 参数：`--node <ID>` - 节点 ID
   - 选项：`--recursive` - 递归删除目录内容

4. **获取信息**：`info` - 获取文件信息
   - 参数：`--path <PATH>` - 文件路径
   - 参数：`--node <ID>` - 节点 ID

### 数据传输
1. **上传**：`upload` - 上传本地文件到 VDFS
   - 参数：`--local-path <PATH>` - 本地文件路径
   - 参数：`--remote-path <PATH>` - 远程文件路径
   - 参数：`--node <ID>` - 目标节点 ID
   - 选项：`--chunk-size <SIZE>` - 块大小（字节）

2. **下载**：`download` - 从 VDFS 下载文件到本地
   - 参数：`--remote-path <PATH>` - 远程文件路径
   - 参数：`--local-path <PATH>` - 本地文件路径
   - 参数：`--node <ID>` - 源节点 ID

3. **传输**：`drop` - 在节点之间传输文件
   - 参数：`--path <PATH>` - 文件路径
   - 参数：`--source-node <ID>` - 源节点 ID
   - 参数：`--target-node <ID>` - 目标节点 ID

### 节点管理
1. **注册**：`register` - 注册节点到集群
   - 参数：`--node-id <ID>` - 节点 ID
   - 参数：`--host <HOST>` - 主机地址
   - 参数：`--port <PORT>` - 端口号

2. **加入**：`join` - 加入集群
   - 参数：`--node-id <ID>` - 节点 ID
   - 参数：`--cluster-id <ID>` - 集群 ID
   - 选项：`--token <TOKEN>` - 加入令牌（如果需要）

3. **离开**：`leave` - 离开集群
   - 参数：`--node-id <ID>` - 节点 ID
   - 参数：`--cluster-id <ID>` - 集群 ID
   - 选项：`--graceful` - 优雅退出（转移数据）

4. **发现**：`discover` - 发现集群中的节点
   - 参数：`--node-id <ID>` - 当前节点 ID
   - 选项：`--network <NETWORK>` - 网络（局域网、公网等）

5. **集群信息**：`cluster-info` - 获取集群信息
   - 参数：`--node-id <ID>` - 节点 ID

### 其他功能
1. **心跳**：`heartbeat` - 手动发送心跳
   - 参数：`--node-id <ID>` - 节点 ID

2. **配置**：`config` - 显示或修改配置
   - 子命令：`show` - 显示当前配置
   - 子命令：`set` - 设置配置项

3. **日志**：`logs` - 查看节点日志
   - 参数：`--node-id <ID>` - 节点 ID
   - 选项：`--tail <N>` - 显示最后 N 行
   - 选项：`--follow` - 持续显示新日志

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


# 项目日志

## 2025-04-29
- 开始 CLI 工具开发实现
  - 创建模块化的代码结构，分离关注点
  - 实现基础模块和命令处理器
  - 完成主要命令框架的搭建
- 已实现的模块：
  - `client.rs`：与 VDFS 服务通信的客户端代码
  - `config.rs`：处理 CLI 配置的模块
  - `util.rs`：实用函数和工具方法
  - `commands/`：各类命令处理程序
- 已实现的命令：
  - 节点管理：启动、停止、状态查询
  - 配置管理：显示和设置配置
- 下一阶段计划：
  - 完成文件操作相关命令
  - 实现文件传输功能
  - 编写节点通信和集群管理功能

## 2025-04-28
- 所有测试文件修复完成，确保通过测试
- 开始规划 CLI 功能开发
  - 详细列出 CLI 所需命令和参数
  - 规划 CLI 结构和各子命令功能
  - 更新 RoadMap，细化 CLI 开发计划
- 主要 CLI 功能需求规划为五大类：
  - 基础命令（启动/停止节点等）
  - 文件操作（列出/创建/删除文件等）
  - 数据传输（上传/下载/节点间传输）
  - 节点管理（注册/加入/离开集群等）
  - 其他功能（心跳/配置/日志）

## 2025-04-27
- 完成所有测试文件的开发与修复
  - 添加文件系统操作测试（`fs_operations.rs`）
  - 添加节点通信测试（`node_discovery.rs`）
  - 添加集群管理测试（`cluster_management.rs`）
  - 添加文件传输测试（`file_transfer.rs`）
  - 修复测试中的类型错误和字段不匹配问题
  - 确保所有测试用例正常运行
- 更新 RoadMap，标记测试部分完成
- 优化文件传输代码，添加错误处理和进度监控功能
- 统一测试文件的编码风格和日志输出格式

## 2025-04-26
- 初始化项目结构
- 设置基本依赖
- 创建 README.md 和项目文档
- 实现基本的文件系统操作接口
- 添加文件系统测试用例
- 完善节点间通信协议
  - 添加 proto 定义：节点信息、集群信息、节点状态
  - 实现节点注册机制
  - 实现集群信息管理
  - 实现节点状态更新
  - 添加节点通信测试用例
- 全面完善节点间通信协议
  - 实现节点发现机制（使用 UDP 多播）
  - 实现节点心跳检测
  - 实现集群管理（加入/离开集群）
  - 实现自动故障检测和节点状态更新
  - 添加完整的测试用例
  - 优化代码结构，添加 cluster 和 discovery 模块
- 更新依赖配置，支持序列化和网络通信
- 更新 README.md 中的进度
- 节点间通信协议完成，标记为已完成

## 许可证

MIT 许可证
