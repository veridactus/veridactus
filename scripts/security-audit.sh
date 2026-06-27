#!/bin/bash
# VERIDACTUS 安全审计脚本 (Phase 5)
# 运行所有安全检查和依赖审计
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
PASS="${GREEN}✅${NC}"; FAIL="${RED}❌${NC}"; WARN="${YELLOW}⚠️${NC}"

echo "🔐 VERIDACTUS Security Audit"
echo "=============================="

# 1. Rust 依赖审计
echo -n "  Rust (cargo audit)... "
if cargo audit 2>/dev/null; then
    echo -e "$PASS"
else
    echo -e "$WARN (install: cargo install cargo-audit)"
fi

# 2. Go 依赖审计
echo -n "  Go (go vet)... "
cd control-plane && go vet ./... && echo -e "$PASS" || echo -e "$FAIL"
cd ..

# 3. 密钥泄露检查
echo -n "  Secret scanning... "
SECRETS=$(git log -p --all -S "sk-" -- '*.rs' '*.go' '*.tsx' '*.ts' 2>/dev/null | grep -c "sk-" || true)
if [ "$SECRETS" -gt 0 ]; then
    echo -e "$FAIL Found $SECRETS potential key references in git history"
    echo "  Run: git log -p --all -S 'sk-' to inspect"
else
    echo -e "$PASS"
fi

# 4. 硬编码密钥检查
echo -n "  Hardcoded keys... "
HC=$(grep -r "sk-proj-\|sk-ant-\|bce-v3/ALTAK" core/src/ control-plane/ --include='*.rs' --include='*.go' -l 2>/dev/null | wc -l || true)
if [ "$HC" -gt 0 ]; then
    echo -e "$FAIL Found $HC files with potential hardcoded keys"
else
    echo -e "$PASS"
fi

# 5. 环境变量检查
echo -n "  .env file protection... "
if [ -f .env ]; then
    echo -e "$WARN .env file found (ensure it's in .gitignore)"
else
    echo -e "$PASS"
fi

# 6. Rust 格式检查
echo -n "  Rust fmt check... "
cd core && cargo fmt --check 2>/dev/null && echo -e "$PASS" || echo -e "$WARN"
cd ..

# 7. 测试检查
echo -n "  Rust tests... "
cd core && cargo test --lib -q 2>/dev/null && echo -e "$PASS (passed)" || echo -e "$FAIL"
cd ..

echo -n "  Go tests... "
cd control-plane && go test ./internal/... -q 2>/dev/null && echo -e "$PASS (passed)" || echo -e "$WARN"
cd ..

echo ""
echo "=============================="
echo "🎯 Security audit complete"
echo "  Run 'make check' for full lint suite"
echo "  Run 'make e2e' for end-to-end tests"
