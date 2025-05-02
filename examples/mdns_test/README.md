# mDNS 服务发现示例

该示例演示了如何使用 [mdns-sd](https://github.com/keepsimple1/mdns-sd) 库进行本地网络上的服务发现和注册。

## 功能

- 注册 mDNS 服务：将服务注册到本地网络，使其他设备可以发现
- 发现 mDNS 服务：在本地网络上发现已注册的服务

## 使用方法

### 注册服务

```bash
# 注册一个 HTTP 服务
cargo run -p mdns_test -- register -s _http._tcp.local. -n my_web_server -p 8080 -a version=1.0 -a path=/api

# 注册一个自定义服务
cargo run -p mdns_test -- register -s _myapp._tcp.local. -n my_service -p 9000 -a key1=value1 -a key2=value2
```

### 发现服务

```bash
# 持续发现 HTTP 服务
cargo run -p mdns_test -- discover -s _http._tcp.local.

# 发现 HTTP 服务，5秒后超时
cargo run -p mdns_test -- discover -s _http._tcp.local. -t 5

# 发现自定义服务
cargo run -p mdns_test -- discover -s _myapp._tcp.local.
```

## 跨平台测试

这个示例可以在 macOS 和 Windows 上运行。在 Windows 上可以通过以下命令运行：

```bash
# 在 Windows 上注册服务
ssh gy@windows.local 'cd E:\librorum && cargo run -p mdns_test -- register -s _http._tcp.local. -n windows_server -p 8080 -a version=1.0'

# 在 Windows 上发现服务
ssh gy@windows.local 'cd E:\librorum && cargo run -p mdns_test -- discover -s _http._tcp.local.'
```

## 日志级别

你可以通过设置环境变量 `RUST_LOG` 来控制日志级别：

```bash
# Mac
RUST_LOG=debug cargo run -p mdns_test -- discover -s _http._tcp.local.

# Windows
ssh gy@windows.local 'cd E:\librorum && $env:RUST_LOG="debug"; cargo run -p mdns_test -- discover -s _http._tcp.local.'
```

日志级别包括：trace, debug, info, warn, error。 