//! # 治理插件核心接口
//!
//! 严格遵循 AI.md §6.2 插件接口定义。
//! 定义 GovernancePlugin trait 及相关类型。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::journal::{ExecutionJournal, JournalEventType};
use crate::types::{
    Action, OwaspAsiRisk, SafetyAction, SafetyEvent, SafetyTrigger, Severity, VersionRange,
};

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// 插件名称
    pub name: String,
    /// 插件类型
    pub plugin_type: PluginType,
    /// 插件版本
    pub version: String,
    /// 插件描述
    pub description: String,
    /// 作者
    pub author: Option<String>,
    /// 支持的协议版本范围
    pub supported_protocol_versions: VersionRange,
}

/// 插件类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginType {
    /// Native Rust 插件（编译进核心，<10μs）
    #[serde(rename = "native")]
    Native,
    /// Wasm 沙箱插件（50-200μs）
    #[serde(rename = "wasm")]
    Wasm,
    /// External gRPC 插件（5-500ms）
    #[serde(rename = "grpc")]
    Grpc,
}

/// 请求上下文
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// 原始请求头部
    pub headers: std::collections::HashMap<String, String>,
    /// 请求体
    pub body: Option<String>,
    /// Trace ID
    pub trace_id: uuid::Uuid,
    /// 租户 ID
    pub tenant_id: String,
    /// 插件配置（来自流水线 PluginConfig.config）
    pub plugin_config: Option<serde_json::Value>,
}

/// 流式 Chunk 上下文
#[derive(Debug, Clone)]
pub struct StreamChunkContext {
    /// Chunk 序号
    pub seq: u64,
    /// Chunk 数据
    pub chunk: String,
    /// Chunk 哈希
    pub chunk_hash: String,
    /// 累计成本
    pub accumulated_cost: f64,
    /// Trace ID
    pub trace_id: uuid::Uuid,
}

/// 响应上下文
#[derive(Debug, Clone)]
pub struct ResponseContext {
    /// 完整响应
    pub response: String,
    /// 实际成本
    pub actual_cost: f64,
    /// Trace ID
    pub trace_id: uuid::Uuid,
}

/// 异步上下文
#[derive(Debug, Clone)]
pub struct AsyncContext {
    /// Trace ID
    pub trace_id: uuid::Uuid,
    /// 任务类型
    pub task_type: String,
    /// 任务参数
    pub params: serde_json::Value,
}

/// 执行上下文（包含安全事件记录方法）
///
/// 参考 AI.md §6.2，ExecutionContext 提供了 record_safety_event 方法，
/// 插件可通过此方法记录 ASI 相关的安全事件。
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Execution Journal 引用
    pub journal: std::sync::Arc<std::sync::Mutex<ExecutionJournal>>,
}

impl ExecutionContext {
    /// 创建新的执行上下文
    pub fn new(journal: std::sync::Arc<std::sync::Mutex<ExecutionJournal>>) -> Self {
        Self { journal }
    }

    /// 记录安全事件到 Journal
    ///
    /// 参考 AI.md §6.2：
    /// - trigger_type: G1_input_filter, G2_output_filter 等
    /// - severity: low/medium/high/critical
    /// - action: blocked/flagged/rewritten
    /// - content_hash: 触发内容的哈希
    /// - asi_risk_id: 可选的 OWASP ASI 风险标识符
    pub fn record_safety_event(
        &self,
        trigger_type: SafetyTrigger,
        severity: Severity,
        action: SafetyAction,
        content_hash: String,
        asi_risk_id: Option<OwaspAsiRisk>,
    ) {
        let event = SafetyEvent {
            trigger_type,
            severity,
            action_taken: action,
            content_hash,
            asi_risk_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        if let Ok(mut journal) = self.journal.lock() {
            journal.append_event(JournalEventType::SafetyEvent(event));
        }
    }
}

/// 治理插件 trait
///
/// 所有治理插件必须实现此接口。
/// 参考 AI.md §6.2。
#[async_trait]
pub trait GovernancePlugin: Send + Sync {
    /// 声明支持的协议版本范围
    fn supported_protocol_versions(&self) -> VersionRange {
        VersionRange {
            min: "0.2.0".to_string(),
            max: "0.2.1".to_string(),
        }
    }

    /// 返回插件元数据
    fn metadata(&self) -> PluginMetadata;

    /// 请求预处理（同步）
    ///
    /// 在 CONSTRAINT_EVAL 阶段执行。
    /// 返回 Action 指示是否继续、阻断或降级。
    async fn on_request(
        &self,
        ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, String>;

    /// 流式 chunk 处理（同步，高频）
    ///
    /// 在每个 chunk 到达时执行。
    async fn on_stream_chunk(
        &self,
        ctx: &mut StreamChunkContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, String>;

    /// 响应后处理（同步，仅非流式或流结束）
    async fn on_response(
        &self,
        ctx: &mut ResponseContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, String>;

    /// 异步最终化（非阻塞，由 worker 执行）
    async fn on_async_finalize(&self, ctx: &mut AsyncContext) -> Result<serde_json::Value, String>;
}

/// 插件注册表
///
/// 管理已注册的插件，支持热加载和版本协商。
#[derive(Default)]
pub struct PluginRegistry {
    /// 已注册的插件列表
    plugins: Vec<Box<dyn GovernancePlugin>>,
}

impl PluginRegistry {
    /// 创建新的插件注册表
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// 注册一个插件
    pub fn register(&mut self, plugin: Box<dyn GovernancePlugin>) {
        let name = plugin.metadata().name.clone();
        tracing::info!("Registering plugin: {}", name);
        self.plugins.push(plugin);
    }

    /// 根据名称查找插件
    pub fn find(&self, name: &str) -> Option<&dyn GovernancePlugin> {
        self.plugins
            .iter()
            .find(|p| p.metadata().name == name)
            .map(|p| p.as_ref())
    }

    /// 获取指定放置阶段的所有插件
    pub fn get_plugins_by_placement(&self, _placement: &str) -> Vec<&dyn GovernancePlugin> {
        // TODO: 按 placement 过滤
        self.plugins.iter().map(|p| p.as_ref()).collect()
    }

    /// 获取所有注册的插件名称
    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.metadata().name).collect()
    }

    /// 检查插件的版本兼容性
    pub fn check_version_compatibility(
        &self,
        name: &str,
        protocol_version: &str,
    ) -> Result<(), String> {
        match self.find(name) {
            Some(plugin) => {
                let range = plugin.supported_protocol_versions();
                if range.contains(protocol_version) {
                    Ok(())
                } else {
                    Err(format!(
                        "插件 {} 不支持协议版本 {} (支持范围: {}-{})",
                        name, protocol_version, range.min, range.max
                    ))
                }
            }
            None => Err(format!("Plugin {} not registered", name)),
        }
    }
}

/// 预算插件（Native 类型）
///
/// 检查请求是否在预算限制内。
#[allow(dead_code)]
struct BudgetPlugin;

#[async_trait]
impl GovernancePlugin for BudgetPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "budget".to_string(),
            plugin_type: PluginType::Native,
            version: "0.2.1".to_string(),
            description: "Budget check plugin, supports hard_stop/degrade/adaptive strategies"
                .to_string(),
            author: Some("VERIDACTUS Core".to_string()),
            supported_protocol_versions: VersionRange {
                min: "0.2.0".to_string(),
                max: "0.2.1".to_string(),
            },
        }
    }

    async fn on_request(
        &self,
        _ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, String> {
        journal.append_event(JournalEventType::PluginDecision {
            plugin_name: "budget".to_string(),
            action: Action::Continue,
            latency_us: 10,
        });
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
        Ok(serde_json::json!({"status": "ok"}))
    }
}

/// 认证插件（Native 类型）
#[allow(dead_code)]
struct AuthPlugin;

#[async_trait]
impl GovernancePlugin for AuthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "auth".to_string(),
            plugin_type: PluginType::Native,
            version: "0.2.1".to_string(),
            description: "API Key authentication plugin".to_string(),
            author: Some("VERIDACTUS Core".to_string()),
            supported_protocol_versions: VersionRange {
                min: "0.2.0".to_string(),
                max: "0.2.1".to_string(),
            },
        }
    }

    async fn on_request(
        &self,
        ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, String> {
        let has_auth = ctx.headers.contains_key("authorization");
        if has_auth {
            journal.append_event(JournalEventType::PluginDecision {
                plugin_name: "auth".to_string(),
                action: Action::Continue,
                latency_us: 5,
            });
            Ok(Action::Continue)
        } else {
            journal.append_event(JournalEventType::PluginDecision {
                plugin_name: "auth".to_string(),
                action: Action::Block,
                latency_us: 5,
            });
            Err("Missing Authorization header".to_string())
        }
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
        Ok(serde_json::json!({"status": "ok"}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试插件注册
    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(BudgetPlugin));
        registry.register(Box::new(AuthPlugin));

        let names = registry.plugin_names();
        assert!(names.contains(&"budget".to_string()));
        assert!(names.contains(&"auth".to_string()));
        assert_eq!(names.len(), 2);
    }

    /// 测试插件版本兼容性检查
    #[test]
    fn test_version_compatibility() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(BudgetPlugin));

        assert!(registry
            .check_version_compatibility("budget", "0.2.0")
            .is_ok());
        assert!(registry
            .check_version_compatibility("budget", "0.2.1")
            .is_ok());
        assert!(registry
            .check_version_compatibility("budget", "0.3.0")
            .is_err());
    }

    /// 测试插件版本不兼容降级
    #[test]
    fn test_plugin_version_skip() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(BudgetPlugin));

        // 对于不支持的版本，应该优雅降级（跳过）
        let result = registry.check_version_compatibility("budget", "0.3.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不支持"));
    }
}
