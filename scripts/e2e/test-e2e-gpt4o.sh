#!/bin/bash
# VERIDACTUS GPT-4o 端到端测试
# 测试: 控制面 CRUD → 推送到数据面 → 数据面转发GPT-4o → Trace审计 → 前端展示
set -e

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
DP="http://localhost:8080"; CP="http://localhost:8081"; UI="http://localhost:3000"
API_KEY="veridactus_e2e_test_key_2026"
GPT4O_TOKEN="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"
PASSED=0; FAILED=0
function pass() { echo -e "${GREEN}✓ $1${NC}"; PASSED=$((PASSED+1)); }
function fail() { echo -e "${RED}✗ $1${NC}"; FAILED=$((FAILED+1)); }

echo "============================================"
echo "  VERIDACTUS GPT-4o E2E 测试 v0.2.1"
echo "============================================"
echo ""

# === [1] 健康检查 ===
curl -sf "$DP/health" > /dev/null && pass "Data Plane 运行中" || fail "Data Plane 未连接"
curl -sf "$CP/api/v1/health" > /dev/null && pass "Control Plane 运行中" || fail "Control Plane 未连接"

# === [2] 控制面注册 GPT-4o ===
EXISTS=$(curl -s "$CP/api/v1/models" | python3 -c "import sys,json; ms=json.load(sys.stdin).get('models',[]); print(any(m['name']=='gpt-4o' for m in ms))" 2>/dev/null)
if [ "$EXISTS" = "True" ]; then
  pass "GPT-4o 已在控制面注册"
else
  echo "注册 GPT-4o..."
  RESP=$(curl -s -X POST "$CP/api/v1/models" -H "Content-Type: application/json" \
    -d '{"name":"gpt-4o","upstream_url":"https://models.inference.ai.azure.com","upstream_model":"gpt-4o","api_key":"'"$GPT4O_TOKEN"'","api_key_header":"Authorization","is_default":false,"status":"active","use_proxy":false}')
  echo "$RESP" | grep -q '"id"' && pass "GPT-4o 注册成功" || fail "GPT-4o 注册失败: $RESP"
fi

# 验证控制面→数据面推送
echo "验证配置推送(gpt-4o 应在 DP models 列表中)"
for i in 1 2 3 5; do
  sleep $i
  FOUND=$(curl -s "$DP/models" 2>/dev/null | python3 -c "import sys,json; ms=json.load(sys.stdin).get('data',[]); print(any('gpt-4o' in m.get('id','') for m in ms))" 2>/dev/null)
  [ "$FOUND" = "True" ] && break
done
[ "$FOUND" = "True" ] && pass "配置推送生效: 数据面已发现 gpt-4o" || fail "配置推送失败: 数据面未发现 gpt-4o"

# === [3] 核心: GPT-4o Chat Completion ===
echo ""
echo "--- GPT-4o 推理 ---"
RESP=$(curl -s -D /tmp/hdrs.txt -X POST "$DP/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -H "VERIDACTUS-Version: 0.2" \
  -H "VERIDACTUS-Budget-Limit: 0.10" \
  -d '{"model":"gpt-4o","messages":[{"role":"system","content":"You are a helpful assistant."},{"role":"user","content":"Say hello in exactly 5 words."}],"max_tokens":50}')
HDRS=$(cat /tmp/hdrs.txt)
TRACE_ID=$(echo "$HDRS" | grep -i veridactus-trace-id | awk '{print $2}' | tr -d '\r\n')
echo "$HDRS" | grep -qi "200" && pass "GPT-4o 返回 HTTP 200" || fail "GPT-4o 返回非 200"
[ -n "$TRACE_ID" ] && pass "返回 Trace ID: $TRACE_ID" || fail "缺少 Trace ID"
echo "$HDRS" | grep -qi "proof" && pass "返回 Proof 头" || fail "缺少 Proof 头"
echo "$HDRS" | grep -qi "cost" && pass "返回 Cost 头" || fail "缺少 Cost 头"

# 验证响应内容
CONTENT=$(echo "$RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('choices',[{}])[0].get('message',{}).get('content',''))" 2>/dev/null)
[ -n "$CONTENT" ] && pass "GPT-4o 响应内容: ${CONTENT:0:80}" || fail "GPT-4o 响应为空"

# === [4] Trace 审计 ===
echo ""
echo "--- Trace 审计 ---"
TJSON=$(curl -s "$DP/v1/traces?id=$TRACE_ID")
echo "$TJSON" | python3 -c "import sys,json; j=json.load(sys.stdin); assert j['trace_id']=='$TRACE_ID'" 2>/dev/null && pass "Trace 可查询" || fail "Trace 查询失败"
echo "$TJSON" | python3 -c "import sys,json; j=json.load(sys.stdin); assert 'FINALIZED' in str(j)" 2>/dev/null && pass "状态 FINALIZED" || fail "状态错误"
echo "$TJSON" | python3 -c "import sys,json; j=json.load(sys.stdin); s=j['proofs']['proof_chain'][0]['signature']; assert len(s)==64" 2>/dev/null && pass "L0 签名正确(64 hex)" || fail "L0 签名错误"
echo "$TJSON" | python3 -c "import sys,json; j=json.load(sys.stdin); assert j['observations']['tokens_count']>0" 2>/dev/null && pass "Token 已记录: $(echo $TJSON | python3 -c "import sys,json; print(json.load(sys.stdin)['observations']['tokens_count'])")" || fail "Token 未记录"
echo "$TJSON" | python3 -c "import sys,json; j=json.load(sys.stdin); assert 'compliance_mappings' in j" 2>/dev/null && pass "合规映射已集成" || fail "合规映射缺失"

# Trace 列表
TLIST=$(curl -s "$DP/v1/traces")
echo "$TLIST" | python3 -c "import sys,json; assert json.load(sys.stdin)['total']>0" 2>/dev/null && pass "Trace 列表非空" || fail "Trace 列表为空"

# === [5] 前端代理测试 ===
echo ""
echo "--- 前端代理 ---"
curl -sf "$UI/" > /dev/null && pass "前端页面可访问" || fail "前端不可用"
curl -sf "$UI/v1/traces" > /dev/null && pass "前端 /v1/traces 代理正常" || fail "前端代理异常"
curl -sf "$UI/api/v1/health" > /dev/null && pass "前端 /api/v1/health 代理正常" || fail "前端CP代理异常"

# === [6] 控制面 CRUD ===
echo ""
echo "--- 控制面 CRUD ---"
NM=$(curl -s -X POST "$CP/api/v1/models" -H "Content-Type: application/json" \
  -d '{"name":"e2e-test","upstream_url":"http://localhost:11434","upstream_model":"test","status":"active"}')
NM_ID=$(echo "$NM" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null)
[ -n "$NM_ID" ] && pass "创建模型成功 (ID: $NM_ID)" || fail "创建模型失败"

# 验证推送: 数据面应在 5秒内出现 e2e-test
for i in 1 2 3 5; do
  sleep $i
  curl -sf "$DP/models" | python3 -c "import sys,json; print([m['id'] for m in json.load(sys.stdin).get('data',[])])" 2>/dev/null | grep -q "e2e-test" && break
done && pass "创建模型推送生效(数据面已发现)" || fail "创建模型推送未生效"

# 更新
curl -sf -X PUT "$CP/api/v1/models/$NM_ID" -H "Content-Type: application/json" \
  -d '{"name":"e2e-test-upd","upstream_url":"http://localhost:11434","upstream_model":"test","status":"active"}' > /dev/null && pass "更新模型成功" || fail "更新模型失败"

# 删除
curl -sf -X DELETE "$CP/api/v1/models/$NM_ID" > /dev/null && pass "删除模型成功" || fail "删除模型失败"

# 验证推送: 数据面应删除 e2e-test-upd
for i in 1 2 3; do
  sleep $i
  curl -sf "$DP/models" | python3 -c "import sys,json; print([m['id'] for m in json.load(sys.stdin).get('data',[])])" 2>/dev/null | grep -qv "e2e-test" && break
done && pass "删除模型推送生效(数据面已移除)" || fail "删除模型推送未生效"

# API Key CRUD
AK=$(curl -s -X POST "$CP/api/v1/apikeys" -H "Content-Type: application/json" -d '{"name":"e2e-key","tenant_id":"test"}')
AK_ID=$(echo "$AK" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null)
[ -n "$AK_ID" ] && pass "创建 API Key 成功" || fail "创建 API Key 失败"
curl -sf -X DELETE "$CP/api/v1/apikeys/$AK_ID" > /dev/null && pass "删除 API Key 成功" || fail "删除 API Key 失败"

# Pipeline CRUD
PL=$(curl -s -X POST "$CP/api/v1/pipelines" -H "Content-Type: application/json" \
  -d '{"tenant":"e2e","stages":[{"placement":"pre_request","parallel":false,"plugins":[{"name":"Budget Guard","type":"native","config":"{}","enabled":true}]}]}')
PL_ID=$(echo "$PL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('plan_id',''))" 2>/dev/null)
[ -n "$PL_ID" ] && pass "创建流水线成功" || fail "创建流水线失败"
curl -sf -X DELETE "$CP/api/v1/pipelines/$PL_ID" > /dev/null && pass "删除流水线成功" || fail "删除流水线失败"

# === [7] GDPR 删除与合规 ===
echo ""
echo "--- GDP 与合规 ---"
# 创建一个新 trace 用于合规报告（避免被 GDPR 删除影响）
COMP_TRACE=$(curl -s -D /tmp/comp_h.txt -X POST "$DP/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"test"}],"max_tokens":5}' | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null)
COMP_TID=$(grep -i "veridactus-trace-id" /tmp/comp_h.txt | awk '{print $2}' | tr -d '\r\n')
[ -n "$COMP_TID" ] && COMP_TID=$COMP_TID || COMP_TID=$COMP_TRACE

# 合规报告（用独立的 trace）
curl -sf "$DP/v1/compliance/report/$COMP_TID" | python3 -c "import sys,json; j=json.load(sys.stdin); assert 'mappings' in j" 2>/dev/null && pass "合规报告可用" || (curl -s "$DP/v1/compliance/report/$COMP_TID" | head -1; fail "合规报告失败")

# GDPR 删除（用之前的 trace）
curl -sf -X POST "$DP/v1/gdpr/delete" -H "Content-Type: application/json" \
  -d "{\"deletion_type\":\"trace_id\",\"target_id\":\"$TRACE_ID\"}" | python3 -c "import sys,json; assert json.load(sys.stdin).get('success')==True" 2>/dev/null && pass "GDPR 删除成功" || fail "GDPR 删除失败"

echo ""
echo "============================================"
echo -e "  ${GREEN}通过: $PASSED${NC}  ${RED}失败: $FAILED${NC}"
echo "============================================"
[ $FAILED -eq 0 ] && echo -e "${GREEN}✓ 全部通过!${NC}" && exit 0 || echo -e "${RED}✗ 部分失败${NC}" && exit 1
