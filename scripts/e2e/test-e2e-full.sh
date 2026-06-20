#!/bin/bash
# VERIDACTUS v0.2.1 端到端测试脚本
# 基于代码实际实现进行验证

set -e

BASE_URL="http://localhost:8080"
API_KEY="veridactus_37f4cccf6f3a5370529389d02fc5af3f9deede89f1f0e14795d20accde45c40a"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass=0
fail=0

test_case() {
    local name="$1"
    local expected="$2"
    local actual="$3"
    
    if [[ "$actual" == *"$expected"* ]]; then
        echo -e "${GREEN}[PASS]${NC} $name"
        ((pass++))
    else
        echo -e "${RED}[FAIL]${NC} $name"
        echo "  Expected: $expected"
        echo "  Actual: $actual"
        ((fail++))
    fi
}

echo "========================================"
echo "VERIDACTUS v0.2.1 端到端测试"
echo "========================================"

# 测试1: 健康检查
echo -e "\n=== 测试1: 健康检查 ==="
result=$(curl -s http://localhost:8080/health)
test_case "健康检查" "VERIDACTUS Proxy v0.2.1 - OK" "$result"

# 测试2: Passthrough模式
echo -e "\n=== 测试2: Passthrough模式 ==="
result=$(curl -s -i -X POST $BASE_URL/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello!"}], "max_tokens": 20}' 2>&1)
test_case "Passthrough返回veridactus-version" "veridactus-version: 0.2" "$result"
test_case "Passthrough返回veridactus-trace-id" "veridactus-trace-id:" "$result"

# 测试3: 治理模式
echo -e "\n=== 测试3: 治理模式 ==="
result=$(curl -s -i -X POST $BASE_URL/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "VERIDACTUS-Version: 0.2" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello!"}], "max_tokens": 20}' 2>&1)
test_case "治理模式返回veridactus-version" "veridactus-version: 0.2" "$result"
test_case "治理模式返回veridactus-trace-id" "veridactus-trace-id:" "$result"
test_case "治理模式返回veridactus-cost-consumed" "veridactus-cost-consumed:" "$result"
test_case "治理模式返回veridactus-proof-levels" "veridactus-proof-levels: L0" "$result"

# 测试4: 版本协商（降级）
echo -e "\n=== 测试4: 版本协商（降级）==="
result=$(curl -s -i -X POST $BASE_URL/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "VERIDACTUS-Version: 1.0" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello!"}], "max_tokens": 20}' 2>&1)
test_case "版本1.0降级到0.2" "veridactus-version: 0.2" "$result"

# 测试5: 认证失败
echo -e "\n=== 测试5: 认证失败 ==="
result=$(curl -s -i -X POST $BASE_URL/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "VERIDACTUS-Version: 0.2" \
  -H "Authorization: Bearer invalid-key" \
  -d '{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello!"}], "max_tokens": 20}' 2>&1)
test_case "认证失败返回401" "HTTP/1.1 401" "$result"
test_case "认证失败返回错误码" "VERIDACTUS_AUTH_REQUIRED" "$result"

# 测试6: 预算控制（零预算）
echo -e "\n=== 测试6: 预算控制（零预算）==="
result=$(curl -s -i -X POST $BASE_URL/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "VERIDACTUS-Version: 0.2" \
  -H "VERIDACTUS-Budget-Limit: 0" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello!"}], "max_tokens": 20}' 2>&1)
test_case "零预算返回错误" "VERIDACTUS_BUDGET_EXCEEDED" "$result"

# 测试7: 模型路由（未知模型）
echo -e "\n=== 测试7: 模型路由（未知模型降级）==="
result=$(curl -s -i -X POST $BASE_URL/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "unknown-model", "messages": [{"role": "user", "content": "Hello!"}], "max_tokens": 20}' 2>&1)
test_case "未知模型返回200" "HTTP/1.1 200" "$result"

# 测试8: Prometheus指标
echo -e "\n=== 测试8: Prometheus指标 ==="
result=$(curl -s $BASE_URL/metrics)
test_case "Prometheus指标veridactus_requests_total" "veridactus_requests_total" "$result"
test_case "Prometheus指标veridactus_budget_remaining" "veridactus_budget_remaining" "$result"

# 测试9: Trace查询
echo -e "\n=== 测试9: Trace查询 ==="
result=$(curl -s $BASE_URL/v1/traces)
test_case "Trace列表返回" "trace_id" "$result"

# 汇总
echo -e "\n========================================"
echo "测试结果汇总"
echo "========================================"
echo -e "${GREEN}通过: $pass${NC}"
echo -e "${RED}失败: $fail${NC}"
echo -e "总计: $((pass + fail))"

if [ $fail -eq 0 ]; then
    echo -e "\n${GREEN}所有测试通过!${NC}"
    exit 0
else
    echo -e "\n${RED}存在失败测试${NC}"
    exit 1
fi
