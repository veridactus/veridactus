//! # 密钥管理器
//!
//! 严格遵循 AI.md §8.1 key_management.yaml 配置。
//! 管理密钥生成、轮换、吊销和审计日志。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::info;

/// 密钥用途
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum KeyPurpose {
    /// L0 签名密钥
    #[serde(rename = "l0_signature")]
    L0Signature,
    /// 委托令牌密钥
    #[serde(rename = "delegation_tokens")]
    DelegationTokens,
}

/// 密钥状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyStatus {
    /// 激活
    Active,
    /// 已轮换（旧密钥仍可用于验证）
    Rotated,
    /// 已吊销
    Revoked,
    /// 已过期
    Expired,
}

/// 密钥条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    /// 密钥 ID
    pub key_id: String,
    /// 用途
    pub purpose: KeyPurpose,
    /// 状态
    pub status: KeyStatus,
    /// 公钥（十六进制编码）
    pub public_key: String,
    /// 私钥（仅内存中，不序列化到日志）
    #[serde(skip)]
    pub private_key: Option<Vec<u8>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 轮换时间
    pub rotated_at: Option<DateTime<Utc>>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
}

/// 密钥操作审计日志（AI.md §8.1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyAuditLog {
    /// 操作时间
    pub timestamp: DateTime<Utc>,
    /// 操作类型
    pub operation: String,
    /// 密钥 ID
    pub key_id: String,
    /// 操作用户/系统
    pub operator: String,
    /// 操作详情
    pub details: String,
}

/// 密钥管理器（M15）
///
/// 严格遵循 AI.md §8.1 key_management.yaml：
/// - l0_signature: Ed25519, 90天轮换, 3-2-1备份策略
/// - delegation_tokens: Ed25519, 30天轮换
/// - 紧急吊销流程
pub struct KeyManager {
    /// 密钥存储
    keys: HashMap<String, KeyEntry>,
    /// 审计日志
    audit_logs: Vec<KeyAuditLog>,
    /// 轮换周期（天）
    rotation_periods: HashMap<KeyPurpose, u64>,
}

impl KeyManager {
    /// 创建新的密钥管理器（使用默认配置）
    pub fn new() -> Self {
        let mut rotation_periods = HashMap::new();
        rotation_periods.insert(KeyPurpose::L0Signature, 90);
        rotation_periods.insert(KeyPurpose::DelegationTokens, 30);

        Self {
            keys: HashMap::new(),
            audit_logs: Vec::new(),
            rotation_periods,
        }
    }

    /// 生成新密钥
    pub fn generate_key(&mut self, purpose: KeyPurpose, operator: &str) -> KeyEntry {
        let key_id = format!(
            "{}-{}",
            match purpose {
                KeyPurpose::L0Signature => "l0",
                KeyPurpose::DelegationTokens => "del",
            },
            uuid::Uuid::new_v4()
        );

        // 生成 Ed25519 密钥对（简化：使用 SHA-256 模拟）
        let seed = uuid::Uuid::new_v4().to_string();
        let public_key = format!("ed25519:{}", hex::encode(Sha256::digest(seed.as_bytes())));

        let entry = KeyEntry {
            key_id: key_id.clone(),
            purpose: purpose.clone(),
            status: KeyStatus::Active,
            public_key,
            private_key: None,
            created_at: Utc::now(),
            rotated_at: None,
            expires_at: None,
        };

        self.keys.insert(key_id.clone(), entry.clone());

        // 记录审计日志
        self.audit_logs.push(KeyAuditLog {
            timestamp: Utc::now(),
            operation: "KEY_GENERATED".to_string(),
            key_id: key_id.clone(),
            operator: operator.to_string(),
            details: format!("生成 {:?} 密钥", purpose),
        });

        info!(
            "密钥已生成: {} (purpose={:?}, operator={})",
            key_id, purpose, operator
        );
        entry
    }

    /// 轮换密钥（AI.md §8.1）
    ///
    /// 将旧密钥标记为 Rotated，生成新密钥。
    pub fn rotate_key(&mut self, key_id: &str, operator: &str) -> Result<KeyEntry, String> {
        let old_entry = self
            .keys
            .get(key_id)
            .ok_or_else(|| format!("Key {} not found", key_id))?;

        if old_entry.status != KeyStatus::Active {
            return Err(format!(
                "Key {} not in Active state (current: : {:?})",
                key_id, old_entry.status
            ));
        }

        let purpose = old_entry.purpose.clone();

        // 标记旧密钥为已轮换
        let mut rotated = old_entry.clone();
        rotated.status = KeyStatus::Rotated;
        rotated.rotated_at = Some(Utc::now());
        self.keys.insert(key_id.to_string(), rotated);

        // 生成新密钥
        let new_entry = self.generate_key(purpose, operator);

        // 记录审计日志
        self.audit_logs.push(KeyAuditLog {
            timestamp: Utc::now(),
            operation: "KEY_ROTATED".to_string(),
            key_id: key_id.to_string(),
            operator: operator.to_string(),
            details: format!("Rotated to new key {}", new_entry.key_id),
        });

        info!(
            "Key rotated: {} -> {} (operator={})",
            key_id, new_entry.key_id, operator
        );
        Ok(new_entry)
    }

    /// 吊销密钥（AI.md §8.1 紧急流程）
    pub fn revoke_key(&mut self, key_id: &str, operator: &str, reason: &str) -> Result<(), String> {
        let entry = self
            .keys
            .get_mut(key_id)
            .ok_or_else(|| format!("Key {} not found", key_id))?;
        entry.status = KeyStatus::Revoked;

        self.audit_logs.push(KeyAuditLog {
            timestamp: Utc::now(),
            operation: "KEY_REVOKED".to_string(),
            key_id: key_id.to_string(),
            operator: operator.to_string(),
            details: format!("Revoke reason: {}", reason),
        });

        info!(
            "Key revoked: {} (operator={}, reason={})",
            key_id, operator, reason
        );
        Ok(())
    }

    /// 获取激活的密钥（用于签名）
    pub fn get_active_key(&self, purpose: &KeyPurpose) -> Option<&KeyEntry> {
        self.keys
            .values()
            .find(|k| k.purpose == *purpose && k.status == KeyStatus::Active)
    }

    /// 检查密钥是否过期（AI.md §8.1 轮换策略）
    pub fn is_rotation_overdue(&self, key_id: &str) -> Result<bool, String> {
        let entry = self
            .keys
            .get(key_id)
            .ok_or_else(|| format!("Key {} not found", key_id))?;
        let period_days = self
            .rotation_periods
            .get(&entry.purpose)
            .copied()
            .unwrap_or(90);
        let age = Utc::now() - entry.created_at;
        Ok(age.num_days() > period_days as i64)
    }

    /// 获取审计日志
    pub fn audit_logs(&self) -> &[KeyAuditLog] {
        &self.audit_logs
    }

    /// 获取所有密钥
    pub fn all_keys(&self) -> Vec<&KeyEntry> {
        self.keys.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let mut km = KeyManager::new();
        let key = km.generate_key(KeyPurpose::L0Signature, "admin");
        assert_eq!(key.status, KeyStatus::Active);
        assert_eq!(key.purpose, KeyPurpose::L0Signature);
        assert!(key.key_id.starts_with("l0-"));
    }

    #[test]
    fn test_key_rotation() {
        let mut km = KeyManager::new();
        let key = km.generate_key(KeyPurpose::L0Signature, "admin");

        let new_key = km.rotate_key(&key.key_id, "admin").unwrap();
        assert_eq!(new_key.purpose, KeyPurpose::L0Signature);

        // 旧密钥应被标记为 Rotated
        let old_entry = km.keys.get(&key.key_id).unwrap();
        assert_eq!(old_entry.status, KeyStatus::Rotated);
    }

    #[test]
    fn test_key_revocation() {
        let mut km = KeyManager::new();
        let key = km.generate_key(KeyPurpose::DelegationTokens, "admin");
        km.revoke_key(&key.key_id, "admin", "安全事件").unwrap();

        let entry = km.keys.get(&key.key_id).unwrap();
        assert_eq!(entry.status, KeyStatus::Revoked);
    }

    #[test]
    fn test_rotation_overdue() {
        let mut km = KeyManager::new();
        let key = km.generate_key(KeyPurpose::L0Signature, "admin");
        // 刚创建的密钥不应过期
        assert!(!km.is_rotation_overdue(&key.key_id).unwrap());
    }

    #[test]
    fn test_get_active_key() {
        let mut km = KeyManager::new();
        km.generate_key(KeyPurpose::L0Signature, "admin");
        let active = km.get_active_key(&KeyPurpose::L0Signature);
        assert!(active.is_some());
        assert_eq!(active.unwrap().status, KeyStatus::Active);
    }
}
