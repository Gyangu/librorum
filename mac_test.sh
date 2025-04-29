#!/usr/bin/env bash
# Mac 平台自动化通信测试脚本
set -e

BINARY="./target/debug/librorum"

# 停止可能已经运行的服务
echo "[mac_test] 停止已运行的守护进程（如果有）"
$BINARY stop || true

# 启动守护进程
echo "[mac_test] 启动守护进程"
$BINARY start

# 等待服务就绪
echo "[mac_test] 等待服务启动完毕..."
sleep 3

# 查看日志
echo "[mac_test] 获取最新日志"
$BINARY logs --tail 10 | cat

# 测试连接 Windows 节点 - 使用直接连接模式
WINDOWS_ADDR="windows.local:50052"
echo "[mac_test] 测试直接连接 Windows 节点: $WINDOWS_ADDR"
$BINARY test-connect $WINDOWS_ADDR || echo "连接失败，尝试备用IP地址"

# 尝试连接备用IP地址
echo "[mac_test] 尝试连接备用Windows IP: 192.168.31.92:50052"
$BINARY test-connect 192.168.31.92:50052 || echo "备用IP连接也失败"

# 停止守护进程
echo "[mac_test] 停止守护进程"
$BINARY stop

echo "[mac_test] Mac 平台通信测试完成" 