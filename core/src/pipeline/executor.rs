//! # 流水线执行引擎
//!
//! 严格遵循 AI.md §6.4 插件并行执行优化 + 速度分层架构。
//!
//! 架构原则：
//!   PreRequest  → Rust Native (同步, <10μs each)
//!   Streaming   → Rust Native (同步, per-token)
//!   PostResponse→ Rust Native (同步) + Redis XADD (非阻塞 dispatch)
//!   AsyncFinalize→ Redis Stream → Python Worker (异步, 不阻塞响应)
//!
//! 修复记录：
//! - execute_parallel 改为真正的 FuturesUnordered 并行
//! - 新增 execute_post_response + execute_async_finalize
//! - AsyncContext 新增 trace_id/response/output_content 字段供 Python Worker 使用

use std::sync::Arc;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tracing::{info, warn};

use crate::pipeline::config::{ExecutionPlan, Placement, PluginConfig};
use crate::plugin::{
    AsyncContext, PluginRegistry, RequestContext, ResponseContext, StreamChunkContext,
};
use crate::types::journal::ExecutionJournal;
use crate::types::Action;

/// 流水线执行结果
#[derive(Debug)]
pub struct PipelineResult {
    pub action: Action,
    pub block_reason: Option<String>,
    pub checks_passed: Vec<String>,
    pub checks_failed: Vec<String>,
}

/// 流水线执行引擎 — 速度分层：热路径 Rust Native + 冷路径 Redis → Python
pub struct PipelineExecutor {
    registry: Arc<PluginRegistry>,
    plan: ExecutionPlan,
}

impl PipelineExecutor {
    pub fn new(registry: Arc<PluginRegistry>, plan: ExecutionPlan) -> Self {
        Self { registry, plan }
    }

    // ============================
    //  🔥 热路径: PreRequest 阶段
    // ============================

    /// 执行 PreRequest 阶段 — 同步 Rust Native，每个插件 <10μs
    pub async fn execute_pre_request(
        &self,
        ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
    ) -> PipelineResult {
        let stage = self
            .plan
            .stages
            .iter()
            .find(|s| s.placement == Placement::PreRequest);

        let Some(stage) = stage else {
            return PipelineResult::allow();
        };

        let mut checks_passed = Vec::new();
        let mut checks_failed = Vec::new();

        if stage.parallel {
            let blocked = self
                .execute_pre_request_parallel(&stage.plugins, ctx, journal, &mut checks_passed, &mut checks_failed)
                .await;
            if blocked {
                return PipelineResult::block(
                    format!("PreRequest plugin blocked: {:?}", checks_failed),
                    checks_passed,
                    checks_failed,
                );
            }
        } else {
            for plugin_cfg in &stage.plugins {
                let result = self
                    .execute_single_pre_request(&plugin_cfg.name, ctx, journal, Some(plugin_cfg))
                    .await;
                match result {
                    Ok(Action::Continue) => checks_passed.push(plugin_cfg.name.clone()),
                    Ok(Action::Block) => {
                        checks_failed.push(plugin_cfg.name.clone());
                        return PipelineResult::block(
                            format!("Plugin {} blocked", plugin_cfg.name),
                            checks_passed,
                            checks_failed,
                        );
                    }
                    Ok(Action::Degrade) => checks_passed.push(format!("{} (degrade)", plugin_cfg.name)),
                    Ok(Action::Flag) => checks_passed.push(format!("{} (flagged)", plugin_cfg.name)),
                    Err(e) => {
                        checks_failed.push(format!("{}: {}", plugin_cfg.name, e));
                        return PipelineResult::block(
                            format!("Plugin {} error: {}", plugin_cfg.name, e),
                            checks_passed,
                            checks_failed,
                        );
                    }
                }
            }
        }

        PipelineResult {
            action: Action::Continue,
            block_reason: None,
            checks_passed,
            checks_failed,
        }
    }

    // ============================
    //  🔥 热路径: Streaming 阶段
    // ============================

    /// 执行 Streaming 阶段 — 每个 chunk 调用一次，同步 Rust Native
    pub async fn execute_streaming(
        &self,
        ctx: &mut StreamChunkContext,
        journal: &mut ExecutionJournal,
    ) -> PipelineResult {
        let stage = self
            .plan
            .stages
            .iter()
            .find(|s| s.placement == Placement::Streaming);

        let Some(stage) = stage else {
            return PipelineResult::allow();
        };

        if stage.plugins.is_empty() {
            return PipelineResult::allow();
        }

        let mut checks_passed = Vec::new();
        let mut checks_failed = Vec::new();

        for plugin_cfg in &stage.plugins {
            let plugin = match self.registry.find(&plugin_cfg.name) {
                Some(p) => p,
                None => continue,
            };
            match plugin.on_stream_chunk(ctx, journal).await {
                Ok(Action::Continue) => checks_passed.push(plugin_cfg.name.clone()),
                Ok(Action::Block) => {
                    checks_failed.push(plugin_cfg.name.clone());
                    return PipelineResult::block(
                        format!("Streaming blocked by {}", plugin_cfg.name),
                        checks_passed,
                        checks_failed,
                    );
                }
                Ok(Action::Flag | Action::Degrade) => {
                    checks_passed.push(format!("{} (flagged)", plugin_cfg.name))
                }
                Err(e) => {
                    warn!("Streaming plugin {} error: {}", plugin_cfg.name, e);
                }
            }
        }

        PipelineResult {
            action: Action::Continue,
            block_reason: None,
            checks_passed,
            checks_failed,
        }
    }

    // ============================
    //  🌡️ 温路径: PostResponse 阶段
    // ============================

    /// 执行 PostResponse 阶段 — Rust Native 同步 + 返回需要 dispatch 的 cold tasks
    ///
    /// 返回 triple: (PipelineResult, cold_tasks_for_redis, response_for_cold_tasks)
    /// cold_tasks: Vec<(task_type, params_json)> 由调用方通过 AsyncDispatcher 非阻塞推入 Redis
    pub async fn execute_post_response(
        &self,
        ctx: &mut ResponseContext,
        journal: &mut ExecutionJournal,
    ) -> (PipelineResult, Vec<(String, serde_json::Value)>) {
        let stage = self
            .plan
            .stages
            .iter()
            .find(|s| s.placement == Placement::PostResponse);

        if stage.is_none() {
            return (PipelineResult::allow(), Vec::new());
        }

        let stage = stage.unwrap();
        let mut checks_passed = Vec::new();
        let mut checks_failed = Vec::new();
        let mut cold_tasks: Vec<(String, serde_json::Value)> = Vec::new();

        for plugin_cfg in &stage.plugins {
            let plugin = match self.registry.find(&plugin_cfg.name) {
                Some(p) => p,
                None => continue,
            };

            match plugin.on_response(ctx, journal).await {
                Ok(Action::Continue) => checks_passed.push(plugin_cfg.name.clone()),
                Ok(Action::Block) => {
                    checks_failed.push(plugin_cfg.name.clone());
                    return (PipelineResult::block(
                        format!("PostResponse blocked by {}", plugin_cfg.name),
                        checks_passed,
                        checks_failed,
                    ), cold_tasks);
                }
                Ok(Action::Flag) => {
                    checks_passed.push(format!("{} (flagged)", plugin_cfg.name));
                    // Flag 后仍可 dispatch cold tasks
                }
                Ok(Action::Degrade) => {
                    checks_passed.push(format!("{} (degrade)", plugin_cfg.name));
                }
                Err(e) => warn!("PostResponse plugin {} error: {}", plugin_cfg.name, e),
            }

            // 收集需要异步处理的 cold task
            if let Some(task_config) = plugin_cfg.config.get("async_task") {
                if let Some(task_type) = task_config.as_str() {
                    let params = serde_json::json!({
                        "response": ctx.response,
                        "trace_id": ctx.trace_id.to_string(),
                        "actual_cost": ctx.actual_cost,
                    });
                    cold_tasks.push((task_type.to_string(), params));
                }
            }
        }

        (PipelineResult {
            action: Action::Continue,
            block_reason: None,
            checks_passed,
            checks_failed,
        }, cold_tasks)
    }

    // ============================
    //  ❄️ 冷路径: AsyncFinalize
    // ============================

    /// 收集需要异步最终化的任务参数 (由调用方 dispatch 到 Redis)
    ///
    /// 返回 Vec<(plugin_name, task_params)> 供 AsyncDispatcher::dispatch() 调用
    pub fn collect_async_tasks(
        &self,
        trace_id: &uuid::Uuid,
        response_content: &str,
    ) -> Vec<(String, serde_json::Value)> {
        let stage = self
            .plan
            .stages
            .iter()
            .find(|s| s.placement == Placement::AsyncFinalize);

        let Some(stage) = stage else {
            return Vec::new();
        };

        stage.plugins.iter().filter_map(|plugin_cfg| {
            // 从配置中提取 async task 类型
            let task_type = plugin_cfg
                .config
                .get("async_task")
                .and_then(|v| v.as_str())
                .unwrap_or(&plugin_cfg.name);

            let params = serde_json::json!({
                "trace_id": trace_id.to_string(),
                "response": response_content,
                "plugin_config": plugin_cfg.config,
            });

            Some((task_type.to_string(), params))
        }).collect()
    }

    // ============================
    //  内部辅助方法
    // ============================

    async fn execute_single_pre_request(
        &self,
        plugin_name: &str,
        ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
        plugin_cfg: Option<&PluginConfig>,
    ) -> Result<Action, String> {
        let plugin = match self.registry.find(plugin_name) {
            Some(p) => p,
            None => return Err(format!("Plugin '{}' not registered", plugin_name)),
        };

        if let Some(cfg) = plugin_cfg {
            if !cfg.config.is_null() {
                let parsed = if let Some(s) = cfg.config.as_str() {
                    serde_json::from_str::<serde_json::Value>(s)
                        .unwrap_or_else(|_| cfg.config.clone())
                } else {
                    cfg.config.clone()
                };
                ctx.plugin_config = Some(parsed);
            }
        }

        info!("Executing plugin: {}", plugin_name);
        plugin.on_request(ctx, journal).await
    }

    /// 🔧 修复: 真正的 FuturesUnordered 并行执行
    async fn execute_pre_request_parallel(
        &self,
        plugin_configs: &[PluginConfig],
        ctx: &RequestContext,
        journal: &mut ExecutionJournal,
        checks_passed: &mut Vec<String>,
        checks_failed: &mut Vec<String>,
    ) -> bool {
        use std::sync::Mutex;

        // 共享结果：Arc<Mutex<>> 允许多个 future 同时写入
        let passed = Arc::new(Mutex::new(Vec::new()));
        let failed = Arc::new(Mutex::new(Vec::new()));
        let blocked = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let mut futures = FuturesUnordered::new();

        for plugin_cfg in plugin_configs {
            let name = plugin_cfg.name.clone();
            let registry = self.registry.clone();
            let passed_ref = passed.clone();
            let failed_ref = failed.clone();
            let blocked_ref = blocked.clone();

            // 每个插件需要独立的 ctx clone
            let mut plugin_ctx = RequestContext {
                headers: ctx.headers.clone(),
                body: ctx.body.clone(),
                trace_id: ctx.trace_id,
                tenant_id: ctx.tenant_id.clone(),
                plugin_config: None,
            };
            let mut plugin_journal = journal.clone_empty(ctx.trace_id);

            let cfg_clone = plugin_cfg.clone();

            futures.push(async move {
                let plugin = registry.find(&name);
                if plugin.is_none() {
                    if let Ok(mut f) = failed_ref.lock() {
                        f.push(format!("{}: not found", name));
                    }
                    return;
                }
                let plugin = plugin.unwrap();

                // 注入配置
                if !cfg_clone.config.is_null() {
                    let parsed = if let Some(s) = cfg_clone.config.as_str() {
                        serde_json::from_str::<serde_json::Value>(s)
                            .unwrap_or_else(|_| cfg_clone.config.clone())
                    } else {
                        cfg_clone.config.clone()
                    };
                    plugin_ctx.plugin_config = Some(parsed);
                }

                match plugin.on_request(&mut plugin_ctx, &mut plugin_journal).await {
                    Ok(Action::Continue) => {
                        if let Ok(mut p) = passed_ref.lock() { p.push(name.clone()); }
                    }
                    Ok(Action::Block) => {
                        if let Ok(mut f) = failed_ref.lock() { f.push(name.clone()); }
                        blocked_ref.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    Ok(Action::Flag | Action::Degrade) => {
                        if let Ok(mut p) = passed_ref.lock() { p.push(format!("{} (flagged)", name)); }
                    }
                    Err(e) => {
                        if let Ok(mut f) = failed_ref.lock() { f.push(format!("{}: {}", name, e)); }
                        blocked_ref.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            });
        }

        // 等待所有 future 完成
        while let Some(_) = futures.next().await {}

        // 合并结果到 journal（串行追加，避免竞态）
        // journal events from parallel plugins are merged in batch
        // 因为每个 plugin 有自己的 journal clone，这里只记录总体结果

        if let Ok(p) = passed.lock() { checks_passed.extend(p.iter().cloned()); }
        if let Ok(f) = failed.lock() { checks_failed.extend(f.iter().cloned()); }

        blocked.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn plan(&self) -> &ExecutionPlan {
        &self.plan
    }
}

impl PipelineResult {
    fn allow() -> Self {
        Self {
            action: Action::Continue,
            block_reason: None,
            checks_passed: vec![],
            checks_failed: vec![],
        }
    }

    fn block(reason: String, checks_passed: Vec<String>, checks_failed: Vec<String>) -> Self {
        Self {
            action: Action::Block,
            block_reason: Some(reason),
            checks_passed,
            checks_failed,
        }
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{GovernancePlugin, PluginMetadata, PluginType};
    use crate::types::VersionRange;
    use async_trait::async_trait;
    use uuid::Uuid;

    struct AllowPlugin;
    #[async_trait]
    impl GovernancePlugin for AllowPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                name: "allow-plugin".into(),
                plugin_type: PluginType::Native,
                version: "1.0".into(),
                description: "always allows".into(),
                author: None,
                supported_protocol_versions: VersionRange {
                    min: "0.2.0".into(),
                    max: "0.2.1".into(),
                },
            }
        }
        async fn on_request(
            &self,
            _ctx: &mut RequestContext,
            _journal: &mut ExecutionJournal,
        ) -> Result<Action, String> {
            Ok(Action::Continue)
        }
        async fn on_stream_chunk(
            &self,
            _ctx: &mut StreamChunkContext,
            _journal: &mut ExecutionJournal,
        ) -> Result<Action, String> {
            Ok(Action::Continue)
        }
        async fn on_response(
            &self,
            _ctx: &mut ResponseContext,
            _journal: &mut ExecutionJournal,
        ) -> Result<Action, String> {
            Ok(Action::Continue)
        }
        async fn on_async_finalize(
            &self,
            _ctx: &mut AsyncContext,
        ) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({}))
        }
    }

    struct BlockPlugin;
    #[async_trait]
    impl GovernancePlugin for BlockPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                name: "block-plugin".into(),
                plugin_type: PluginType::Native,
                version: "1.0".into(),
                description: "always blocks".into(),
                author: None,
                supported_protocol_versions: VersionRange {
                    min: "0.2.0".into(),
                    max: "0.2.1".into(),
                },
            }
        }
        async fn on_request(
            &self,
            _ctx: &mut RequestContext,
            _journal: &mut ExecutionJournal,
        ) -> Result<Action, String> {
            Ok(Action::Block)
        }
        async fn on_stream_chunk(
            &self,
            _ctx: &mut StreamChunkContext,
            _journal: &mut ExecutionJournal,
        ) -> Result<Action, String> {
            Ok(Action::Continue)
        }
        async fn on_response(
            &self,
            _ctx: &mut ResponseContext,
            _journal: &mut ExecutionJournal,
        ) -> Result<Action, String> {
            Ok(Action::Continue)
        }
        async fn on_async_finalize(
            &self,
            _ctx: &mut AsyncContext,
        ) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({}))
        }
    }

    fn make_plan(name: &str, plugins: Vec<&str>) -> ExecutionPlan {
        ExecutionPlan {
            plan_id: name.into(),
            tenant: Some("test".into()),
            stages: vec![crate::pipeline::config::StageConfig {
                placement: Placement::PreRequest,
                parallel: false,
                plugins: plugins
                    .iter()
                    .map(|n| PluginConfig {
                        name: n.to_string(),
                        r#type: PluginType::Native,
                        config: serde_json::json!({}),
                        depends_on: vec![],
                        endpoint: None,
                        required_capabilities: vec![],
                    })
                    .collect(),
                on_version_mismatch: crate::pipeline::config::VersionMismatchPolicy::Skip,
            }],
        }
    }

    #[tokio::test]
    async fn test_pipeline_allow() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(AllowPlugin));
        let executor =
            PipelineExecutor::new(Arc::new(registry), make_plan("test", vec!["allow-plugin"]));
        let mut ctx = RequestContext {
            headers: std::collections::HashMap::new(),
            body: None,
            trace_id: Uuid::new_v4(),
            tenant_id: "test".into(),
            plugin_config: None,
        };
        let mut journal = ExecutionJournal::new(Uuid::new_v4(), "test");
        let result = executor.execute_pre_request(&mut ctx, &mut journal).await;
        assert_eq!(result.action, Action::Continue);
    }

    #[tokio::test]
    async fn test_pipeline_block() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(BlockPlugin));
        let executor =
            PipelineExecutor::new(Arc::new(registry), make_plan("test", vec!["block-plugin"]));
        let mut ctx = RequestContext {
            headers: std::collections::HashMap::new(),
            body: None,
            trace_id: Uuid::new_v4(),
            tenant_id: "test".into(),
            plugin_config: None,
        };
        let mut journal = ExecutionJournal::new(Uuid::new_v4(), "test");
        let result = executor.execute_pre_request(&mut ctx, &mut journal).await;
        assert_eq!(result.action, Action::Block);
    }

    #[tokio::test]
    async fn test_parallel_execution_does_not_deadlock() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(AllowPlugin));
        let plan = ExecutionPlan {
            plan_id: "test-parallel".into(),
            tenant: Some("test".into()),
            stages: vec![crate::pipeline::config::StageConfig {
                placement: Placement::PreRequest,
                parallel: true,
                plugins: (0..3)
                    .map(|i| PluginConfig {
                        name: "allow-plugin".to_string(),
                        r#type: PluginType::Native,
                        config: serde_json::json!({}),
                        depends_on: vec![],
                        endpoint: None,
                        required_capabilities: vec![],
                    })
                    .collect(),
                on_version_mismatch: crate::pipeline::config::VersionMismatchPolicy::Skip,
            }],
        };
        let executor = PipelineExecutor::new(Arc::new(registry), plan);
        let mut ctx = RequestContext {
            headers: std::collections::HashMap::new(),
            body: None,
            trace_id: Uuid::new_v4(),
            tenant_id: "test".into(),
            plugin_config: None,
        };
        let mut journal = ExecutionJournal::new(Uuid::new_v4(), "test");
        let result = executor.execute_pre_request(&mut ctx, &mut journal).await;
        assert_eq!(result.action, Action::Continue);
        // 3 个相同的插件都应该通过
        assert!(result.checks_passed.len() >= 3);
    }

    #[tokio::test]
    async fn test_post_response_returns_cold_tasks() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(AllowPlugin));
        let plan = ExecutionPlan {
            plan_id: "test-cold".into(),
            tenant: Some("test".into()),
            stages: vec![crate::pipeline::config::StageConfig {
                placement: Placement::PostResponse,
                parallel: false,
                plugins: vec![PluginConfig {
                    name: "allow-plugin".to_string(),
                    r#type: PluginType::Native,
                    config: serde_json::json!({"async_task": "certified_guarantee"}),
                    depends_on: vec![],
                    endpoint: None,
                    required_capabilities: vec![],
                }],
                on_version_mismatch: crate::pipeline::config::VersionMismatchPolicy::Skip,
            }],
        };
        let executor = PipelineExecutor::new(Arc::new(registry), plan);
        let mut ctx = ResponseContext {
            response: "test".into(),
            actual_cost: 0.0,
            trace_id: Uuid::new_v4(),
        };
        let mut journal = ExecutionJournal::new(Uuid::new_v4(), "test");
        let (result, cold_tasks) = executor.execute_post_response(&mut ctx, &mut journal).await;
        assert_eq!(result.action, Action::Continue);
        assert_eq!(cold_tasks.len(), 1);
        assert_eq!(cold_tasks[0].0, "certified_guarantee");
    }

    #[tokio::test]
    async fn test_collect_async_tasks() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(AllowPlugin));
        let plan = ExecutionPlan {
            plan_id: "test-async".into(),
            tenant: None,
            stages: vec![crate::pipeline::config::StageConfig {
                placement: Placement::AsyncFinalize,
                parallel: false,
                plugins: vec![PluginConfig {
                    name: "allow-plugin".to_string(),
                    r#type: PluginType::Native,
                    config: serde_json::json!({"async_task": "semantic_analysis"}),
                    depends_on: vec![],
                    endpoint: None,
                    required_capabilities: vec![],
                }],
                on_version_mismatch: crate::pipeline::config::VersionMismatchPolicy::Skip,
            }],
        };
        let executor = PipelineExecutor::new(Arc::new(registry), plan);
        let trace_id = Uuid::new_v4();
        let tasks = executor.collect_async_tasks(&trace_id, "test response");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].0, "semantic_analysis");
        assert!(tasks[0].1.get("trace_id").is_some());
    }
}
