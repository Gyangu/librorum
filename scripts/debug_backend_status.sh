#!/bin/bash

echo "🔍 Librorum Backend Status Debugger"
echo "==================================="

# 检查应用是否运行
if pgrep -f librorum > /dev/null; then
    echo "✅ Librorum app is running"
    APP_PID=$(pgrep -f librorum | head -1)
    echo "📱 App PID: $APP_PID"
else
    echo "❌ Librorum app is not running"
    echo "💡 Try: ./test_app_launch.sh"
    exit 1
fi

echo ""
echo "🔍 Checking backend connections..."

# 检查50051端口
if lsof -i :50051 > /dev/null 2>&1; then
    echo "✅ Port 50051 is in use"
    echo "📡 Processes on port 50051:"
    lsof -i :50051 | head -5
else
    echo "❌ Port 50051 is not in use"
    echo "⚠️  Backend service may not be running"
fi

echo ""
echo "🔍 Checking mock backend files..."

# 检查模拟后端相关文件
DATA_DIR="$HOME/Library/Application Support/librorum"
if [ -d "$DATA_DIR" ]; then
    echo "✅ Data directory exists: $DATA_DIR"
    
    if [ -f "$DATA_DIR/mock_backend.sh" ]; then
        echo "✅ Mock backend script exists"
        echo "📝 Script size: $(wc -c < "$DATA_DIR/mock_backend.sh") bytes"
    else
        echo "❌ Mock backend script not found"
    fi
    
    if [ -f "$DATA_DIR/mock_backend.pid" ]; then
        echo "✅ Mock backend PID file exists"
        MOCK_PID=$(cat "$DATA_DIR/mock_backend.pid" 2>/dev/null)
        if ps -p "$MOCK_PID" > /dev/null 2>&1; then
            echo "✅ Mock backend process is running (PID: $MOCK_PID)"
        else
            echo "❌ Mock backend process not running"
        fi
    else
        echo "❌ Mock backend PID file not found"
    fi
    
    echo ""
    echo "📁 Data directory contents:"
    ls -la "$DATA_DIR" 2>/dev/null || echo "  (empty or inaccessible)"
    
else
    echo "❌ Data directory does not exist"
fi

echo ""
echo "🔍 Testing HTTP connection to localhost:50051..."

# 测试HTTP连接
if curl -s --max-time 3 "http://localhost:50051" > /dev/null 2>&1; then
    echo "✅ HTTP connection to localhost:50051 successful"
    echo "📡 Response:"
    curl -s --max-time 3 "http://localhost:50051" | head -3
else
    echo "❌ HTTP connection to localhost:50051 failed"
fi

echo ""
echo "💡 Troubleshooting tips:"
echo "1. If port 50051 is not in use, the backend may not have started"
echo "2. Check Console.app for detailed app logs with 'librorum' filter"
echo "3. If mock backend script exists but process isn't running, check permissions"
echo "4. Look for debug messages in Console.app starting with:"
echo "   🔧 CoreManager:"
echo "   🚀 BackendLaunchManager:"

echo ""
echo "🔄 To restart and test again:"
echo "   pkill -f librorum && ./test_app_launch.sh"