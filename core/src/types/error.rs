//! # Error 错误类型
//!
//! 严格遵循 VERIDACTUS v0.2.1 §11.0 Error Handling & Response Contract。
//! 标准化错误语义是强制性的互操作性契约。

use serde::{Deserialize, Serialize};

/// 错误代码枚举（§11.5 Error Code Registry）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VeridactusErrorCode {
    /// 约束格式错误
    #[serde(rename = "VERIDACTUS_INVALID_CONSTRAINT")]
    InvalidConstraint,
    /// 约束冲突
    #[serde(rename = "VERIDACTUS_BAD_CONSTRAINT_COMBINATION")]
    BadConstraintCombination,
    /// 意图解析失败
    #[serde(rename = "VERIDACTUS_INTENT_RESOLUTION_FAILED")]
    IntentResolutionFailed,
    /// 版本不匹配
    #[serde(rename = "VERIDACTUS_VERSION_MISMATCH")]
    VersionMismatch,
    /// 无效委托
    #[serde(rename = "VERIDACTUS_INVALID_DELEGATION")]
    InvalidDelegation,
    /// 需要认证
    #[serde(rename = "VERIDACTUS_AUTH_REQUIRED")]
    AuthRequired,
    /// 权限拒绝
    #[serde(rename = "VERIDACTUS_PERMISSION_DENIED")]
    PermissionDenied,
    /// 委托拒绝
    #[serde(rename = "VERIDACTUS_DELEGATION_DENIED")]
    DelegationDenied,
    /// 基线未找到
    #[serde(rename = "VERIDACTUS_BASELINE_NOT_FOUND")]
    BaselineNotFound,
    /// 模式违反
    #[serde(rename = "VERIDACTUS_SCHEMA_VIOLATION")]
    SchemaViolation,
    /// JCS 规范化失败
    #[serde(rename = "VERIDACTUS_JCS_CANONICALIZATION_FAILED")]
    JcsCanonicalizationFailed,
    /// 经认证保证失败
    #[serde(rename = "VERIDACTUS_CERTIFIED_GUARANTEE_FAILED")]
    CertifiedGuaranteeFailed,
    /// 预算超限
    #[serde(rename = "VERIDACTUS_BUDGET_EXCEEDED")]
    BudgetExceeded,
    /// 速率限制
    #[serde(rename = "VERIDACTUS_RATE_LIMITED")]
    RateLimited,
    /// 风险阈值超限
    #[serde(rename = "VERIDACTUS_RISK_THRESHOLD_EXCEEDED")]
    RiskThresholdExceeded,
    /// 主动预防阻断
    #[serde(rename = "VERIDACTUS_ACTIVE_PREVENTION_BLOCKED")]
    ActivePreventionBlocked,
    /// ASI 风险阈值超限
    #[serde(rename = "VERIDACTUS_ASI_RISK_THRESHOLD")]
    AsiRiskThreshold,
    /// DP 预算耗尽
    #[serde(rename = "VERIDACTUS_DP_BUDGET_EXHAUSTED")]
    DpBudgetExhausted,
    /// 内部错误
    #[serde(rename = "VERIDACTUS_INTERNAL_ERROR")]
    InternalError,
    /// 上游断开
    #[serde(rename = "VERIDACTUS_UPSTREAM_DISCONNECT")]
    UpstreamDisconnect,
    /// 服务不可用
    #[serde(rename = "VERIDACTUS_STATE_UNAVAILABLE")]
    StateUnavailable,
    /// ZK 证明不可用
    #[serde(rename = "VERIDACTUS_ZK_PROOF_UNAVAILABLE")]
    ZkProofUnavailable,
    /// 上游超时
    #[serde(rename = "VERIDACTUS_UPSTREAM_TIMEOUT")]
    UpstreamTimeout,
}

impl VeridactusErrorCode {
    /// 获取对应的 HTTP 状态码（§11.1）
    pub fn http_status(&self) -> u16 {
        match self {
            Self::InvalidConstraint
            | Self::BadConstraintCombination
            | Self::IntentResolutionFailed
            | Self::VersionMismatch
            | Self::InvalidDelegation => 400,
            Self::AuthRequired => 401,
            Self::PermissionDenied | Self::DelegationDenied => 403,
            Self::BaselineNotFound => 404,
            Self::SchemaViolation
            | Self::JcsCanonicalizationFailed
            | Self::CertifiedGuaranteeFailed => 422,
            Self::BudgetExceeded
            | Self::RateLimited
            | Self::RiskThresholdExceeded
            | Self::ActivePreventionBlocked
            | Self::AsiRiskThreshold
            | Self::DpBudgetExhausted => 429,
            Self::InternalError => 500,
            Self::UpstreamDisconnect => 502,
            Self::StateUnavailable | Self::ZkProofUnavailable => 503,
            Self::UpstreamTimeout => 504,
        }
    }

    /// 获取是否可重试（§11.1）
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::AuthRequired
                | Self::CertifiedGuaranteeFailed
                | Self::BudgetExceeded
                | Self::RateLimited
                | Self::InternalError
                | Self::UpstreamDisconnect
                | Self::StateUnavailable
                | Self::ZkProofUnavailable
                | Self::UpstreamTimeout
        )
    }
}

/// 错误对象（§3.0 $defs.error_object）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 错误详情（键以 _ 开头的字段递归排除在审计签名外）
    pub details: Option<serde_json::Value>,
}

/// 结构化错误响应（§11.2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// 错误信息
    pub error: ErrorResponseDetail,
}

/// 错误响应详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponseDetail {
    /// 错误消息
    pub message: String,
    /// 错误类型
    pub r#type: String,
    /// 错误代码
    pub code: String,
    /// 关联参数
    pub param: Option<String>,
    /// 错误详情
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// 创建标准错误响应
    pub fn new(
        message: impl Into<String>,
        code: VeridactusErrorCode,
        param: Option<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            error: ErrorResponseDetail {
                message: message.into(),
                r#type: "veridactus_error".to_string(),
                code: serde_json::to_value(&code)
                    .and_then(|v| Ok(v.as_str().unwrap_or("").to_string()))
                    .unwrap_or_default(),
                param,
                details,
            },
        }
    }

    /// 创建最小错误响应（无审计令牌时使用）
    pub fn new_minimal(
        message: impl Into<String>,
        code: VeridactusErrorCode,
    ) -> Self {
        Self {
            error: ErrorResponseDetail {
                message: message.into(),
                r#type: "veridactus_error".to_string(),
                code: serde_json::to_value(&code)
                    .and_then(|v| Ok(v.as_str().unwrap_or("").to_string()))
                    .unwrap_or_default(),
                param: None,
                details: None,
            },
        }
    }
}
