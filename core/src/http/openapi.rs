//! # OpenAPI 规范生成模块
//!
//! 使用 utoipa 从代码自动生成 OpenAPI 3.0 规范文档。
//!
//! 生成命令：
//! ```bash
//! cd core && cargo run --bin generate-openapi
//! ```
//!
//! 输出文件：
//! - `openapi/data-plane-v0.2.1.json` - JSON格式规范
//! - `openapi/data-plane-v0.2.1.yaml` - YAML格式规范

use serde::{Deserialize, Serialize};
use utoipa::OpenApi;
use utoipa::openapi::security::SecurityRequirement;

/// 聊天完成请求
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChatCompletionRequest {
    /// 模型标识符
    pub model: String,
    /// 消息列表
    pub messages: Vec<Message>,
    /// 最大令牌数
    pub max_tokens: Option<u32>,
    /// 温度参数
    pub temperature: Option<f32>,
    /// 流式响应
    pub stream: Option<bool>,
}

/// 消息结构
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct Message {
    /// 角色：system/user/assistant
    pub role: String,
    /// 消息内容
    pub content: String,
}

/// 聊天完成响应
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChatCompletionResponse {
    /// 响应ID
    pub id: String,
    /// 模型
    pub model: String,
    /// 完成内容
    pub choices: Vec<Choice>,
    /// 使用量
    pub usage: Usage,
}

/// 选择项
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct Choice {
    /// 索引
    pub index: u32,
    /// 消息
    pub message: Message,
    /// 完成原因
    pub finish_reason: String,
}

/// 使用量
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct Usage {
    /// 提示令牌数
    pub prompt_tokens: u32,
    /// 完成令牌数
    pub completion_tokens: u32,
    /// 总令牌数
    pub total_tokens: u32,
}

/// 轨迹摘要
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct TraceSummary {
    /// 轨迹ID
    pub trace_id: String,
    /// 模型名称
    pub model: String,
    /// 创建时间
    pub created_at: String,
    /// 执行状态
    pub state: String,
}

/// 轨迹详情
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct TraceDetail {
    /// 轨迹ID
    pub trace_id: String,
    /// 模型名称
    pub model: String,
    /// 创建时间
    pub created_at: String,
    /// 执行状态
    pub state: String,
    /// 输入
    pub input: Input,
    /// 输出
    pub output: Output,
    /// L0签名
    pub signature: Option<String>,
    /// 合规报告
    pub compliance: Option<ComplianceReport>,
}

/// 输入结构
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct Input {
    /// 消息列表
    pub messages: Vec<Message>,
    /// 请求参数
    pub parameters: serde_json::Value,
}

/// 输出结构
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct Output {
    /// 响应内容
    pub content: String,
    /// 完成原因
    pub finish_reason: String,
    /// 使用量
    pub usage: Usage,
}

/// 合规报告
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ComplianceReport {
    /// 报告ID
    pub report_id: String,
    /// 框架
    pub framework: String,
    /// 控制项
    pub controls: Vec<Control>,
}

/// 控制项
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct Control {
    /// 控制ID
    pub control_id: String,
    /// 描述
    pub description: String,
    /// 状态
    pub status: String,
}

/// 重放分支
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReplayBranch {
    /// 分支ID
    pub branch_id: String,
    /// 分支名称
    pub name: String,
    /// 父分支ID
    pub parent_id: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 状态
    pub status: String,
}

/// 签名验证结果
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct VerificationResult {
    /// 是否验证通过
    pub verified: bool,
    /// 验证详情
    pub details: VerificationDetails,
}

/// 验证详情
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct VerificationDetails {
    /// 算法
    pub algorithm: String,
    /// 签名
    pub signature: String,
    /// 时间戳
    pub timestamp: String,
    /// 证明级别
    pub proof_level: String,
}

/// GDPR删除请求
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct GdprDeleteRequest {
    /// 目标ID
    pub target_id: String,
    /// 删除类型
    pub deletion_type: Option<String>,
    /// 理由
    pub reason: Option<String>,
}

/// GDPR删除响应
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct GdprDeleteResponse {
    /// 请求ID
    pub request_id: String,
    /// 状态
    pub status: String,
    /// 删除证明
    pub deletion_proof: Option<DeletionProof>,
}

/// 删除证明
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct DeletionProof {
    /// 证明类型
    pub proof_type: String,
    /// 证明内容
    pub proof: serde_json::Value,
    /// 验证方式
    pub verification: String,
}

/// 实时指标
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct RealtimeMetrics {
    /// 请求总数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均延迟(ms)
    pub avg_latency_ms: f64,
    /// 总成本(USD)
    pub total_cost_usd: f64,
    /// 活跃轨迹数
    pub active_traces: u64,
}

/// 模型信息
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModelInfo {
    /// 模型ID
    pub id: String,
    /// 模型名称
    pub name: String,
    /// 供应商
    pub provider: String,
    /// 状态
    pub status: String,
}

/// 错误响应
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    /// 错误代码
    pub error: ErrorDetail,
}

/// 错误详情
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ErrorDetail {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 提示信息
    pub hint: Option<String>,
}

/// 健康检查响应
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// 状态
    pub status: String,
    /// 版本
    pub version: String,
}

/// 审计日志项
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLogEntry {
    /// 日志ID
    pub id: String,
    /// 时间戳
    pub timestamp: String,
    /// 事件类型
    pub event_type: String,
    /// 轨迹ID
    pub trace_id: Option<String>,
    /// 详情
    pub details: serde_json::Value,
}

/// 防护统计
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct PreventionStats {
    /// 阻止的请求数
    pub blocked_requests: u64,
    /// 遮蔽的PII数
    pub pii_masked: u64,
    /// 防护的事件
    pub events: Vec<PreventionEvent>,
}

/// 防护事件
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct PreventionEvent {
    /// 事件类型
    pub event_type: String,
    /// 严重程度
    pub severity: String,
    /// 轨迹ID
    pub trace_id: String,
    /// 时间戳
    pub timestamp: String,
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "健康检查", body = HealthResponse)
    ),
    tag = "系统"
)]
pub fn health_check_handler() -> axum::Json<HealthResponse> {
    axum::Json(HealthResponse {
        status: "ok".to_string(),
        version: "0.2.1".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/v1/traces",
    params(
        ("limit" = Option<u32>, Query, description = "返回数量限制"),
        ("offset" = Option<u32>, Query, description = "偏移量")
    ),
    responses(
        (status = 200, description = "轨迹列表", body = Vec<TraceSummary>),
        (status = 401, description = "未授权", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "轨迹管理"
)]
pub fn list_traces_handler() {}

#[utoipa::path(
    get,
    path = "/v1/traces/{id}",
    params(
        ("id" = String, Path, description = "轨迹ID")
    ),
    responses(
        (status = 200, description = "轨迹详情", body = TraceDetail),
        (status = 404, description = "轨迹不存在", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "轨迹管理"
)]
pub fn get_trace_handler() {}

#[utoipa::path(
    post,
    path = "/v1/chat/completions",
    request_body = ChatCompletionRequest,
    responses(
        (status = 200, description = "聊天完成", body = ChatCompletionResponse),
        (status = 400, description = "请求错误", body = ErrorResponse),
        (status = 401, description = "未授权", body = ErrorResponse),
        (status = 429, description = "超出预算", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "AI代理"
)]
pub fn chat_completion_handler() {}

#[utoipa::path(
    get,
    path = "/v1/traces/{id}/verify",
    params(
        ("id" = String, Path, description = "轨迹ID")
    ),
    responses(
        (status = 200, description = "验证结果", body = VerificationResult),
        (status = 404, description = "轨迹不存在", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "签名验证"
)]
pub fn verify_signature_handler() {}

#[utoipa::path(
    post,
    path = "/v1/traces/{id}/replay",
    params(
        ("id" = String, Path, description = "轨迹ID")
    ),
    responses(
        (status = 200, description = "重放结果", body = TraceDetail),
        (status = 404, description = "轨迹不存在", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "轨迹重放"
)]
pub fn replay_trace_handler() {}

#[utoipa::path(
    post,
    path = "/v1/gdpr/delete",
    request_body = GdprDeleteRequest,
    responses(
        (status = 200, description = "删除请求已提交", body = GdprDeleteResponse),
        (status = 400, description = "请求错误", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "GDPR合规"
)]
pub fn gdpr_delete_handler() {}

#[utoipa::path(
    get,
    path = "/v1/metrics/realtime",
    responses(
        (status = 200, description = "实时指标", body = RealtimeMetrics)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "监控"
)]
pub fn realtime_metrics_handler() {}

#[utoipa::path(
    get,
    path = "/v1/audit/log",
    params(
        ("limit" = Option<u32>, Query, description = "返回数量"),
        ("start_time" = Option<String>, Query, description = "开始时间"),
        ("end_time" = Option<String>, Query, description = "结束时间")
    ),
    responses(
        (status = 200, description = "审计日志", body = Vec<AuditLogEntry>)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "审计"
)]
pub fn audit_log_handler() {}

#[utoipa::path(
    get,
    path = "/v1/prevention/stats",
    responses(
        (status = 200, description = "防护统计", body = PreventionStats)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "安全防护"
)]
pub fn prevention_stats_handler() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check_handler,
        list_traces_handler,
        get_trace_handler,
        chat_completion_handler,
        verify_signature_handler,
        replay_trace_handler,
        gdpr_delete_handler,
        realtime_metrics_handler,
        audit_log_handler,
        prevention_stats_handler,
    ),
    components(
        schemas(
            ChatCompletionRequest,
            ChatCompletionResponse,
            Message,
            Choice,
            Usage,
            TraceSummary,
            TraceDetail,
            Input,
            Output,
            ComplianceReport,
            Control,
            ReplayBranch,
            VerificationResult,
            VerificationDetails,
            GdprDeleteRequest,
            GdprDeleteResponse,
            DeletionProof,
            RealtimeMetrics,
            ModelInfo,
            ErrorResponse,
            ErrorDetail,
            HealthResponse,
            AuditLogEntry,
            PreventionStats,
            PreventionEvent,
        )
    ),
    tags(
        (name = "系统", description = "系统健康检查和管理"),
        (name = "轨迹管理", description = "轨迹的创建、查询和验证"),
        (name = "AI代理", description = "OpenAI兼容的聊天完成API"),
        (name = "签名验证", description = "L0/L2A/L2B密码学证明验证"),
        (name = "轨迹重放", description = "确定性轨迹重放和分支管理"),
        (name = "GDPR合规", description = "GDPR数据删除和证明"),
        (name = "监控", description = "实时指标和监控"),
        (name = "审计", description = "审计日志查询"),
        (name = "安全防护", description = "安全防护统计")
    ),
    modifiers(&SecurityAddon)
)]
pub struct VeridactusDataPlaneApi;

/// 安全插件
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let security_requirement = SecurityRequirement::new("api_key", Vec::<String>::new());
        openapi.security = Some(vec![security_requirement]);
    }
}

impl VeridactusDataPlaneApi {
    pub fn openapi() -> utoipa::openapi::OpenApi {
        <Self as OpenApi>::openapi()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_openapi() {
        let openapi = VeridactusDataPlaneApi::openapi();

        // 验证生成成功
        assert!(!openapi.paths().is_empty());

        // 验证关键端点存在
        let paths = openapi.paths();
        assert!(paths.get("/health").is_some());
        assert!(paths.get("/v1/traces").is_some());
        assert!(paths.get("/v1/chat/completions").is_some());
    }

    #[test]
    fn test_openapi_json_serialization() {
        let openapi = VeridactusDataPlaneApi::openapi();

        // 验证JSON序列化
        let json = serde_json::to_string_pretty(&openapi).unwrap();
        assert!(json.contains("openapi"));
        assert!(json.contains("3.0"));

        // 验证可以反序列化
        let _ = serde_json::from_str::<utoipa::openapi::OpenApi>(&json).unwrap();
    }
}
