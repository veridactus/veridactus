//! # HTTP 中间件模块
//!
//! 提供标准 HTTP 中间件层：
//! - `AuditTokenMiddleware` — 审计令牌验证，控制错误详情可见性（§11.2.0）
//! - `IdempotencyMiddleware` — 幂等键去重，防止重复计费（§11.4）

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ==================== 审计令牌中间件 ====================

/// 审计令牌中间件 — 控制错误详情的可见性
///
/// 遵循 §11.2.0：对于未认证客户端或无有效审计令牌的请求，
/// 错误响应 MUST 只返回 code/message/trace_id。
/// 详细错误上下文仅对持有有效 VERIDACTUS-Audit-Token 的客户端可见。
#[derive(Clone)]
pub struct AuditTokenValidator {
    /// 有效的审计令牌集合
    valid_tokens: Arc<RwLock<HashMap<String, AuditTokenInfo>>>,
    /// 是否启用严格控制
    strict_mode: bool,
}

#[derive(Debug, Clone)]
pub struct AuditTokenInfo {
    /// 令牌标识
    pub token_id: String,
    /// 关联的租户
    pub tenant_id: String,
    /// 过期时间
    pub expires_at: Option<String>,
    /// 允许访问的审计级别
    pub access_level: AuditAccessLevel,
}

/// 审计访问级别
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditAccessLevel {
    /// 基础级别：仅 trace_id
    Basic,
    /// 标准级别：trace_id + 错误码详情
    Standard,
    /// 完整级别：所有详细信息（包含预算、token 数）
    Full,
}

impl AuditTokenValidator {
    pub fn new(strict: bool) -> Self {
        Self {
            valid_tokens: Arc::new(RwLock::new(HashMap::new())),
            strict_mode: strict,
        }
    }

    /// 注册审计令牌
    pub async fn register_token(&self, token: &str, info: AuditTokenInfo) {
        self.valid_tokens
            .write()
            .await
            .insert(token.to_string(), info);
    }

    /// 吊销审计令牌
    pub async fn revoke_token(&self, token: &str) {
        self.valid_tokens.write().await.remove(token);
    }

    /// 验证审计令牌并返回访问级别
    pub async fn validate(&self, token_opt: Option<&str>) -> AuditAccessLevel {
        if !self.strict_mode {
            return AuditAccessLevel::Full;
        }

        match token_opt {
            Some(token) => {
                let tokens = self.valid_tokens.read().await;
                match tokens.get(token) {
                    Some(info) => info.access_level.clone(),
                    None => AuditAccessLevel::Basic,
                }
            }
            None => AuditAccessLevel::Basic,
        }
    }

    /// 根据访问级别过滤错误详情
    pub fn filter_error_details(
        &self,
        access_level: &AuditAccessLevel,
        details: &serde_json::Value,
    ) -> serde_json::Value {
        match access_level {
            AuditAccessLevel::Basic => {
                serde_json::json!({
                    "trace_id": details.get("trace_id").cloned().unwrap_or(serde_json::Value::Null),
                })
            }
            AuditAccessLevel::Standard => {
                let mut d = serde_json::json!({});
                if let Some(v) = details.get("trace_id") {
                    d["trace_id"] = v.clone();
                }
                if let Some(v) = details.get("truncated") {
                    d["truncated"] = v.clone();
                }
                d
            }
            AuditAccessLevel::Full => details.clone(),
        }
    }
}

// ==================== 幂等键中间件 ====================

/// 幂等键中间件 — 防止重复请求导致的重复计费
///
/// 遵循 §11.4：客户端 SHOULD 使用 Idempotency-Key: {trace_id}
/// 实现幂等去重。代理 MUST 返回已持久化的 Trace（如果 trace_id 已存在）。
#[derive(Clone)]
pub struct IdempotencyGuard {
    /// 已处理的请求 trace_id 缓存
    processed_traces: Arc<RwLock<HashMap<Uuid, ProcessedRequestInfo>>>,
    /// TTL 秒数
    ttl_seconds: u64,
    /// 最大缓存条目
    max_entries: usize,
}

#[derive(Debug, Clone)]
pub struct ProcessedRequestInfo {
    pub trace_id: Uuid,
    pub processed_at: String,
    pub response_status: u16,
    pub idempotency_key: Option<String>,
}

impl IdempotencyGuard {
    pub fn new(ttl_seconds: u64, max_entries: usize) -> Self {
        Self {
            processed_traces: Arc::new(RwLock::new(HashMap::new())),
            ttl_seconds,
            max_entries,
        }
    }

    /// 检查请求是否已被处理（幂等）
    ///
    /// 返回 `Some(ProcessedRequestInfo)` 如果已处理，
    /// 返回 `None` 如果是新请求
    pub async fn check(&self, trace_id: &Uuid) -> Option<ProcessedRequestInfo> {
        let traces = self.processed_traces.read().await;
        traces.get(trace_id).cloned()
    }

    /// 记录请求已被处理
    pub async fn record(&self, trace_id: Uuid, status: u16, idempotency_key: Option<&str>) {
        let mut traces = self.processed_traces.write().await;

        // 清理过期条目
        if traces.len() >= self.max_entries {
            // 简单清理：移除最旧的 25%
            let remove_count = self.max_entries / 4;
            let mut keys: Vec<Uuid> = traces.keys().cloned().collect();
            keys.sort_by_key(|k| {
                traces
                    .get(k)
                    .map(|v| v.processed_at.clone())
                    .unwrap_or_default()
            });
            for key in keys.iter().take(remove_count) {
                traces.remove(key);
            }
        }

        traces.insert(
            trace_id,
            ProcessedRequestInfo {
                trace_id,
                processed_at: chrono::Utc::now().to_rfc3339(),
                response_status: status,
                idempotency_key: idempotency_key.map(|s| s.to_string()),
            },
        );
    }

    /// 获取缓存的条目数
    pub async fn entry_count(&self) -> usize {
        self.processed_traces.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_token_basic_access() {
        let validator = AuditTokenValidator::new(true);
        let level = validator.validate(None).await;
        assert_eq!(level, AuditAccessLevel::Basic);

        let details = serde_json::json!({
            "trace_id": "test-id",
            "consumed_usd": 0.05,
        });
        let filtered = validator.filter_error_details(&AuditAccessLevel::Basic, &details);
        assert_eq!(filtered["trace_id"], "test-id");
        assert!(filtered.get("consumed_usd").is_none());
    }

    #[tokio::test]
    async fn test_idempotency_guard() {
        let guard = IdempotencyGuard::new(3600, 100);
        let trace_id = Uuid::new_v4();

        assert!(guard.check(&trace_id).await.is_none());
        guard.record(trace_id, 200, Some("key-123")).await;
        let info = guard.check(&trace_id).await;
        assert!(info.is_some());
        assert_eq!(info.unwrap().response_status, 200);
    }
}
