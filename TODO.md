# 📋 VERIDACTUS 产品化 TODO — 从 v0.3.0-dev 到 v1.0.0 完整执行计划

> **文档定位**：基于 [AI-1.md](AI-1.md) 终极产品蓝图，结合当前 v0.3.0-dev 实际实现状态，按优先级分阶段的产品化待办清单。
> **更新频率**：每个 Sprint 结束时更新状态标记。

---

## 执行状态图例

| 标记 | 含义 |
|:---:|------|
| ✅ | 已完成交付 |
| 🚧 | 开发中 |
| 📋 | 已规划，待开始 |
| 🔬 | 调研/预研中 |
| ⏸️ | 暂时搁置 |

---

## 一、当前已完成（v0.3.0-dev 交付物）

> 以下功能已实现并可正常工作，不在 TODO 范围内，仅供对照参考。

| 模块 | 交付物 | 
|------|--------|
| **多租户基础设施** | Organization → Workspace 三级隔离；JWT HS256 认证（含 org_id/workspace_id/role/plan）；5 角色 RBAC；行级数据隔离 |
| **用户体系** | 邮箱+密码注册/登录；access_token 15min + refresh_token 自动刷新；30min 无操作自动退出 |
| **存储** | PostgreSQL 统一存储（cp 业务数据 + dp trace JSONB）；SQLite 开发模式保留 |
| **Chat 安全沙箱** | 多会话侧边栏；模型/流水线选择器；VERIDACTUS 治理协议头（L0/L2A/L2B）；动态 PII 安全盾牌；SSE 流式输出；Trace ID 面板 |
| **Dev Hub Playground** | 三栏布局（prompt editor + 流式输出 + X-Ray 面板）；模型/流水线选择器；VERIDACTUS 治理头 |
| **Holo-Trace Vault** | 按对话 session 分组浏览；时间倒序；trace 详情页；对话标题显示；搜索过滤 |
| **控制面 API** | 30+ REST 端点（org/workspace/pipeline/model/apikey/virtual-key/wallet/settings/conversation/trace） |
| **数据面** | OpenAI 兼容 `/v1/chat/completions`；7 插件治理流水线；L0 JCS+SHA-256 签名链；L2A Merkle 验证；L2B ZK 证明框架；流式预算守卫 |
| **流水线管理** | CP CRUD + 发布推送 DP；DP config/poll 自动同步；Pipeline Designer（ReactFlow） |
| **前端基础设施** | AuthGuard 路由保护；Sidebar 导航；JWT 自动刷新；localStorage 用户隔离；Dark 主题 |
| **核心安全** | 租户数据隔离（traces/conversations/models/pipelines 全链路）；API Key 管理 |

---

## 二、TODO 清单（按优先级分阶段）

---

### 🔴 P0 — 阻碍上线的致命缺失（3-4 周）

#### P0-1: 自动化测试体系

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-001 | Go 控制面单元测试 — 覆盖 auth、store、handler 核心逻辑（目标 >70%） | P0 | 📋 |
| T-002 | Rust 数据面单元测试 — 覆盖 plugin executor、crypto/signature、budget guard（目标 >80%） | P0 | 📋 |
| T-003 | Rust `cargo fuzz` 模糊测试 — 针对 DFA 模式匹配与 JCS 规范化 | P0 | 📋 |
| T-004 | 前端 E2E 测试 — Cypress/Playwright 覆盖 Chat 发送、Vault 浏览、登录流程 | P0 | 📋 |
| T-005 | API 契约测试 — 基于 `docs/api/openapi.yaml` 验证所有端点 | P0 | 📋 |
| T-006 | 性能基准测试 — Rust benches 覆盖签名、序列化、插件执行热路径 | P0 | 📋 |

#### P0-2: 错误处理与可观测性

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-007 | CP 统一错误响应格式 — 所有 handler 使用 `jsonError()` 返回结构化错误 | P0 | 📋 |
| T-008 | DP 统一错误响应 — passthrough + governance 模式错误格式对齐 OpenAI API | P0 | 📋 |
| T-009 | 前端全局错误边界 — React ErrorBoundary + API 错误 Toast 提示 | P0 | 📋 |
| T-010 | 请求日志结构化 — CP 统一使用 `slog`/`zerolog` JSON 格式日志 | P0 | 📋 |
| T-011 | Prometheus 指标导出 — DP 导出 request_count/latency/budget_exceeded 等核心指标 | P0 | 📋 |

#### P0-3: 部署与运维

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-012 | Docker 多架构镜像 — amd64 + arm64，发布到 `ghcr.io/veridactus` | P0 | 📋 |
| T-013 | Docker Compose 一键部署 — 包含 PostgreSQL + CP + DP + UI 的完整栈 | P0 | 📋 |
| T-014 | Kubernetes Helm Chart — 完整的 values.yaml + 模板（CP/DP/UI/PG） | P0 | 📋 |
| T-015 | 健康检查端点加固 — CP `/api/v1/health` 返回 PG 连接状态 | P0 | 📋 |
| T-016 | graceful shutdown — CP/DP 支持 SIGTERM 优雅关闭 | P0 | 📋 |
| T-017 | CI/CD pipeline — GitHub Actions 构建 + 测试 + 镜像推送 | P0 | 📋 |

#### P0-4: 安全加固

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-018 | JWT 签名算法升级 — HS256 → RS256（非对称密钥，支持微服务间验证） | P0 | 📋 |
| T-019 | API Rate Limiting — CP 中间件限制 /api/v1 端点频率 | P0 | 📋 |
| T-020 | SQL 注入审计 — 全量审查所有动态 SQL 拼接（postgres.go 中使用参数化查询） | P0 | 📋 |
| T-021 | XSS 防护 — CSP headers + React 默认转义 + DOMPurify（如有富文本） | P0 | 📋 |
| T-022 | CORS 严格化 — 生产环境仅允许白名单域名 | P0 | 📋 |
| T-023 | 依赖安全审计 — `cargo audit` + `go mod tidy` + `npm audit` 集成 CI | P0 | 📋 |
| T-024 | Secrets 扫描 — `.env` 文件加入 `.gitignore`；GitHub secret scanning | P0 | 📋 |

---

### 🟡 P1 — 核心商业闭环（4-6 周）

#### P1-1: FinOps 计费引擎

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-025 | 钱包系统完善 — `wallets` 表增加 `balance` 精度（DECIMAL 18,6）；`transactions` 表记录充值/扣费明细 | P1 | 📋 |
| T-026 | Stripe 集成 — 信用卡绑定、充值、自动扣款；Webhook 回调处理 | P1 | 📋 |
| T-027 | 定价模型 — 按 token 计费（personal: $0.01/1K tokens, enterprise: 阶梯折扣） | P1 | 📋 |
| T-028 | 用量仪表盘 — 前端 Dashboard 展示本月用量/费用/预算剩余 | P1 | 📋 |
| T-029 | 余额不足处理 — DP 返回 402 Payment Required + 预算耗尽事件 | P1 | 📋 |

#### P1-2: 双轨制 Key 管理

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-030 | BYOK 加密存储 — AES-256-GCM 信封加密方案；密钥由 CP 管理，DP 运行时解密 | P1 | 📋 |
| T-031 | Key 路由解析优化 — DP `resolve_model` 支持从 CP 获取 workspace 专属 API key | P1 | 📋 |
| T-032 | 平台 LLM Pool — 平台统一维护多模型 Master Key（GLM-5.1/DeepSeek/GPT-4o/Claude） | P1 | 📋 |
| T-033 | Virtual Key 分发 — workspace admin 可为成员创建/撤销 Virtual Key（带用量限制） | P1 | 📋 |

#### P1-3: 企业 SSO

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-034 | OAuth2 登录 — GitHub OAuth（个人版入口） | P1 | 📋 |
| T-035 | 企业 SAML/OIDC — Okta / Azure AD / 飞书 / 钉钉 SSO 对接 | P1 | 📋 |
| T-036 | SSO 配置管理 — Org Admin 在 Settings 页面配置 Identity Provider 参数 | P1 | 📋 |

---

### 🟢 P2 — 产品体验跃升（4-6 周）

#### P2-1: Chat 沙箱增强

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-037 | A/B 对比模式 — 同一 prompt 并发请求两个模型，左右分屏流式对比 | P2 | 📋 |
| T-038 | PII 动态盾牌增强 — 增加"呼吸灯效"动画 + 遮罩/明文切换按钮 | P2 | 📋 |
| T-039 | 对话导出 — 支持导出为 Markdown/JSON 格式 | P2 | 📋 |
| T-040 | Prompt 模板库 — 预设安全审计/合规检查等常用 prompt 模板 | P2 | 📋 |
| T-041 | 对话搜索 — 支持按关键词搜索历史对话 | P2 | 📋 |

#### P2-2: Holo-Trace Vault 增强

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-042 | 密码学自证组件 — 浏览器端 Web Crypto API 计算 JCS+SHA-256 哈希，与后端 L0 签名比对 | P2 | 📋 |
| T-043 | 粒子爆发动效 — 验证通过时 Framer Motion 全屏绿色粒子动效 | P2 | 📋 |
| T-044 | Raw/Sanitized 分屏对比 — 详情页左右 50/50 + 滚动条同步联动 | P2 | 📋 |
| T-045 | Trace 批量导出 — 选择多条 trace 导出为 CSV/JSON | P2 | 📋 |
| T-046 | Trace 删除 — 支持 GDPR 删除请求（单条/批量/按 session） | P2 | 📋 |

#### P2-3: Developer Hub 增强

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-047 | X-Ray 面板增强 — 实时显示 Raw Request / Sanitized Request（差异红色高亮） | P2 | 📋 |
| T-048 | 请求历史 — Playground 本地保存最近 20 条请求历史 | P2 | 📋 |
| T-049 | Token 消耗实时仪表 — 模拟仪表盘显示请求速率 | P2 | 📋 |

#### P2-4: Pipeline Studio 升级

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-050 | 暗色系科技风 UI — 升级 Pipeline Designer 视觉风格（背景 `#0B0F19`，霓虹连线） | P2 | 📋 |
| T-051 | 插件市场抽屉 — 左侧可拖拽的插件列表（BudgetGuard/PiiDetector 等） | P2 | 📋 |
| T-052 | 一键模板 — "OWASP Top 10 防护" / "GDPR 合规" 等预设模板自动生成画布 | P2 | 📋 |
| T-053 | 属性面板 — 右侧属性编辑面板（插件参数 JSON 编辑器） | P2 | 📋 |
| T-054 | 实时预览 — Pipeline 执行模拟 + 预期延迟/成本估算 | P2 | 📋 |

---

### 🔵 P3 — 企业级管控（4-5 周）

#### P3-1: 审计指挥舱

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-055 | 全局风险大盘 — ECharts/Recharts 展示"今日拦截次数"、"PII 泄露尝试"按 workspace 聚合 | P3 | 📋 |
| T-056 | 风险级别分布 — 饼图/柱状图展示 safe/flagged/blocked 比例 | P3 | 📋 |
| T-057 | 合规报告 PDF — 异步生成 Merkle Root + ZK 证明综合报告（含时间戳签名） | P3 | 📋 |
| T-058 | 离线验证脚本 — 提供 Python 脚本用于离线验证合规报告的密码学完整性 | P3 | 📋 |

#### P3-2: 白标定制

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-059 | Logo 上传 — Org Admin 上传品牌 Logo（支持 SVG/PNG） | P3 | 📋 |
| T-060 | 主题色定制 — Primary Color / Secondary Color CSS Variables 动态注入 | P3 | 📋 |
| T-061 | 品牌化 Chat UI — Chat 页面自动继承企业主题色 | P3 | 📋 |
| T-062 | 自定义域名 — 企业用户绑定自有域名（CNAME 验证） | P3 | 📋 |

#### P3-3: 合规与法规

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-063 | GDPR Right to Erasure — 前端发起删除请求 → 异步任务删除所有关联 trace | P3 | 📋 |
| T-064 | EU AI Act 合规映射 — 自动将 pipeline 插件映射到法规条款 | P3 | 📋 |
| T-065 | NIST AI RMF 对齐 — 合规报告包含 NIST 600-1 对照表 | P3 | 📋 |
| T-066 | 审计日志导出 — CSV/JSON 格式导出全量审计事件 | P3 | 📋 |

---

### ⚪ P4 — 性能与规模化（3-4 周）

#### P4-1: 存储架构升级

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-067 | Redis 集成 — 高频预算计数器 + Lua 原子扣减脚本 | P4 | 📋 |
| T-068 | Redis Pipeline 热缓存 — DP 启动时从 CP 拉取 pipeline 配置并缓存到 Redis | P4 | 📋 |
| T-069 | ClickHouse OLAP — 海量 trace 秒级聚合分析（按 org/workspace/timestamp 分区） | P4 | 📋 |
| T-070 | MinIO 对象存储 — Raw Trace JSON + ZK 证明对象存储，PG 仅存 URL 引用 | P4 | 📋 |
| T-071 | 数据库连接池优化 — PG pool 大小调优 + 读写分离 | P4 | 📋 |

#### P4-2: 性能优化

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-072 | DP 插件并行执行 — `FuturesUnordered` 替代串行 `for` 循环（parallel stages） | P4 | 📋 |
| T-073 | SSE 流式输出优化 — `requestAnimationFrame` 批量渲染减少重排 | P4 | 📋 |
| T-074 | 前端 Bundle 优化 — Code splitting、lazy loading、Tree shaking | P4 | 📋 |
| T-075 | 性能目标 — P50 < 5ms（插件开销），P99 < 20ms（含签名） | P4 | 📋 |
| T-076 | Grafana 仪表盘 — Prometheus + Grafana 全栈监控面板 | P4 | 📋 |

---

### ⬜ P5 — 生态与社区（持续）

#### P5-1: 文档与内容

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-077 | 多语言文档站点 — zh-CN + en-US（Docusaurus / VitePress） | P5 | 📋 |
| T-078 | API Reference 自动生成 — OpenAPI → Swagger UI / Scalar | P5 | 📋 |
| T-079 | 快速入门教程 — 5 分钟从零到首次 AI 治理调用 | P5 | 📋 |
| T-080 | 架构深度解析 — 博客文章系列（密码学证明链、多租户隔离、插件体系） | P5 | 📋 |

#### P5-2: SDK 与集成

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-081 | Python SDK — `pip install veridactus`，支持同步/流式调用 | P5 | 📋 |
| T-082 | TypeScript SDK — `npm install @veridactus/sdk`，支持浏览器/Node.js | P5 | 📋 |
| T-083 | gRPC 插件 SDK — Python/Go SDK 用于开发外部治理插件 | P5 | 📋 |
| T-084 | LangChain 集成 — VERIDACTUS 作为 LangChain callback handler | P5 | 📋 |
| T-085 | Vercel AI SDK 集成 — `useChat` hook 适配 VERIDACTUS 协议 | P5 | 📋 |

#### P5-3: 开源社区

| ID | 任务 | 优先级 | 状态 |
|:---|------|:---:|:---:|
| T-086 | OpenSSF Scorecard — 达到 Silver/Gold 级别 | P5 | 📋 |
| T-087 | SBOM 生成 — SPDX 格式软件物料清单 | P5 | 📋 |
| T-088 | Dependabot 自动依赖更新 | P5 | 📋 |
| T-089 | CONTRIBUTING 指南完善 — First Good Issue 标签 + 开发环境搭建指南 | P5 | 📋 |
| T-090 | CNCF 一致性测试 — 通过 Cloud Native 互操作性验证 | P5 | 📋 |

---

## 三、Sprint 执行计划

### Sprint 1（本周 — 第1周）：测试 + 安全加固
```
T-001~T-006  测试体系搭建
T-018~T-024  安全加固（JWT RS256 / Rate Limit / SQL注入审计 / CORS）
T-012~T-013  Docker 镜像 + Compose 部署
```

### Sprint 2（第2-3周）：可观测性 + CI/CD
```
T-007~T-011  错误处理统一 + 日志 + Prometheus
T-014~T-017  Helm Chart + 健康检查 + Graceful Shutdown + CI/CD
```

### Sprint 3（第4-6周）：FinOps 计费
```
T-025~T-029  钱包 + Stripe + 定价 + 用量仪表盘
T-030~T-033  双轨制 Key + Key 加密存储 + 平台 Pool
```

### Sprint 4（第7-9周）：企业 SSO + Chat 增强
```
T-034~T-036  OAuth + SAML SSO
T-037~T-041  A/B 对比 + 对话导出 + Prompt 模板
```

### Sprint 5（第10-12周）：Trace Vault + Pipeline Studio
```
T-042~T-049  密码学自证 + 分屏对比 + X-Ray
T-050~T-054  Pipeline Studio 科技风升级 + 插件市场 + 模板
```

### Sprint 6（第13-16周）：企业级管控
```
T-055~T-058  审计指挥舱 + 合规报告 PDF
T-059~T-062  白标定制 + 自定义域名
T-063~T-066  合规法规对齐
```

---

## 四、风险与依赖

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Zhipu API 稳定性 | Chat/Playground 无法使用 | 接入至少 2 个模型供应商（DeepSeek + GLM），平台 Pool 自动 fallback |
| PostgreSQL 单点故障 | 全系统不可用 | P4 引入读写分离 + 流复制；短期使用 Docker 健康检查自动重启 |
| JWT Secret 泄露 | 所有租户数据可被伪造访问 | P0 升级 RS256；密钥轮换机制 |
| 前端体验不达标 | 用户流失 | P2 重点投入动效与交互优化；收集早期用户反馈 |
| Go 团队人力不足 | CP 功能延期 | Python SDK 可由社区贡献；核心功能 Go 优先 |

---

## 五、成功指标

| 指标 | 当前 | 目标（v1.0） |
|------|:---:|:---:|
| 测试覆盖率 (Go) | ~5% | >70% |
| 测试覆盖率 (Rust) | ~10% | >80% |
| P50 插件延迟 | <10μs | <10μs |
| P99 总延迟（含签名） | <50ms | <20ms |
| 支持模型供应商 | 1 (Zhipu) | ≥3 (Zhipu + DeepSeek + OpenAI) |
| 认证方式 | 1 (Email) | ≥3 (Email + GitHub + SAML) |
| 部署方式 | 手动脚本 | Docker Compose + Helm + SaaS |
| 租户数据隔离 | ✅ TEE 级别 | ✅ 密码学可验证 |
| Docker 镜像 | ❌ | ✅ ghcr.io 多架构 |
| 文档语言 | 中文 | zh-CN + en-US |
