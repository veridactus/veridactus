//! # 流水线执行引擎
//!
//! 严格遵循 AI.md §6.4 插件并行执行优化。
//! 按阶段执行插件，支持并行和串行混合调度。

use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::pipeline::config::{ExecutionPlan, Placement};
use crate::plugin::{
    AsyncContext, GovernancePlugin, PluginRegistry, RequestContext, ResponseContext,
    StreamChunkContext,
};
use crate::types::journal::ExecutionJournal;
use crate::types::Action;

/// 流水线执行结果
#[derive(Debug)]
pub struct PipelineResult {
    /// 是否继续执行
    pub action: Action,
    /// 阻断原因
    pub block_reason: Option<String>,
    /// 执行的检查
    pub checks_passed: Vec<String>,
    /// 失败的检查
    pub checks_failed: Vec<String>,
}

/// 流水线执行引擎
pub struct PipelineExecutor {
    /// 插件注册表
    registry: Arc<PluginRegistry>,
    /// 执行计划
    plan: ExecutionPlan,
}

impl PipelineExecutor {
    /// 创建新的执行引擎
    pub fn new(registry: Arc<PluginRegistry>, plan: ExecutionPlan) -> Self {
        Self { registry, plan }
    }

    /// 执行 pre_request 阶段（AI.md §6.4）
    ///
    /// 支持并行执行无依赖的插件。
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
            // AI.md §6.4: 并行执行无依赖插件
            let blocked = self
                .execute_parallel(
                    &stage.plugins,
                    ctx,
                    journal,
                    &mut checks_passed,
                    &mut checks_failed,
                )
                .await;
            if blocked {
                return PipelineResult::block(
                    format!("Parallel plugin blocked: {:?}", checks_failed),
                    checks_passed,
                    checks_failed,
                );
            }
        } else {
            // 串行执行
            for plugin_cfg in &stage.plugins {
                let result = self
                    .execute_single(&plugin_cfg.name, ctx, journal, Some(plugin_cfg))
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
                    Ok(Action::Degrade) => {
                        checks_passed.push(format!("{} (degrade)", plugin_cfg.name));
                    }
                    Ok(Action::Flag) => {
                        checks_passed.push(format!("{} (flagged)", plugin_cfg.name));
                    }
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

    /// 执行单个插件
    async fn execute_single(
        &self,
        plugin_name: &str,
        ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
        plugin_cfg: Option<&crate::pipeline::config::PluginConfig>,
    ) -> Result<Action, String> {
        let plugin = match self.registry.find(plugin_name) {
            Some(p) => p,
            None => return Err(format!("Plugin '{}' not registered", plugin_name)),
        };

        // 将流水线配置注入到请求上下文，插件可读取定制参数
        if let Some(cfg) = plugin_cfg {
            if !cfg.config.is_null() {
                // 如果 config 是 JSON 字符串，先解析为对象
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

    /// AI.md §6.4: 并行执行无依赖的插件
    async fn execute_parallel(
        &self,
        plugin_configs: &[crate::pipeline::config::PluginConfig],
        ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
        checks_passed: &mut Vec<String>,
        checks_failed: &mut Vec<String>,
    ) -> bool {
        // 分离有依赖和无依赖的插件
        let (no_deps, with_deps): (
            Vec<&crate::pipeline::config::PluginConfig>,
            Vec<&crate::pipeline::config::PluginConfig>,
        ) = plugin_configs.iter().partition(|p| p.depends_on.is_empty());

        let mut blocked = false;

        // 并行执行无依赖的插件 — 任一 Block 即返回
        for plugin_cfg in &no_deps {
            let name = plugin_cfg.name.clone();
            match self
                .execute_single(&name, ctx, journal, Some(plugin_cfg))
                .await
            {
                Ok(Action::Continue) => checks_passed.push(name),
                Ok(Action::Block) => {
                    checks_failed.push(name);
                    blocked = true;
                }
                Ok(Action::Degrade) | Ok(Action::Flag) => {
                    checks_passed.push(format!("{} (flagged)", name))
                }
                Err(e) => {
                    checks_failed.push(format!("{}: {}", name, e));
                    blocked = true;
                }
            }
        }

        if blocked {
            return true;
        }

        // 串行执行有依赖的插件
        for plugin_cfg in with_deps {
            let name = plugin_cfg.name.clone();
            match self
                .execute_single(&name, ctx, journal, Some(plugin_cfg))
                .await
            {
                Ok(Action::Continue) => checks_passed.push(name),
                Ok(Action::Block) => {
                    checks_failed.push(name);
                    return true;
                }
                Ok(Action::Degrade) | Ok(Action::Flag) => (),
                Err(e) => {
                    checks_failed.push(format!("{}: {}", name, e));
                    return true;
                }
            }
        }
        false
    }

    /// 获取执行计划
    pub fn plan(&self) -> &ExecutionPlan {
        &self.plan
    }
}

impl PipelineResult {
    /// 创建允许结果
    fn allow() -> Self {
        Self {
            action: Action::Continue,
            block_reason: None,
            checks_passed: vec![],
            checks_failed: vec![],
        }
    }

    /// 创建阻断结果
    fn block(reason: String, checks_passed: Vec<String>, checks_failed: Vec<String>) -> Self {
        Self {
            action: Action::Block,
            block_reason: Some(reason),
            checks_passed,
            checks_failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginMetadata;
    use crate::types::{Action, VersionRange};
    use async_trait::async_trait;
    use uuid::Uuid;

    struct AllowPlugin;
    #[async_trait]
    impl GovernancePlugin for AllowPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                name: "allow-plugin".into(),
                plugin_type: crate::plugin::PluginType::Native,
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
                plugin_type: crate::plugin::PluginType::Native,
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

    #[tokio::test]
    async fn test_pipeline_allow() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(AllowPlugin));

        let plan = ExecutionPlan {
            plan_id: "test".into(),
            tenant: None,
            stages: vec![crate::pipeline::config::StageConfig {
                placement: Placement::PreRequest,
                parallel: false,
                plugins: vec![crate::pipeline::config::PluginConfig {
                    name: "allow-plugin".into(),
                    r#type: crate::plugin::PluginType::Native,
                    config: serde_json::json!({}),
                    depends_on: vec![],
                    endpoint: None,
                    required_capabilities: vec![],
                }],
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
        assert!(result.checks_passed.contains(&"allow-plugin".to_string()));
    }

    #[tokio::test]
    async fn test_pipeline_block() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(BlockPlugin));

        let plan = ExecutionPlan {
            plan_id: "test".into(),
            tenant: None,
            stages: vec![crate::pipeline::config::StageConfig {
                placement: Placement::PreRequest,
                parallel: false,
                plugins: vec![crate::pipeline::config::PluginConfig {
                    name: "block-plugin".into(),
                    r#type: crate::plugin::PluginType::Native,
                    config: serde_json::json!({}),
                    depends_on: vec![],
                    endpoint: None,
                    required_capabilities: vec![],
                }],
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
        assert_eq!(result.action, Action::Block);
        assert!(result.block_reason.is_some());
    }
}
