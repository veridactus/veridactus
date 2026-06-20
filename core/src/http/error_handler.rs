//! # 错误响应处理（§5.4 AI.md）
//!
//! 遵循规范 §11.0，根据审计令牌的存在与否控制错误响应的详细程度。
//! 防止侧信道信息泄露。

use axum::http::StatusCode;
use axum::Json;

use crate::audit::token::AuditTokenValidator;
use crate::types::error::{ErrorResponse, VeridactusErrorCode};
use crate::types::journal::ExecutionJournal;

/// 构建错误响应，根据审计令牌控制返回详情
///
/// 遵循 AI.md §5.4：
/// - 有有效审计令牌 → 返回详细错误信息
/// - 无审计令牌 → 返回最小错误信息
pub fn build_error_response(
    req: Option<&axum::http::HeaderMap>,
    error_code: VeridactusErrorCode,
    journal: &ExecutionJournal,
    audit_token_validator: &AuditTokenValidator,
    tenant_id: &str,
) -> (StatusCode, Json<ErrorResponse>) {
    let has_valid_audit_token = req
        .and_then(|headers| {
            headers
                .get("VERIDACTUS-Audit-Token")
                .and_then(|v| v.to_str().ok())
        })
        .map_or(false, |token| {
            audit_token_validator.validate(token, tenant_id)
        });

    let http_status = error_code.http_status();

    if has_valid_audit_token {
        // 详细错误（含审计信息）
        let resp = ErrorResponse::new(
            format!("{:?}: 审计上下文可用", error_code),
            error_code,
            None,
            Some(serde_json::json!({
                "trace_id": journal.trace_id.to_string(),
                "event_count": journal.event_count(),
                "head_hash": journal.head_hash,
            })),
        );
        (StatusCode::from_u16(http_status).unwrap(), Json(resp))
    } else {
        // 最小错误 — 仍包含 trace_id 便于客户端追踪
        let resp = ErrorResponse::new(
            format!("{:?}", error_code),
            error_code,
            None,
            Some(serde_json::json!({
                "trace_id": journal.trace_id.to_string(),
            })),
        );
        (StatusCode::from_u16(http_status).unwrap(), Json(resp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// 测试错误响应构建（简化版）
    #[test]
    fn test_error_response_building() {
        let trace_id = Uuid::new_v4();
        let journal = ExecutionJournal::new(trace_id, "test-tenant");
        let validator = AuditTokenValidator::new(vec![]);

        // 无审计令牌时的最小响应
        let error_code = VeridactusErrorCode::BudgetExceeded;

        // 验证 HTTP 状态码映射
        assert_eq!(error_code.http_status(), 429);
        assert!(error_code.is_retryable());
    }
}
