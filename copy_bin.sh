#!/bin/bash

# 复制Rust二进制文件到Swift应用资源目录的脚本
# 该脚本由cargo-post在构建后自动执行

# 设置错误时退出
set -e

echo "开始执行二进制文件复制操作..."

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "脚本所在目录: $SCRIPT_DIR"

# 设置项目根目录为脚本所在目录
# 确保根目录包含client和core子目录
if [ -d "$SCRIPT_DIR/client" ] && [ -d "$SCRIPT_DIR/core" ]; then
    PROJECT_ROOT="$SCRIPT_DIR"
else
    # 如果脚本不在正确的位置，尝试推断项目根目录
    PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
    if [ ! -d "$PROJECT_ROOT/client" ] || [ ! -d "$PROJECT_ROOT/core" ]; then
        # 如果上一级也没有正确的结构，尝试硬编码为librorum目录
        if [ -d "/Users/gy/librorum" ]; then
            PROJECT_ROOT="/Users/gy/librorum"
        else
            echo "无法确定项目根目录，请检查脚本位置或手动设置PROJECT_ROOT变量"
            exit 1
        fi
    fi
fi

echo "使用项目根目录: $PROJECT_ROOT"

TARGET_DIR="$PROJECT_ROOT/target"
CORE_DIR="$PROJECT_ROOT/core"
PROFILE=${PROFILE:-debug}  # 如果未设置，默认使用debug配置
BINARY_NAME="librorum"
SWIFT_APP_DIR="$PROJECT_ROOT/client/librorum"
RESOURCES_DIR="$SWIFT_APP_DIR/Resources"

echo "目标目录: $TARGET_DIR"
echo "构建配置: $PROFILE"
echo "Swift应用目录: $SWIFT_APP_DIR"
echo "资源目录: $RESOURCES_DIR"

# 寻找编译后的二进制文件
BINARY_PATH=""
POSSIBLE_PATHS=(
    "$TARGET_DIR/$PROFILE/$BINARY_NAME"
    "$TARGET_DIR/$BINARY_NAME"
    "$CORE_DIR/target/$PROFILE/$BINARY_NAME"
    "$CORE_DIR/target/$BINARY_NAME"
    "$(pwd)/target/$PROFILE/$BINARY_NAME"
    "$(pwd)/target/$BINARY_NAME"
)

for path in "${POSSIBLE_PATHS[@]}"; do
    echo "检查路径: $path"
    if [ -f "$path" ]; then
        BINARY_PATH="$path"
        echo "找到二进制文件: $BINARY_PATH"
        break
    fi
done

if [ -z "$BINARY_PATH" ]; then
    echo "错误: 未找到编译后的二进制文件"
    echo "尝试使用find命令查找..."
    
    # 使用find命令查找
    FOUND_BIN=$(find "$PROJECT_ROOT" -name "$BINARY_NAME" -type f -executable | grep -v "Resources" | head -n 1)
    
    if [ -n "$FOUND_BIN" ]; then
        BINARY_PATH="$FOUND_BIN"
        echo "使用find命令找到二进制文件: $BINARY_PATH"
    else
        echo "查找失败，退出"
        exit 1
    fi
fi

# 确保目标目录存在
mkdir -p "$RESOURCES_DIR"
echo "确保资源目录存在: $RESOURCES_DIR"

# 复制二进制文件
TARGET_FILE="$RESOURCES_DIR/$BINARY_NAME"
echo "复制 $BINARY_PATH 到 $TARGET_FILE"
cp "$BINARY_PATH" "$TARGET_FILE"

# 设置可执行权限
chmod +x "$TARGET_FILE"
echo "设置可执行权限"

# 验证文件是否成功复制
if [ -f "$TARGET_FILE" ]; then
    FILE_SIZE=$(stat -f %z "$TARGET_FILE" 2>/dev/null || stat -c %s "$TARGET_FILE" 2>/dev/null)
    echo "确认文件已成功复制到: $TARGET_FILE"
    echo "文件大小: $FILE_SIZE 字节"
else
    echo "错误: 复制操作后文件不存在: $TARGET_FILE"
    exit 1
fi

echo "复制操作完成"
