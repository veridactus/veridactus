//! # VERIDACTUS Key 路由解析客户端 (Phase 2)
//!
//! Rust 数据面在接收请求时，通过 HTTP 调用 Go 控制面的
//! `/internal/resolve-key` 接口，解析 Virtual Key 并获取
//! 解密后的真实 LLM Provider Key（仅内存，不持久化）。
//!
//! 参考: SPECIFICATION.md §2.1.3

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Key 解析请求
#[derive(Debug, Serialize)]
pub struct KeyResolveRequest {
    pub virtual_key_hash: String,
    pub model: String,
}

/// Key 解析响应
#[derive(Debug, Deserialize)]
pub struct KeyResolveResponse {
    pub resolved: bool,
    #[serde(default)]
    pub provider_key: String,
    #[serde(default)]
    pub workspace_id: String,
    #[serde(default)]
    pub budget_remaining_micro: i64,
    #[serde(default)]
    pub rate_limit_rpm: i32,
    #[serde(default)]
    pub rate_limit_tpm: i32,
    #[serde(default)]
    pub error: String,
}

/// Key 解析客户端
///
/// 缓存控制面 URL，每次请求时调用解析接口。
/// 生产环境应加本地 LRU 缓存减少控制面调用。
pub struct KeyResolver {
    client: HttpClient,
    control_plane_url: String,
    /// 本地缓存：optionally in the future
    _cache_enabled: bool,
}

impl KeyResolver {
    /// 创建 Key 解析客户端
    pub fn new(control_plane_url: String, http_client: HttpClient) -> Self {
        Self {
            client: http_client,
            control_plane_url,
            _cache_enabled: false,
        }
    }

    /// 解析 Virtual Key 并返回解密后的 Provider Key 和上下文
    ///
    /// # 参数
    /// * `virtual_key` - 用户提交的 Virtual Key（如 vd-abc123...）
    /// * `model` - 请求的模型名称
    ///
    /// # 返回
    /// 解析结果，包含真实 LLM Key（如果解析成功）
    pub async fn resolve(
        &self,
        virtual_key: &str,
        model: &str,
    ) -> Result<KeyResolveResponse, String> {
        let started = Instant::now();
        let key_hash = hash_key(virtual_key);

        let request = KeyResolveRequest {
            virtual_key_hash: key_hash,
            model: model.to_string(),
        };

        let url = format!("{}/internal/resolve-key", self.control_plane_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Key resolve request failed: {}", e))?;

        let elapsed = started.elapsed();
        let status = response.status();

        if !status.is_success() {
            return Err(format!(
                "Key resolve HTTP {}: took {:?}",
                status.as_u16(),
                elapsed
            ));
        }

        let result: KeyResolveResponse = response
            .json()
            .await
            .map_err(|e| format!("Key resolve parse error: {}", e))?;

        if !result.resolved {
            warn!(
                "Key NOT resolved: key_prefix={}, model={}, error={}, took={:?}",
                &virtual_key[..virtual_key.len().min(10)],
                model,
                result.error,
                elapsed
            );
            return Err(format!("Key not resolved: {}", result.error));
        }

        info!(
            "Key RESOLVED: workspace={}, budget_remaining=${:.6}, took={:?}",
            result.workspace_id,
            result.budget_remaining_micro as f64 / 1_000_000.0,
            elapsed
        );

        Ok(result)
    }

    /// 快速检查：仅验证 Key 是否有效（不返回解密后的 Provider Key）
    pub async fn verify_only(
        &self,
        virtual_key: &str,
    ) -> Result<bool, String> {
        let result = self.resolve(virtual_key, "").await;
        match result {
            Ok(r) => Ok(r.resolved),
            Err(_) => Ok(false),
        }
    }
}

/// 计算 Virtual Key 的 SHA-256 哈希（与服务端存储的 key_hash 比对）
pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_key_deterministic() {
        let key = "vd-abc123def456";
        let h1 = hash_key(key);
        let h2 = hash_key(key);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_hash_key_unique() {
        let h1 = hash_key("vd-abc");
        let h2 = hash_key("vd-abcd");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_key_resolve_response_serde() {
        let json = r#"{
            "resolved": true,
            "provider_key": "sk-test123",
            "workspace_id": "ws-1",
            "budget_remaining_micro": 500000,
            "rate_limit_rpm": 60,
            "rate_limit_tpm": 100000,
            "error": ""
        }"#;
        let resp: KeyResolveResponse = serde_json::from_str(json).unwrap();
        assert!(resp.resolved);
        assert_eq!(resp.provider_key, "sk-test123");
        assert_eq!(resp.budget_remaining_micro, 500000);
    }
}
