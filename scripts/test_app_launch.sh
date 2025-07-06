#!/bin/bash

echo "🚀 Testing Librorum App Launch with Backend Debugging"
echo "======================================================"

# 查找app路径（优先选择Build/Products中的版本，排除Index.noindex）
APP_PATH=$(find ~/Library/Developer/Xcode/DerivedData -path "*/Build/Products/Debug/librorum.app" 2>/dev/null | grep -v "Index.noindex" | head -1)

if [ -z "$APP_PATH" ]; then
    echo "❌ App not found in DerivedData, trying to build..."
    cd /Users/gy/librorum/librorum
    xcodebuild build -scheme librorum -destination 'platform=macOS' > /dev/null 2>&1
    
    if [ $? -eq 0 ]; then
        echo "✅ Build successful"
        APP_PATH=$(find ~/Library/Developer/Xcode/DerivedData -name "librorum.app" 2>/dev/null | head -1)
        if [ -z "$APP_PATH" ]; then
            echo "❌ App still not found after build"
            exit 1
        fi
    else
        echo "❌ Build failed"
        exit 1
    fi
fi

echo "📱 App found at: $APP_PATH"

# 杀掉现有的应用实例
pkill -f "librorum.app" 2>/dev/null

# 先启动控制台监控
echo "🔍 Starting console monitoring..."
osascript << EOF &
tell application "Terminal"
    activate
    do script "echo 'Librorum Console Monitor - Look for debug messages:' && echo '🔧 CoreManager:' && echo '🚀 BackendLaunchManager:' && echo '🔍 Health checks:' && echo '' && log stream --predicate 'process == \"librorum\"' --style compact"
end tell
EOF

sleep 1

echo "🚀 Launching app..."
echo "Watch the Terminal window for debug output..."

# 启动app
open "$APP_PATH"

# 等待一下确保应用启动
sleep 2

# 检查应用是否在运行
if pgrep -f "librorum.app" > /dev/null; then
    echo "✅ App is running!"
else
    echo "⚠️ App may not have started properly"
fi

echo "✅ App launched!"
echo ""
echo "📝 Check the Terminal window for real-time console output"
echo "🔍 Look for debug messages starting with:"
echo "   🔧 CoreManager:"
echo "   🚀 BackendLaunchManager:"
echo "   🔍 Health checks:"
echo ""
echo "📋 Expected flow:"
echo "1. App startup and initialization"
echo "2. Backend launch manager creates mock backend (in DEBUG mode)"
echo "3. Backend startup process with progress updates"
echo "4. Service readiness check"
echo "5. UI transition to main interface"
echo ""
echo "If you see issues, the debug output will help identify where the problem occurs."