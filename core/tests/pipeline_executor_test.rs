//! # Pipeline Executor 集成测试
//!
//! 测试流水线执行引擎的核心功能：插件查找、串行/并行执行、阻断语义。

use std::sync::Arc;
use veridactus_core::plugin::{
    PluginRegistry, GovernancePlugin, PluginMetadata, PluginType,
    RequestContext, AsyncContext, StreamChunkContext, ResponseContext,
};
use veridactus_core::types::{Action, VersionRange};
use veridactus_core::types::journal::ExecutionJournal;
use veridactus_core::pipeline::config::{ExecutionPlan, StageConfig, Placement, PluginConfig, VersionMismatchPolicy};
use veridactus_core::pipeline::executor::PipelineExecutor;
use async_trait::async_trait;
use uuid::Uuid;

/// 测试用插件：总是通过
struct PassPlugin;
#[async_trait]
impl GovernancePlugin for PassPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "test-pass".into(), plugin_type: PluginType::Native,
            version: "1.0".into(), description: "test".into(), author: None,
            supported_protocol_versions: VersionRange { min: "0.2.0".into(), max: "0.2.1".into() },
        }
    }
    async fn on_request(&self, _: &mut RequestContext, _: &mut ExecutionJournal) -> Result<Action, String> { Ok(Action::Continue) }
    async fn on_stream_chunk(&self, _: &mut StreamChunkContext, _: &mut ExecutionJournal) -> Result<Action, String> { Ok(Action::Continue) }
    async fn on_response(&self, _: &mut ResponseContext, _: &mut ExecutionJournal) -> Result<Action, String> { Ok(Action::Continue) }
    async fn on_async_finalize(&self, _: &mut AsyncContext) -> Result<serde_json::Value, String> { Ok(serde_json::json!({})) }
}

/// 测试用插件：总是阻断
struct BlockPlugin;
#[async_trait]
impl GovernancePlugin for BlockPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "test-block".into(), plugin_type: PluginType::Native,
            version: "1.0".into(), description: "blocks".into(), author: None,
            supported_protocol_versions: VersionRange { min: "0.2.0".into(), max: "0.2.1".into() },
        }
    }
    async fn on_request(&self, _: &mut RequestContext, _: &mut ExecutionJournal) -> Result<Action, String> { Ok(Action::Block) }
    async fn on_stream_chunk(&self, _: &mut StreamChunkContext, _: &mut ExecutionJournal) -> Result<Action, String> { Ok(Action::Block) }
    async fn on_response(&self, _: &mut ResponseContext, _: &mut ExecutionJournal) -> Result<Action, String> { Ok(Action::Block) }
    async fn on_async_finalize(&self, _: &mut AsyncContext) -> Result<serde_json::Value, String> { Ok(serde_json::json!({})) }
}

#[tokio::test]
async fn test_executor_serial_pass() {
    let mut registry = PluginRegistry::new();
    registry.register(Box::new(PassPlugin));

    let plan = ExecutionPlan {
        plan_id: "test-plan".into(),
        tenant: Some("test".into()),
        stages: vec![StageConfig {
            placement: Placement::PreRequest,
            parallel: false,
            plugins: vec![PluginConfig {
                name: "test-pass".into(),
                r#type: PluginType::Native,
                config: serde_json::json!({}),
                depends_on: vec![],
                endpoint: None,
                required_capabilities: vec![],
            }],
            on_version_mismatch: VersionMismatchPolicy::Skip,
        }],
    };

    let executor = PipelineExecutor::new(Arc::new(registry), plan);
    let mut ctx = RequestContext {
        headers: std::collections::HashMap::new(),
        body: Some("test".into()),
        trace_id: Uuid::new_v4(),
        tenant_id: "test".into(),
        plugin_config: None,
    };
    let mut journal = ExecutionJournal::new(Uuid::new_v4(), "test");

    let result = executor.execute_pre_request(&mut ctx, &mut journal).await;
    assert_eq!(result.action, Action::Continue);
    assert!(result.checks_passed.contains(&"test-pass".to_string()));
}

#[tokio::test]
async fn test_executor_serial_block() {
    let mut registry = PluginRegistry::new();
    registry.register(Box::new(BlockPlugin));

    let plan = ExecutionPlan {
        plan_id: "test-plan".into(),
        tenant: Some("test".into()),
        stages: vec![StageConfig {
            placement: Placement::PreRequest,
            parallel: false,
            plugins: vec![PluginConfig {
                name: "test-block".into(),
                r#type: PluginType::Native,
                config: serde_json::json!({}),
                depends_on: vec![],
                endpoint: None,
                required_capabilities: vec![],
            }],
            on_version_mismatch: VersionMismatchPolicy::Skip,
        }],
    };

    let executor = PipelineExecutor::new(Arc::new(registry), plan);
    let mut ctx = RequestContext {
        headers: std::collections::HashMap::new(),
        body: Some("test".into()),
        trace_id: Uuid::new_v4(),
        tenant_id: "test".into(),
        plugin_config: None,
    };
    let mut journal = ExecutionJournal::new(Uuid::new_v4(), "test");

    let result = executor.execute_pre_request(&mut ctx, &mut journal).await;
    assert_eq!(result.action, Action::Block);
    assert!(result.block_reason.is_some());
}

#[tokio::test]
async fn test_executor_plugin_not_found() {
    let registry = PluginRegistry::new(); // 空注册表

    let plan = ExecutionPlan {
        plan_id: "test-plan".into(),
        tenant: Some("test".into()),
        stages: vec![StageConfig {
            placement: Placement::PreRequest,
            parallel: false,
            plugins: vec![PluginConfig {
                name: "nonexistent".into(),
                r#type: PluginType::Native,
                config: serde_json::json!({}),
                depends_on: vec![],
                endpoint: None,
                required_capabilities: vec![],
            }],
            on_version_mismatch: VersionMismatchPolicy::Skip,
        }],
    };

    let executor = PipelineExecutor::new(Arc::new(registry), plan);
    let mut ctx = RequestContext {
        headers: std::collections::HashMap::new(),
        body: Some("test".into()),
        trace_id: Uuid::new_v4(),
        tenant_id: "test".into(),
        plugin_config: None,
    };
    let mut journal = ExecutionJournal::new(Uuid::new_v4(), "test");

    let result = executor.execute_pre_request(&mut ctx, &mut journal).await;
    assert_eq!(result.action, Action::Block);
    assert!(result.checks_failed.iter().any(|f| f.contains("nonexistent")));
}

#[tokio::test]
async fn test_executor_no_stages_returns_allow() {
    let registry = PluginRegistry::new();
    let plan = ExecutionPlan {
        plan_id: "empty".into(),
        tenant: Some("test".into()),
        stages: vec![],
    };

    let executor = PipelineExecutor::new(Arc::new(registry), plan);
    let mut ctx = RequestContext {
        headers: std::collections::HashMap::new(),
        body: Some("test".into()),
        trace_id: Uuid::new_v4(),
        tenant_id: "test".into(),
        plugin_config: None,
    };
    let mut journal = ExecutionJournal::new(Uuid::new_v4(), "test");

    let result = executor.execute_pre_request(&mut ctx, &mut journal).await;
    assert_eq!(result.action, Action::Continue);
}

#[tokio::test]
async fn test_executor_plugin_config_injection() {
    let mut registry = PluginRegistry::new();
    registry.register(Box::new(PassPlugin));

    let plan = ExecutionPlan {
        plan_id: "test-plan".into(),
        tenant: Some("test".into()),
        stages: vec![StageConfig {
            placement: Placement::PreRequest,
            parallel: false,
            plugins: vec![PluginConfig {
                name: "test-pass".into(),
                r#type: PluginType::Native,
                config: serde_json::json!({"limit": 0.05}),
                depends_on: vec![],
                endpoint: None,
                required_capabilities: vec![],
            }],
            on_version_mismatch: VersionMismatchPolicy::Skip,
        }],
    };

    let executor = PipelineExecutor::new(Arc::new(registry), plan);
    let mut ctx = RequestContext {
        headers: std::collections::HashMap::new(),
        body: Some("test".into()),
        trace_id: Uuid::new_v4(),
        tenant_id: "test".into(),
        plugin_config: None,
    };
    let mut journal = ExecutionJournal::new(Uuid::new_v4(), "test");

    let result = executor.execute_pre_request(&mut ctx, &mut journal).await;
    // 配置应被注入到 ctx.plugin_config
    assert_eq!(result.action, Action::Continue);
    // PassPlugin 不应修改 ctx，但 executor 应设置 plugin_config
    assert!(ctx.plugin_config.is_some(), "应注入插件配置");
    let cfg = ctx.plugin_config.unwrap();
    assert_eq!(cfg.get("limit").and_then(|v| v.as_f64()), Some(0.05));
}
