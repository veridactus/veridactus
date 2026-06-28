//! # 配置拉取客户端
//!
//! 数据平面通过长轮询从控制平面拉取配置变更。

use futures::Future;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::pipeline::config::ExecutionPlan;
use crate::store::facade::ConfigStoreAdapter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersions {
    #[serde(rename = "pipeline_version")]
    pub pipeline_version: i64,
    #[serde(rename = "policy_version")]
    pub policy_version: i64,
    #[serde(rename = "plugin_version")]
    pub plugin_version: i64,
    #[serde(rename = "storage_version")]
    pub storage_version: Option<i64>,
    #[serde(rename = "model_version")]
    pub model_version: Option<i64>,
}

impl Default for ConfigVersions {
    fn default() -> Self {
        Self {
            pipeline_version: 0,
            policy_version: 0,
            plugin_version: 0,
            storage_version: Some(0),
            model_version: Some(0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangePayload {
    #[serde(rename = "change_type")]
    pub change_type: String,
    pub data: serde_json::Value,
    pub version: ConfigVersions,
    /// 附带的最新模型列表（pipeline 变更时一起推送，避免额外 poll）
    #[serde(default)]
    pub models: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPlaneStorageConfig {
    pub id: String,
    pub name: String,
    #[serde(rename = "postgres_url")]
    pub postgres_url: String,
    #[serde(rename = "redis_url")]
    pub redis_url: String,
    #[serde(rename = "s3_endpoint")]
    pub s3_endpoint: Option<String>,
    #[serde(rename = "s3_bucket")]
    pub s3_bucket: Option<String>,
    #[serde(rename = "s3_access_key")]
    pub s3_access_key: Option<String>,
    #[serde(rename = "s3_secret_key")]
    pub s3_secret_key: Option<String>,
    #[serde(rename = "is_active")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    #[serde(rename = "upstream_url")]
    pub upstream_url: String,
    #[serde(rename = "upstream_model")]
    pub upstream_model: String,
    #[serde(rename = "api_key")]
    pub api_key: Option<String>,
    #[serde(rename = "api_key_header")]
    pub api_key_header: Option<String>,
    #[serde(rename = "use_proxy")]
    pub use_proxy: bool,
    #[serde(rename = "proxy_url")]
    pub proxy_url: Option<String>,
    #[serde(rename = "is_default")]
    pub is_default: bool,
    #[serde(rename = "supported_versions")]
    pub supported_versions: Option<Vec<String>>,
    pub status: String,
}

pub struct ConfigPullClient {
    control_plane_url: String,
    current_version: Arc<RwLock<ConfigVersions>>,
    http_client: reqwest::Client,
    config_store: Arc<ConfigStoreAdapter>,
    model_config_updater: Option<
        Arc<dyn Fn(Vec<ModelConfig>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
    >,
}

impl ConfigPullClient {
    pub fn new(
        control_plane_url: impl Into<String>,
        config_store: Arc<ConfigStoreAdapter>,
    ) -> Self {
        Self {
            control_plane_url: control_plane_url.into(),
            current_version: Arc::new(RwLock::new(ConfigVersions::default())),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(70))
                .no_proxy()
                .build()
                .unwrap(),
            config_store,
            model_config_updater: None,
        }
    }

    pub fn with_model_updater<F>(mut self, updater: F) -> Self
    where
        F: Fn(Vec<ModelConfig>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
    {
        self.model_config_updater = Some(Arc::new(updater));
        self
    }

    pub async fn pull(&self) -> Result<Option<ConfigChangePayload>, String> {
        let version = self.current_version.read().await;
        let url = format!(
            "{}/api/v1/config/poll?pv={}&plv={}&sv={}&mv={}",
            self.control_plane_url.trim_end_matches('/'),
            version.pipeline_version,
            version.plugin_version,
            version.storage_version.unwrap_or(0),
            version.model_version.unwrap_or(0),
        );
        drop(version);

        match self.http_client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let payload: ConfigChangePayload = resp
                        .json()
                        .await
                        .map_err(|e| format!("配置解析失败: {}", e))?;

                    *self.current_version.write().await = payload.version.clone();

                    self.apply_config_change(&payload).await?;

                    info!("配置已更新: change_type={}", payload.change_type);
                    Ok(Some(payload))
                } else if resp.status().as_u16() == 304 {
                    Ok(None)
                } else {
                    Err(format!("配置请求失败: {}", resp.status()))
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    Ok(None)
                } else {
                    warn!("配置同步连接失败: {}", e);
                    Err(format!("配置同步失败: {}", e))
                }
            }
        }
    }

    async fn apply_config_change(&self, payload: &ConfigChangePayload) -> Result<(), String> {
        match payload.change_type.as_str() {
            "pipeline" => {
                if let Some(pipelines) = payload.data.as_array() {
                    for pipeline_data in pipelines {
                        if let Ok(plan) =
                            serde_json::from_value::<PipelineData>(pipeline_data.clone())
                        {
                            let execution_plan = self.convert_to_execution_plan(&plan);
                            let tenant = plan.tenant.unwrap_or_default();
                            self.config_store.set_pipeline(&tenant, execution_plan);
                        }
                    }
                }
                // 同时处理附带的最新模型列表（避免额外 poll）
                if let Some(models) = payload.models.as_ref().and_then(|m| m.as_array()) {
                    let mut model_configs = Vec::new();
                    for model_data in models {
                        if let Ok(model_config) =
                            serde_json::from_value::<ModelConfig>(model_data.clone())
                        {
                            model_configs.push(model_config);
                        }
                    }
                    let model_count = model_configs.len();
                    if let Some(updater) = &self.model_config_updater {
                        updater(model_configs).await;
                    }
                    info!("附带模型更新: {} models", model_count);
                }
            }
            "model" => {
                if let Some(models) = payload.data.as_array() {
                    let mut model_configs = Vec::new();
                    for model_data in models {
                        if let Ok(model_config) =
                            serde_json::from_value::<ModelConfig>(model_data.clone())
                        {
                            model_configs.push(model_config);
                        }
                    }
                    let model_count = model_configs.len();
                    if let Some(updater) = &self.model_config_updater {
                        updater(model_configs).await;
                    }
                    info!("收到模型配置更新，共 {} models", model_count);
                }
            }
            "plugin" | "storage" => {
                info!("收到 {} 配置更新 (data={})", payload.change_type,
                    serde_json::to_string(&payload.data).unwrap_or_default().chars().take(100).collect::<String>());
                // 🔧 Phase 2.5: plugin/storage config 更新记录
                // 后续 Phase 3 实现动态 Sidecar/Wasm 插件热加载
            }
            _ => {
                warn!("未知配置变更类型: {}", payload.change_type);
            }
        }
        Ok(())
    }

    fn convert_to_execution_plan(&self, pipeline: &PipelineData) -> ExecutionPlan {
        use crate::pipeline::config::{Placement, PluginConfig, StageConfig};
        use crate::plugin::PluginType;

        ExecutionPlan {
            plan_id: pipeline.plan_id.clone(),
            tenant: pipeline.tenant.clone(),
            stages: pipeline
                .stages
                .iter()
                .map(|s| StageConfig {
                    placement: match s.placement.as_str() {
                        "pre_request" => Placement::PreRequest,
                        "streaming" => Placement::Streaming,
                        "post_response" => Placement::PostResponse,
                        "async_finalize" | "async" => Placement::AsyncFinalize,
                        _ => Placement::PreRequest,
                    },
                    parallel: s.parallel,
                    plugins: s
                        .plugins
                        .iter()
                        .map(|p| PluginConfig {
                            name: p.name.clone(),
                            r#type: match p.ptype.as_str() {
                                "native" => PluginType::Native,
                                "wasm" => PluginType::Wasm,
                                "sidecar" => PluginType::Sidecar,
                                "grpc" => PluginType::Grpc,
                                _ => PluginType::Native,
                            },
                            config: serde_json::from_str(&p.config).unwrap_or_default(),
                            depends_on: p.depends_on.clone(),
                            endpoint: p.endpoint.clone(),
                            required_capabilities: p.capabilities.clone(),
                        })
                        .collect(),
                    on_version_mismatch: crate::pipeline::config::VersionMismatchPolicy::Skip,
                })
                .collect(),
        }
    }

    pub async fn start_poll_loop(&self) {
        let client = self.clone();
        tokio::spawn(async move {
            // 心跳降级轮询: 60秒间隔
            let mut ticker = interval(Duration::from_secs(60));
            loop {
                ticker.tick().await;
                match client.pull().await {
                    Ok(Some(_)) => info!("配置变更已应用（心跳降级）"),
                    Ok(None) => {}
                    Err(e) => error!("配置拉取失败: {}", e),
                }
            }
        });
    }

    pub async fn get_current_version(&self) -> ConfigVersions {
        self.current_version.read().await.clone()
    }
}

impl Clone for ConfigPullClient {
    fn clone(&self) -> Self {
        Self {
            control_plane_url: self.control_plane_url.clone(),
            current_version: self.current_version.clone(),
            http_client: self.http_client.clone(),
            config_store: self.config_store.clone(),
            model_config_updater: self.model_config_updater.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PipelineData {
    #[serde(rename = "plan_id")]
    pub plan_id: String,
    pub tenant: Option<String>,
    pub stages: Vec<StageData>,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StageData {
    pub placement: String,
    pub parallel: bool,
    pub plugins: Vec<PluginData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginData {
    pub name: String,
    #[serde(rename = "type")]
    pub ptype: String,       // "native" | "sidecar" | "wasm" | "grpc"
    pub config: String,
    #[serde(default)]
    pub endpoint: Option<String>,  // 🔧 Sidecar/Wasm 端点 URL
    #[serde(default)]
    pub depends_on: Vec<String>,   // 🔧 依赖的插件名
    #[serde(default)]
    pub capabilities: Vec<String>,  // 所需能力
    #[serde(default)]
    pub enabled: Option<bool>,
}
