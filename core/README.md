# VDFS Core 模块

VDFS 核心实现模块，提供分布式文件系统的基础功能。

## 目录结构

- `src/` - 源代码目录
  - `config.rs` - 配置管理
  - `error.rs` - 错误处理
  - `fs/` - 文件系统实现
  - `metadata.rs` - 元数据管理
  - `proto/` - Protocol Buffer 定义
  - `service/` - gRPC 服务实现
  - `sync.rs` - 节点同步
  - `tests/` - 测试用例

## 主要功能

### 配置管理 (config.rs)
- 节点配置：定义单个节点的配置参数
- 集群配置：定义集群级别的配置参数
- 配置加载：支持从文件或环境变量加载配置

### 错误处理 (error.rs)
- 自定义错误类型
- 错误转换
- 错误处理工具

### 文件系统 (fs/)
- 本地文件系统实现
- 文件操作接口
- 文件系统监视器

### 元数据管理 (metadata.rs)
- 文件元数据
- 节点状态
- 元数据存储

### 协议定义 (proto/)
- gRPC 服务定义
- 消息类型定义
- 序列化/反序列化

### 服务实现 (service/)
- gRPC 服务实现
- 文件操作服务
- 元数据同步服务
- 文件传输服务

### 节点同步 (sync.rs)
- 节点状态同步
- 元数据同步
- 文件传输

## 测试

运行所有测试：
```bash
cargo test
```

运行特定测试：
```bash
cargo test test_name
```

## 使用示例

```rust
use librorum_core::config::{NodeConfig, ClusterConfig};
use librorum_core::start_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let node_config = NodeConfig::load()?;
    let cluster_config = ClusterConfig::load()?;
    
    start_server(node_config, cluster_config).await?;
    
    Ok(())
}
``` 