//! # Constraints 约束数据类型
//!
//! 严格遵循 VERIDACTUS v0.2.1 §5.0 Governance Policy & Constraints。
//! 约束是声明式规则，定义了单次推理执行的边界。

use serde::{Deserialize, Serialize};

use crate::types::error::VeridactusErrorCode;

/// 约束冲突类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictType {
    /// 硬冲突 - 两个约束不能同时使用
    HardConflict,
    /// 条件冲突 - 在特定条件下冲突
    ConditionalConflict,
}

impl std::fmt::Display for ConflictType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictType::HardConflict => write!(f, "HARD_CONFLICT"),
            ConflictType::ConditionalConflict => write!(f, "CONDITIONAL_CONFLICT"),
        }
    }
}

/// 单个约束冲突记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintConflict {
    /// 冲突类型
    pub conflict_type: ConflictType,
    /// 约束A
    pub constraint_a: String,
    /// 约束A的值
    pub value_a: String,
    /// 约束B
    pub constraint_b: String,
    /// 约束B的值
    pub value_b: String,
    /// 冲突原因
    pub reason: String,
    /// 冲突路径（用于错误报告）
    pub conflict_path: String,
}

/// 约束冲突检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintConflictResult {
    /// 是否有冲突
    pub has_conflicts: bool,
    /// 冲突列表
    pub conflicts: Vec<ConstraintConflict>,
    /// 条件冲突的解决建议
    pub recommendations: Vec<String>,
}

impl Default for ConstraintConflictResult {
    fn default() -> Self {
        Self {
            has_conflicts: false,
            conflicts: Vec::new(),
            recommendations: Vec::new(),
        }
    }
}

// ==================== Active Prevention 主动预防（§5.3.2）====================

/// 主动预防动作类型（§5.3.2 Active Prevention Action Enum）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PreventionAction {
    /// 阻止特定 token 生成，强制使用替代 token
    #[serde(rename = "block_token")]
    BlockToken,
    /// 将危险 token 替换为安全等价物
    #[serde(rename = "rewrite_token")]
    RewriteToken,
    /// 检测到禁止模式时终止生成
    #[serde(rename = "truncate_sequence")]
    TruncateSequence,
    /// 重写整个响应为安全内容
    #[serde(rename = "rewrite_response")]
    RewriteResponse,
}

impl std::fmt::Display for PreventionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreventionAction::BlockToken => write!(f, "block_token"),
            PreventionAction::RewriteToken => write!(f, "rewrite_token"),
            PreventionAction::TruncateSequence => write!(f, "truncate_sequence"),
            PreventionAction::RewriteResponse => write!(f, "rewrite_response"),
        }
    }
}

/// 被阻止的模式配置（§5.3.2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreventedPattern {
    /// 模式名称/标识符
    pub name: String,
    /// 正则表达式模式
    pub pattern: String,
    /// 匹配时采取的动作
    pub action: PreventionAction,
    /// 动作参数（如替换文本）
    pub action_params: Option<serde_json::Value>,
    /// 严重级别
    pub severity: String,
    /// 是否启用
    pub enabled: bool,
}

/// 主动预防配置（§5.3.2）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivePrevention {
    /// 是否启用约束解码
    pub constrained_decoding: Option<bool>,
    /// 被阻止的模式列表（增强版）
    pub prevented_patterns: Option<Vec<PreventedPattern>>,
    /// 阻止事件的采样率（0.0-1.0，用于性能优化）
    pub sampling_rate: Option<f64>,
    /// 是否记录被阻止的 token 详情
    pub log_blocked_tokens: Option<bool>,
    /// 是否在响应头中报告阻止事件
    pub report_in_header: Option<bool>,
    /// 自定义阻止词表路径
    pub custom_vocabulary_path: Option<String>,
    /// 允许的最大阻止次数（防止过度阻止）
    pub max_block_count: Option<u32>,
}

impl ActivePrevention {
    /// 检查是否启用主动预防
    pub fn is_enabled(&self) -> bool {
        self.constrained_decoding.unwrap_or(false) || 
        self.prevented_patterns.as_ref().map_or(false, |p| !p.is_empty())
    }

    /// 获取所有启用的模式
    pub fn get_enabled_patterns(&self) -> Vec<&PreventedPattern> {
        self.prevented_patterns
            .as_ref()
            .map_or(Vec::new(), |patterns| {
                patterns.iter().filter(|p| p.enabled).collect()
            })
    }

    /// 获取指定严重级别的模式
    pub fn get_patterns_by_severity(&self, severity: &str) -> Vec<&PreventedPattern> {
        self.get_enabled_patterns()
            .into_iter()
            .filter(|p| p.severity.eq_ignore_ascii_case(severity))
            .collect()
    }
}

// ==================== Adaptive Constraints 自适应约束（§5.9）====================

/// 自适应策略状态（§5.9.2）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AdaptiveState {
    /// 软告警 - 仅记录，不阻止
    #[serde(rename = "soft_alert")]
    SoftAlert,
    /// 降级 - 降低质量或切换模型
    #[serde(rename = "degrade")]
    Degrade,
    /// 硬停止 - 立即终止执行
    #[serde(rename = "hard_stop")]
    HardStop,
}

impl std::fmt::Display for AdaptiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdaptiveState::SoftAlert => write!(f, "soft_alert"),
            AdaptiveState::Degrade => write!(f, "degrade"),
            AdaptiveState::HardStop => write!(f, "hard_stop"),
        }
    }
}

/// 自适应阈值配置（§5.9.1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveThreshold {
    /// 从 soft_alert 升级到 degrade 的阈值
    pub soft_to_degrade: f64,
    /// 从 degrade 升级到 hard_stop 的阈值
    pub degrade_to_hard: f64,
    /// 从 degrade 降级回 soft_alert 的阈值（滞后阈值）
    pub degrade_to_soft: f64,
    /// 从 hard_stop 降级回 soft_alert 的阈值（滞后阈值）
    pub hard_to_soft: f64,
}

impl Default for AdaptiveThreshold {
    fn default() -> Self {
        Self {
            soft_to_degrade: 0.7,    // 70% 风险触发降级
            degrade_to_hard: 0.9,    // 90% 风险触发硬停止
            degrade_to_soft: 0.5,    // 50% 风险降级回软告警
            hard_to_soft: 0.3,       // 30% 风险从硬停止恢复
        }
    }
}

/// 自适应约束配置（§5.9）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdaptiveConstraint {
    /// 是否启用自适应约束
    pub enabled: Option<bool>,
    /// 当前状态
    pub current_state: Option<AdaptiveState>,
    /// 阈值配置
    pub thresholds: Option<AdaptiveThreshold>,
    /// 风险评分方法
    pub scoring_method: Option<String>,
    /// 考虑的风险因素
    pub risk_factors: Option<Vec<String>>,
    /// 是否启用自动恢复
    pub auto_recovery: Option<bool>,
    /// 恢复冷却时间（秒）
    pub recovery_cooldown_seconds: Option<u64>,
    /// 最大连续硬停止次数
    pub max_hard_stop_count: Option<u32>,
    /// 降级时的具体动作
    pub degrade_actions: Option<Vec<String>>,
}

impl AdaptiveConstraint {
    /// 根据风险分数计算下一个状态
    pub fn compute_next_state(&self, current_risk_score: f64) -> AdaptiveState {
        let thresholds = self.thresholds.clone().unwrap_or_default();
        let current_state = self.current_state.clone().unwrap_or(AdaptiveState::SoftAlert);

        match current_state {
            AdaptiveState::SoftAlert => {
                if current_risk_score >= thresholds.soft_to_degrade {
                    AdaptiveState::Degrade
                } else {
                    AdaptiveState::SoftAlert
                }
            }
            AdaptiveState::Degrade => {
                if current_risk_score >= thresholds.degrade_to_hard {
                    AdaptiveState::HardStop
                } else if current_risk_score <= thresholds.degrade_to_soft {
                    AdaptiveState::SoftAlert
                } else {
                    AdaptiveState::Degrade
                }
            }
            AdaptiveState::HardStop => {
                if self.auto_recovery.unwrap_or(false) && 
                   current_risk_score <= thresholds.hard_to_soft {
                    AdaptiveState::SoftAlert
                } else {
                    AdaptiveState::HardStop
                }
            }
        }
    }

    /// 检查是否应该阻止请求
    pub fn should_block(&self, current_risk_score: f64) -> bool {
        let next_state = self.compute_next_state(current_risk_score);
        next_state == AdaptiveState::HardStop
    }

    /// 检查是否应该降级
    pub fn should_degrade(&self, current_risk_score: f64) -> bool {
        let next_state = self.compute_next_state(current_risk_score);
        next_state == AdaptiveState::Degrade
    }
}

// ==================== 降级动作类型（§5.3.1）====================

/// 降级动作类型（§5.3.1 Degrade Action Enum）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DegradeActionType {
    /// 切换到备用模型
    #[serde(rename = "switch_model")]
    SwitchModel,
    /// 降低最大输出 token 数
    #[serde(rename = "reduce_max_tokens")]
    ReduceMaxTokens,
    /// 跳过可选插件
    #[serde(rename = "skip_optional_plugin")]
    SkipOptionalPlugin,
    /// 降低采样质量以加速
    #[serde(rename = "reduce_sampling_quality")]
    ReduceSamplingQuality,
    /// 回退到缓存响应
    #[serde(rename = "fallback_cached")]
    FallbackCached,
    /// 降低温度参数
    #[serde(rename = "reduce_temperature")]
    ReduceTemperature,
}

impl std::fmt::Display for DegradeActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DegradeActionType::SwitchModel => write!(f, "switch_model"),
            DegradeActionType::ReduceMaxTokens => write!(f, "reduce_max_tokens"),
            DegradeActionType::SkipOptionalPlugin => write!(f, "skip_optional_plugin"),
            DegradeActionType::ReduceSamplingQuality => write!(f, "reduce_sampling_quality"),
            DegradeActionType::FallbackCached => write!(f, "fallback_cached"),
            DegradeActionType::ReduceTemperature => write!(f, "reduce_temperature"),
        }
    }
}

/// 降级动作配置（§5.3.1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradeAction {
    /// 动作类型
    pub action_type: DegradeActionType,
    /// 动作参数
    pub params: Option<serde_json::Value>,
    /// 优先级（数值越小优先级越高）
    pub priority: u32,
}

// ==================== 策略评估引擎 ====================

/// 风险因素贡献（用于自适应评分）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactorContribution {
    /// 风险因素名称
    pub factor: String,
    /// 贡献分数（0.0-1.0）
    pub score: f64,
    /// 权重（0.0-1.0）
    pub weight: f64,
    /// 是否超过阈值
    pub exceeded: bool,
}

/// 策略评估结果（§5.4）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    /// 决策结果
    pub decision: Option<String>,
    /// 通过的检查
    pub checks_passed: Option<Vec<String>>,
    /// 失败的检查
    pub checks_failed: Option<Vec<String>>,
    /// 协商后的能力
    pub negotiated_capabilities: Option<Vec<String>>,
    /// 降级动作
    pub degrade_action: Option<DegradeAction>,
    /// 意图解析轨迹
    pub intent_resolution: Option<Vec<IntentResolution>>,
    /// 升级轨迹
    pub escalation_trail: Option<Vec<EscalationStep>>,
    /// DSL 源哈希
    pub dsl_source_hash: Option<String>,
    /// 当前风险分数
    pub current_risk_score: Option<f64>,
    /// 风险因素贡献明细
    pub risk_factor_contributions: Option<Vec<RiskFactorContribution>>,
    /// 自适应状态
    pub adaptive_state: Option<AdaptiveState>,
    /// 主动预防事件计数
    pub prevention_events_count: Option<u32>,
}

impl Default for PolicyEvaluation {
    fn default() -> Self {
        Self {
            decision: Some("allow".to_string()),
            checks_passed: Some(Vec::new()),
            checks_failed: Some(Vec::new()),
            negotiated_capabilities: Some(Vec::new()),
            degrade_action: None,
            intent_resolution: None,
            escalation_trail: None,
            dsl_source_hash: None,
            current_risk_score: Some(0.0),
            risk_factor_contributions: Some(Vec::new()),
            adaptive_state: Some(AdaptiveState::SoftAlert),
            prevention_events_count: Some(0),
        }
    }
}

/// 意图解析记录（§5.2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResolution {
    /// 声明的意图
    pub intent: String,
    /// 解决为的具体执行动作
    pub resolved_to: String,
    /// 解决理由
    pub rationale: String,
    /// 时间戳
    pub timestamp: String,
}

/// 升级步骤（§5.9.2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationStep {
    /// 起始策略
    pub from_strategy: String,
    /// 目标策略
    pub to_strategy: String,
    /// 触发分数
    pub trigger_score: f64,
    /// 触发原因
    pub trigger_reason: String,
    /// 时间戳
    pub timestamp: String,
    /// 风险因素明细
    pub risk_factors: Option<Vec<RiskFactorContribution>>,
}

// ==================== 约束冲突检测（§5.5）====================

/// 检查约束冲突（§5.5 约束冲突矩阵）
///
/// 根据协议定义，以下冲突需要检测：
/// - privacy.level=hash_only + reproducibility.mode=strict → HARD
/// - privacy.level=hash_only + reproducibility.mode=bounded → CONDITIONAL
/// - privacy.level=masked + active_prevention.constrained_decoding → CONDITIONAL
/// - budget.strategy=degrade_model + VERIDACTUS-Action:replay → CONDITIONAL
/// - budget.strategy=awareness + privacy.level=hash_only → HARD
/// - guardrails.G4 + privacy.level=hash_only → CONDITIONAL
/// - compliance_profile=EU_AI_ACT_GPAI + privacy.level=raw → HARD
/// - active_prevention.constrained_decoding=true + reproducibility.mode=strict → CONDITIONAL
/// - privacy.level=tee_private + reproducibility.mode=strict → HARD
/// - budget.strategy=awareness + reproducibility.mode=strict → HARD
/// - guardrails.G4 + active_prevention.constrained_decoding=true → CONDITIONAL
/// - instruction_hierarchy=strict + P2 override attempting to override system → Auto-blocked
pub fn check_constraint_conflicts(
    privacy_level: &Option<PrivacyLevel>,
    reproducibility_mode: &Option<ReproducibilityMode>,
    budget_strategy: &Option<BudgetStrategy>,
    guardrails_active: &Option<Vec<String>>,
    active_prevention: &Option<ActivePrevention>,
    compliance_profile: &Option<String>,
    adaptive_constraint: &Option<AdaptiveConstraint>,
) -> ConstraintConflictResult {
    let mut result = ConstraintConflictResult::default();

    let privacy = privacy_level.as_ref();
    let reproducibility = reproducibility_mode.as_ref();
    let budget = budget_strategy.as_ref();
    let guardrails = guardrails_active.as_ref();
    let active_prev = active_prevention.as_ref();
    let compliance = compliance_profile.as_ref();
    let adaptive = adaptive_constraint.as_ref();

    // 1. privacy.level=hash_only + reproducibility.mode=strict → HARD
    if let (Some(PrivacyLevel::HashOnly), Some(ReproducibilityMode::Strict)) = (privacy, reproducibility) {
        result.conflicts.push(ConstraintConflict {
            conflict_type: ConflictType::HardConflict,
            constraint_a: "privacy.level".to_string(),
            value_a: "hash_only".to_string(),
            constraint_b: "reproducibility.mode".to_string(),
            value_b: "strict".to_string(),
            reason: "hash_only discards plaintext, strict needs full payload comparison".to_string(),
            conflict_path: "constraints_applied.privacy_level + constraints_applied.reproducibility_mode".to_string(),
        });
        result.recommendations.push("Either relax reproducibility.mode to 'bounded' or increase privacy.level to 'masked'".to_string());
    }

    // 2. privacy.level=hash_only + reproducibility.mode=bounded → CONDITIONAL
    if let (Some(PrivacyLevel::HashOnly), Some(ReproducibilityMode::Bounded)) = (privacy, reproducibility) {
        result.conflicts.push(ConstraintConflict {
            conflict_type: ConflictType::ConditionalConflict,
            constraint_a: "privacy.level".to_string(),
            value_a: "hash_only".to_string(),
            constraint_b: "reproducibility.mode".to_string(),
            value_b: "bounded".to_string(),
            reason: "Allowed only if focus_fields are provided that can be compared via hashes".to_string(),
            conflict_path: "constraints_applied.privacy_level + constraints_applied.reproducibility_mode".to_string(),
        });
        result.recommendations.push("Ensure focus_fields are specified for hash-based comparison".to_string());
    }

    // 3. privacy.level=hash_only + budget.strategy=awareness → HARD
    if let (Some(BudgetStrategy::Awareness), Some(PrivacyLevel::HashOnly)) = (budget, privacy) {
        result.conflicts.push(ConstraintConflict {
            conflict_type: ConflictType::HardConflict,
            constraint_a: "budget.strategy".to_string(),
            value_a: "awareness".to_string(),
            constraint_b: "privacy.level".to_string(),
            value_b: "hash_only".to_string(),
            reason: "awareness requires budget information in prompt, conflicting with hash_only".to_string(),
            conflict_path: "constraints_applied.budget_strategy + constraints_applied.privacy_level".to_string(),
        });
        result.recommendations.push("Choose either budget.strategy=hard_stop or privacy.level=masked".to_string());
    }

    // 4. guardrails.G4 + privacy.level=hash_only → CONDITIONAL
    if let (Some(g), Some(PrivacyLevel::HashOnly)) = (guardrails, privacy) {
        if g.iter().any(|r| r == "G4") {
            result.conflicts.push(ConstraintConflict {
                conflict_type: ConflictType::ConditionalConflict,
                constraint_a: "guardrails".to_string(),
                value_a: "G4".to_string(),
                constraint_b: "privacy.level".to_string(),
                value_b: "hash_only".to_string(),
                reason: "Red team probes may require content visibility; allowed only with explicit approval".to_string(),
                conflict_path: "constraints_applied.guardrails_active + constraints_applied.privacy_level".to_string(),
            });
            result.recommendations.push("Requires explicit approval for G4 + hash_only combination".to_string());
        }
    }

    // 5. compliance_profile=EU_AI_ACT + privacy.level=raw → HARD
    if let (Some(c), Some(PrivacyLevel::Raw)) = (compliance, privacy) {
        if c.contains("EU_AI_ACT") {
            result.conflicts.push(ConstraintConflict {
                conflict_type: ConflictType::HardConflict,
                constraint_a: "compliance_profile".to_string(),
                value_a: c.clone(),
                constraint_b: "privacy.level".to_string(),
                value_b: "raw".to_string(),
                reason: "EU AI Act requires data minimization; raw PII storage conflicts".to_string(),
                conflict_path: "constraints_applied.compliance_profile + constraints_applied.privacy_level".to_string(),
            });
            result.recommendations.push("Set privacy.level to 'masked' for EU AI Act compliance".to_string());
        }
    }

    // 6. active_prevention.constrained_decoding=true + reproducibility.mode=strict → CONDITIONAL
    if let (Some(ap), Some(ReproducibilityMode::Strict)) = (active_prev, reproducibility) {
        if ap.constrained_decoding.unwrap_or(false) || ap.is_enabled() {
            result.conflicts.push(ConstraintConflict {
                conflict_type: ConflictType::ConditionalConflict,
                constraint_a: "active_prevention".to_string(),
                value_a: "enabled".to_string(),
                constraint_b: "reproducibility.mode".to_string(),
                value_b: "strict".to_string(),
                reason: "Acceptable if prevented patterns are documented and consistent across runs".to_string(),
                conflict_path: "constraints_applied.active_prevention + constraints_applied.reproducibility_mode".to_string(),
            });
            result.recommendations.push("Document all prevented patterns to ensure reproducibility".to_string());
        }
    }

    // 7. privacy.level=tee_private + reproducibility.mode=strict → HARD
    if let (Some(PrivacyLevel::TeePrivate), Some(ReproducibilityMode::Strict)) = (privacy, reproducibility) {
        result.conflicts.push(ConstraintConflict {
            conflict_type: ConflictType::HardConflict,
            constraint_a: "privacy.level".to_string(),
            value_a: "tee_private".to_string(),
            constraint_b: "reproducibility.mode".to_string(),
            value_b: "strict".to_string(),
            reason: "Strict replay requires comparing full payloads, which conflicts with TEE's external storage of hashes only".to_string(),
            conflict_path: "constraints_applied.privacy_level + constraints_applied.reproducibility_mode".to_string(),
        });
        result.recommendations.push("Use reproducibility.mode=bounded with TEE private mode".to_string());
    }

    // 8. budget.strategy=awareness + reproducibility.mode=strict → HARD
    if let (Some(BudgetStrategy::Awareness), Some(ReproducibilityMode::Strict)) = (budget, reproducibility) {
        result.conflicts.push(ConstraintConflict {
            conflict_type: ConflictType::HardConflict,
            constraint_a: "budget.strategy".to_string(),
            value_a: "awareness".to_string(),
            constraint_b: "reproducibility.mode".to_string(),
            value_b: "strict".to_string(),
            reason: "Injecting budget prompt changes the input, breaking strict replay guarantees".to_string(),
            conflict_path: "constraints_applied.budget_strategy + constraints_applied.reproducibility_mode".to_string(),
        });
        result.recommendations.push("Choose either budget.strategy=hard_stop or reproducibility.mode=bounded".to_string());
    }

    // 9. guardrails.G4 + active_prevention.constrained_decoding=true → CONDITIONAL
    if let (Some(g), Some(ap)) = (guardrails, active_prev) {
        if g.iter().any(|r| r == "G4") && (ap.constrained_decoding.unwrap_or(false) || ap.is_enabled()) {
            result.conflicts.push(ConstraintConflict {
                conflict_type: ConflictType::ConditionalConflict,
                constraint_a: "guardrails".to_string(),
                value_a: "G4".to_string(),
                constraint_b: "active_prevention".to_string(),
                value_b: "enabled".to_string(),
                reason: "G4 may dynamically modify policies; allowed only if the defense agent is explicitly authorized and changes are logged".to_string(),
                conflict_path: "constraints_applied.guardrails_active + constraints_applied.active_prevention".to_string(),
            });
            result.recommendations.push("Ensure G4 defense agent is authorized and changes are logged".to_string());
        }
    }

    // 10. privacy.level=masked + active_prevention.constrained_decoding=true → CONDITIONAL (§5.5.1 row 3)
    if let (Some(PrivacyLevel::Masked), Some(ap)) = (privacy, active_prev) {
        if ap.constrained_decoding.unwrap_or(false) || ap.is_enabled() {
            result.conflicts.push(ConstraintConflict {
                conflict_type: ConflictType::ConditionalConflict,
                constraint_a: "privacy.level".to_string(),
                value_a: "masked".to_string(),
                constraint_b: "active_prevention".to_string(),
                value_b: "enabled".to_string(),
                reason: "Allowed; constrained decoding prevents generation, masking protects stored traces. They are complementary.".to_string(),
                conflict_path: "constraints_applied.privacy_level + constraints_applied.active_prevention".to_string(),
            });
            // 这是一个允许的组合，给出积极建议
            result.recommendations.push("Good combination: active_prevention prevents harmful output, masking protects stored data".to_string());
        }
    }

    // 11. budget.strategy=degrade_model + reproducibility.mode=strict → CONDITIONAL
    if let (Some(BudgetStrategy::DegradeModel), Some(ReproducibilityMode::Strict)) = (budget, reproducibility) {
        result.conflicts.push(ConstraintConflict {
            conflict_type: ConflictType::ConditionalConflict,
            constraint_a: "budget.strategy".to_string(),
            value_a: "degrade_model".to_string(),
            constraint_b: "reproducibility.mode".to_string(),
            value_b: "strict".to_string(),
            reason: "Degradation may change model; replay integrity might be affected, MUST be documented".to_string(),
            conflict_path: "constraints_applied.budget_strategy + constraints_applied.reproducibility_mode".to_string(),
        });
        result.recommendations.push("Document degradation scenarios that may affect reproducibility".to_string());
    }

    // 12. adaptive + reproducibility.mode=strict → CONDITIONAL
    if let (Some(true), Some(ReproducibilityMode::Strict)) = (adaptive.map(|a| a.enabled.unwrap_or(false)), reproducibility) {
        result.conflicts.push(ConstraintConflict {
            conflict_type: ConflictType::ConditionalConflict,
            constraint_a: "adaptive.enabled".to_string(),
            value_a: "true".to_string(),
            constraint_b: "reproducibility.mode".to_string(),
            value_b: "strict".to_string(),
            reason: "Adaptive constraints may trigger degradation affecting reproducibility; requires careful configuration".to_string(),
            conflict_path: "constraints_applied.adaptive + constraints_applied.reproducibility_mode".to_string(),
        });
        result.recommendations.push("Configure adaptive thresholds carefully when using strict reproducibility".to_string());
    }

    result.has_conflicts = !result.conflicts.is_empty();
    result
}

/// 检查约束冲突并返回错误（如果存在硬冲突）
pub fn validate_constraints(
    privacy_level: &Option<PrivacyLevel>,
    reproducibility_mode: &Option<ReproducibilityMode>,
    budget_strategy: &Option<BudgetStrategy>,
    guardrails_active: &Option<Vec<String>>,
    active_prevention: &Option<ActivePrevention>,
    compliance_profile: &Option<String>,
    adaptive_constraint: &Option<AdaptiveConstraint>,
) -> Result<ConstraintConflictResult, (VeridactusErrorCode, String)> {
    let result = check_constraint_conflicts(
        privacy_level,
        reproducibility_mode,
        budget_strategy,
        guardrails_active,
        active_prevention,
        compliance_profile,
        adaptive_constraint,
    );

    if !result.has_conflicts {
        return Ok(result);
    }

    let hard_conflicts: Vec<_> = result.conflicts.iter()
        .filter(|c| c.conflict_type == ConflictType::HardConflict)
        .collect();

    if !hard_conflicts.is_empty() {
        let conflict_details: Vec<String> = hard_conflicts.iter()
            .map(|c| format!("{}={} conflicts with {}={}: {}",
                c.constraint_a, c.value_a, c.constraint_b, c.value_b, c.reason))
            .collect();

        return Err((
            VeridactusErrorCode::BadConstraintCombination,
            format!("Constraint conflict detected: {}", conflict_details.join("; ")),
        ));
    }

    Ok(result)
}

// ==================== 基础枚举类型 ====================

/// 隐私级别（§8.1）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PrivacyLevel {
    /// 完整明文存储
    #[serde(rename = "raw")]
    Raw,
    /// 脱敏后存储
    #[serde(rename = "masked")]
    Masked,
    /// 仅存储哈希
    #[serde(rename = "hash_only")]
    HashOnly,
    /// TEE 私有模式
    #[serde(rename = "tee_private")]
    TeePrivate,
}

/// 预算策略（§5.3）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetStrategy {
    /// 硬停止
    #[serde(rename = "hard_stop")]
    HardStop,
    /// 模型降级
    #[serde(rename = "degrade_model")]
    DegradeModel,
    /// 软告警
    #[serde(rename = "soft_alert")]
    SoftAlert,
    /// 自适应
    #[serde(rename = "adaptive")]
    Adaptive,
    /// 感知模式
    #[serde(rename = "awareness")]
    Awareness,
}

/// 可重现模式（§9.4）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReproducibilityMode {
    /// 无可重现要求
    #[serde(rename = "none")]
    None,
    /// 有界可重现
    #[serde(rename = "bounded")]
    Bounded,
    /// 严格可重现
    #[serde(rename = "strict")]
    Strict,
}

/// 守卫级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GuardrailLevel {
    #[serde(rename = "G1")]
    G1,
    #[serde(rename = "G2")]
    G2,
    #[serde(rename = "G3")]
    G3,
    #[serde(rename = "G4")]
    G4,
}

/// 指令层次模式（§5.7.2）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstructionHierarchyMode {
    /// 严格模式
    #[serde(rename = "strict")]
    Strict,
    /// 告警模式
    #[serde(rename = "warn")]
    Warn,
    /// 关闭
    #[serde(rename = "off")]
    Off,
    /// 验证模式
    #[serde(rename = "verified")]
    Verified,
}

/// 差分隐私预算（§8.6）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpBudget {
    /// 消耗的 epsilon
    pub epsilon_consumed: Option<f64>,
    /// 消耗的 delta
    pub delta_consumed: Option<f64>,
    /// 机制类型
    pub mechanism: Option<String>,
}

/// 应用的约束快照（§5.4 constraints_applied）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConstraintsApplied {
    /// 预算上限（美元）
    pub budget_limit_usd: Option<f64>,
    /// 实际预算消耗（美元）
    pub budget_actual_usd: Option<f64>,
    /// 预算策略
    pub budget_strategy: Option<BudgetStrategy>,
    /// 隐私级别
    pub privacy_level: Option<PrivacyLevel>,
    /// 隐私脱敏字段
    pub privacy_masked_fields: Option<Vec<String>>,
    /// 主动预防
    pub active_prevention: Option<ActivePrevention>,
    /// 自适应约束
    pub adaptive: Option<AdaptiveConstraint>,
    /// 可重现模式
    pub reproducibility_mode: Option<ReproducibilityMode>,
    /// 可重现种子
    pub reproducibility_seed: Option<u64>,
    /// 激活的守卫级别
    pub guardrails_active: Option<Vec<String>>,
    /// 守卫严格度
    pub guardrails_strictness: Option<String>,
    /// 指令层次模式
    pub instruction_hierarchy_mode: Option<InstructionHierarchyMode>,
    /// 策略评估结果
    pub policy_evaluation: Option<PolicyEvaluation>,
    /// 降级动作
    pub degrade_action: Option<DegradeAction>,
    /// 差分隐私预算
    pub dp_budget: Option<DpBudget>,
    /// 约束冲突结果
    pub conflict_result: Option<ConstraintConflictResult>,
}

/// 约束评估上下文（用于策略评估引擎）
#[derive(Debug, Clone)]
pub struct ConstraintEvaluationContext {
    /// 当前风险分数
    pub current_risk_score: f64,
    /// 风险因素明细
    pub risk_factors: Vec<RiskFactorContribution>,
    /// 执行阶段
    pub execution_phase: String,
    /// 已消耗预算（美元）
    pub budget_consumed_usd: f64,
    /// 当前 token 计数
    pub token_count: u32,
    /// 主动预防事件计数
    pub prevention_event_count: u32,
    /// 是否为流式响应
    pub is_streaming: bool,
    /// 时间戳
    pub timestamp: String,
}

/// 策略评估引擎（§5.4）
pub struct PolicyEvaluationEngine;

impl PolicyEvaluationEngine {
    /// 评估约束并生成策略决策
    pub fn evaluate(
        constraints: &ConstraintsApplied,
        context: &ConstraintEvaluationContext,
    ) -> PolicyEvaluation {
        let mut evaluation = PolicyEvaluation::default();
        
        // 设置风险分数
        evaluation.current_risk_score = Some(context.current_risk_score);
        evaluation.risk_factor_contributions = Some(context.risk_factors.clone());
        evaluation.prevention_events_count = Some(context.prevention_event_count);

        // 评估自适应约束
        if let Some(adaptive) = &constraints.adaptive {
            if adaptive.enabled.unwrap_or(false) {
                let next_state = adaptive.compute_next_state(context.current_risk_score);
                evaluation.adaptive_state = Some(next_state.clone());

                // 更新升级轨迹
                if let Some(current_state) = &adaptive.current_state {
                    if current_state != &next_state {
                        let step = EscalationStep {
                            from_strategy: current_state.to_string(),
                            to_strategy: next_state.to_string(),
                            trigger_score: context.current_risk_score,
                            trigger_reason: "Risk score exceeded threshold".to_string(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            risk_factors: Some(context.risk_factors.clone()),
                        };
                        evaluation.escalation_trail = Some(vec![step]);
                    }
                }

                // 设置决策
                match next_state {
                    AdaptiveState::HardStop => {
                        evaluation.decision = Some("block".to_string());
                        evaluation.checks_failed = Some(vec!["adaptive_hard_stop".to_string()]);
                    }
                    AdaptiveState::Degrade => {
                        evaluation.decision = Some("degrade".to_string());
                        evaluation.degrade_action = constraints.degrade_action.clone();
                        evaluation.checks_passed = Some(vec!["adaptive_degrade".to_string()]);
                    }
                    AdaptiveState::SoftAlert => {
                        evaluation.decision = Some("allow".to_string());
                        evaluation.checks_passed = Some(vec!["adaptive_allow".to_string()]);
                    }
                }
            }
        }

        evaluation
    }

    /// 计算综合风险分数
    pub fn compute_risk_score(
        risk_factors: &[RiskFactorContribution],
        weights: Option<&[f64]>,
    ) -> f64 {
        if risk_factors.is_empty() {
            return 0.0;
        }

        let default_weights: Vec<f64> = vec![1.0 / risk_factors.len() as f64; risk_factors.len()];
        let actual_weights = weights.unwrap_or(&default_weights);

        risk_factors.iter()
            .enumerate()
            .map(|(i, factor)| factor.score * actual_weights[i.min(actual_weights.len() - 1)])
            .sum()
    }
}