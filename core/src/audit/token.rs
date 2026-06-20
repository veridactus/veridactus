//! # 审计令牌验证器
//!
//! 遵循规范 §11.2.0 和 AI.md §5.4。
//!
//! 审计令牌用于控制错误响应的详细程度：
//! - 有有效审计令牌 → 返回详细错误信息
//! - 无审计令牌 → 返回最小错误信息

use std::collections::HashSet;
use tracing::warn;

/// 审计令牌验证器
///
/// 管理有效审计令牌列表，验证请求中的审计令牌。
/// 防止侧信道信息泄露（BudgetLeak 等）。
#[derive(Debug, Clone)]
pub struct AuditTokenValidator {
    /// 有效令牌集合
    valid_tokens: HashSet<String>,
}

impl AuditTokenValidator {
    /// 创建新的审计令牌验证器
    ///
    /// # 参数
    /// * `tokens` - 初始有效令牌列表
    pub fn new(tokens: Vec<String>) -> Self {
        Self {
            valid_tokens: tokens.into_iter().collect(),
        }
    }

    /// 添加一个有效令牌
    pub fn add_token(&mut self, token: String) {
        self.valid_tokens.insert(token);
    }

    /// 撤销一个令牌
    pub fn revoke_token(&mut self, token: &str) {
        self.valid_tokens.remove(token);
    }

    /// 验证审计令牌是否有效
    ///
    /// # 参数
    /// * `token` - 待验证的令牌
    /// * `tenant_id` - 请求的租户 ID
    ///
    /// # 返回
    /// `true` 表示令牌有效
    pub fn validate(&self, token: &str, _tenant_id: &str) -> bool {
        if self.valid_tokens.contains(token) {
            true
        } else {
            warn!("无效的审计令牌尝试");
            false
        }
    }

    /// 检查令牌是否已被撤销
    pub fn is_revoked(&self, token: &str) -> bool {
        !self.valid_tokens.contains(token)
    }

    /// 获取当前有效令牌数量
    pub fn token_count(&self) -> usize {
        self.valid_tokens.len()
    }
}

impl Default for AuditTokenValidator {
    fn default() -> Self {
        Self {
            valid_tokens: HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试审计令牌验证
    #[test]
    fn test_audit_token_validation() {
        let mut validator = AuditTokenValidator::new(vec!["valid-token-1".to_string()]);
        validator.add_token("valid-token-2".to_string());

        assert!(validator.validate("valid-token-1", "tenant-1"));
        assert!(validator.validate("valid-token-2", "tenant-1"));
        assert!(!validator.validate("invalid-token", "tenant-1"));
    }

    /// 测试审计令牌撤销
    #[test]
    fn test_audit_token_revocation() {
        let mut validator = AuditTokenValidator::new(vec!["token-to-revoke".to_string()]);

        assert!(validator.validate("token-to-revoke", "tenant-1"));
        validator.revoke_token("token-to-revoke");
        assert!(!validator.validate("token-to-revoke", "tenant-1"));
    }

    /// 测试默认构造器
    #[test]
    fn test_default_validator() {
        let validator = AuditTokenValidator::default();
        assert_eq!(validator.token_count(), 0);
        assert!(!validator.validate("any-token", "tenant-1"));
    }
}
