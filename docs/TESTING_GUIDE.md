# 📚 Librorum 测试指南

这是 Librorum 分布式文件系统 Swift 客户端的完整测试文档。

## 🧪 测试概览

### 测试套件结构
```
librorumTests/
├── Models/                  # 数据模型测试
│   ├── NodeInfoTests.swift
│   ├── FileItemTests.swift
│   ├── UserPreferencesTests.swift
│   └── SystemHealthTests.swift
├── Services/               # 服务层测试
│   ├── LibrorumClientTests.swift
│   └── CoreManagerTests.swift
├── Utilities/              # 工具类测试
│   ├── DeviceUtilitiesTests.swift
│   └── FormatUtilitiesTests.swift
└── Integration/            # 集成测试
    ├── AppLifecycleTests.swift
    ├── MockGRPCConnectionTests.swift
    └── RealGRPCConnectionTests.swift  # ✨ 新增：真实后端集成测试
```

### 测试统计
- **总文件**: 11 个测试文件
- **总代码**: 3500+ 行
- **覆盖率**: 全面覆盖前端功能

## 🚀 快速开始

### 方法 1: 使用 Xcode (推荐)
```bash
# 打开项目
open librorum.xcodeproj

# 在 Xcode 中按 ⌘+U 运行所有测试
```

### 方法 2: 使用命令行脚本
```bash
# 运行 Mock 测试
./run_tests.sh mock

# 运行真实 gRPC 测试 (需要后端)
./run_real_backend_tests.sh

# 运行所有测试
./run_tests.sh all
```

## 🔧 Xcode 配置问题解决

如果遇到测试配置问题，请手动执行：

1. **打开 Xcode**
   ```bash
   open librorum.xcodeproj
   ```

2. **在 Xcode 中**:
   - 选择 "librorum" scheme (左上角下拉菜单)
   - 点击 Product → Test (⌘+U)
   - 或者使用 Test Navigator (⌘+6) 运行特定测试

3. **如果 scheme 有问题**:
   - Product → Scheme → Manage Schemes
   - 确保 "librorum" scheme 启用了测试
   - 检查 Test 部分包含 librorumTests 和 librorumUITests

## 🌐 真实 gRPC 测试

### 启动后端服务
```bash
# 方法 1: 使用测试脚本 (推荐)
./start_backend_for_tests.sh

# 方法 2: 手动启动
cd ../core
cargo build --release
./target/release/librorum start --config test_config.toml
```

### 运行真实 gRPC 测试
```bash
# 在另一个终端窗口
./run_tests.sh real
```

### 真实测试覆盖范围
- ✅ 后端可用性检查
- ✅ gRPC 心跳服务
- ✅ 系统健康状态
- ✅ 节点发现
- ✅ 错误处理
- ✅ 性能测试
- ✅ 并发操作

## 📊 测试类型说明

### 1. 数据模型测试 (Models/)
- **SwiftData 集成**: 持久化、查询、关系
- **数据验证**: 边界条件、类型检查
- **并发安全**: 多线程访问测试

### 2. 服务层测试 (Services/)
- **Mock gRPC 客户端**: 无需后端的完整测试
- **核心管理器**: 后端生命周期管理
- **错误处理**: 网络失败、超时处理

### 3. 工具类测试 (Utilities/)
- **设备检测**: macOS/iOS 平台差异
- **格式化工具**: 文件大小、时间、网络格式化
- **跨平台兼容**: 条件编译测试

### 4. 集成测试 (Integration/)
- **应用生命周期**: 启动、后台、恢复
- **Mock gRPC**: 完整的服务端/客户端模拟
- **真实 gRPC**: 与实际后端的端到端测试

## 🔍 运行特定测试

### 在 Xcode 中
1. 打开 Test Navigator (⌘+6)
2. 找到想要的测试文件或方法
3. 点击测试名称旁的播放按钮

### 使用命令行
```bash
# 运行特定测试套件
xcodebuild test -project librorum.xcodeproj -scheme librorum \
  -destination 'platform=macOS' \
  -only-testing:librorumTests/DeviceUtilitiesTests

# 运行特定测试方法
xcodebuild test -project librorum.xcodeproj -scheme librorum \
  -destination 'platform=macOS' \
  -only-testing:librorumTests/DeviceUtilitiesTests/testDeviceUtilitiesPlatformDetection
```

## 🐛 故障排除

### 常见问题

1. **"Unable to find module dependency: 'librorum'"**
   - 解决方案: 在 Xcode 中运行测试而不是命令行

2. **"Scheme librorum is not currently configured for the test action"**
   - 解决方案: 在 Xcode 中重新配置 scheme

3. **gRPC 连接失败**
   - 检查后端是否运行在 127.0.0.1:50051
   - 使用 `./start_backend_for_tests.sh` 启动测试后端

4. **@MainActor 警告**
   - 这些是正常的并发警告，不影响测试功能

### 调试技巧

1. **查看详细日志**:
   ```bash
   xcodebuild test -project librorum.xcodeproj -scheme librorum \
     -destination 'platform=macOS' -verbose
   ```

2. **在 Xcode 中设置断点**:
   - 在测试方法中设置断点
   - 使用 Debug → Debug Workflow → Always Show Disassembly

3. **检查测试覆盖率**:
   - 在 Xcode 中 Product → Test (with Coverage)
   - 查看 Report Navigator 中的覆盖率报告

## 📈 测试最佳实践

### 编写新测试时
1. 使用描述性的测试名称
2. 遵循 AAA 模式 (Arrange, Act, Assert)
3. 确保测试隔离性 (使用内存数据库)
4. 添加适当的错误处理

### Mock vs 真实测试
- **Mock 测试**: 快速、稳定、无外部依赖
- **真实测试**: 端到端验证、实际网络通信

### 性能测试
- 使用 `Date()` 测量执行时间
- 设置合理的超时限制
- 测试并发场景

## 🎯 下一步计划

### 当前状态
- ✅ 完整的测试框架
- ✅ Mock gRPC 测试
- ⚠️ 需要 Xcode 配置修复
- ❓ 真实 gRPC 需要后端集成

## 🌐 真实后端集成测试

### 概述
`RealGRPCConnectionTests.swift` 提供与实际运行的 Rust 后端的完整集成测试。

### 前置条件
1. **构建后端**：
   ```bash
   cd ../  # 到项目根目录
   cargo build --release
   ```

2. **初始化配置**：
   ```bash
   ./target/release/librorum init
   ```

### 运行方式

#### 方法 1: 自动化脚本 (推荐)
```bash
# 自动启动后端并运行集成测试
./run_real_backend_tests.sh
```

这个脚本会：
- ✅ 检查并构建 Rust 后端
- ✅ 自动启动后端服务
- ✅ 运行所有真实 gRPC 测试
- ✅ 自动清理后端进程

#### 方法 2: 手动运行
```bash
# 1. 启动后端
cd ../
./target/release/librorum start --config librorum.toml

# 2. 在另一个终端运行测试
cd librorum/
export ENABLE_REAL_GRPC_TESTS=1
xcodebuild -project librorum.xcodeproj -scheme librorum -destination 'platform=macOS' test -only-testing librorumTests/RealGRPCConnectionTests

# 3. 停止后端
./target/release/librorum stop
```

### 测试覆盖范围

**连接测试**：
- ✅ 建立和断开 gRPC 连接
- ✅ 连接失败处理
- ✅ 并发连接处理

**服务操作测试**：
- ✅ 心跳检测 (Heartbeat)
- ✅ 获取连接的节点列表
- ✅ 获取系统健康状态
- ✅ 节点添加和删除

**性能测试**：
- ✅ 网络延迟测量
- ✅ 并发请求处理
- ✅ 响应时间验证

**协议测试**：
- ✅ gRPC 响应格式验证
- ✅ 错误处理验证
- ✅ 与 CoreManager 集成

### 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `ENABLE_REAL_GRPC_TESTS` | 启用真实 gRPC 测试 | `0` (禁用) |

### 跳过测试
如果后端未运行，测试会自动跳过并显示提示信息：
```
⚠️  Skipping test: Rust backend is not running. Start backend with: ./target/release/librorum start
```

### 改进建议
1. 添加 UI 测试 (SwiftUI)
2. 集成代码覆盖率报告
3. 添加持续集成 (CI)
4. 性能基准测试

## 📞 支持

如果遇到问题:
1. 查看本文档的故障排除部分
2. 检查 Xcode 控制台输出
3. 确保使用最新版本的依赖
4. 在 GitHub 项目中提交 issue

---

**Happy Testing! 🧪✨**