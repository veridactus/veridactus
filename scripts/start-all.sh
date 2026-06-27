#!/bin/bash
# VERIDACTUS 一键启动脚本
# 启动顺序: 基础设施 → 数据平面 → 控制平面 → Worker → UI

set -e

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# ==================== 加载环境变量 ====================
if [ -f "$ROOT_DIR/.env" ]; then
  set -a  # 自动 export 所有变量
  source "$ROOT_DIR/.env"
  set +a
  echo "✅ Loaded .env configuration"
else
  echo "⚠️  WARNING: .env not found — JWT_SECRET and MASTER_KEY will be randomly generated"
  echo "   Create .env from .env.example for persistent secrets"
fi

echo "═══════════════════════════════════════"
echo "  VERIDACTUS 全栈启动"
echo "═══════════════════════════════════════"

# 1. 启动基础设施
echo "[1/5] 启动基础设施 (Redis + PostgreSQL + MinIO)..."
cd "$ROOT_DIR"
docker compose -f scripts/docker-compose.yml up -d redis postgres minio
sleep 3
echo "  ✅ 基础设施就绪"

# 2. 启动 Rust 数据平面
echo "[2/5] 启动数据平面 (Rust:8080)..."
cd "$ROOT_DIR/core"
DATABASE_URL="${DATABASE_URL:-postgres://veridactus:veridactus@localhost:5432/veridactus}" \
VERIDACTUS_ADMIN_KEY="${VERIDACTUS_ADMIN_KEY:-veridactus-admin-dev-2026}" \
VERIDACTUS_STORE_BACKEND="${VERIDACTUS_STORE_BACKEND:-postgres}" \
RUST_LOG=info cargo run --release --bin veridactus-core &
DATA_PLANE_PID=$!
echo "  ✅ 数据平面启动中 (PID=$DATA_PLANE_PID)"

# 3. 启动 Go 控制平面
echo "[3/5] 启动控制平面 (Go:8081)..."
cd "$ROOT_DIR/control-plane"
go run ./cmd/server/ &
CONTROL_PLANE_PID=$!
echo "  ✅ 控制平面启动中 (PID=$CONTROL_PLANE_PID)"

# 4. 启动 Python Worker
echo "[4/5] 启动 Python Worker (:8001)..."
cd "$ROOT_DIR/python-worker"
source .venv/bin/activate 2>/dev/null || true
pip install -q -r requirements.txt 2>/dev/null
uvicorn app.main:app --host 0.0.0.0 --port 8001 &
PYTHON_PID=$!
echo "  ✅ Python Worker 启动中 (PID=$PYTHON_PID)"

# 5. 启动 UI
echo "[5/5] 启动 UI (React:3000)..."
cd "$ROOT_DIR/ui"
npm install --silent 2>/dev/null
npm run dev &
UI_PID=$!
echo "  ✅ UI 启动中 (PID=$UI_PID)"

echo ""
echo "═══════════════════════════════════════"
echo "  🎉 VERIDACTUS 全栈已启动"
echo "  数据平面:     http://localhost:8080/health"
echo "  控制平面:     http://localhost:8081/api/v1/health"
echo "  Python Worker: http://localhost:8001/health"
echo "  UI:           http://localhost:3000"
echo "  Redis:        localhost:6379"
echo "  PostgreSQL:   localhost:5432"
echo "═══════════════════════════════════════"
echo ""
echo "按 Ctrl+C 停止所有服务"

# 等待子进程
trap "kill $DATA_PLANE_PID $CONTROL_PLANE_PID $PYTHON_PID $UI_PID 2>/dev/null; exit" INT TERM
wait
