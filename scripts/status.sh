#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════
#  VERIDACTUS 一键状态检查脚本
#  用法: bash scripts/status.sh [--docker] [--local] [--json]
#  --docker : 检查 Docker Compose 部署
#  --local  : 检查本地进程
#  --json   : JSON 格式输出（供 CI/CD 使用）
# ═══════════════════════════════════════════════════════════════════════
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
MODE="${1:---local}"
JSON_MODE=false
[[ "${2:-}" == "--json" ]] && JSON_MODE=true

check_http() {
  local url="$1" label="$2" expected_code="${3:-200}"
  local code; code=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 3 --max-time 5 "$url" 2>/dev/null || echo "000")
  if [ "$code" == "$expected_code" ]; then
    $JSON_MODE && echo "{\"service\":\"$label\",\"endpoint\":\"$url\",\"status\":\"ok\",\"code\":$code}" || echo -e "  ${GREEN}✅${NC} $label ($url) → HTTP $code"
  else
    $JSON_MODE && echo "{\"service\":\"$label\",\"endpoint\":\"$url\",\"status\":\"fail\",\"code\":$code}" || echo -e "  ${RED}❌${NC} $label ($url) → HTTP $code (expected $expected_code)"
  fi
}

check_tcp() {
  local host="$1" port="$2" label="$3"
  if nc -z -w3 "$host" "$port" 2>/dev/null; then
    $JSON_MODE && echo "{\"service\":\"$label\",\"host\":\"$host\",\"port\":$port,\"status\":\"ok\"}" || echo -e "  ${GREEN}✅${NC} $label ($host:$port) → open"
  else
    $JSON_MODE && echo "{\"service\":\"$label\",\"host\":\"$host\",\"port\":$port,\"status\":\"fail\"}" || echo -e "  ${RED}❌${NC} $label ($host:$port) → closed"
  fi
}

check_docker() {
  local name="$1" label="$2"
  local status; status=$(docker inspect -f '{{.State.Status}}' "$name" 2>/dev/null || echo "not_found")
  if [ "$status" == "running" ]; then
    $JSON_MODE && echo "{\"service\":\"$label\",\"container\":\"$name\",\"status\":\"running\"}" || echo -e "  ${GREEN}✅${NC} $label ($name) → running"
  else
    $JSON_MODE && echo "{\"service\":\"$label\",\"container\":\"$name\",\"status\":\"$status\"}" || echo -e "  ${RED}❌${NC} $label ($name) → $status"
  fi
}

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  VERIDACTUS 系统状态检查${NC}"
echo -e "${CYAN}  $(date '+%Y-%m-%d %H:%M:%S')${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"

if $JSON_MODE; then echo "["; fi

if [ "$MODE" == "--docker" ]; then
  echo -e "\n${YELLOW}[基础设施 — Docker]${NC}"
  check_docker "veridactus-postgres" "PostgreSQL"
  check_docker "veridactus-redis" "Redis"
  check_docker "veridactus-minio" "MinIO"
  
  echo -e "\n${YELLOW}[应用服务 — Docker]${NC}"
  check_docker "veridactus-cp" "Go Control Plane"
  check_docker "veridactus-core" "Rust Data Plane"
  check_docker "veridactus-ui" "React Frontend"
  
  check_http "http://localhost:8081/api/v1/health" "Go CP Health" "200"
  check_http "http://localhost:8080/health" "Rust DP Health" "200"
  check_http "http://localhost:3000" "Frontend" "200"
  
  # 可选服务
  echo -e "\n${YELLOW}[可选服务]${NC}"
  check_docker "veridactus-python-worker" "Python Worker" 2>/dev/null
  check_docker "veridactus-clickhouse" "ClickHouse" 2>/dev/null
  check_docker "veridactus-jaeger" "Jaeger" 2>/dev/null
  check_http "http://localhost:8001/health" "Worker Health" "200" 2>/dev/null
else
  echo -e "\n${YELLOW}[基础设施]${NC}"
  check_tcp "localhost" 5432 "PostgreSQL"
  check_tcp "localhost" 6379 "Redis"
  check_tcp "localhost" 9000 "MinIO"
  
  echo -e "\n${YELLOW}[应用服务]${NC}"
  check_http "http://localhost:8081/api/v1/health" "Go CP (8081)"
  check_http "http://localhost:8080/health" "Rust DP (8080)"
  check_http "http://localhost:3000" "Frontend (3000)"
  check_http "http://localhost:8001/health" "Python Worker (8001)"
fi

echo ""
echo -e "${CYAN}─────────────────────────────────────────────────────${NC}"
echo -e "${YELLOW}查看日志:${NC}"
echo -e "  Go CP:        ${CYAN}docker compose logs -f veridactus-cp${NC}  (或: ${CYAN}tail -f /tmp/veridactus-cp.log${NC})"
echo -e "  Rust DP:      ${CYAN}docker compose logs -f veridactus-core${NC} (或: ${CYAN}tail -f /tmp/veridactus-dp.log${NC})"
echo -e "  Python:       ${CYAN}docker compose logs -f veridactus-python-worker${NC}"
echo -e "  Frontend:     ${CYAN}docker compose logs -f veridactus-ui${NC}"
echo -e "  全部:          ${CYAN}docker compose logs -f${NC}"
echo ""
echo -e "${YELLOW}常见问题排查:${NC}"
echo -e "  1. 检查环境变量:  ${CYAN}cat .env${NC}"
echo -e "  2. 数据库连接:    ${CYAN}docker exec veridactus-postgres psql -U veridactus -d veridactus -c '\\dt'${NC}"
echo -e "  3. Redis 连接:    ${CYAN}docker exec veridactus-redis redis-cli PING${NC}"
echo -e "  4. MinIO Bucket:  ${CYAN}docker exec veridactus-minio mc ls local/veridactus-traces${NC}"
echo -e "  5. JWT 令牌测试:  ${CYAN}curl -s http://localhost:8081/api/v1/auth/login${NC}"
echo -e "  6. Chat 测试:     ${CYAN}curl -s http://localhost:8080/v1/chat/completions${NC}"
echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"

if $JSON_MODE; then echo "]"; fi