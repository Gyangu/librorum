# Librorum 项目参考文档

## 一、项目简介

Librorum 是一个开源的分布式文件系统，后端核心由 Rust 编写，客户端采用 Swift 实现，目标是为 macOS 和 iOS 提供高效、安全的文件存储与管理。

## 二、目录结构与主要内容

```
librorum/
├── core/          # Rust 核心实现
│   ├── src/
│   │   ├── main.rs         # 程序入口
│   │   ├── lib.rs          # 库入口
│   │   ├── daemon.rs       # 守护进程管理
│   │   ├── logger.rs       # 日志系统
│   │   ├── config.rs       # 配置系统
│   │   ├── node_manager/   # 节点管理模块
│   │   └── proto/          # gRPC 协议定义
│   └── Cargo.toml          # Rust 项目配置
├── client/         # Swift 客户端实现
│   ├── librorum/   # SwiftUI 主体代码，含 Models、Views、Services 等
│   ├── librorum.xcodeproj  # Xcode 工程文件
│   ├── build/      # 构建产物
│   ├── librorumTests/ librorumUITests/ # 测试代码
├── examples/       # 示例与测试用例
│   ├── mdns_test/  # mDNS 相关测试
│   ├── tklog_test/ # 日志相关测试
│   └── tracing_test/ # tracing 框架测试
├── doc/            # 文档目录（当前为空）
├── librorum.toml   # 默认配置文件
├── README.md       # 项目说明文档
```

### 主要模块说明
- **core/**：后端服务主逻辑，包含守护进程、配置、日志、节点管理等核心功能。
- **client/**：SwiftUI 客户端，支持 macOS/iOS，包含 UI、服务调用、本地数据管理等。
- **examples/**：包含多种测试用例和示例，便于开发和调试。
- **doc/**：项目文档目录。

## 三、配置文件说明（librorum.toml）

| 配置项              | 说明                 | 默认值 |
|---------------------|----------------------|--------|
| node_prefix         | 节点名称前缀         | node   |
| bind_host           | 服务绑定主机地址     | 0.0.0.0|
| bind_port           | 服务绑定端口         | 50051  |
| log_level           | 日志级别             | info   |
| data_dir            | 数据存储目录         | /Users/gy/Library/Application Support/librorum |
| heartbeat_interval  | 心跳间隔（秒）       | 5      |
| discovery_interval  | 节点发现间隔（秒）   | 10     |

## 四、构建与运行

### Rust 后端
```bash
cargo build --release
```

### Swift 客户端
```bash
cd client
swift build --target LibrorumIOS   # 构建 iOS 模块
swift build --target LibrorumMac   # 构建 macOS 模块
```

## 五、服务管理命令示例
```bash
./target/release/librorum init      # 初始化配置
./target/release/librorum start     # 启动服务
./target/release/librorum status    # 查看服务状态
./target/release/librorum stop      # 停止服务
./target/release/librorum logs      # 查看日志
```

## 六、开发计划与贡献
- 完善 macOS 客户端兼容性
- 实现更高级的文件同步功能
- 添加加密和权限控制
- 支持更多操作系统平台

欢迎贡献代码与建议！详细开发计划见 README.md。

---

> 本文档由 AI 自动生成，建议与 README.md、源码实际内容结合查阅。
