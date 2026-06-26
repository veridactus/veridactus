# 🚀 VERIDACTUS 终极产品演进与代码整改指令白皮书
**—— 从“底层硬核引擎”到“杀手级商业产品”的全面重构作战地图**

**文档受众**：核心研发团队、AI 代码生成器、技术合伙人、产品总监
**文档定位**：基于现有代码现状与终极产品方案的**Gap Analysis（差距分析）**，输出像素级的整改指令与重构需求。本文档是研发团队未来 3-6 个月的**最高执行准则**。

---

## 一、 现状剖析与差距分析 (As-Is vs To-Be Gap Analysis)

通过对 `github.com/veridactus/veridactus` 仓库的深度代码审查，我们确认：**VERIDACTUS 已经拥有了全球领先的“底层信任引擎”，但严重缺乏“商业产品外壳”与“企业级业务逻辑”。**

### 1.1 现状肯定（我们的护城河）
*   **Rust 数据面 (`core/`)**：基于 Axum 的 SSE 流式代理、7 大生产级治理插件、JCS/L0/L2A/L2B 密码学证明链已实现。**这是超越 LiteLLM 的物理级壁垒，必须保持并优化。**
*   **协议规范性**：完全兼容 OpenAI API，`VERIDACTUS-*` Header 规范定义清晰。

### 1.2 致命缺失（必须整改的深水区）
| 维度 | 当前代码现状 (As-Is) | 终极产品方案要求 (To-Be) | 差距与整改定性 |
| :--- | :--- | :--- | :--- |
| **用户与权限** | **完全空白**。无注册、无登录、无 Auth 模块。 | 支持微信/手机号/GitHub 注册；企业级 SSO/SAML；基于 Casbin 的多租户 RBAC/ABAC 模型。 | **🔴 致命缺失**：无用户体系则无 SaaS 商业化基础。 |
| **组织与多租户** | **单租户/无隔离**。Go 控制面仅用 SQLite 存简单配置。 | Organization -> Workspace -> Virtual Key 的三级隔离架构；数据物理/逻辑隔离。 | **🔴 致命缺失**：无法支撑企业版“统采统分”的核心诉求。 |
| **Key 与计费** | **无 Key 管理，无计费**。 | **双轨制**（BYOK 加密托管 / Unified 平台聚合）；Virtual Key 分发；微美元 FinOps 钱包与 Stripe 计费。 | **🔴 致命缺失**：无法解决“谁出钱、谁担责”的商业闭环。 |
| **前端形态** | **仅有 Pipeline Designer**。缺乏面向终端用户的交互。 | **双引擎工作台**：VERIDACTUS Chat (安全沙箱) + Developer Hub (Playground) + Holo-Trace Vault (全息金库)。 | **🟡 体验缺失**：缺乏“前台体感”，无法打动非技术决策者。 |
| **存储架构** | **SQLite (控制面)**。 | **异构混合存储**：PostgreSQL (业务) + Redis (高频预算/限流) + ClickHouse (OLAP) + MinIO (对象存储)。 | **🔴 架构瓶颈**：SQLite 无法支撑高并发预算扣减与海量 Trace 分析。 |

---

## 二、 Phase 1 整改指令：基础设施与用户体系重构 (Foundation & Auth)

**目标**：彻底抛弃 SQLite 的单租户局限，构建支撑 SaaS 与企业私有化的多租户底座。

### 2.1 存储层异构化升级指令
*   **指令 1.1 (废弃 SQLite)**：将 `control-plane/` 中的 SQLite 依赖彻底移除（或仅保留为本地 CLI 调试模式）。
*   **指令 1.2 (引入 PostgreSQL)**：使用 GORM 迁移所有 Schema。建立 `organizations`, `workspaces`, `users`, `virtual_keys`, `pipelines` 等核心表。所有表必须包含 `org_id` 和 `workspace_id` 字段，实现**行级数据隔离**。
*   **指令 1.3 (引入 Redis Cluster)**：
    *   用于**微美元预算计数器**（支持 Lua 脚本原子扣减）。
    *   用于 API 限流（Rate Limit 令牌桶）。
    *   用于 Pipeline 配置的热缓存（Rust 数据面启动时从 Redis 拉取，避免频繁查 PG）。
*   **指令 1.4 (引入 ClickHouse & MinIO)**：
    *   定义 ClickHouse 的 `traces` 表（按 `org_id`, `workspace_id`, `timestamp` 分区），用于 FinOps 报表的秒级聚合。
    *   MinIO 用于存储完整的 Raw Trace JSON 和 L2B ZK 证明对象，PG/CH 中仅存 Object URL 和元数据。

### 2.2 控制面 Auth 与多租户重构指令 (Go)
*   **指令 2.1 (统一认证中心)**：引入 `Casbin` 实现 RBAC/ABAC 模型。定义 `Platform Admin`, `Org Admin`, `Workspace Admin`, `Developer`, `Auditor` 五种角色。
*   **指令 2.2 (多端注册登录)**：
    *   **个人版**：实现 GitHub/Google OAuth2 登录（使用 `goth` 库）。
    *   **企业版**：实现基于 OIDC/SAML 的 SSO 对接（如 Okta, 飞书, 钉钉）。
*   **指令 2.3 (Session 与 JWT)**：签发包含 `org_id`, `workspace_id`, `role` 的 JWT，所有控制面 API 必须通过中间件校验 JWT 及数据归属权。

---

## 三、 Phase 2 整改指令：双轨制 Key 与 FinOps 引擎 (Key & Billing)

**目标**：实现商业闭环，解决“Key 来源”与“成本管控”两大核心痛点。

### 3.1 双轨制 Key 路由引擎指令
*   **指令 3.1 (BYOK 安全托管)**：
    *   在 `virtual_keys` 表中增加 `provider_key_encrypted` 字段。
    *   控制面集成 AWS KMS / HashiCorp Vault 接口，实现真实 LLM Key 的**信封加密存储**。
*   **指令 3.2 (Unified 平台聚合)**：
    *   建立全局 `platform_llm_pool`，由平台统一维护各大模型的 Master Key。
*   **指令 3.3 (动态路由解析)**：
    *   Rust 数据面在接收到请求时，通过 gRPC 调用 Go 控制面的 `/internal/resolve-key` 接口。
    *   Go 控制面根据 Virtual Key 的归属（BYOK 或 Unified），返回解密后的真实 Key 或路由策略，**确保真实 Key 永远不暴露给前端和 Rust 数据面的内存持久层**。

### 3.2 微美元 FinOps 计费引擎指令
*   **指令 4.1 (钱包与账单系统)**：
    *   在 PG 中建立 `wallets`, `transactions`, `invoices` 表。
    *   集成 **Stripe API**，实现 SaaS 版的信用卡绑定、充值与自动扣款。
*   **指令 4.2 (流式实时熔断联动)**：
    *   **核心逻辑**：Rust 数据面在 SSE 流式接收 Token 时，每 10 个 Token 向 Redis 发起一次 Lua 脚本调用：`DECRBY workspace:{id}:budget {cost}`。
    *   如果 Redis 返回 `budget_exceeded`，Rust 端**立即切断 SSE 流**，向前端发送 `[VERIDACTUS:BUDGET_EXCEEDED]` 事件，并记录 Trace 状态为 `BLOCKED`。

---

## 四、 Phase 3 整改指令：前端双引擎与极致体感重塑 (Frontend Dual-Engine)

**目标**：彻底重构 `veridactus-ui/`，从“工程师后台”跃升为“消费级体验的杀手级应用”。
**技术栈强制要求**：Next.js 14 (App Router) + TailwindCSS + shadcn/ui + Framer Motion + Vercel AI SDK。

### 4.1 沉浸式 Onboarding 与双引擎入口
*   **指令 5.1 (双轨制引导页)**：注册后首次登录，强制进入 Onboarding 流程。提供两个极具视觉冲击力的卡片：“我有 API Key (安全托管)” vs “我需要 API Key (平台聚合)”。配合 Lottie 动效（如保险箱上锁、全球节点点亮）。
*   **指令 5.2 (双引擎侧边栏)**：主界面左侧导航分为两大引擎：**"Chat 沙箱"** 与 **"Developer Hub"**，以及底部的 **"Holo-Trace Vault"**。

### 4.2 引擎 A：VERIDACTUS Chat (安全沙箱) 指令
*   **指令 6.1 (极简对话与模型路由)**：实现类似 Kimi/DeepSeek 的极简对话流。顶部悬浮 Model Selector，支持一键切换 DeepSeek/GPT-4o。
*   **指令 6.2 (🛡️ 动态安全盾牌 - 核心体感)**：
    *   输入框左侧实现一个动态盾牌组件。
    *   前端通过 WebSocket 或防抖 API 实时检测输入。当识别到 PII（如身份证、银行卡）时，**盾牌由绿变黄并产生呼吸灯效**，下方浮现微提示：“⚠️ 已识别 PII，发送时将自动掩码”。
*   **指令 6.3 (⚔️ A/B 对比模式)**：开启后，界面一分为二，同一个 Prompt 并发请求两个模型，使用 Vercel AI SDK 实现**像素级同步的流式打字机效果**。

### 4.3 引擎 B：Developer Hub (全息调试台) 指令
*   **指令 7.1 (Live Playground)**：三栏布局。左侧 Prompt，中间流式输出，**右侧为“X光面板”**。
*   **指令 7.2 (X光透视)**：右侧面板实时展示 Raw Request、Sanitized Request（差异部分红色高亮）、Token 消耗速率（实时跳动数字）、当前预算剩余。

### 4.4 Holo-Trace Vault (全息证据金库) 指令 —— 🌟 降维打击区
*   **指令 8.1 (列表页看板)**：表格列必须包含：💰 精确成本、🛡️ 安全状态徽章（绿/黄/红）、🏷️ 归属标签。
*   **指令 8.2 (上帝视角分屏)**：详情页左右 50/50 分屏展示 Raw 和 Sanitized 文本。**必须实现滚动条百分比同步联动**。被掩码的文本显示为 `[REDACTED]` 并带有删除线与红色微光。
*   **指令 8.3 (🔗 密码学自证 - 核心震撼)**：
    *   页面底部展示“L0 审计证书”。
    *   点击“🔍 验证 (Verify)”按钮，前端调用原生 **Web Crypto API** 计算 JCS 字符串的 SHA-256 哈希，与后端签名比对。
    *   **动效要求**：比对成功时，使用 Framer Motion 触发**绿色粒子爆发全屏动效**，弹出“✅ 密码学验证通过：此记录未被篡改”。

---

## 五、 Phase 4 整改指令：企业级管控与合规指挥 (Enterprise & Compliance)

**目标**：为 CISO 和 CIO 提供“上帝视角”的管控武器，支撑高客单价的企业版销售。

### 5.1 Pipeline Studio (可视化治理编排) 升级
*   **指令 9.1**：将现有的 ReactFlow 画布升级为**暗色系科技风**。
*   **指令 9.2**：左侧增加“插件市场”抽屉，支持拖拽 `BudgetGuard`, `PiiDetector` 等节点。右侧增加“属性面板”。
*   **指令 9.3**：实现“一键模板”功能（如“OWASP Top 10 防护模板”），点击后自动在画布生成节点并连线。

### 5.2 审计指挥舱 (Auditor Command Center)
*   **指令 10.1 (全局风险大盘)**：为 Auditor 角色提供专属视图。使用 ECharts/Recharts 展示“今日拦截次数”、“PII 泄露尝试热力图”（按 Workspace 聚合）。
*   **指令 10.2 (一键合规报告)**：实现异步任务，打包指定时间段的 Merkle Root (L2A) 和 ZK 证明 (L2B)，生成带有数字签名的 PDF 报告，并提供离线验证 Python 脚本下载。

### 5.3 白标定制 (White-label)
*   **指令 11.1**：在 Org Admin 设置中增加“品牌定制”模块，支持上传 Logo、设置 Primary Color。
*   **指令 11.2**：前端通过 CSS Variables 动态注入主题色，实现 Chat UI 的完全企业品牌化。

---

## 六、 给 AI 代码生成器与研发团队的执行规范

为了确保整改的高质量落地，所有代码生成与提交必须遵循以下规范：

1.  **接口契约优先 (API-First)**：
    *   所有控制面 API 必须先定义 OpenAPI 3.0 (Swagger) 规范，存放在 `docs/api/` 目录。
    *   Rust 数据面与 Go 控制面之间的内部通信，必须使用 **gRPC + Protobuf**，定义在 `proto/` 目录。
2.  **密码学严谨性**：
    *   Rust 端的 JCS 规范化必须严格遵循 **RFC 8785** 标准。
    *   所有涉及 Key 存储的代码，必须通过 KMS 接口，**严禁在代码或配置文件中硬编码或明文存储真实 LLM Key**。
3.  **前端体验红线**：
    *   **零抖动**：SSE 流式输出必须使用 `requestAnimationFrame` 优化，禁止使用会导致重排的 CSS 属性。
    *   **暗黑模式**：所有组件必须原生支持 Dark/Light 主题切换，默认采用深邃的暗黑科技风（背景色 `#0B0F19`）。
4.  **测试覆盖率**：
    *   Rust 数据面的 DFA 匹配与预算熔断逻辑，必须达到 **90% 以上**的单元测试覆盖率，并使用 `cargo fuzz` 进行模糊测试。
    *   前端核心交互（如分屏同步、密码学验证）必须编写 Cypress/Playwright E2E 测试。

---

## 七、 架构师结语

当前的 `veridactus/veridactus` 仓库拥有令人惊叹的底层引擎，但它就像一台**没有方向盘和挡风玻璃的 F1 赛车发动机**。

这份整改指令白皮书，就是为这台发动机装上**防滚架（多租户隔离）**、**方向盘（双引擎前端）**、**仪表盘（FinOps 与全息金库）** 和**装甲（密码学自证）** 的施工图纸。

请研发团队严格按照上述 Phase 1 到 Phase 4 的顺序，以“周”为单位进行 Sprint 迭代。我们要让每一个接触到 VERIDACTUS 的用户，无论是极客还是 CISO，都能在**前 5 分钟内**，被我们“肉眼可见的安全感”和“数学级的信任感”彻底征服。

**开始重构，定义 AI 时代的信任标准！**