# VDFS CLI 模块

VDFS 命令行工具，提供与 VDFS 节点交互的接口。

## 目录结构

- `src/` - 源代码目录
  - `main.rs` - 主程序入口
  - `commands/` - 命令实现
  - `client/` - gRPC 客户端

## 主要功能

### 命令实现
- 节点管理
  - 启动节点
  - 停止节点
  - 查看节点状态
- 文件操作
  - 列出目录
  - 获取文件信息
  - 创建文件/目录
  - 删除文件/目录
  - 移动文件/目录
  - 复制文件/目录
- 文件传输
  - 在节点间传输文件
  - 查看传输状态

### gRPC 客户端
- 连接管理
- 请求处理
- 响应处理

## 使用示例

启动节点：
```bash
cargo run --bin librorum-cli -- start --node-config examples/node1.toml --cluster-config examples/cluster.toml
```

列出目录：
```bash
cargo run --bin librorum-cli -- list --path / --node node1
```

传输文件：
```bash
cargo run --bin librorum-cli -- drop --path /test.txt --source-node node1 --target-node node2
```

## 命令参考

### 节点管理
```bash
# 启动节点
start [--node-config <FILE>] [--cluster-config <FILE>]

# 停止节点
stop [--node-id <ID>]

# 查看节点状态
status [--node-id <ID>]
```

### 文件操作
```bash
# 列出目录
list --path <PATH> [--node <ID>]

# 获取文件信息
info --path <PATH> [--node <ID>]

# 创建文件/目录
create --path <PATH> --type <TYPE> [--node <ID>]

# 删除文件/目录
delete --path <PATH> [--node <ID>]

# 移动文件/目录
move --source <PATH> --target <PATH> [--source-node <ID>] [--target-node <ID>]

# 复制文件/目录
copy --source <PATH> --target <PATH> [--source-node <ID>] [--target-node <ID>]
```

### 文件传输
```bash
# 传输文件
drop --path <PATH> --source-node <ID> --target-node <ID>

# 查看传输状态
transfer-status [--transfer-id <ID>]
``` 