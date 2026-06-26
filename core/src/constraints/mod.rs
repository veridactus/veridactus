//! # 约束评估引擎
//!
//! 实现 VERIDACTUS v0.2.1 §5.0 完整约束评估流程：
//! - 静态约束检查（预算、隐私、指令层次）
//! - 动态约束评估（流式预算预占）
//! - 约束违反处理

use crate::types::constraints::{
    ActivePrevention, BudgetStrategy, ConstraintsApplied, InstructionHierarchyMode, PrivacyLevel,
};
use crate::types::trace::Trace;
use tracing::{info, warn};

/// 约束评估结果
#[derive(Debug, Clone)]
pub struct ConstraintEvaluationResult {
    pub allowed: bool,
    pub checks_passed: Vec<String>,
    pub checks_failed: Vec<String>,
    pub warnings: Vec<String>,
    pub degrade_action: Option<String>,
}

impl Default for ConstraintEvaluationResult {
    fn default() -> Self {
        Self {
            allowed: true,
            checks_passed: Vec::new(),
            checks_failed: Vec::new(),
            warnings: Vec::new(),
            degrade_action: None,
        }
    }
}

/// 约束评估器
///
/// 从 Trace 中提取所有约束应用快照，并对其执行标准检查。
/// 修复：原先 get_constraints() 始终返回 None，导致
/// check_instruction_hierarchy / check_guardrails / check_active_prevention
/// 三个方法永远无法获取约束数据。
pub struct ConstraintEvaluator {
    /// 完整的约束快照引用（修复：之前丢失了此引用）
    constraints: Option<ConstraintsApplied>,
    budget_limit: Option<f64>,
    budget_actual: Option<f64>,
    budget_strategy: Option<BudgetStrategy>,
    privacy_level: Option<PrivacyLevel>,
    guardrails_active: Option<Vec<String>>,
    instruction_hierarchy_mode: Option<InstructionHierarchyMode>,
    active_prevention: Option<ActivePrevention>,
}

impl ConstraintEvaluator {
    pub fn new(trace: &Trace) -> Self {
        let constraints = trace.constraints_applied.clone();
        let c = constraints.as_ref();
        Self {
            budget_limit: c.and_then(|c| c.budget_limit_usd),
            budget_actual: c.and_then(|c| c.budget_actual_usd),
            budget_strategy: c.and_then(|c| c.budget_strategy.clone()),
            privacy_level: c.and_then(|c| c.privacy_level.clone()),
            guardrails_active: c.and_then(|c| c.guardrails_active.clone()),
            instruction_hierarchy_mode: c.and_then(|c| c.instruction_hierarchy_mode.clone()),
            active_prevention: c.and_then(|c| c.active_prevention.clone()),
            constraints,
        }
    }

    /// 执行完整约束评估
    pub fn evaluate(&self, _trace: &Trace) -> ConstraintEvaluationResult {
        let mut result = ConstraintEvaluationResult::default();

        // 1. 预算约束检查
        self.check_budget_constraint(&mut result);

        // 2. 隐私约束检查
        self.check_privacy_constraint(&mut result);

        // 3. 指令层次检查
        self.check_instruction_hierarchy(&mut result);

        // 4. 守卫级别检查
        self.check_guardrails(&mut result);

        // 5. 主动预防检查
        self.check_active_prevention(&mut result);

        result
    }

    /// 预算约束检查（§5.3）
    fn check_budget_constraint(&self, result: &mut ConstraintEvaluationResult) {
        if let Some(limit) = self.budget_limit {
            if limit <= 0.0 {
                result.allowed = false;
                result.checks_failed.push("budget_limit_zero".to_string());
                warn!("Budget limit is zero or negative: {}", limit);
                return;
            }

            result.checks_passed.push("budget_limit_valid".to_string());
            info!("Budget check passed: $ {}", limit);

            if let Some(actual) = self.budget_actual {
                let usage_ratio = actual / limit;

                match self
                    .budget_strategy
                    .as_ref()
                    .unwrap_or(&BudgetStrategy::HardStop)
                {
                    BudgetStrategy::HardStop => {
                        if usage_ratio >= 1.0 {
                            result.allowed = false;
                            result
                                .checks_failed
                                .push("budget_exceeded_hard_stop".to_string());
                            result.degrade_action = Some("hard_stop".to_string());
                            warn!("Budget exceeded hard stop: {}/{}$", actual, limit);
                        }
                    }
                    BudgetStrategy::DegradeModel => {
                        if usage_ratio >= 0.8 {
                            result
                                .warnings
                                .push(format!("Budget usage reached {}%", usage_ratio * 100.0));
                            result.degrade_action = Some("degrade_to_gpt4o_mini".to_string());
                            info!(
                                "Budget usage triggers degrade: {}/{}$ ({}%)",
                                actual,
                                limit,
                                usage_ratio * 100.0
                            );
                        }
                    }
                    BudgetStrategy::SoftAlert => {
                        if usage_ratio >= 0.9 {
                            result
                                .warnings
                                .push(format!("Budget usage near limit: {}%", usage_ratio * 100.0));
                            info!(
                                "Budget usage soft alert: {}/{}$ ({}%)",
                                actual,
                                limit,
                                usage_ratio * 100.0
                            );
                        }
                    }
                    BudgetStrategy::Adaptive => {
                        if usage_ratio >= 0.7 {
                            result
                                .warnings
                                .push(format!("Budget usage high: {}%", usage_ratio * 100.0));
                            result.degrade_action = Some("adaptive_model_selection".to_string());
                        }
                    }
                    BudgetStrategy::Awareness => {
                        result
                            .warnings
                            .push(format!("Current budget usage: {}%", usage_ratio * 100.0));
                    }
                }
            }
        } else {
            result.checks_passed.push("budget_no_limit".to_string());
        }
    }

    /// 隐私约束检查（§8.1）
    fn check_privacy_constraint(&self, result: &mut ConstraintEvaluationResult) {
        match self.privacy_level {
            Some(PrivacyLevel::TeePrivate) => {
                result.checks_passed.push("privacy_tee_private".to_string());
            }
            Some(PrivacyLevel::Masked) => {
                result.checks_passed.push("privacy_masked".to_string());
            }
            Some(PrivacyLevel::HashOnly) => {
                result.checks_passed.push("privacy_hash_only".to_string());
            }
            Some(PrivacyLevel::Raw) | None => {
                result.checks_passed.push("privacy_raw".to_string());
            }
        }
    }

    /// 指令层次检查（§5.7.2）
    fn check_instruction_hierarchy(&self, result: &mut ConstraintEvaluationResult) {
        match &self.instruction_hierarchy_mode {
            Some(InstructionHierarchyMode::Strict) => {
                result
                    .checks_passed
                    .push("instruction_hierarchy_strict".to_string());
            }
            Some(InstructionHierarchyMode::Warn) => {
                result
                    .checks_passed
                    .push("instruction_hierarchy_warn".to_string());
                result
                    .warnings
                    .push("Instruction hierarchy set to warn".to_string());
            }
            Some(InstructionHierarchyMode::Verified) => {
                result
                    .checks_passed
                    .push("instruction_hierarchy_verified".to_string());
            }
            Some(InstructionHierarchyMode::Off) | None => {
                result
                    .checks_passed
                    .push("instruction_hierarchy_default".to_string());
            }
        }
    }

    /// 守卫级别检查（§7.0）
    fn check_guardrails(&self, result: &mut ConstraintEvaluationResult) {
        if let Some(active) = &self.guardrails_active {
            if !active.is_empty() {
                for guardrail in active {
                    result
                        .checks_passed
                        .push(format!("guardrail_{}", guardrail));
                }
            }
        }
        result
            .checks_passed
            .push("guardrails_evaluated".to_string());
    }

    /// 主动预防检查（§5.3.2）
    fn check_active_prevention(&self, result: &mut ConstraintEvaluationResult) {
        if let Some(prevention) = &self.active_prevention {
            if prevention.constrained_decoding == Some(true) || prevention.is_enabled() {
                result
                    .checks_passed
                    .push("active_prevention_enabled".to_string());
            }
        }
    }

    /// 返回完整约束快照的引用（修复：之前始终返回 None）
    fn get_constraints(&self) -> Option<&ConstraintsApplied> {
        self.constraints.as_ref()
    }
}

#[cfg(test)]
mod constraint_tests {
    use super::*;
    use crate::types::constraints::{
        ActivePrevention, BudgetStrategy, ConstraintsApplied, InstructionHierarchyMode, PrivacyLevel,
    };
    use crate::types::trace::Trace;

    fn make_test_trace(constraints: ConstraintsApplied) -> Trace {
        let mut t = Trace::new("test-model");
        t.constraints_applied = Some(constraints);
        t
    }

    #[test]
    fn test_evaluator_returns_constraints() {
        let c = ConstraintsApplied {
            budget_limit_usd: Some(5.0),
            budget_strategy: Some(BudgetStrategy::HardStop),
            privacy_level: Some(PrivacyLevel::Masked),
            guardrails_active: Some(vec!["G1".to_string(), "G2".to_string()]),
            instruction_hierarchy_mode: Some(InstructionHierarchyMode::Strict),
            active_prevention: Some(ActivePrevention {
                constrained_decoding: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        let t = make_test_trace(c);
        let evaluator = ConstraintEvaluator::new(&t);
        let result = evaluator.evaluate(&t);

        assert!(result.allowed);
        assert!(result.checks_passed.contains(&"budget_limit_valid".to_string()));
        assert!(result.checks_passed.contains(&"privacy_masked".to_string()));
        assert!(result.checks_passed.contains(&"instruction_hierarchy_strict".to_string()));
        assert!(result.checks_passed.contains(&"guardrail_G1".to_string()));
        assert!(result.checks_passed.contains(&"guardrail_G2".to_string()));
        assert!(result.checks_passed.contains(&"active_prevention_enabled".to_string()));
    }

    #[test]
    fn test_evaluator_without_constraints() {
        let t = Trace::new("test-model");
        let evaluator = ConstraintEvaluator::new(&t);
        let result = evaluator.evaluate(&t);
        assert!(result.allowed);
        assert!(result.checks_passed.contains(&"budget_no_limit".to_string()));
    }

    #[test]
    fn test_get_constraints_returns_value() {
        let c = ConstraintsApplied {
            budget_limit_usd: Some(1.0),
            ..Default::default()
        };
        let mut t = Trace::new("test-model");
        t.constraints_applied = Some(c);
        let evaluator = ConstraintEvaluator::new(&t);
        assert!(evaluator.get_constraints().is_some());
        assert_eq!(evaluator.get_constraints().unwrap().budget_limit_usd, Some(1.0));
    }
}

/// 执行期间预算预占检查
pub struct BudgetPreAllocator {
    limit: f64,
    reserved: f64,
    strategy: BudgetStrategy,
}

impl BudgetPreAllocator {
    pub fn new(limit: f64, strategy: BudgetStrategy) -> Self {
        Self {
            limit,
            reserved: 0.0,
            strategy,
        }
    }

    /// 预占预算（流式开始时调用）
    pub fn reserve(&mut self, amount: f64) -> bool {
        if self.reserved + amount > self.limit {
            match self.strategy {
                BudgetStrategy::HardStop => {
                    warn!(
                        "Budget reservation failed: {} + {} > {}",
                        self.reserved, amount, self.limit
                    );
                    return false;
                }
                BudgetStrategy::SoftAlert => {
                    warn!(
                        "Budget reservation exceeds soft limit: {} + {} > {}",
                        self.reserved, amount, self.limit
                    );
                    self.reserved += amount;
                    return true;
                }
                _ => {
                    self.reserved += amount;
                    return true;
                }
            }
        }
        self.reserved += amount;
        true
    }

    /// 释放未使用的预算
    pub fn release(&mut self, amount: f64) {
        self.reserved = (self.reserved - amount).max(0.0);
    }

    /// 检查是否超出预算
    pub fn is_exceeded(&self, current_cost: f64) -> bool {
        current_cost > self.limit
    }

    /// 获取剩余预算
    pub fn remaining(&self) -> f64 {
        (self.limit - self.reserved).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_preallocator_hard_stop() {
        let mut allocator = BudgetPreAllocator::new(1.0, BudgetStrategy::HardStop);

        assert!(allocator.reserve(0.5));
        assert!(allocator.reserve(0.5));
        assert!(!allocator.reserve(0.1)); // 超出限制
    }

    #[test]
    fn test_budget_preallocator_soft_alert() {
        let mut allocator = BudgetPreAllocator::new(1.0, BudgetStrategy::SoftAlert);

        assert!(allocator.reserve(0.5));
        assert!(allocator.reserve(0.5));
        assert!(allocator.reserve(0.1)); // 软限制允许超出但发出警告
    }
}
