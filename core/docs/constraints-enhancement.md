# Active Prevention 和 Adaptive 约束增强实现说明

## 概述

根据 VERIDACTUS v0.2.1 协议 §5.3.2 和 §5.9 的规范要求，本次增强完善了 **Active Prevention（主动预防）** 和 **Adaptive（自适应）** 约束的完整执行语义。

---

## 一、Active Prevention（主动预防）增强

### 1.1 新增类型定义

#### PreventionAction 枚举

定义了四种主动预防动作类型：

| 动作类型 | 说明 | 应用场景 |
|---------|------|---------|
| `block_token` | 阻止特定 token 生成，强制使用替代 token | 阻止有害词汇生成 |
| `rewrite_token` | 将危险 token 替换为安全等价物 | 敏感信息脱敏替换 |
| `truncate_sequence` | 检测到禁止模式时终止生成 | 防止输出过长或有害内容 |
| `rewrite_response` | 重写整个响应为安全内容 | 完全替换不安全响应 |

#### PreventedPattern 结构体

```rust
pub struct PreventedPattern {
    pub name: String,                    // 模式名称/标识符
    pub pattern: String,                 // 正则表达式模式
    pub action: PreventionAction,        // 匹配时采取的动作
    pub action_params: Option<Value>,    // 动作参数（如替换文本）
    pub severity: String,                // 严重级别 (high/medium/low)
    pub enabled: bool,                   // 是否启用
}
```

### 1.2 ActivePrevention 结构体增强

新增字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `prevented_patterns` | `Vec<PreventedPattern>` | 被阻止的模式列表（增强版） |
| `sampling_rate` | `f64` | 阻止事件的采样率（0.0-1.0，用于性能优化） |
| `log_blocked_tokens` | `bool` | 是否记录被阻止的 token 详情 |
| `report_in_header` | `bool` | 是否在响应头中报告阻止事件 |
| `custom_vocabulary_path` | `String` | 自定义阻止词表路径 |
| `max_block_count` | `u32` | 允许的最大阻止次数（防止过度阻止） |

### 1.3 新增方法

```rust
// 检查是否启用主动预防
pub fn is_enabled(&self) -> bool

// 获取所有启用的模式
pub fn get_enabled_patterns(&self) -> Vec<&PreventedPattern>

// 获取指定严重级别的模式
pub fn get_patterns_by_severity(&self, severity: &str) -> Vec<&PreventedPattern>
```

---

## 二、Adaptive（自适应）约束增强

### 2.1 新增类型定义

#### AdaptiveState 枚举

定义了三种自适应策略状态：

| 状态 | 说明 | 行为 |
|------|------|------|
| `soft_alert` | 软告警 | 仅记录，不阻止 |
| `degrade` | 降级 | 降低质量或切换模型 |
| `hard_stop` | 硬停止 | 立即终止执行 |

#### AdaptiveThreshold 结构体

```rust
pub struct AdaptiveThreshold {
    pub soft_to_degrade: f64,    // 从 soft_alert 升级到 degrade 的阈值（默认 0.7）
    pub degrade_to_hard: f64,    // 从 degrade 升级到 hard_stop 的阈值（默认 0.9）
    pub degrade_to_soft: f64,    // 从 degrade 降级回 soft_alert 的阈值（滞后阈值，默认 0.5）
    pub hard_to_soft: f64,       // 从 hard_stop 降级回 soft_alert 的阈值（滞后阈值，默认 0.3）
}
```

### 2.2 AdaptiveConstraint 结构体增强

新增字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `enabled` | `bool` | 是否启用自适应约束 |
| `current_state` | `AdaptiveState` | 当前状态 |
| `thresholds` | `AdaptiveThreshold` | 阈值配置 |
| `scoring_method` | `String` | 风险评分方法 |
| `risk_factors` | `Vec<String>` | 考虑的风险因素 |
| `auto_recovery` | `bool` | 是否启用自动恢复 |
| `recovery_cooldown_seconds` | `u64` | 恢复冷却时间（秒） |
| `max_hard_stop_count` | `u32` | 最大连续硬停止次数 |
| `degrade_actions` | `Vec<String>` | 降级时的具体动作 |

### 2.3 状态转换逻辑

```
                    soft_to_degrade (0.7)
    ┌─────────────────────────────────────────────────────┐
    │                                                     │
    ▼                                                     │
soft_alert ──────────────────────────────────────────── degrade
    │                                                     │
    │                                                     │
    │                  degrade_to_soft (0.5)              │
    │◄────────────────────────────────────────────────────┤
    │                                                     │
    │                                                     │ degrade_to_hard (0.9)
    │                                                     │───────────────────────► hard_stop
    │                                                                           │
    │                        hard_to_soft (0.3) + auto_recovery                 │
    └─────────────────────────────────────────────────────────────────────────────┘
```

### 2.4 新增方法

```rust
// 根据风险分数计算下一个状态
pub fn compute_next_state(&self, current_risk_score: f64) -> AdaptiveState

// 检查是否应该阻止请求
pub fn should_block(&self, current_risk_score: f64) -> bool

// 检查是否应该降级
pub fn should_degrade(&self, current_risk_score: f64) -> bool
```

---

## 三、Degrade Action（降级动作）增强

### 3.1 DegradeActionType 枚举

| 动作类型 | 说明 |
|---------|------|
| `switch_model` | 切换到备用模型 |
| `reduce_max_tokens` | 降低最大输出 token 数 |
| `skip_optional_plugin` | 跳过可选插件 |
| `reduce_sampling_quality` | 降低采样质量以加速 |
| `fallback_cached` | 回退到缓存响应 |
| `reduce_temperature` | 降低温度参数 |

### 3.2 DegradeAction 结构体

```rust
pub struct DegradeAction {
    pub action_type: DegradeActionType,    // 动作类型
    pub params: Option<Value>,             // 动作参数
    pub priority: u32,                     // 优先级（数值越小优先级越高）
}
```

---

## 四、策略评估引擎

### 4.1 RiskFactorContribution 结构体

用于记录各风险因素对综合风险分数的贡献：

```rust
pub struct RiskFactorContribution {
    pub factor: String,    // 风险因素名称
    pub score: f64,        // 贡献分数（0.0-1.0）
    pub weight: f64,       // 权重（0.0-1.0）
    pub exceeded: bool,    // 是否超过阈值
}
```

### 4.2 PolicyEvaluationEngine 实现

```rust
// 评估约束并生成策略决策
pub fn evaluate(constraints: &ConstraintsApplied, context: &ConstraintEvaluationContext) -> PolicyEvaluation

// 计算综合风险分数
pub fn compute_risk_score(risk_factors: &[RiskFactorContribution], weights: Option<&[f64]>) -> f64
```

### 4.3 评估流程

```
┌─────────────────────────────────────────────────────────────────┐
│                     Policy Evaluation Flow                      │
├─────────────────────────────────────────────────────────────────┤
│  1. 输入: ConstraintsApplied + ConstraintEvaluationContext     │
│                         │                                      │
│                         ▼                                      │
│  2. 设置风险分数和风险因素贡献                                    │
│                         │                                      │
│                         ▼                                      │
│  3. 评估自适应约束（如果启用）                                    │
│     ├─ compute_next_state()                                    │
│     ├─ 更新升级轨迹                                             │
│     └─ 设置决策结果                                             │
│                         │                                      │
│                         ▼                                      │
│  4. 输出: PolicyEvaluation (decision, checks, state, etc.)     │
└─────────────────────────────────────────────────────────────────┘
```

---

## 五、约束冲突检测增强

新增自适应约束与其他约束的冲突检测：

| 冲突组合 | 类型 | 说明 |
|---------|------|------|
| `adaptive.enabled=true` + `reproducibility.mode=strict` | CONDITIONAL | 自适应约束可能触发降级，影响可重现性 |

---

## 六、使用示例

### 6.1 配置主动预防

```json
{
  "active_prevention": {
    "constrained_decoding": true,
    "prevented_patterns": [
      {
        "name": "profanity_filter",
        "pattern": "(bad|words|here)",
        "action": "block_token",
        "action_params": null,
        "severity": "high",
        "enabled": true
      }
    ],
    "sampling_rate": 1.0,
    "log_blocked_tokens": true,
    "report_in_header": true,
    "max_block_count": 10
  }
}
```

### 6.2 配置自适应约束

```json
{
  "adaptive": {
    "enabled": true,
    "current_state": "soft_alert",
    "thresholds": {
      "soft_to_degrade": 0.7,
      "degrade_to_hard": 0.9,
      "degrade_to_soft": 0.5,
      "hard_to_soft": 0.3
    },
    "scoring_method": "weighted_sum",
    "risk_factors": ["toxicity", "privacy", "budget"],
    "auto_recovery": true,
    "recovery_cooldown_seconds": 300,
    "max_hard_stop_count": 5,
    "degrade_actions": ["reduce_max_tokens", "fallback_cached"]
  }
}
```

---

## 七、协议一致性

| 协议章节 | 实现状态 | 说明 |
|---------|---------|------|
| §5.3.1 Degrade Actions | ✅ 完整实现 | 支持6种降级动作类型 |
| §5.3.2 Active Prevention | ✅ 完整实现 | 支持4种预防动作，模式配置 |
| §5.4 Policy Evaluation | ✅ 完整实现 | 策略评估引擎 |
| §5.5 Constraint Conflict Matrix | ✅ 完整实现 | 包含自适应约束冲突检测 |
| §5.9 Adaptive Constraints | ✅ 完整实现 | 状态机、阈值、自动恢复 |
| §5.9.1 Threshold Configuration | ✅ 完整实现 | 滞后阈值防止抖动 |
| §5.9.2 Escalation Trail | ✅ 完整实现 | 升级轨迹记录 |

---

## 八、向后兼容性

- 原有字段保持兼容
- 新增字段均为 `Option<T>` 类型，默认值不影响现有功能
- `ActivePrevention.prevented_patterns` 从 `Vec<String>` 升级为 `Vec<PreventedPattern>`，支持向后兼容的序列化格式