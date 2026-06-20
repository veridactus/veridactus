#!/bin/bash
# VERIDACTUS 系统快速启动脚本
# 使用: chmod +x start-all.sh && ./start-all.sh

set -e

echo ""
echo "========================================"
echo "  VERIDACTUS 系统启动脚本"
echo "========================================"
echo ""

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
echo "根目录: $ROOT_DIR"

# 清理端口
echo "[1] 清理旧进程..."
python3 -c "
import psutil, os, signal
for proc in psutil.process_iter(['pid','name','connections']):
    try:
        for conn in proc.connections():
            if conn.status == 'LISTEN' and conn.laddr.port in [3000,8080,8081,8001]:
                os.kill(proc.pid, signal.SIGKILL)
                print(f'  Killed PID {proc.pid} (:${conn.laddr.port})')
    except: pass
" 2>/dev/null || echo "  (无或已清理)"
sleep 1

# 设置 Rust 编译环境
export SDKROOT="/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
export LIBRARY_PATH="$SDKROOT/usr/lib"
if [ -f /Library/Developer/CommandLineTools/usr/bin/cc ]; then
  export CC="/Library/Developer/CommandLineTools/usr/bin/cc"
  export PATH="/Library/Developer/CommandLineTools/usr/bin:$PATH"
fi

# 启动 Control Plane (Go)
echo ""
echo "[2] 启动 Control Plane (:8081)..."
cd "$ROOT_DIR/control-plane"
go run cmd/server/main.go &
echo "  PID: $!"
sleep 2
echo "  ✅ Control Plane running on http://localhost:8081"

# 启动 Data Plane (Rust)
echo ""
echo "[3] 编译并启动 Data Plane (:8080)..."
cd "$ROOT_DIR/core"
export VERIDACTUS_ADMIN_KEY="veridactus_e2e_test_key_2026"
cargo run > /tmp/data-plane.log 2>&1 &
DP_PID=$!
echo "  PID: $DP_PID (编译中，需 30-60 秒)"
echo "  日志: tail -f /tmp/data-plane.log"

# 启动 Python Worker
echo ""
echo "[4] 启动 Python Worker (:8001)..."
cd "$ROOT_DIR/python-worker"
python3 -m uvicorn app.main:app --host 0.0.0.0 --port 8001 &
echo "  PID: $!"
sleep 2
echo "  ✅ Python Worker running on http://localhost:8001"

# 启动 Frontend
echo ""
echo "[5] 启动 Frontend (:3000)..."
cd "$ROOT_DIR/veridactus-ui"
export PATH="/Users/williamlee/.workbuddy/binaries/node/versions/20.18.0/bin:$PATH"
npx vite --port 3000 --host &
echo "  PID: $!"
sleep 3

echo ""
echo "========================================"
echo "  VERIDACTUS 服务列表"
echo "========================================"
echo ""
echo "  Frontend:      http://localhost:3000"
echo "  Data Plane:    http://localhost:8080"
echo "  Control Plane: http://localhost:8081"
echo "  Python Worker: http://localhost:8001"
echo ""
echo "  Data Plane 编译中，请等待..."
echo "  查看日志: tail -f /tmp/data-plane.log"
echo ""
echo "  获取 API 密钥后可在前端测试 Chat"
echo "========================================"
