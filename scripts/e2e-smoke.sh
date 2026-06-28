#!/bin/bash
# VERIDACTUS E2E 冒烟测试 (Phase 5)
# 验证核心功能端到端可用性
set -euo pipefail

GREEN='\033[0;32m'; RED='\033[0;31m'; NC='\033[0m'
PASS="${GREEN}✅${NC}"; FAIL="${RED}❌${NC}"

BASE_URL="${VERIDACTUS_BASE_URL:-http://localhost:8080}"
CP_URL="${VERIDACTUS_CP_URL:-http://localhost:8081}"
PASSED=0; TOTAL=0

check() {
    TOTAL=$((TOTAL + 1))
    local desc="$1"; shift
    echo -n "  [$TOTAL] $desc... "
    if "$@" >/dev/null 2>&1; then
        echo -e "$PASS"; PASSED=$((PASSED + 1))
    else
        echo -e "$FAIL"
    fi
}

echo "🚀 VERIDACTUS E2E Smoke Test"
echo "  Data Plane: $BASE_URL"
echo "  Control Plane: $CP_URL"
echo "=============================="

# Phase 1: 基础设施
check "DP health check" curl -sf "$BASE_URL/health"
check "CP health check" curl -sf "$CP_URL/api/v1/health"

# Phase 1: 多租户
check "List organizations" curl -sf "$CP_URL/api/v1/orgs"
check "List workspaces" curl -sf "$CP_URL/api/v1/workspaces"

# Phase 2: Key 管理
check "List virtual keys" curl -sf "$CP_URL/api/v1/virtual-keys"
check "List wallets" curl -sf "$CP_URL/api/v1/wallets"

# Phase 3: Chat
check "Chat completion (non-stream)" \
    curl -sf -X POST "$BASE_URL/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d '{"model":"deepseek-r1:14b","messages":[{"role":"user","content":"hi"}],"max_tokens":5}'

# Phase 3: Trace
check "List traces" curl -sf "$BASE_URL/v1/traces"

# Phase 4: 审计
check "Audit events" curl -sf "$CP_URL/api/v1/audit/events"

# Phase 5: Metrics
check "Prometheus metrics" curl -sf "$BASE_URL/metrics"

echo ""
echo "=============================="
echo -e "Results: ${PASSED}/${TOTAL} passed"
if [ "$PASSED" -eq "$TOTAL" ]; then
    echo -e "${GREEN}🎉 All E2E smoke tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
fi
