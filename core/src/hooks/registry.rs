//! # Hook Registry（§6.3.1）
//!
//! 钩子注册中心，负责管理所有注册的钩子并协调它们的执行。

use crate::types::journal::JournalEventType;
use crate::types::trace::{ExecutionState, Trace};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{info, warn};

/// 钩子执行结果
#[derive(Debug, Clone)]
pub enum HookResult {
    /// 继续执行
    Continue,
    /// 修改后继续执行（含修改后的 Trace）
    Modified(Trace),
    /// 中止执行（含错误信息）
    Abort(String),
    /// 降级执行
    Degrade,
}

/// 钩子注册中心
#[derive(Debug, Clone)]
pub struct HookRegistry {
    hooks: Arc<RwLock<HashMap<String, Box<dyn Hook + Send + Sync>>>>,
}

impl HookRegistry {
    /// 创建新的钩子注册中心
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册钩子
    pub fn register<H: Hook + Send + Sync + 'static>(&self, name: &str, hook: H) {
        self.hooks
            .write()
            .unwrap()
            .insert(name.to_string(), Box::new(hook));
    }

    /// 取消注册钩子
    pub fn unregister(&self, name: &str) {
        self.hooks.write().unwrap().remove(name);
    }

    /// 执行所有 pre_execute 钩子
    pub fn run_pre_execute(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("pre_execute", trace)
    }

    /// 执行所有 on_token 钩子
    pub fn run_on_token(&self, trace: &mut Trace, _token: &str, _position: usize) -> HookResult {
        self.run_hooks("on_token", trace)
    }

    /// 执行所有 on_certified_guarantee 钩子
    pub fn run_on_certified_guarantee(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("on_certified_guarantee", trace)
    }

    /// 执行所有 on_observation 钩子
    pub fn run_on_observation(&self, trace: &mut Trace, _event: &JournalEventType) -> HookResult {
        trace.execution_state = Some(ExecutionState::Validation);
        self.run_hooks("on_observation", trace)
    }

    /// 执行所有 on_budget_exceeded 钩子
    pub fn run_on_budget_exceeded(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("on_budget_exceeded", trace)
    }

    /// 执行所有 on_safety_event 钩子
    pub fn run_on_safety_event(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("on_safety_event", trace)
    }

    /// 执行所有 on_constraint_violation 钩子
    pub fn run_on_constraint_violation(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("on_constraint_violation", trace)
    }

    /// 执行所有 on_red_team_event 钩子
    pub fn run_on_red_team_event(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("on_red_team_event", trace)
    }

    /// 执行所有 on_finalized 钩子
    pub fn run_on_finalized(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("on_finalized", trace)
    }

    /// 执行所有 pre_stream 钩子
    pub fn run_pre_stream(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("pre_stream", trace)
    }

    /// 执行所有 on_degradation 钩子
    pub fn run_on_degradation(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("on_degradation", trace)
    }

    /// 执行所有 on_active_prevention 钩子
    pub fn run_on_active_prevention(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("on_active_prevention", trace)
    }

    /// 执行所有 post_stream 钩子
    pub fn run_post_stream(&self, trace: &mut Trace) -> HookResult {
        self.run_hooks("post_stream", trace)
    }

    /// 执行指定类型的所有钩子
    fn run_hooks(&self, hook_type: &str, trace: &mut Trace) -> HookResult {
        let hooks = self.hooks.read().unwrap();
        let matching_hooks: Vec<_> = hooks
            .iter()
            .filter(|(name, _)| name.starts_with(hook_type))
            .collect();

        for (name, hook) in matching_hooks {
            info!("执行钩子: {}", name);
            match hook.run(trace) {
                HookResult::Continue => continue,
                HookResult::Modified(new_trace) => {
                    *trace = new_trace;
                }
                HookResult::Abort(msg) => {
                    warn!("钩子 {} 中止执行: {}", name, msg);
                    return HookResult::Abort(msg);
                }
                HookResult::Degrade => {
                    info!("钩子 {} 触发降级", name);
                    return HookResult::Degrade;
                }
            }
        }

        HookResult::Continue
    }
}

/// 钩子 trait
pub trait Hook: std::fmt::Debug {
    /// 执行钩子
    fn run(&self, trace: &mut Trace) -> HookResult;
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}
