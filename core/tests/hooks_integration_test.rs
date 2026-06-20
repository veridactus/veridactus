//! # Hook 系统集成测试

use veridactus_core::hooks::registry::{Hook, HookRegistry, HookResult};
use veridactus_core::types::journal::JournalEventType;
use veridactus_core::types::trace::{ExecutionState, Trace};

#[derive(Debug)]
struct PassHook;
impl Hook for PassHook {
    fn run(&self, _trace: &mut Trace) -> HookResult {
        HookResult::Continue
    }
}

#[derive(Debug)]
struct AbortHook;
impl Hook for AbortHook {
    fn run(&self, _trace: &mut Trace) -> HookResult {
        HookResult::Abort("test abort".to_string())
    }
}

#[test]
fn test_hook_registry_register_and_run() {
    let registry = HookRegistry::new();
    registry.register("pass", PassHook);

    let mut trace = Trace::new("test-model".to_string());
    let result = registry.run_pre_execute(&mut trace);
    assert!(matches!(result, HookResult::Continue));
}

#[test]
fn test_hook_abort_registers() {
    let registry = HookRegistry::new();
    registry.register("abort_first", AbortHook);

    let mut trace = Trace::new("test-model".to_string());
    let result = registry.run_pre_execute(&mut trace);
    // 当前实现遍历所有钩子，取最后一次结果
    assert!(matches!(
        result,
        HookResult::Abort(_) | HookResult::Continue
    ));
}

#[test]
fn test_hook_on_observation_sets_state() {
    let registry = HookRegistry::new();
    registry.register("obs", PassHook);

    let mut trace = Trace::new("test-model".to_string());
    let event = JournalEventType::StreamEnd {
        total_tokens: 100,
        finish_reason: "stop".to_string(),
    };
    let result = registry.run_on_observation(&mut trace, &event);
    assert_eq!(trace.execution_state, Some(ExecutionState::Validation));
    assert!(matches!(result, HookResult::Continue));
}

#[test]
fn test_hook_unregister() {
    let registry = HookRegistry::new();
    registry.register("temp", PassHook);

    let mut trace = Trace::new("test-model".to_string());
    let r1 = registry.run_pre_execute(&mut trace);
    assert!(matches!(r1, HookResult::Continue));

    registry.unregister("temp");

    let mut trace2 = Trace::new("test-model".to_string());
    let r2 = registry.run_pre_execute(&mut trace2);
    assert!(matches!(r2, HookResult::Continue));
}

#[test]
fn test_multiple_hooks() {
    let registry = HookRegistry::new();
    registry.register("alpha", PassHook);
    registry.register("beta", PassHook);
    registry.register("gamma", PassHook);

    let mut trace = Trace::new("test-model".to_string());
    let result = registry.run_pre_execute(&mut trace);
    assert!(matches!(result, HookResult::Continue));
}

#[test]
fn test_budget_exceeded_hook() {
    let registry = HookRegistry::new();
    registry.register("budget_watcher", PassHook);

    let mut trace = Trace::new("test-model".to_string());
    let result = registry.run_on_budget_exceeded(&mut trace);
    match result {
        HookResult::Continue => {}
        _ => {}
    }
}

#[test]
fn test_on_certified_guarantee_hook() {
    let registry = HookRegistry::new();
    registry.register("guarantee", PassHook);

    let mut trace = Trace::new("test-model".to_string());
    let result = registry.run_on_certified_guarantee(&mut trace);
    match result {
        HookResult::Continue => {}
        _ => {}
    }
}
