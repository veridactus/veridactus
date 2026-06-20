//! # Governance DSL Compiler
//!
//! 将解析后的 DSL 策略编译为内部约束对象.
//!
//! 遵循 §5.8.2 编译和审计要求：
//! - DSL 源哈希用于审计追踪
//! - 意图自动解析为强制规则
//! - 规则编译为可执行策略树

use crate::types::constraints::{
    ActivePrevention, AdaptiveState, BudgetStrategy, ConstraintsApplied,
    InstructionHierarchyMode, IntentResolution, PolicyEvaluation, PrivacyLevel,
    PreventedPattern, PreventionAction, ReproducibilityMode,
};
use crate::types::trace::Trace;
use chrono::Utc;
use std::collections::HashMap;

use super::parser::{GovernanceDsl, PolicyDefinition, PolicyRule, PolicyType};

#[derive(Debug, Clone)]
pub struct DslCompiler {
    intent_resolvers: HashMap<String, IntentResolver>,
}

impl Default for DslCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl DslCompiler {
    pub fn new() -> Self {
        let mut intent_resolvers = HashMap::new();

        intent_resolvers.insert(
            "cost_effective".to_string(),
            IntentResolver {
                target_strategy: BudgetStrategy::HardStop,
                target_model: Some("openai/gpt-4o-mini".to_string()),
                target_limit_usd: Some(0.05),
                target_privacy: None,
                target_guardrails: None,
            },
        );

        intent_resolvers.insert(
            "pii_not_stored".to_string(),
            IntentResolver {
                target_strategy: BudgetStrategy::HardStop,
                target_model: None,
                target_limit_usd: None,
                target_privacy: Some(PrivacyLevel::Masked),
                target_guardrails: None,
            },
        );

        intent_resolvers.insert(
            "block_harmful".to_string(),
            IntentResolver {
                target_strategy: BudgetStrategy::HardStop,
                target_model: None,
                target_limit_usd: None,
                target_privacy: None,
                target_guardrails: Some(vec!["G1".to_string(), "G2".to_string()]),
            },
        );

        Self { intent_resolvers }
    }

    pub fn compile(&self, dsl: &GovernanceDsl, _trace: &mut Trace) -> Result<ConstraintsApplied, CompileError> {
        let mut constraints = ConstraintsApplied::default();
        let mut intent_resolutions = Vec::new();
        let mut checks_passed = Vec::new();
        let mut checks_failed = Vec::new();

        if let Some(ref intents) = dsl.intents {
            if let Some(ref budget_intent) = intents.budget {
                if let Some(resolver) = self.intent_resolvers.get(budget_intent) {
                    let resolved_to = format!(
                        "model={}, strategy={}",
                        resolver.target_model.as_deref().unwrap_or("default"),
                        format!("{:?}", resolver.target_strategy).to_lowercase()
                    );
                    intent_resolutions.push(IntentResolution {
                        intent: budget_intent.clone(),
                        resolved_to: resolved_to.clone(),
                        rationale: "Resolved via Governance DSL intent resolution".to_string(),
                        timestamp: Utc::now().to_rfc3339(),
                    });
                    checks_passed.push(format!("intent_resolved:{}->{}", budget_intent, resolved_to));

                    if resolver.target_model.is_some() {
                        constraints.reproducibility_mode = Some(ReproducibilityMode::Bounded);
                    }
                    if let Some(limit) = resolver.target_limit_usd {
                        constraints.budget_limit_usd = Some(limit);
                    }
                    constraints.budget_strategy = Some(resolver.target_strategy.clone());
                }
            }

            if let Some(ref privacy_intent) = intents.privacy {
                if let Some(resolver) = self.intent_resolvers.get(privacy_intent) {
                    if let Some(ref privacy) = resolver.target_privacy {
                        intent_resolutions.push(IntentResolution {
                            intent: privacy_intent.clone(),
                            resolved_to: format!("{:?}", privacy).to_lowercase(),
                            rationale: "Privacy intent resolved to privacy level".to_string(),
                            timestamp: Utc::now().to_rfc3339(),
                        });
                        constraints.privacy_level = Some(privacy.clone());
                        checks_passed.push(format!("intent_resolved:{}", privacy_intent));
                    }
                }
            }

            if let Some(ref safety_intent) = intents.safety {
                if let Some(resolver) = self.intent_resolvers.get(safety_intent) {
                    if let Some(ref guardrails) = resolver.target_guardrails {
                        intent_resolutions.push(IntentResolution {
                            intent: safety_intent.clone(),
                            resolved_to: format!("guardrails={}", guardrails.join(",")),
                            rationale: "Safety intent resolved to guardrail levels".to_string(),
                            timestamp: Utc::now().to_rfc3339(),
                        });
                        constraints.guardrails_active = Some(guardrails.clone());
                        checks_passed.push(format!("intent_resolved:{}", safety_intent));
                    }
                }
            }
        }

        for policy in &dsl.policies {
            match self.compile_policy(policy, &mut constraints) {
                Ok(_) => checks_passed.push(format!("policy_compiled:{}", policy.id)),
                Err(e) => checks_failed.push(format!("policy_failed:{}:{}", policy.id, e)),
            }
        }

        let source_hash = dsl.compute_source_hash();

        constraints.policy_evaluation = Some(PolicyEvaluation {
            decision: Some("allow".to_string()),
            checks_passed: Some(checks_passed),
            checks_failed: Some(checks_failed),
            negotiated_capabilities: Some(vec!["veridactus.ai/v1/governance_dsl@1".to_string()]),
            degrade_action: None,
            intent_resolution: Some(intent_resolutions),
            escalation_trail: None,
            dsl_source_hash: Some(source_hash),
            current_risk_score: Some(0.0),
            risk_factor_contributions: Some(vec![]),
            adaptive_state: Some(AdaptiveState::SoftAlert),
            prevention_events_count: Some(0),
        });

        Ok(constraints)
    }

    fn compile_policy(
        &self,
        policy: &PolicyDefinition,
        constraints: &mut ConstraintsApplied,
    ) -> Result<(), CompileError> {
        match policy.policy_type {
            PolicyType::Budget => self.compile_budget_policy(policy, constraints),
            PolicyType::ActivePrevention => self.compile_active_prevention_policy(policy, constraints),
            PolicyType::Guardrails => self.compile_guardrails_policy(policy, constraints),
            PolicyType::Compliance => self.compile_compliance_policy(policy, constraints),
            PolicyType::Reproducibility => self.compile_reproducibility_policy(policy, constraints),
            PolicyType::InstructionHierarchy => self.compile_instruction_hierarchy_policy(policy, constraints),
            PolicyType::ToolConstraint => {
                constraints.guardrails_active = Some(vec!["G1".to_string()]);
                Ok(())
            }
        }
    }

    fn compile_budget_policy(
        &self,
        policy: &PolicyDefinition,
        constraints: &mut ConstraintsApplied,
    ) -> Result<(), CompileError> {
        let defaults = &policy.defaults;
        if let Some(limit) = defaults.get("limit_usd") {
            if let Some(limit_f64) = limit.as_f64() {
                constraints.budget_limit_usd = Some(limit_f64);
            }
        }
        if let Some(strategy) = defaults.get("strategy") {
            if let Some(strategy_str) = strategy.as_str() {
                constraints.budget_strategy = match strategy_str {
                    "hard_stop" => Some(BudgetStrategy::HardStop),
                    "degrade_model" => Some(BudgetStrategy::DegradeModel),
                    "soft_alert" => Some(BudgetStrategy::SoftAlert),
                    "adaptive" => Some(BudgetStrategy::Adaptive),
                    "awareness" => Some(BudgetStrategy::Awareness),
                    _ => Some(BudgetStrategy::HardStop),
                };
            }
        }
        if let Some(buffer) = defaults.get("buffer_ratio") {
            if let Some(_buffer_f64) = buffer.as_f64() {
            }
        }
        Ok(())
    }

    fn compile_active_prevention_policy(
        &self,
        policy: &PolicyDefinition,
        constraints: &mut ConstraintsApplied,
    ) -> Result<(), CompileError> {
        let prevented_patterns: Vec<PreventedPattern> = policy
            .rules
            .iter()
            .enumerate()
            .map(|(i, _rule)| PreventedPattern {
                name: format!("pattern_{}", i),
                pattern: "dangerous_content".to_string(),
                action: PreventionAction::BlockToken,
                action_params: None,
                severity: "high".to_string(),
                enabled: true,
            })
            .collect();

        constraints.active_prevention = Some(ActivePrevention {
            constrained_decoding: Some(true),
            prevented_patterns: Some(prevented_patterns),
            sampling_rate: Some(1.0),
            log_blocked_tokens: Some(true),
            report_in_header: Some(false),
            custom_vocabulary_path: None,
            max_block_count: Some(100),
        });
        Ok(())
    }

    fn compile_guardrails_policy(
        &self,
        policy: &PolicyDefinition,
        constraints: &mut ConstraintsApplied,
    ) -> Result<(), CompileError> {
        let mut guardrails = Vec::new();
        for rule in &policy.rules {
            match rule {
                PolicyRule::Block { .. } => guardrails.push("G1".to_string()),
                PolicyRule::Flag => guardrails.push("G2".to_string()),
                PolicyRule::RequireApproval { .. } => guardrails.push("G2".to_string()),
                _ => guardrails.push("G3".to_string()),
            }
        }

        constraints.guardrails_active = Some(guardrails);
        constraints.guardrails_strictness = Some("high".to_string());
        Ok(())
    }

    fn compile_compliance_policy(
        &self,
        policy: &PolicyDefinition,
        _constraints: &mut ConstraintsApplied,
    ) -> Result<(), CompileError> {
        if let Some(_profile) = policy.defaults.get("profile") {
        }
        Ok(())
    }

    fn compile_reproducibility_policy(
        &self,
        policy: &PolicyDefinition,
        constraints: &mut ConstraintsApplied,
    ) -> Result<(), CompileError> {
        if let Some(mode) = policy.defaults.get("mode") {
            if let Some(mode_str) = mode.as_str() {
                constraints.reproducibility_mode = match mode_str {
                    "none" => Some(ReproducibilityMode::None),
                    "bounded" => Some(ReproducibilityMode::Bounded),
                    "strict" => Some(ReproducibilityMode::Strict),
                    _ => Some(ReproducibilityMode::None),
                };
            }
        }
        if let Some(seed) = policy.defaults.get("seed") {
            if let Some(seed_i64) = seed.as_i64() {
                constraints.reproducibility_seed = Some(seed_i64 as u64);
            }
        }
        Ok(())
    }

    fn compile_instruction_hierarchy_policy(
        &self,
        policy: &PolicyDefinition,
        constraints: &mut ConstraintsApplied,
    ) -> Result<(), CompileError> {
        if let Some(mode) = policy.defaults.get("mode") {
            if let Some(mode_str) = mode.as_str() {
                constraints.instruction_hierarchy_mode = match mode_str {
                    "strict" => Some(InstructionHierarchyMode::Strict),
                    "warn" => Some(InstructionHierarchyMode::Warn),
                    "off" => Some(InstructionHierarchyMode::Off),
                    "verified" => Some(InstructionHierarchyMode::Verified),
                    _ => Some(InstructionHierarchyMode::Strict),
                };
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct IntentResolver {
    pub target_strategy: BudgetStrategy,
    pub target_model: Option<String>,
    pub target_limit_usd: Option<f64>,
    pub target_privacy: Option<PrivacyLevel>,
    pub target_guardrails: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub policy_id: String,
    pub message: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Compile error in policy '{}': {}", self.policy_id, self.message)
    }
}

impl std::error::Error for CompileError {}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DSL: &str = r#"
version: "0.2.0"
intents:
  budget: "cost_effective"
  privacy: "pii_not_stored"
policies:
  - id: "production-budget"
    type: budget
    defaults:
      strategy: "hard_stop"
      limit_usd: 0.10
"#;

    #[test]
    fn test_compile_dsl() {
        let dsl = GovernanceDsl::parse(SAMPLE_DSL).expect("Failed to parse");
        let compiler = DslCompiler::new();
        let mut trace = Trace::new("test-model");
        let constraints = compiler.compile(&dsl, &mut trace).expect("Failed to compile");
        assert!(constraints.budget_limit_usd.is_some());
        assert_eq!(constraints.budget_strategy, Some(BudgetStrategy::HardStop));
    }
}