//! # 约束冲突检测
//!
//! 严格遵循协议 §5.5.1 Normative Conflict Matrix。
//! 检测约束之间的语义冲突，防止非法组合。

use serde::{Deserialize, Serialize};

use super::constraints::{
    BudgetStrategy, InstructionHierarchyMode, PrivacyLevel, ReproducibilityMode,
};

/// 冲突级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictLevel {
    /// 明确冲突（YES）
    Hard,
    /// pipelines件冲突（CONDITIONAL）
    Conditional,
    /// 无冲突（NO）
    None,
}

/// 冲突结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResult {
    /// 是否存在冲突
    pub has_conflict: bool,
    /// 冲突描述列表
    pub conflicts: Vec<String>,
    /// 建议的解决方式
    pub suggestions: Vec<String>,
}

/// 约束冲突检测器（协议 §5.5.1）
pub struct ConflictDetector;

impl ConflictDetector {
    /// 检测隐私级别与可重现模式的冲突
    pub fn check_privacy_reproducibility(
        privacy: &PrivacyLevel,
        reproducibility: &ReproducibilityMode,
    ) -> ConflictResult {
        let mut conflicts = Vec::new();
        let mut suggestions = Vec::new();

        match (privacy, reproducibility) {
            (PrivacyLevel::HashOnly, ReproducibilityMode::Strict) => {
                conflicts
                    .push("hash_only + strict: hash_only 丢弃明文, strict 需要完整载荷比较".into());
                suggestions.push("将 reproducibility 降级为 bounded，或设置 focus_fields".into());
                return ConflictResult {
                    has_conflict: true,
                    conflicts,
                    suggestions,
                };
            }
            (PrivacyLevel::HashOnly, ReproducibilityMode::Bounded) => {
                conflicts
                    .push("hash_only + bounded: pipelines件允许 — 需要提供 focus_fields".into());
                suggestions.push("提供 VERIDACTUS-Focus-Fields 头部以启用哈希比较".into());
                return ConflictResult {
                    has_conflict: true,
                    conflicts,
                    suggestions,
                };
            }
            (PrivacyLevel::TeePrivate, ReproducibilityMode::Strict) => {
                conflicts.push("tee_private + strict: TEE 外部仅存哈希，无法严格比较".into());
                suggestions.push("将 reproducibility 降级为 bounded".into());
                return ConflictResult {
                    has_conflict: true,
                    conflicts,
                    suggestions,
                };
            }
            _ => {}
        }

        ConflictResult {
            has_conflict: false,
            conflicts,
            suggestions,
        }
    }

    /// 检测预算策略与隐私级别的冲突
    pub fn check_budget_privacy(
        budget_strategy: &BudgetStrategy,
        privacy: &PrivacyLevel,
    ) -> ConflictResult {
        if *budget_strategy == BudgetStrategy::Awareness && *privacy == PrivacyLevel::HashOnly {
            return ConflictResult {
                has_conflict: true,
                conflicts: vec![
                    "awareness + hash_only: 预算信息必须出现在提示词中，与 hash_only 冲突".into(),
                ],
                suggestions: vec!["将预算策略改为 hard_stop，或将隐私级别改为 masked".into()],
            };
        }
        ConflictResult {
            has_conflict: false,
            conflicts: vec![],
            suggestions: vec![],
        }
    }

    /// 检测预算策略与重放动作的冲突
    pub fn check_budget_replay(
        budget_strategy: &BudgetStrategy,
        is_replay: bool,
    ) -> ConflictResult {
        if *budget_strategy == BudgetStrategy::DegradeModel && is_replay {
            return ConflictResult {
                has_conflict: true,
                conflicts: vec!["degrade + replay: 降级可能改变模型，影响重放完整性".into()],
                suggestions: vec!["将预算策略改为 hard_stop，或记录模型变更".into()],
            };
        }
        ConflictResult {
            has_conflict: false,
            conflicts: vec![],
            suggestions: vec![],
        }
    }

    /// 全面检测所有约束组合
    pub fn detect_all(
        privacy: Option<&PrivacyLevel>,
        reproducibility: Option<&ReproducibilityMode>,
        budget_strategy: Option<&BudgetStrategy>,
        instruction_mode: Option<&InstructionHierarchyMode>,
        is_replay: bool,
    ) -> ConflictResult {
        let mut all_conflicts = Vec::new();
        let mut all_suggestions = Vec::new();

        // 隐私 + 可重现
        if let (Some(p), Some(r)) = (privacy, reproducibility) {
            let r1 = Self::check_privacy_reproducibility(p, r);
            all_conflicts.extend(r1.conflicts);
            all_suggestions.extend(r1.suggestions);
        }

        // 预算 + 隐私
        if let (Some(b), Some(p)) = (budget_strategy, privacy) {
            let r2 = Self::check_budget_privacy(b, p);
            all_conflicts.extend(r2.conflicts);
            all_suggestions.extend(r2.suggestions);
        }

        // 预算 + 重放
        if let Some(b) = budget_strategy {
            let r3 = Self::check_budget_replay(b, is_replay);
            all_conflicts.extend(r3.conflicts);
            all_suggestions.extend(r3.suggestions);
        }

        ConflictResult {
            has_conflict: !all_conflicts.is_empty(),
            conflicts: all_conflicts,
            suggestions: all_suggestions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_only_strict_conflict() {
        let r = ConflictDetector::check_privacy_reproducibility(
            &PrivacyLevel::HashOnly,
            &ReproducibilityMode::Strict,
        );
        assert!(r.has_conflict);
    }

    #[test]
    fn test_raw_no_conflict() {
        let r = ConflictDetector::check_privacy_reproducibility(
            &PrivacyLevel::Raw,
            &ReproducibilityMode::Strict,
        );
        assert!(!r.has_conflict);
    }

    #[test]
    fn test_awareness_hash_only_conflict() {
        let r = ConflictDetector::check_budget_privacy(
            &BudgetStrategy::Awareness,
            &PrivacyLevel::HashOnly,
        );
        assert!(r.has_conflict);
    }

    #[test]
    fn test_degrade_replay_conflict() {
        let r = ConflictDetector::check_budget_replay(&BudgetStrategy::DegradeModel, true);
        assert!(r.has_conflict);
    }

    #[test]
    fn test_comprehensive_detection() {
        let r = ConflictDetector::detect_all(
            Some(&PrivacyLevel::HashOnly),
            Some(&ReproducibilityMode::Strict),
            Some(&BudgetStrategy::HardStop),
            None,
            false,
        );
        assert!(r.has_conflict, "hash_only + strict 应检测到冲突");
        assert!(!r.conflicts.is_empty());
    }
}
