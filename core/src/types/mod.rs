//! # VERIDACTUS 核心数据类型
//!
//! 遵循 VERIDACTUS Protocol Specification v0.2.1 §3.0 Data Model & Trace Schema。
//! 所有类型与 `trace-schema.json` 严格对齐。

pub mod trace;
pub mod journal;
pub mod proof;
pub mod constraints;
pub mod conflicts;
pub mod error;

pub use trace::*;
pub use journal::*;
pub use proof::*;
pub use constraints::*;
pub use error::*;

use serde::{Deserialize, Serialize};

/// 版本范围，用于插件版本协商
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRange {
    /// 最小支持版本
    pub min: String,
    /// 最大支持版本
    pub max: String,
}

impl VersionRange {
    /// 检查给定版本是否在此范围内
    pub fn contains(&self, version: &str) -> bool {
        version >= self.min.as_str() && version <= self.max.as_str()
    }
}

/// 执行动作枚举 - 插件决策结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    /// 允许继续执行
    #[serde(rename = "continue")]
    Continue,
    /// 阻断执行
    #[serde(rename = "block")]
    Block,
    /// 降级执行（切换到更安全的模型/配置）
    #[serde(rename = "degrade")]
    Degrade,
    /// 标记但允许继续
    #[serde(rename = "flag")]
    Flag,
}

/// 降级动作类型（§5.3.1）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DegradeAction {
    /// 切换到备用模型
    #[serde(rename = "SWITCH_MODEL")]
    SwitchModel,
    /// 降低最大输出 token 数
    #[serde(rename = "REDUCE_MAX_TOKENS")]
    ReduceMaxTokens,
    /// 跳过非核心治理插件
    #[serde(rename = "SKIP_OPTIONAL_PLUGIN")]
    SkipOptionalPlugin,
    /// 降低采样质量
    #[serde(rename = "REDUCE_SAMPLING_QUALITY")]
    ReduceSamplingQuality,
    /// 回退到缓存响应
    #[serde(rename = "FALLBACK_CACHED")]
    FallbackCached,
}

/// 安全触发类型（§5.6）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SafetyTrigger {
    /// G1: 输入过滤器
    #[serde(rename = "G1_input_filter")]
    G1InputFilter,
    /// G2: 输出过滤器
    #[serde(rename = "G2_output_filter")]
    G2OutputFilter,
    /// G3: 语义守卫
    #[serde(rename = "G3_semantic_guard")]
    G3SemanticGuard,
    /// G4: 红队防御
    #[serde(rename = "G4_red_team")]
    G4RedTeam,
    /// 指令层次违反
    #[serde(rename = "instruction_hierarchy_violation")]
    InstructionHierarchyViolation,
    /// 失败事件
    #[serde(rename = "on_failure")]
    OnFailure,
    /// 主动预防
    #[serde(rename = "active_prevention")]
    ActivePrevention,
    /// 流结束
    #[serde(rename = "post_stream")]
    PostStream,
    /// 降级事件
    #[serde(rename = "on_degradation")]
    OnDegradation,
}

/// 严重级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "critical")]
    Critical,
}

/// 安全动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SafetyAction {
    #[serde(rename = "blocked")]
    Blocked,
    #[serde(rename = "flagged")]
    Flagged,
    #[serde(rename = "rewritten")]
    Rewritten,
    #[serde(rename = "logged")]
    Logged,
    #[serde(rename = "degraded")]
    Degraded,
}

/// OWASP ASI 风险标识符
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OwaspAsiRisk {
    #[serde(rename = "ASI01")]
    AgentGoalHijack,
    #[serde(rename = "ASI02")]
    ExcessiveAgency,
    #[serde(rename = "ASI03")]
    CrossPluginRequestForgery,
    #[serde(rename = "ASI04")]
    UnboundedActionLoops,
    #[serde(rename = "ASI05")]
    MultiAgentCollusion,
    #[serde(rename = "ASI06")]
    RogueAgents,
    #[serde(rename = "ASI07")]
    SensitiveDataExfiltration,
    #[serde(rename = "ASI08")]
    ToolOutputPoisoning,
    #[serde(rename = "ASI09")]
    UnboundedResourceConsumption,
    #[serde(rename = "ASI10")]
    AgentImpersonation,
}

/// 安全性事件（§3.0 observations.safety_events）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyEvent {
    /// 触发类型
    pub trigger_type: SafetyTrigger,
    /// 严重级别
    pub severity: Severity,
    /// 采取的动作
    pub action_taken: SafetyAction,
    /// 触发内容哈希
    pub content_hash: String,
    /// 关联的 OWASP ASI 风险 ID
    pub asi_risk_id: Option<OwaspAsiRisk>,
    /// 事件时间戳
    pub timestamp: String,
}

/// 红队事件（§3.0 observations.red_team_events）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamEvent {
    /// 攻击向量
    pub attack_vector: String,
    /// 置信度
    pub confidence: f64,
    /// 采取的动作
    pub action_taken: String,
    /// 攻击基准标识符
    pub attack_benchmark: Option<String>,
    /// 关联的 OWASP ASI 风险 ID
    pub asi_risk_id: Option<OwaspAsiRisk>,
    /// 事件时间戳
    pub timestamp: String,
}
