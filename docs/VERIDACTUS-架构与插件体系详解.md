# VERIDACTUS 架构与插件体系详解

## 一、系统总览

VERIDACTUS 由四个核心组件构成，形成"三层服务 + 一层监控"的云原生架构：

```
┌─────────────────────────────────────────────────────────────────┐
│                        用户/客户端                               │
│               curl / SDK / 前端 UI / CI/CD Pipeline              │
└──────────────┬──────────────────────────┬────────────────────────┘
               │                          │
               ▼                          ▼
┌──────────────────────┐    ┌──────────────────────────────────────┐
│   React 前端 (:3000)  │    │   Rust 数据面 (:8080)                 │
│   管理后台 UI         │    │  AI 代理网关 + 密码学公证处           │
│                      │    │                                      │
│  - 仪表盘             │    │  ┌──────────────────────────────┐   │
│  - 流水线设计器        │    │  │  同步流程（毫秒级）            │   │
│  - 模型路由管理        │    │  │  ① 请求解析 → 幂等检查       │   │
│  - API Key 管理        │    │  │  ② Passthrough 判断          │   │
│  - 插件库              │    │  │  ③ 版本协商 → Action 分发    │   │
│  - 审计中心            │    │  │  ④ 委托验证 → 能力协商       │   │
│  - 系统设置            │    │  │  ⑤ DSL 编译 → 约束检测       │   │
└──────────┬─────────────┘    │  │  ⑥ PipelineExecutor 插件执行  │   │
           │                  │  │  ⑦ 上游 LLM 转发             │   │
           │ REST API         │  │  ⑧ G2 输出扫描 + DFA 拦截    │   │
           ▼                  │  │  ⑨ L0/L2A 证明生成           │   │
┌──────────────────────┐    │  └──────────────────────────────┘   │
│  Go 控制面 (:8081)    │    │                                      │
│  配置管理中心         │    │  异步流程（后台，不阻塞用户）：        │
│                      │    │  - L2B ZK 证明框架                   │
│  - Pipelines CRUD    │    │  - C-SafeGen 认证保证                │
│  - Models CRUD       │    │  - 公平性审计                         │
│  - API Keys CRUD      │    │  - 合规报告生成                      │
│  - Plugins/Policies   │    │  - Redis Stream 任务分发             │
│  - 配置版本轮询        │    │                                      │
│  - SQLite 持久化      │    │  存储后端（可选）：                   │
│  - 配置推送到 DP      │    │  内存 / 本地文件 / PostgreSQL         │
└──────────────────────┘    └──────────────┬───────────────────────┘
                                           │ HTTP 调用
                                           ▼
                              ┌───────────────────────────────────┐
                              │  Python Worker (:8001)             │
                              │  增强计算服务（可选）               │
                              │                                   │
                              │  - PII 深度检测（IP/API Key）      │
                              │  - C-SafeGen 多维安全评分           │
                              │  - 语义漂移检测（Jaccard）          │
                              │  - Redis Stream 异步消费            │
                              └───────────────────────────────────┘
```

---

## 二、约束（Constraints）配置体系

### 2.1 约束不是硬编码，有三层动态配置

```
请求头（最灵活） > DSL（请求体） > 流水线预设（管理员配置） > 系统默认值
```

### 2.2 第一层：HTTP 请求头（每次请求动态）

客户端每次调用可通过 16 个 HTTP 头实时设定约束：

| 头部 | 作用 | 示例值 |
|------|------|--------|
| `VERIDACTUS-Budget-Limit` | 预算上限 | `0.10` (美元) |
| `VERIDACTUS-Budget-Strategy` | 预算耗尽策略 | `hard_stop` / `degrade_model` / `adaptive` |
| `VERIDACTUS-Privacy-Level` | 隐私级别 | `raw` / `masked` / `hash_only` / `tee_private` |
| `VERIDACTUS-Guardrails` | 启用的守卫层 | `G1,G2` |
| `VERIDACTUS-Guardrails-Strictness` | 守卫严格度 | `high` / `medium` / `low` |
| `VERIDACTUS-Instruction-Hierarchy` | 指令层级模式 | `strict` / `warn` / `off` / `verified` |
| `VERIDACTUS-Compliance-Profile` | 合规框架 | `EU_AI_ACT_GPAI` |
| `VERIDACTUS-Action` | 治理动作 | `save-baseline` / `replay` / `drift-test` |

### 2.3 第二层：请求体 DSL（每次请求动态）

用户可在 JSON 请求体中内嵌治理 DSL：

```json
{
  "model": "glm-5.1",
  "messages": [...],
  "veridactus_dsl": {
    "intents": {
      "budget_outcome": "cost_effective",
      "privacy_outcome": "pii_not_stored"
    },
    "constraints": {
      "budget": { "limit_usd": 0.05, "strategy": "hard_stop" },
      "privacy": { "level": "masked" },
      "guardrails": { "levels": ["G1","G2"], "strictness": "high" }
    }
  }
}
```

### 2.4 第三层：控制面流水线（管理员 UI 配置，按租户生效）

管理员在 Pipeline Designer（可视化拖拽编辑器）中配置四阶段流水线：

```
pre_request（串行）
  ├── Budget Guard: limit_usd=0.10, strategy=hard_stop
  └── Auth Validator: 验证 API Key

streaming（并行）
  ├── Keyword Guardrail: patterns=["violence","hate"]
  └── PII Masking: level=masked, action=mask

post_response（串行）
  └── Trace Finalizer: L0 签名

async（并行）
  ├── Drift Detector: threshold=0.7
  └── TEE Attestation: platform=tdx
```

配置流程：**UI → 控制面 REST API → SQLite → 控制面推送到数据面**，秒级生效。

---

## 三、隐私保护：如何防止泄露

### 3.1 三道防线

```
输入 PII 脱敏（不让 LLM 看到）
      ↓
流式 DFA 拦截（不让用户看到）
      ↓
输出 G2 扫描（记录 + 改写）
```

### 3.2 检测的 PII 类型

| 类型 | 检测方式 | 正则模式 |
|------|---------|---------|
| 身份证号 | 18 位 + 校验位 | `[1-9]\d{5}(18\|19\|20)\d{2}...` |
| 信用卡号 | Luhn 算法 + 前缀 | `4[0-9]{12}...` (Visa/MC/AmEx) |
| 手机号 | 11 位中国号段 | `1[3-9]\d{9}` |
| 邮箱 | RFC 5322 格式 | `xxx@xxx.xxx` |
| 银行账号 | 16-19 位数字 | `[0-9]{16,19}` |
| 护照号 | 字母+数字 | `[A-Z]{1,2}[0-9]{6,9}` |

### 3.3 三种处置动作

| 动作 | 说明 | 适用场景 |
|------|------|---------|
| `Block` | 直接拒绝请求，返回 429 | 金融合规、严格隐私场景 |
| `Mask` | 保留首尾字符，中间替换为 `*` | 客服场景（能看到部分但不可用） |
| `Flag` | 标记但不拦截，写入审计日志 | 内部监控、合规审计 |

### 3.4 为什么不是"不可信"的正则？

VERIDACTUS 的价值不在正则本身，而在于**密码学审计**：

- 每次检测 → 生成 `SafetyEvent` → 写入 `ExecutionJournal`
- Journal → 打入 `Trace` → L0 JCS+SHA-256 签名
- 检测过程**不可篡改**——改一行，全链断裂，密码学可验证

**类比**：银行的监控摄像头不保证拍到所有小偷，但录像有防篡改水印。

---

## 四、插件体系

### 4.1 三种插件类型

| 类型 | 延迟 | 集成方式 | 例子 |
|------|:--:|------|------|
| **Native** | <10μs | 编译进 Rust 二进制，直接函数调用 | BudgetGuard, PiiDetector |
| **WASM** | 50-200μs | `.wasm` 运行时加载，沙箱隔离 | KeywordGuardrail |
| **gRPC/HTTP** | 5-500ms | 独立进程，HTTP/gRPC 调用 | Python Worker |

### 4.2 插件接口（每个插件必须实现）

```rust
#[async_trait]
pub trait GovernancePlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    
    async fn execute_request(&self, ctx, journal) -> Action;     // pre_request
    async fn execute_stream_chunk(&self, ctx, journal) -> Action; // streaming
    async fn execute_response(&self, ctx, journal) -> Action;    // post_response
    async fn execute_async(&self, ctx, journal) -> Action;       // async
}
```

### 4.3 已注册的生产级插件（7 个）

| 插件 | 阶段 | 功能 | 拦截动作 |
|------|------|------|---------|
| BudgetGuardPlugin | pre_request | 预算检查、硬截断 | Block(429) |
| PiiDetectorPlugin | pre_request | 输入 PII 脱敏 | Mask |
| InputSanitizerPlugin | pre_request | 输入消毒 | Rewrite |
| G1InputFilter | pre_request | 注入/越狱拦截 | Block(400) |
| G2OutputFilter | post_response | 输出 PII/有害内容扫描 | Flag |
| G3SemanticGuard | post_response | 语义一致性检查 | Flag/Block |
| ResponseValidatorPlugin | post_response | 响应格式验证 | Flag |

### 4.4 插件执行引擎

```rust
// PipelineExecutor 支持四种调度模式：

// 1. 串行执行：逐个调用，任一 Block 则停止
for plugin in plugins {
    match plugin.execute() {
        Block => return Blocked,
        Continue => continue,
    }
}

// 2. 并行执行：tokio::join_all 同时跑多个
let results = futures::join_all(plugins.map(|p| p.execute())).await;
```

### 4.5 插件如何添加到流水线

```
管理员 UI (Pipeline Designer)
  → 拖拽插件到 4 个阶段
  → 配置每个插件的参数（limit_usd、masked_fields 等）
  → 保存到控制面 SQLite
  → 控制面推送到数据面 (/v1/admin/config/sync)
  → 数据面更新 PipelineExecutionPlan
  → 下次请求时，PipelineExecutor 按新计划执行
```

---

## 五、Python Worker（增强计算服务）

### 5.1 定位

**增强层，不是必选项。** Rust 数据面内置了核心 PII 检测和 G2 输出过滤。Python Worker 提供额外的语义级分析。

### 5.2 实现的功能

| 功能 | 端点 | 说明 |
|------|------|------|
| PII 深度检测 | `POST /api/v1/pii-detection` | 比 Rust 多了 IP 地址、API Key 检测 |
| 认证保证计算 | `POST /api/v1/compute-guarantee` | 多维安全评分（毒性/PII/偏见/幻觉），Bonferroni 校正 |
| 语义漂移检测 | `POST /api/v1/drift-detection` | Jaccard 相似度比较 |
| Redis 异步消费 | Stream `veridactus:tasks` | 消费数据面推送的异步任务 |

### 5.3 与 Rust 数据面的关系

```
数据面 (Rust) 请求处理中：

  轻量检测 → Rust 本地完成（毫秒级）
    例：正则匹配 PII、DFA 前缀检查、预算预检
  
  重量计算 → 推给 Python Worker（HTTP 调用，10s 超时）
    例：C-SafeGen 安全评分、语义漂移 Jaccard 计算
  
  Worker 挂掉？→ 降级到 Rust 本地检测，不影响核心链路
```

### 5.4 认证保证计算详解

```
输入：LLM 输出的文本内容
       ↓
四个维度的安全评分：
  1. 毒性 (35%)：检测 hate/violence/self-harm/harassment 关键词
  2. PII (30%)：检测邮箱/手机/SSN/信用卡/API Key 模式
  3. 偏见 (20%)：检测 all/always/never/stereotype 绝对化语言
  4. 幻觉 (15%)：检测 "I am trained on..." 等幻觉模式
       ↓
加权计算综合风险 → Bonferroni 校正 → 长度正则化 → 宽恕因子
       ↓
返回：satisfied (是否在风险边界内) + actual_risk (实际风险值)
```

---

## 六、启动指南

### 6.1 核心服务

```bash
# 1. 控制面 (Go, :8081)
cd control-plane && go build -o veridactus-cp ./cmd/server/
DB_PATH=./veridactus.db ./veridactus-cp &

# 2. 数据面 (Rust, :8080)
cd core && cargo build --release
CONTROL_PLANE_URL=http://localhost:8081 ./target/release/veridactus-core &

# 3. 前端 (React, :3000)
cd veridactus-ui && npm run dev
```

### 6.2 Python Worker（可选）

```bash
# 先启动 Redis
docker run -d -p 6379:6379 redis:7-alpine

# 启动 Python Worker
cd python-worker
pip install -r requirements.txt
uvicorn app.main:app --host 0.0.0.0 --port 8001

# 验证
curl http://localhost:8001/health
```

### 6.3 完整 E2E 测试

```bash
# 300+ 功能测试
bash scripts/e2e/e2e-300.sh
```

---

## 七、关键设计决策

| 决策 | 原因 |
|------|------|
| Go 控制面 + Rust 数据面 | Go 适合 REST CRUD + SQLite；Rust 适合高性能流式处理 + 密码学 |
| 插件可降级 | Python Worker 挂掉不影响核心链路；正则兜底 |
| 密码学审计 | JCS+SHA-256 L0 签名使 Trace 不可篡改，数学证明 > 日志信任 |
| 协议-实现解耦 | 协议只定义语义合同，实现自由选择技术栈（Rust/Go/Python/任意） |
| 三层插件体系 | Native (<10μs) + WASM (50-200μs) + gRPC (5-500ms) 覆盖所有场景 |
