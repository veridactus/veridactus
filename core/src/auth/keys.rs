//! # API 密钥管理
//!
//! 管理 VERIDACTUS 代理与上游 LLM 之间的认证密钥。
//! 使用 Ed25519 生成随机密钥对。

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::warn;

/// API 密钥管理器
#[derive(Debug, Clone)]
pub struct ApiKeyManager {
    /// 有效密钥 → 租户映射
    keys: HashMap<String, String>,
    /// 上游 LLM 密钥（用于转发请求时的认证）
    upstream_api_key: String,
}

impl ApiKeyManager {
    /// 创建新的密钥管理器
    ///
    /// # 参数
    /// * `upstream_api_key` - 上游 LLM 的 API 密钥
    pub fn new(upstream_api_key: String) -> Self {
        let mut keys = HashMap::new();

        // 支持环境变量配置的固定 admin key（用于测试和生产部署）
        let admin_key =
            std::env::var("VERIDACTUS_ADMIN_KEY").unwrap_or_else(|_| generate_api_key("admin"));
        keys.insert(admin_key, "admin".to_string());

        Self {
            keys,
            upstream_api_key,
        }
    }

    /// 生成一个新的 API 密钥
    pub fn generate_key(&mut self, tenant_id: &str) -> String {
        let key = generate_api_key(tenant_id);
        self.keys.insert(key.clone(), tenant_id.to_string());
        key
    }

    /// 验证 API 密钥并返回租户 ID
    pub fn validate(&self, token: &str) -> Option<&str> {
        // 支持 Bearer 前缀
        let cleaned = token
            .strip_prefix("Bearer ")
            .or(Some(token))
            .unwrap_or(token);

        if let Some(tenant) = self.keys.get(cleaned) {
            Some(tenant.as_str())
        } else {
            warn!(
                "Invalid API key attempt: {}...",
                &cleaned[..cleaned.len().min(8)]
            );
            None
        }
    }

    /// 添加静态 API 密钥（用于 E2E 测试和部署配置）
    pub fn add_static_key(&mut self, name: &str, key: &str) {
        self.keys.insert(key.to_string(), name.to_string());
    }

    /// 撤销一个 API 密钥
    pub fn revoke_key(&mut self, key: &str) {
        self.keys.remove(key);
    }

    /// 获取上游 API 密钥
    pub fn upstream_key(&self) -> &str {
        &self.upstream_api_key
    }

    /// 获取所有租户列表
    pub fn tenants(&self) -> Vec<&str> {
        self.keys.values().map(|s| s.as_str()).collect()
    }
}

/// 生成随机 API 密钥
///
/// 使用 SHA-256(tenant_id + random) 生成 64 字符十六进制字符串。
pub fn generate_api_key(tenant_id: &str) -> String {
    let random = uuid::Uuid::new_v4().to_string();
    let mut hasher = Sha256::new();
    hasher.update(tenant_id.as_bytes());
    hasher.update(b":");
    hasher.update(random.as_bytes());
    format!("veridactus_{}", hex::encode(hasher.finalize()))
}

/// 生成随机的上游 API 密钥（用于测试/搭建时）
pub fn generate_upstream_key() -> String {
    let uuid = uuid::Uuid::new_v4();
    format!("sk-{}", uuid.to_string().replace('-', ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation_and_validation() {
        let upstream = generate_upstream_key();
        let mut manager = ApiKeyManager::new(upstream);

        let key = manager.generate_key("test-tenant");
        assert!(manager.validate(&key).is_some());
        assert_eq!(manager.validate(&key).unwrap(), "test-tenant");
    }

    #[test]
    fn test_bearer_prefix() {
        let upstream = generate_upstream_key();
        let mut manager = ApiKeyManager::new(upstream);

        let key = manager.generate_key("tenant");
        assert!(manager.validate(&format!("Bearer {}", key)).is_some());
    }

    #[test]
    fn test_invalid_key() {
        let upstream = generate_upstream_key();
        let manager = ApiKeyManager::new(upstream);

        assert!(manager.validate("invalid-key").is_none());
    }

    #[test]
    fn test_revocation() {
        let upstream = generate_upstream_key();
        let mut manager = ApiKeyManager::new(upstream);

        let key = manager.generate_key("tenant");
        assert!(manager.validate(&key).is_some());
        manager.revoke_key(&key);
        assert!(manager.validate(&key).is_none());
    }
}
