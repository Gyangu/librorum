#!/bin/bash

# 🚀 Data Portal 发布脚本
# 
# 使用方法:
#   1. 确保已在 crates.io 注册账户
#   2. 运行: ./publish.sh [YOUR_CRATES_IO_TOKEN]

set -e

echo "🌀 Data Portal 发布脚本"
echo "======================"

# 检查参数
if [ $# -eq 0 ]; then
    echo "❌ 请提供 crates.io API token"
    echo "使用方法: ./publish.sh [YOUR_CRATES_IO_TOKEN]"
    echo ""
    echo "💡 获取 token:"
    echo "   1. 访问 https://crates.io/me"
    echo "   2. 点击 'New Token'"
    echo "   3. 复制生成的 token"
    exit 1
fi

CRATES_TOKEN=$1

echo "🔐 登录 crates.io..."
cargo login $CRATES_TOKEN

echo "🧪 运行测试..."
cargo test

echo "📦 验证包构建..."
cargo package

echo "🔍 检查包内容..."
cargo package --list

echo ""
echo "✅ 准备就绪！即将发布到 crates.io"
echo "📊 包信息:"
echo "   名称: data-portal"
echo "   版本: $(grep '^version' Cargo.toml | cut -d'"' -f2)"
echo "   大小: $(du -h target/package/data-portal-*.crate | cut -f1)"
echo ""

read -p "🚀 确认发布？(y/N): " confirm
if [[ $confirm == [yY] || $confirm == [yY][eE][sS] ]]; then
    echo "🚀 发布中..."
    cargo publish
    
    echo ""
    echo "🎉 发布成功！"
    echo "📦 包地址: https://crates.io/crates/data-portal"
    echo "📚 文档: https://docs.rs/data-portal"
    echo ""
    echo "✨ 现在用户可以使用:"
    echo '   [dependencies]'
    echo '   data-portal = "2.0.0"'
    echo ""
    echo "🏷️ 建议创建 git tag:"
    echo "   git tag v2.0.0"
    echo "   git push origin v2.0.0"
else
    echo "❌ 发布已取消"
fi