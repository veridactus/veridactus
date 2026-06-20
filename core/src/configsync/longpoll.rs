//! # 配置长轮询同步
//!
//! 数据平面通过长轮询从控制平面拉取配置变更。
//! 遵循 AI.md §2.1 架构图: ConfigSync → Long Poll → HttpServer
//!
//! 实现方式: 数据平面定时请求控制平面的 /api/v1/config/poll 端点，
//! 如果配置版本没有变化则保持连接 (长轮询)，有变化时立即返回。

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

/// 配置版本号
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigVersion {
    /// 流水线配置版本
    pub pipeline_version: u64,
    /// 策略配置版本
    pub policy_version: u64,
    /// 插件配置版本
    pub plugin_version: u64,
}

impl Default for ConfigVersion {
    fn default() -> Self {
        Self { pipeline_version: 0, policy_version: 0, plugin_version: 0 }
    }
}

/// 配置变更载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPayload {
    /// 变更类型
    pub change_type: String,
    /// 变更数据
    pub data: serde_json::Value,
    /// 新版本号
    pub version: ConfigVersion,
}

/// 配置同步客户端
pub struct ConfigSyncClient {
    /// 控制平面 URL
    control_plane_url: String,
    /// 当前版本
    current_version: ConfigVersion,
    /// HTTP 客户端
    client: Client,
}

impl ConfigSyncClient {
    /// 创建新的配置同步客户端
    pub fn new(control_plane_url: impl Into<String>) -> Self {
        Self {
            control_plane_url: control_plane_url.into(),
            current_version: ConfigVersion::default(),
            client: Client::builder()
                .timeout(Duration::from_secs(60)) // 长轮询超时
                .build()
                .unwrap(),
        }
    }

    /// 执行长轮询拉取
    ///
    /// 请求控制平面，如果有配置变更则返回 Some。
    /// 如果无变更，会保持连接直到超时。
    pub async fn poll(&mut self) -> Result<Option<ConfigPayload>, String> {
        let url = format!(
            "{}/api/v1/config/poll?v={}&pv={}&plv={}",
            self.control_plane_url.trim_end_matches('/'),
            self.current_version.pipeline_version,
            self.current_version.policy_version,
            self.current_version.plugin_version,
        );

        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let payload: ConfigPayload = resp
                        .json()
                        .await
                        .map_err(|e| format!("配置解析失败: {}", e))?;

                    // 更新本地版本
                    self.current_version = payload.version.clone();

                    info!("配置已更新: {:?}", payload.change_type);
                    Ok(Some(payload))
                } else if resp.status().as_u16() == 304 {
                    // 304 Not Modified — 无变更
                    Ok(None)
                } else {
                    Err(format!("配置请求失败: {}", resp.status()))
                }
            }
            Err(e) => {
                // 连接超时 = 无变更（长轮询的正常行为）
                if e.is_timeout() {
                    Ok(None)
                } else {
                    warn!("配置同步连接失败: {}", e);
                    Err(format!("配置同步失败: {}", e))
                }
            }
        }
    }

    /// 获取当前版本
    pub fn current_version(&self) -> &ConfigVersion {
        &self.current_version
    }
}

/// 配置同步循环 (在后台运行)
///
/// # 参数
/// * `client` - 配置同步客户端
/// * `on_update` - 配置变更回调
pub async fn config_sync_loop<F>(
    mut client: ConfigSyncClient,
    on_update: F,
) where
    F: Fn(ConfigPayload),
{
    loop {
        match client.poll().await {
            Ok(Some(payload)) => {
                on_update(payload);
            }
            Ok(None) => {
                // 无变更，继续轮询
            }
            Err(e) => {
                warn!("配置同步错误: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
