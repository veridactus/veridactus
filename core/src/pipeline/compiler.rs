//! # 流水线编译器
//!
//! 严格遵循 AI.md §6.3, §6.5。
//! 编译 DAG 执行计划，进行版本检查和能力协商。

use tracing::warn;

use crate::pipeline::config::*;
use crate::plugin::PluginRegistry;
#[cfg(test)]
use std::collections::HashMap;

/// 编译错误
#[derive(Debug)]
pub enum CompileError {
    /// 插件未找到
    PluginNotFound(String),
    /// 版本不匹配
    VersionMismatch {
        plugin: String,
        expected: String,
        actual: String,
    },
    /// 能力不足
    CapabilityMissing { plugin: String, cap: String },
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PluginNotFound(p) => write!(f, "Plugin '{}' not registered", p),
            Self::VersionMismatch {
                plugin,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "插件 '{}' 版本不匹配: 需要 {}, 运行时 {}",
                    plugin, expected, actual
                )
            }
            Self::CapabilityMissing { plugin, cap } => {
                write!(f, "插件 '{}' 需要能力 '{}' 但运行时不可用", plugin, cap)
            }
        }
    }
}

/// 流水线编译器（AI.md §6.5 compile_stage）
pub struct PipelineCompiler {
    /// 插件注册表
    registry: PluginRegistry,
}

impl PipelineCompiler {
    /// 创建新编译器
    pub fn new(registry: PluginRegistry) -> Self {
        Self { registry }
    }

    /// 编译执行计划
    ///
    /// 编译步骤：
    /// 1. 遍历每个阶段
    /// 2. 对每个阶段中的插件做版本兼容性检查
    /// 3. 对版本不兼容的插件应用降级策略
    /// 4. 检查运行时能力
    ///
    /// # 参数
    /// * `plan` - 输入的执行计划配置
    /// * `runtime` - 运行时能力
    ///
    /// # 返回
    /// 编译后的执行计划（不兼容插件可能被跳过）
    pub fn compile(
        &self,
        plan: &ExecutionPlan,
        runtime: &RuntimeCapabilities,
    ) -> Result<ExecutionPlan, CompileError> {
        let mut compiled_stages = Vec::new();

        for stage in &plan.stages {
            let mut compiled_plugins = Vec::new();

            for plugin_cfg in &stage.plugins {
                // 查找插件
                let plugin = match self.registry.find(&plugin_cfg.name) {
                    Some(p) => p,
                    None => {
                        return Err(CompileError::PluginNotFound(plugin_cfg.name.clone()));
                    }
                };

                // 版本兼容性检查（AI.md §6.5）
                let version_range = plugin.supported_protocol_versions();
                if !version_range.contains(&runtime.protocol_version) {
                    match stage.on_version_mismatch {
                        VersionMismatchPolicy::Skip => {
                            warn!(
                                "跳过插件 {}: 版本不兼容 (需要 {}-{}, 运行时 {})",
                                plugin_cfg.name,
                                version_range.min,
                                version_range.max,
                                runtime.protocol_version
                            );
                            continue;
                        }
                        VersionMismatchPolicy::Fail => {
                            return Err(CompileError::VersionMismatch {
                                plugin: plugin_cfg.name.clone(),
                                expected: format!("{}-{}", version_range.min, version_range.max),
                                actual: runtime.protocol_version.clone(),
                            });
                        }
                    }
                }

                // 运行时能力检查（AI.md §6.5）
                for cap in &plugin_cfg.required_capabilities {
                    if !runtime.contains(cap) {
                        warn!("插件 {} 需要能力 {} 但不可用", plugin_cfg.name, cap);
                    }
                }

                compiled_plugins.push(plugin_cfg.clone());
            }

            compiled_stages.push(StageConfig {
                placement: stage.placement.clone(),
                parallel: stage.parallel,
                plugins: compiled_plugins,
                on_version_mismatch: stage.on_version_mismatch.clone(),
            });
        }

        Ok(ExecutionPlan {
            plan_id: plan.plan_id.clone(),
            tenant: plan.tenant.clone(),
            stages: compiled_stages,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{GovernancePlugin, PluginMetadata};
    use crate::types::journal::ExecutionJournal;
    use crate::types::{Action, VersionRange};
    use async_trait::async_trait;

    macro_rules! make_plugin {
        ($name:ident, $plugin_name:expr) => {
            struct $name;
            #[async_trait]
            impl GovernancePlugin for $name {
                fn metadata(&self) -> PluginMetadata {
                    PluginMetadata {
                        name: $plugin_name.into(),
                        plugin_type: crate::plugin::PluginType::Native,
                        version: "1.0".into(),
                        description: "test".into(),
                        author: None,
                        supported_protocol_versions: VersionRange {
                            min: "0.2.0".into(),
                            max: "0.2.1".into(),
                        },
                    }
                }
                async fn on_request(
                    &self,
                    _ctx: &mut crate::plugin::RequestContext,
                    _journal: &mut ExecutionJournal,
                ) -> Result<Action, String> {
                    Ok(Action::Continue)
                }
                async fn on_stream_chunk(
                    &self,
                    _ctx: &mut crate::plugin::StreamChunkContext,
                    _journal: &mut ExecutionJournal,
                ) -> Result<Action, String> {
                    Ok(Action::Continue)
                }
                async fn on_response(
                    &self,
                    _ctx: &mut crate::plugin::ResponseContext,
                    _journal: &mut ExecutionJournal,
                ) -> Result<Action, String> {
                    Ok(Action::Continue)
                }
                async fn on_async_finalize(
                    &self,
                    _ctx: &mut crate::plugin::AsyncContext,
                ) -> Result<serde_json::Value, String> {
                    Ok(serde_json::json!({}))
                }
            }
        };
    }
    make_plugin!(BudgetGuardPlugin, "budget-guard");
    make_plugin!(AuthPlugin, "auth");

    #[test]
    fn test_compile_valid_plan() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(BudgetGuardPlugin));
        let compiler = PipelineCompiler::new(registry);

        let plan = ExecutionPlan::default_plan();
        let runtime = RuntimeCapabilities {
            protocol_version: "0.2.1".into(),
            ..Default::default()
        };

        let result = compiler.compile(&plan, &runtime);
        assert!(result.is_ok(), "编译应成功: {:?}", result.err());
    }

    #[test]
    fn test_version_skip_on_mismatch() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(BudgetGuardPlugin));
        let compiler = PipelineCompiler::new(registry);

        let plan = ExecutionPlan::default_plan();
        let runtime = RuntimeCapabilities {
            protocol_version: "0.3.0".into(),
            ..Default::default()
        };

        let result = compiler.compile(&plan, &runtime).unwrap();
        // 默认策略是 Skip, 所以插件应被跳过
        for stage in &result.stages {
            assert!(stage.plugins.is_empty(), "版本不兼容时应跳过所有插件");
        }
    }
}
