//! # Semantic Hooks 模块（§6.3）
//!
//! 实现协议定义的 9 个语义钩子，提供执行流程的可扩展拦截点。
//!
//! Hooks 在以下时机触发：
//! 1. pre_execute - LLM 执行前
//! 2. on_token - 每个 token 生成时
//! 3. on_certified_guarantee - 收到认证保证时
//! 4. on_fairness_check - 公平性审计时
//! 5. on_failure - 状态转换为 FAILED 时
//! 6. on_active_prevention - constrained_decoding 阻止 token 时
//! 7. on_observation - 生成观察数据时
//! 8. on_budget_exceeded - 预算超支时
//! 9. on_finalized - 执行最终化时

pub mod handlers;
pub mod registry;

pub use handlers::{
    OnActivePreventionHook, OnBudgetExceededHook, OnCertifiedGuaranteeHook,
    OnConstraintViolationHook, OnFailureHook, OnFairnessCheckHook, OnFinalizedHook,
    OnObservationHook, OnRedTeamEventHook, OnSafetyEventHook, OnTokenHook, PostStreamHook,
    PreExecuteHook,
};
pub use registry::{HookRegistry, HookResult};
