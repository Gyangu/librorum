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

- [x] 完成基础服务架构设计
- [x] 实现节点发现功能
- [x] 添加Windows跨平台支持
- [x] 添加配置文件管理
- [x] 修复守护进程在macOS/Linux上无法正常保持运行的问题
- [ ] 实现文件同步功能
- [ ] 完善用户界面
- [ ] 增加文件版本管理
- [ ] 增加文件权限控制
- [ ] 增加文件分享功能

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

## 2025-04-30
- 实现系统服务管理功能
  - 完善守护进程功能，支持脱离终端运行
  - 添加日志管理模块，实现日志轮转
  - 实现日志查看命令，支持查看指定行数的日志
  - 增强服务状态查询功能
  - 支持 Mac 和 Windows 平台日志存储位置的规范化
  - 添加日志清理功能，可以自动删除过期日志
  - 添加跨平台通信测试