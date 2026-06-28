//! # SidecarPlugin — 同步 HTTP 桥接 Python/外部服务
//!
//! 将 Python ML 库 (numpy/scipy/sklearn/transformers) 等外部服务的能力，
//! 通过 HTTP REST 桥接到 GovernancePlugin trait。
//!
//! 设计原则:
//!   - HTTP REST (非 gRPC) — 降低复杂度，Python FastAPI 天然支持
//!   - 统一端点 /plugin/execute — 单一 Sidecar 进程托管多个插件
//!   - JSON 序列化 — 跨语言最简单兼容
//!   - 超时控制 — 5s 默认，可配置
//!   - 失败降级 — HTTP 调用失败时返回 Action::Flag (不阻断)
//!
//! 架构:
//!   Rust PipelineExecutor
//!     ↓ SidecarPlugin::on_request()
//!     ↓ POST http://sidecar:8001/plugin/execute
//!     ↓ {"plugin":"my-python-ml-plugin","stage":"pre_request","context":{...}}
//!   Python Sidecar
//!     ↓ PluginRouter.dispatch("my-python-ml-plugin")
//!     ↓ MyPythonMLPlugin.execute(context) → Action
//!     ↓ {action:"Continue"|"Block"|"Flag", ...}

use async_trait::async_trait;
use std::time::Duration;
use tracing::{info, warn};

use crate::plugin::{
    AsyncContext, GovernancePlugin, PluginMetadata, PluginType, RequestContext, ResponseContext,
    StreamChunkContext,
};
use crate::types::journal::ExecutionJournal;
use crate::types::Action;

/// Sidecar 插件 — HTTP REST 远程调用
///
/// 每实例化一个 SidecarPlugin 代表一个远程 Python 插件。
/// 多个 SidecarPlugin 可共享同一个 Sidecar 端点（只要路径不同）。
pub struct SidecarPlugin {
    /// 插件元数据
    metadata: PluginMetadata,
    /// Sidecar 端点 URL (如 http://127.0.0.1:8001/plugin/execute)
    endpoint: String,
    /// HTTP 客户端 (复用连接池)
    client: reqwest::Client,
    /// 请求超时
    timeout: Duration,
    /// 失败时是否阻断 (默认 false = Flag)
    fail_on_error: bool,
}

impl SidecarPlugin {
    /// 创建新的 Sidecar 插件实例
    ///
    /// # 参数
    /// * `name` - 插件名称 (对应 Python 侧 PluginRouter 的 key)
    /// * `endpoint` - Sidecar 端点 URL
    /// * `description` - 插件描述
    /// * `timeout` - 请求超时
    pub fn new(
        name: impl Into<String>,
        endpoint: impl Into<String>,
        description: impl Into<String>,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            metadata: PluginMetadata {
                name: name.into(),
                plugin_type: PluginType::Sidecar,
                version: "1.0".into(),
                description: description.into(),
                author: None,
                supported_protocol_versions: crate::types::VersionRange {
                    min: "0.2.0".into(),
                    max: "0.3.0".into(),
                },
            },
            endpoint: endpoint.into(),
            client: reqwest::Client::builder()
                .pool_max_idle_per_host(10)
                .build()
                .unwrap_or_default(),
            timeout: timeout.unwrap_or(Duration::from_secs(5)),
            fail_on_error: false,
        }
    }

    /// 设置失败时是否阻断请求 (默认仅 Flag)
    pub fn with_fail_on_error(mut self, fail: bool) -> Self {
        self.fail_on_error = fail;
        self
    }

    /// 核心远程调用 — 向 Sidecar 发送 JSON 请求
    async fn call_sidecar(
        &self,
        stage: &str,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let payload = serde_json::json!({
            "plugin": self.metadata.name,
            "stage": stage,
            "request": request,
        });

        match self
            .client
            .post(&self.endpoint)
            .json(&payload)
            .timeout(self.timeout)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| format!("Sidecar response parse error: {}", e))?;

                if status.is_success() {
                    info!(
                        "Sidecar plugin {} ({}) returned: {:?}",
                        self.metadata.name, stage, body
                    );
                    Ok(body)
                } else {
                    let err_msg = body
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("unknown");
                    Err(format!("Sidecar error ({}): {}", status, err_msg))
                }
            }
            Err(e) => {
                let is_timeout = e.is_timeout();
                warn!(
                    "Sidecar plugin {} call failed: {} (timeout={})",
                    self.metadata.name, e, is_timeout
                );
                Err(if is_timeout {
                    format!("Sidecar timeout after {:?}", self.timeout)
                } else {
                    format!("Sidecar unreachable: {}", e)
                })
            }
        }
    }

    /// 解析 Sidecar 响应中的 action
    fn parse_action(body: &serde_json::Value) -> Action {
        match body.get("action").and_then(|a| a.as_str()) {
            Some("block") | Some("Block") => Action::Block,
            Some("flag") | Some("Flag") => Action::Flag,
            Some("degrade") | Some("Degrade") => Action::Degrade,
            _ => Action::Continue,
        }
    }
}

#[async_trait]
impl GovernancePlugin for SidecarPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    async fn on_request(
        &self,
        ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, String> {
        let request = serde_json::json!({
            "headers": ctx.headers,
            "body": ctx.body,
            "trace_id": ctx.trace_id.to_string(),
            "tenant_id": ctx.tenant_id,
            "plugin_config": ctx.plugin_config,
        });

        match self.call_sidecar("pre_request", request).await {
            Ok(body) => Ok(Self::parse_action(&body)),
            Err(e) => {
                if self.fail_on_error {
                    Err(e)
                } else {
                    warn!(
                        "Sidecar plugin {} pre_request failed (flagged): {}",
                        self.metadata.name, e
                    );
                    Ok(Action::Flag)
                }
            }
        }
    }

    async fn on_stream_chunk(
        &self,
        ctx: &mut StreamChunkContext,
        _journal: &mut ExecutionJournal,
    ) -> Result<Action, String> {
        // ⚠️ 警告: Sidecar 插件在 streaming 阶段不建议使用 (延迟太高)
        // 仅提供 fallback 实现
        let request = serde_json::json!({
            "seq": ctx.seq,
            "chunk": ctx.chunk,
            "chunk_hash": ctx.chunk_hash,
            "accumulated_cost": ctx.accumulated_cost,
            "trace_id": ctx.trace_id.to_string(),
        });

        match self.call_sidecar("streaming", request).await {
            Ok(body) => Ok(Self::parse_action(&body)),
            Err(_) => Ok(Action::Continue), // streaming 阶段失败不阻断
        }
    }

    async fn on_response(
        &self,
        ctx: &mut ResponseContext,
        _journal: &mut ExecutionJournal,
    ) -> Result<Action, String> {
        let request = serde_json::json!({
            "response": ctx.response,
            "actual_cost": ctx.actual_cost,
            "trace_id": ctx.trace_id.to_string(),
        });

        match self.call_sidecar("post_response", request).await {
            Ok(body) => Ok(Self::parse_action(&body)),
            Err(e) => {
                if self.fail_on_error {
                    Err(e)
                } else {
                    warn!(
                        "Sidecar plugin {} post_response failed (flagged): {}",
                        self.metadata.name, e
                    );
                    Ok(Action::Flag)
                }
            }
        }
    }

    async fn on_async_finalize(&self, ctx: &mut AsyncContext) -> Result<serde_json::Value, String> {
        let request = serde_json::json!({
            "trace_id": ctx.trace_id.to_string(),
            "task_type": ctx.task_type,
            "params": ctx.params,
        });

        match self.call_sidecar("async_finalize", request).await {
            Ok(body) => Ok(body),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidecar_plugin_metadata() {
        let plugin = SidecarPlugin::new(
            "test-python-plugin",
            "http://127.0.0.1:8001/plugin/execute",
            "Test Python ML plugin",
            None,
        );
        let meta = plugin.metadata();
        assert_eq!(meta.name, "test-python-plugin");
        assert_eq!(meta.plugin_type, PluginType::Sidecar);
    }

    #[test]
    fn test_parse_action_continue() {
        assert_eq!(
            SidecarPlugin::parse_action(&serde_json::json!({"action": "continue"})),
            Action::Continue
        );
    }

    #[test]
    fn test_parse_action_block() {
        assert_eq!(
            SidecarPlugin::parse_action(&serde_json::json!({"action": "Block"})),
            Action::Block
        );
    }

    #[test]
    fn test_parse_action_flag() {
        assert_eq!(
            SidecarPlugin::parse_action(&serde_json::json!({"action": "flag"})),
            Action::Flag
        );
    }

    #[test]
    fn test_parse_action_default() {
        assert_eq!(
            SidecarPlugin::parse_action(&serde_json::json!({})),
            Action::Continue
        );
    }
}
