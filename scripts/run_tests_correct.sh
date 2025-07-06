#!/bin/bash

# 正确的 Librorum 测试运行脚本
# 基于正确的 xcodebuild 命令行方法

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

echo "🧪 Librorum 正确的命令行测试方法"
echo "================================="

# 项目配置
PROJECT_PATH="librorum.xcodeproj"
SCHEME_NAME="librorum"
DESTINATION="platform=macOS"

# 检查项目文件
if [ ! -f "$PROJECT_PATH/project.pbxproj" ]; then
    echo "❌ 未找到 Xcode 项目文件"
    exit 1
fi

echo "📋 项目信息:"
echo "  项目: $PROJECT_PATH"
echo "  Scheme: $SCHEME_NAME" 
echo "  目标平台: $DESTINATION"
echo

# 步骤 1: 构建供测试使用
echo "🔨 步骤 1: 构建供测试使用 (build-for-testing)"
echo "================================================"

xcodebuild \
  -project "$PROJECT_PATH" \
  -scheme "$SCHEME_NAME" \
  -destination "$DESTINATION" \
  build-for-testing

BUILD_STATUS=$?

if [ $BUILD_STATUS -ne 0 ]; then
    echo "❌ 构建失败 (退出码: $BUILD_STATUS)"
    echo
    echo "🔧 可能的解决方案:"
    echo "1. 在 Xcode 中打开项目: open $PROJECT_PATH"
    echo "2. 检查 scheme 配置 (Product → Scheme → Manage Schemes)"
    echo "3. 确保所有源文件都能编译"
    echo "4. 使用 Xcode 界面运行测试 (⌘+U)"
    exit $BUILD_STATUS
fi

echo "✅ 构建成功!"
echo

# 步骤 2: 运行测试
echo "🧪 步骤 2: 运行测试 (test-without-building)"
echo "============================================"

xcodebuild \
  -project "$PROJECT_PATH" \
  -scheme "$SCHEME_NAME" \
  -destination "$DESTINATION" \
  test-without-building

TEST_STATUS=$?

echo
if [ $TEST_STATUS -eq 0 ]; then
    echo "🎉 测试成功完成!"
    echo
    echo "📊 测试结果:"
    echo "- 所有测试已运行"
    echo "- 检查控制台输出获取详细结果"
else
    echo "⚠️  测试执行有问题 (退出码: $TEST_STATUS)"
    echo
    echo "🔍 调试建议:"
    echo "1. 查看上方的详细错误信息"
    echo "2. 在 Xcode 中运行特定测试进行调试"
    echo "3. 检查测试代码中的断言和逻辑"
fi

echo
echo "🔧 其他有用的命令:"
echo
echo "# 一步完成 (构建+测试):"
echo "xcodebuild -project $PROJECT_PATH -scheme $SCHEME_NAME -destination '$DESTINATION' test"
echo
echo "# 运行特定测试:"
echo "xcodebuild -project $PROJECT_PATH -scheme $SCHEME_NAME -destination '$DESTINATION' test -only-testing:librorumTests/DeviceUtilitiesTests"
echo
echo "# 启用代码覆盖率:"
echo "xcodebuild -project $PROJECT_PATH -scheme $SCHEME_NAME -destination '$DESTINATION' test -enableCodeCoverage YES"
echo
echo "# 使用 xcpretty 美化输出:"
echo "xcodebuild ... test | xcpretty"
echo

exit $TEST_STATUS