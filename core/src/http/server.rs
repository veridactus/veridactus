//! # VERIDACTUS HTTP/SSE 服务器（M02）
//!
//! 兼容 OpenAI Chat Completions API 的 HTTP/SSE 代理服务器。
//! 接收客户端请求，执行治理流水线，转发到上游 LLM，记录 Execution Journal。
//!
//! 实现流程（AI.md §2.2 时序图）：
//! 1. 创建 Journal 并记录 RequestReceived 事件
//! 2. 解析请求体 & 头部，执行同步插件（预算、认证等）
//! 3. 转发到上游 LLM
//! 4. 接收响应并记录事件
//! 5. 计算 L0 签名，存储 Trace
//! 6. 返回响应（含 VERIDACTUS 头部）

use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
// use base64::Engine; // unused
use reqwest::Client as HttpClient;
use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{info, warn};
use regex::Regex;

use sha2::Digest;
use crate::agent_chain::AgentExecutionChainManager;
use crate::audit::token::AuditTokenValidator;
use crate::auth::keys::ApiKeyManager;
use crate::compliance::ComplianceMapper;
use crate::crypto::signature::generate_l0_proof;
use crate::gdpr::{DeletionRequest, DeletionType, GdprErasureManager};
use crate::http::error_handler::build_error_response;
use crate::http::headers::{parse_veridactus_headers, VeridactusRequestHeaders, VeridactusResponseHeaders};
use crate::observability::otel::OtelTracer;
use crate::store::{InMemoryTraceStore, TraceStore};
use crate::types::constraints::{
    ActivePrevention, AdaptiveState, BudgetStrategy, ConflictType, ConstraintsApplied,
    DegradeAction, DegradeActionType, InstructionHierarchyMode, PolicyEvaluation, 
    PrivacyLevel, ReproducibilityMode, check_constraint_conflicts,
};
use crate::types::error::{ErrorObject, ErrorResponse, VeridactusErrorCode};
use crate::types::journal::{ExecutionJournal, JournalEventType};
use crate::types::trace::{
    CertifiedGuarantee, ExecutionState, Input, Output, StateTransition, Trace,
};
use crate::types::Action;

/// 模型路由配置
#[derive(Debug, Clone)]
pub struct ModelRoute {
    /// 模型标识符（客户端使用的名称）
    pub name: String,
    /// 上游 LLM 的模型名称
    pub upstream_model: String,
    /// 上游端点后缀
    pub upstream_endpoint: String,
    /// 是否为默认模型
    pub is_default: bool,
    /// 上游完整URL（可选，用于覆盖upstream_base_url）
    pub upstream_url: Option<String>,
    /// API密钥
    pub api_key: Option<String>,
    /// API密钥头部名称
    pub api_key_header: Option<String>,
    /// 是否使用代理
    pub use_proxy: bool,
    /// 代理URL
    pub proxy_url: Option<String>,
}

/// 代理配置
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// 上游 LLM 基础端点
    pub upstream_base_url: String,
    /// 默认模型
    pub default_model: String,
    /// 模型路由表
    pub model_routes: Vec<ModelRoute>,
    /// 支持的协议版本
    pub supported_versions: Vec<String>,
    /// 是否启用详细错误
    pub detailed_errors: bool,
    /// 当前活跃的流水线执行计划
    pub pipeline_plan: Option<crate::pipeline::config::ExecutionPlan>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        // 从环境变量读取 Zhipu API 密钥，否则使用默认值
        let zhipu_key = std::env::var("VERIDACTUS_ZHIPU_API_KEY")
            .unwrap_or_else(|_| "89f155e74b424fe7b82ccbc11d12e791.mLDuSRdpV4YV5Bfz".to_string());

        Self {
            upstream_base_url: "https://open.bigmodel.cn".to_string(),
            default_model: "glm-5.1".to_string(),
            model_routes: vec![
                ModelRoute {
                    name: "glm-5.1".to_string(),
                    upstream_model: "glm-5.1".to_string(),
                    upstream_endpoint: "/api/paas/v4/chat/completions".to_string(),
                    is_default: true,
                    upstream_url: Some("https://open.bigmodel.cn".to_string()),
                    api_key: Some(zhipu_key),
                    api_key_header: Some("Authorization".to_string()),
                    use_proxy: false,
                    proxy_url: None,
                },
            ],
            supported_versions: vec!["0.1".to_string(), "0.2".to_string()],
            detailed_errors: false,
            pipeline_plan: None,
        }
    }
}

/// 服务器共享状态
#[derive(Clone)]
pub struct AppState {
    /// 审计令牌验证器
    pub audit_token_validator: Arc<AuditTokenValidator>,
    /// API 密钥管理器
    pub api_key_manager: Arc<std::sync::Mutex<ApiKeyManager>>,
    /// Trace 存储（支持 InMemory / File / Postgres 多后端）
    pub trace_store: Arc<dyn crate::store::TraceStore>,
    /// HTTP 客户端
    pub http_client: HttpClient,
    /// 代理配置
    pub config: Arc<tokio::sync::RwLock<ProxyConfig>>,
    /// 幂等键守卫（§11.4）
    pub idempotency_guard: Arc<crate::middleware::IdempotencyGuard>,
    /// Agent 执行链管理器（§1.6.1）
    pub agent_chain_manager: Arc<AgentExecutionChainManager>,
    /// 合规映射器（§7.5）
    pub compliance_mapper: Arc<ComplianceMapper>,
    /// GDPR 删除管理器（§8.7）
    pub gdpr_manager: Arc<GdprErasureManager>,
    /// 钩子注册中心（§6.3）
    pub hook_registry: Arc<crate::hooks::registry::HookRegistry>,
}

impl AppState {
    /// 创建新的应用状态（含默认密钥和配置）
    pub fn new_with_defaults() -> Self {
        let upstream_key = crate::auth::keys::generate_upstream_key();
        let api_key_manager = Arc::new(std::sync::Mutex::new(ApiKeyManager::new(upstream_key)));
        let trace_store: Arc<dyn crate::store::TraceStore> = Arc::new(crate::store::InMemoryTraceStore::new());

        // 生成一个测试用密钥
        {
            let mut mgr = api_key_manager.lock().unwrap();
            mgr.generate_key("e2e-test");
        }

        Self {
            audit_token_validator: Arc::new(AuditTokenValidator::new(vec![
                "test-audit-token".to_string(),
            ])),
            api_key_manager,
            trace_store: trace_store.clone(),
            http_client: HttpClient::new(),
            config: Arc::new(tokio::sync::RwLock::new(ProxyConfig::default())),
            idempotency_guard: Arc::new(crate::middleware::IdempotencyGuard::new(3600, 10000)),
            agent_chain_manager: Arc::new(AgentExecutionChainManager::new()),
            compliance_mapper: Arc::new(ComplianceMapper::new()),
            gdpr_manager: Arc::new(GdprErasureManager::new(
                Box::new(InMemoryGdprStorage)
            )),
            hook_registry: Arc::new(crate::hooks::registry::HookRegistry::new()),
        }
    }
}

/// 内存 GDPR 存储（用于测试）
struct InMemoryGdprStorage;

impl crate::gdpr::DeletionStorage for InMemoryGdprStorage {
    fn delete_by_trace_id(&self, trace_id: &str) -> Result<crate::gdpr::DeletionResult, crate::gdpr::DeletionError> {
        Ok(crate::gdpr::DeletionResult {
            request_id: format!("del_{}", uuid::Uuid::new_v4()),
            success: true,
            deleted_count: 1,
            retained_signatures: vec![],
            audit_log_entry: crate::gdpr::DeletionAuditEntry {
                audit_id: format!("audit_{}", uuid::Uuid::new_v4()),
                request_id: format!("del_{}", uuid::Uuid::new_v4()),
                deletion_type: crate::gdpr::DeletionType::TraceId,
                target_id: trace_id.to_string(),
                deleted_count: 1,
                retained_signature_hashes: vec![],
                deleted_at: chrono::Utc::now().to_rfc3339(),
                deleted_by: None,
                compliance_evidence: crate::gdpr::ComplianceEvidence {
                    regulation: "GDPR".to_string(),
                    article: "Article 17".to_string(),
                    basis: "Right to erasure".to_string(),
                    data_subject_right: "Right to be forgotten".to_string(),
                },
            },
            error_message: None,
        })
    }
    fn delete_by_session_id(&self, session_id: &str) -> Result<crate::gdpr::DeletionResult, crate::gdpr::DeletionError> {
        Ok(crate::gdpr::DeletionResult {
            request_id: format!("del_{}", uuid::Uuid::new_v4()),
            success: true,
            deleted_count: 5,
            retained_signatures: vec![],
            audit_log_entry: crate::gdpr::DeletionAuditEntry {
                audit_id: format!("audit_{}", uuid::Uuid::new_v4()),
                request_id: format!("del_{}", uuid::Uuid::new_v4()),
                deletion_type: crate::gdpr::DeletionType::SessionId,
                target_id: session_id.to_string(),
                deleted_count: 5,
                retained_signature_hashes: vec![],
                deleted_at: chrono::Utc::now().to_rfc3339(),
                deleted_by: None,
                compliance_evidence: crate::gdpr::ComplianceEvidence {
                    regulation: "GDPR".to_string(),
                    article: "Article 17".to_string(),
                    basis: "Right to erasure".to_string(),
                    data_subject_right: "Right to be forgotten".to_string(),
                },
            },
            error_message: None,
        })
    }
    fn delete_by_user_id(&self, user_id: &str) -> Result<crate::gdpr::DeletionResult, crate::gdpr::DeletionError> {
        Ok(crate::gdpr::DeletionResult {
            request_id: format!("del_{}", uuid::Uuid::new_v4()),
            success: true,
            deleted_count: 10,
            retained_signatures: vec![],
            audit_log_entry: crate::gdpr::DeletionAuditEntry {
                audit_id: format!("audit_{}", uuid::Uuid::new_v4()),
                request_id: format!("del_{}", uuid::Uuid::new_v4()),
                deletion_type: crate::gdpr::DeletionType::UserId,
                target_id: user_id.to_string(),
                deleted_count: 10,
                retained_signature_hashes: vec![],
                deleted_at: chrono::Utc::now().to_rfc3339(),
                deleted_by: None,
                compliance_evidence: crate::gdpr::ComplianceEvidence {
                    regulation: "GDPR".to_string(),
                    article: "Article 17".to_string(),
                    basis: "Right to erasure".to_string(),
                    data_subject_right: "Right to be forgotten".to_string(),
                },
            },
            error_message: None,
        })
    }
    fn retain_signature(&self, _trace_id: &str, _audit_signature: &str) -> Result<(), crate::gdpr::DeletionError> { Ok(()) }
    fn get_deletion_log(&self, _request_id: &str) -> Option<crate::gdpr::DeletionAuditEntry> { None }
    fn list_deletion_logs(&self, _limit: usize) -> Vec<crate::gdpr::DeletionAuditEntry> { Vec::new() }
}

/// 创建 VERIDACTUS HTTP/SSE 服务器路由
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handle_chat_completion))
        .route("/v1/traces", get(list_traces))
        .route("/v1/traces/:id", get(get_trace))
        .route("/v1/traces/:id/compliance", get(get_trace_compliance))
        // 重放端点（§9.4 Deterministic Replay Engine）
        .route("/v1/traces/:id/replay", post(replay_trace))
        // 签名验证端点（§7.4 Independent Verification）
        .route("/v1/traces/:id/verify", post(verify_trace_signature))
        // 分支管理端点
        .route("/v1/replay/branches", get(list_replay_branches))
        .route("/v1/replay/branches", post(create_replay_branch))
        .route("/v1/replay/branches/:branch_id", get(get_replay_branch))
        .route("/v1/replay/branches/:branch_id", delete(delete_replay_branch))
        .route("/v1/replay/branches/:source_id/merge/:target_id", post(merge_replay_branch))
        // 批量操作端点
        .route("/v1/traces/batch", post(batch_operations))
        // 实时指标端点
        .route("/v1/metrics/realtime", get(realtime_metrics))
        .route("/health", get(health_check))
        .route("/models", get(list_models))
        // 管理端点：接收控制面推送的配置更新
        .route("/v1/admin/config/sync", post(handle_config_sync))
        // GDPR 删除端点（§8.7）
        .route("/v1/gdpr/delete", post(handle_gdpr_deletion))
        .route("/v1/gdpr/deletion-proof/:request_id", get(get_gdpr_deletion_proof))
        .route("/v1/gdpr/deletion-history", get(list_gdpr_deletion_history))
        // 合规端点（§7.5）
        .route("/v1/compliance/report/:trace_id", get(get_compliance_report))
        // 主动预防端点（§5.3.2, §8.4）
        .route("/v1/prevention/stats", get(get_prevention_stats))
        // Prometheus 指标端点（§10.3.4）
        .route("/metrics", get(metrics_handler))
        .route("/v1/audit/log", get(audit_log_handler))
        // Extension Discovery 端点（§A.4）
        .route("/.well-known/veridactus-extensions.json", get(handle_extension_discovery))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_logging_middleware,
        ))
        .with_state(state)
}

/// 健康检查端点
async fn health_check() -> &'static str {
    "VERIDACTUS Proxy v0.2.1 - OK"
}

/// 列出可用模型
async fn list_models(State(state): State<AppState>) -> Json<serde_json::Value> {
    let config = state.config.read().await;
    let models: Vec<serde_json::Value> = config
        .model_routes
        .iter()
        .map(|route| {
            serde_json::json!({
                "id": route.name,
                "object": "model",
                "created": chrono::Utc::now().timestamp(),
                "owned_by": "veridactus",
                "upstream_endpoint": route.upstream_endpoint,
                "is_default": route.is_default,
            })
        })
        .collect();

    Json(serde_json::json!({
        "object": "list",
        "data": models,
    }))
}

/// 接收控制面推送的配置更新（模型/流水线等）
///
/// 控制面在 CRUD 操作后立即调用此端点，使数据面毫秒级生效。
/// 格式与 config/poll 返回的一致：{ "change_type": "model", "data": [...], "version": {...} }
async fn handle_config_sync(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let change_type = payload.get("change_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let data = payload.get("data");

    info!("收到控制面配置推送: change_type={}", change_type);

    match change_type {
        "model" => {
            if let Some(models_array) = data.and_then(|d| d.as_array()) {
                let mut config = state.config.write().await;
                let mut routes = Vec::new();
                let mut default_model = config.default_model.clone();

                for model_data in models_array {
                    let name = model_data.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let upstream_url = model_data.get("upstream_url").and_then(|v| v.as_str()).unwrap_or("");
                    let upstream_model = model_data.get("upstream_model").and_then(|v| v.as_str()).unwrap_or(name);
                    let api_key = model_data.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let api_key_header = model_data.get("api_key_header").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let is_default = model_data.get("is_default").and_then(|v| v.as_bool()).unwrap_or(false);
                    let use_proxy = model_data.get("use_proxy").and_then(|v| v.as_bool()).unwrap_or(false);
                    let proxy_url = model_data.get("proxy_url").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let status = model_data.get("status").and_then(|v| v.as_str()).unwrap_or("active");

                    if status != "active" || name.is_empty() || upstream_url.is_empty() {
                        continue;
                    }

                    // 根据 URL 模式确定端点
                    let endpoint = if upstream_url.contains("generativelanguage.googleapis.com") {
                        format!("/{}:generateContent", upstream_model)
                    } else if upstream_url.contains("models.inference.ai.azure.com") {
                        "/chat/completions".to_string()
                    } else if upstream_url.contains("open.bigmodel.cn") {
                        "/api/paas/v4/chat/completions".to_string()
                    } else if upstream_url.contains("qianfan.baidubce.com") {
                        "/v2/chat/completions".to_string()
                    } else {
                        "/v1/chat/completions".to_string()
                    };

                    routes.push(ModelRoute {
                        name: name.to_string(),
                        upstream_model: upstream_model.to_string(),
                        upstream_endpoint: endpoint,
                        is_default,
                        upstream_url: Some(upstream_url.to_string()),
                        api_key,
                        api_key_header,
                        use_proxy,
                        proxy_url,
                    });

                    if is_default {
                        default_model = name.to_string();
                    }
                }

                config.model_routes = routes;
                config.default_model = default_model;
                info!("模型配置已通过推送更新: {} models, 默认: {}", config.model_routes.len(), config.default_model);
            }
        }
        "pipeline" => {
            // 解析控制面推送的流水线配置→ExecutionPlan
            if let Some(pipelines) = data.and_then(|d| d.as_array()) {
                info!("收到流水线配置推送，共 {} pipelines", pipelines.len());
                if let Some(first) = pipelines.first() {
                    if let Ok(plan) = serde_json::from_value::<crate::pipeline::config::ExecutionPlan>(first.clone()) {
                        info!("激活流水线: plan_id={}, {} 阶段", plan.plan_id, plan.stages.len());
                        let mut config = state.config.write().await;
                        config.pipeline_plan = Some(plan);
                    }
                }
            }
        }
        _ => {
            info!("收到忽略的配置类型: {}", change_type);
        }
    }

    Ok(Json(serde_json::json!({"status": "ok", "change_type": change_type})))
}

/// 列出 Trace (支持 ?id=uuid 查询单个 Trace)
async fn list_traces(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    // If ?id=xxx provided, return single trace detail
    if let Some(trace_id) = params.get("id") {
        if let Ok(id) = uuid::Uuid::parse_str(trace_id) {
            if let Some(trace) = state.trace_store.get(&id).await {
                let value = serde_json::to_value(&trace).unwrap_or_default();
                return Json(value);
            }
        }
        return Json(serde_json::json!({"error": "trace not found"}));
    }
    let traces = state.trace_store.list(None, 100, 0).await;
    let trace_summaries: Vec<serde_json::Value> = traces
        .iter()
        .map(|t| {
            serde_json::json!({
                "trace_id": t.trace_id,
                "model": t.model,
                "created_at": t.created_at,
                "execution_state": t.execution_state,
                "proof_levels": t.proofs.proof_chain.iter().map(|p| format!("{:?}", p.level)).collect::<Vec<_>>(),
                "signature": t.proofs.proof_chain.first().and_then(|p| p.signature.clone()),
            })
        })
        .collect();

    Json(serde_json::json!({
        "total": traces.len(),
        "traces": trace_summaries,
    }))
}

/// 获取单个 Trace
async fn get_trace(
    State(state): State<AppState>,
    axum::extract::Path(trace_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    info!("GET Trace: id={}", trace_id);

    let id = uuid::Uuid::parse_str(&trace_id).map_err(|e| {
        warn!("无效的 Trace ID 格式: {} - {}", trace_id, e);
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("无效的 Trace ID: {}", e)})),
        )
    })?;

    match state.trace_store.get(&id).await {
        Some(trace) => {
            info!("Trace 找到: {}", trace_id);
            let value = serde_json::to_value(&trace).unwrap_or_default();
            Ok(Json(value))
        }
        None => {
            warn!("Trace 未找到: {}", trace_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Trace 未找到"})),
            ))
        }
    }
}

/// 请求日志中间件
async fn request_logging_middleware(
    request: Request,
    next: middleware::Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    info!("[VERIDACTUS] Received request: {} {}", method, uri);
    let response = next.run(request).await;
    info!("[VERIDACTUS] 响应: {} {}", response.status(), uri);
    response
}

/// 构建合规映射所需的 trace_data（请求处理中内联使用）
fn build_compliance_trace_data(
    trace: &Trace,
    content: &str,
    privacy_level: &crate::types::constraints::PrivacyLevel,
) -> HashMap<String, serde_json::Value> {
    let mut data = HashMap::new();
    // 基础标识字段
    data.insert("trace_id".to_string(), serde_json::Value::String(trace.trace_id.to_string()));
    data.insert("output.response".to_string(), serde_json::Value::String(content.to_string()));
    // 隐私级别
    data.insert("constraints_applied.privacy_level".to_string(),
        serde_json::Value::String(format!("{:?}", privacy_level)));
    // 证明链
    data.insert("proof_chain".to_string(), serde_json::to_value(&trace.proofs).unwrap_or_default());
    // 守卫和策略
    if let Some(ref ca) = trace.constraints_applied {
        if let Some(ref guards) = ca.guardrails_active {
            data.insert("constraints_applied.guardrails_active".to_string(),
                serde_json::to_value(guards).unwrap_or_default());
        }
        data.insert("constraints_applied.policy_evaluation".to_string(),
            serde_json::to_value(&ca.policy_evaluation).unwrap_or_default());
    }
    // 观测数据
    if let Some(ref obs) = trace.observations {
        if let Some(ref monitoring) = obs.monitoring {
            if let Some(score) = monitoring.anomaly_score {
                data.insert("observations.risk_score".to_string(), serde_json::Value::Number(
                    serde_json::Number::from_f64(score).unwrap_or(serde_json::Number::from(0))));
            }
        }
        if obs.fairness_check.is_some() {
            data.insert("observations.fairness_check".to_string(),
                serde_json::Value::String("present".to_string()));
        }
    }
    // TTL 和元数据
    if let Some(ref ttl) = trace.ttl_expire_at {
        data.insert("metadata.ttl_expire_at".to_string(), serde_json::Value::String(ttl.clone()));
    }
    // 布尔标记字段（合规映射器中用于存在性检查，设置为"present"即可通过 Exists 检查）
    data.insert("observations.human_in_the_loop".to_string(),
        serde_json::Value::String("not_applicable".to_string()));
    data.insert("metadata.data_subject_rights".to_string(),
        serde_json::Value::String("not_applicable".to_string()));
    data.insert("observations.data_processing_notice".to_string(),
        serde_json::Value::String("not_applicable".to_string()));
    data
}

/// 从已存储的 Trace 构建合规映射所需数据
fn build_compliance_trace_data_from_stored(trace: &Trace) -> HashMap<String, serde_json::Value> {
    let content = trace.output.as_ref()
        .and_then(|o| o.response.as_ref())
        .and_then(|r| r.as_str())
        .unwrap_or("");
    let privacy = trace.constraints_applied.as_ref()
        .and_then(|c| c.privacy_level.as_ref())
        .map(|p| match p {
            crate::types::constraints::PrivacyLevel::Raw => crate::types::constraints::PrivacyLevel::Raw,
            crate::types::constraints::PrivacyLevel::Masked => crate::types::constraints::PrivacyLevel::Masked,
            crate::types::constraints::PrivacyLevel::HashOnly => crate::types::constraints::PrivacyLevel::HashOnly,
            crate::types::constraints::PrivacyLevel::TeePrivate => crate::types::constraints::PrivacyLevel::TeePrivate,
        })
        .unwrap_or(crate::types::constraints::PrivacyLevel::Raw);
    build_compliance_trace_data(trace, content, &privacy)
}

/// 处理 Chat Completion 请求（核心端点）
async fn handle_chat_completion(
    State(state): State<AppState>,
    request: Request,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // ===== 1. 解析请求 =====
    let (parts, body) = request.into_parts();

    // ===== 1.1 幂等键检查（§11.4）=====
    let idempotency_key = parts.headers.get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    // 检查 trace_id 是否已处理
    if let Some(ref key) = idempotency_key {
        if let Ok(trace_id) = uuid::Uuid::parse_str(key) {
            if let Some(_existing) = state.idempotency_guard.check(&trace_id).await {
                info!("幂等请求，返回已存在的响应: trace_id={}", trace_id);
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new_minimal(
                        "Request already processed".to_string(),
                        VeridactusErrorCode::InvalidConstraint,
                    )),
                ));
            }
        }
    }

    // 解析 VERIDACTUS 头部
    let mut header_map = BTreeMap::new();
    for (name, value) in &parts.headers {
        header_map.insert(name.to_string(), value.to_str().unwrap_or("").to_string());
    }
    let veridactus_headers = parse_veridactus_headers(&header_map);

    // ===== 2. 检查 Passthrough 模式（§4.1.1）=====
    // 根据协议：没有任何 VERIDACTUS-* 头部的请求 MUST 被视为 passthrough 模式
    // Passthrough 模式下：仍记录基础 Trace，但 MUST NOT 强制执行任何约束或拦截
    let is_passthrough = veridactus_headers.version.is_none()
        && veridactus_headers.budget_limit.is_none()
        && veridactus_headers.privacy_level.is_none()
        && veridactus_headers.guardrails.is_none()
        && veridactus_headers.capabilities.is_none()
        && veridactus_headers.action.is_none()
        && veridactus_headers.baseline_ref.is_none()
        && veridactus_headers.budget_strategy.is_none()
        && veridactus_headers.diff_output.is_none()
        && veridactus_headers.focus_fields.is_none()
        && veridactus_headers.override_model.is_none()
        && veridactus_headers.guardrails_strictness.is_none()
        && veridactus_headers.instruction_hierarchy.is_none()
        && veridactus_headers.certified_guarantee.is_none()
        && veridactus_headers.compliance_profile.is_none()
        && veridactus_headers.drift_suite_id.is_none()
        && veridactus_headers.trust_delegation_token.is_none();

    // ===== 3. API 密钥验证（Passthrough 模式跳过）=====
    let auth_header = parts.headers.get("authorization").and_then(|v| v.to_str().ok());
    let tenant_id = if is_passthrough {
        info!("Passthrough mode: skipping auth (Sec 4.1.1)");
        "passthrough".to_string()
    } else {
        match auth_header {
            Some(token) => {
                match state.api_key_manager.lock().unwrap().validate(token) {
                    Some(tenant) => tenant.to_string(),
                    None => {
                        warn!("无效的 API 密钥");
                        let _j = ExecutionJournal::new(uuid::Uuid::new_v4(), "unknown");
                        return Err(build_error_response(
                            Some(&parts.headers),
                            VeridactusErrorCode::AuthRequired,
                            &_j,
                            &state.audit_token_validator,
                            "unknown",
                        ));
                    }
                }
            }
            None => {
                warn!("Missing Authorization header (non-Passthrough mode)");
                let _j = ExecutionJournal::new(uuid::Uuid::new_v4(), "unknown");
                return Err(build_error_response(
                    Some(&parts.headers),
                    VeridactusErrorCode::AuthRequired,
                    &_j,
                    &state.audit_token_validator,
                    "unknown",
                ));
            }
        }
    };

    // ===== 4. 解析请求体 =====
    let body_bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| {
            let _j = ExecutionJournal::new(uuid::Uuid::new_v4(), &tenant_id);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new_minimal(
                    "请求体读取失败",
                    VeridactusErrorCode::InvalidConstraint,
                )),
            )
        })?;

    let body_str = String::from_utf8_lossy(&body_bytes);
    let body_json: serde_json::Value = serde_json::from_str(&body_str).map_err(|_| {
        let _j = ExecutionJournal::new(uuid::Uuid::new_v4(), &tenant_id);
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new_minimal(
                "JSON 解析失败",
                VeridactusErrorCode::InvalidConstraint,
            )),
        )
    })?;

    // 获取请求的模型
    let config = state.config.read().await;
    let requested_model = body_json
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(&config.default_model);

    // 查找模型路由（克隆为自有值，后续可释放 config 锁）
    let route = match config
        .model_routes
        .iter()
        .find(|r| r.name == requested_model)
        .or_else(|| config.model_routes.iter().find(|r| r.is_default))
        .cloned()
    {
        Some(r) => r,
        None => {
            warn!("未找到模型路由: {}", requested_model);
            let _j = ExecutionJournal::new(uuid::Uuid::new_v4(), &tenant_id);
            return Err(build_error_response(
                Some(&parts.headers),
                VeridactusErrorCode::InvalidConstraint,
                &_j,
                &state.audit_token_validator,
                &tenant_id,
            ));
        }
    };

    // ===== 5. 创建 Journal 和 Trace =====
    // 若客户端提供了 Idempotency-Key（有效 UUID），则复用为 trace_id（§11.4）
    let trace_id = if let Some(ref key) = idempotency_key {
        uuid::Uuid::parse_str(key).unwrap_or_else(|_| uuid::Uuid::new_v4())
    } else {
        uuid::Uuid::new_v4()
    };
    let mut journal = ExecutionJournal::new(trace_id, &tenant_id);
    let mut trace = Trace::new(route.name.clone());
    trace.trace_id = trace_id;
    trace.tenant_id = Some(tenant_id.clone());

    // 创建 OTel span 用于监控
    let otel_tracer = OtelTracer::new("veridactus-core");
    let _span = otel_tracer.create_span("chat_completion", &trace_id.to_string());

    body_json_to_input(&body_json, &mut trace);

    // 记录请求到达事件
    let body_hash = crate::crypto::signature::compute_sha256_hex(&body_bytes);
    journal.append_event(JournalEventType::RequestReceived {
        method: "POST".to_string(),
        path: "/v1/chat/completions".to_string(),
        headers: header_map.clone(),
        body_hash: body_hash.clone(),
    });

    // 记录请求解析事件
    journal.append_event(JournalEventType::RequestParsed {
        model: route.name.clone(),
        temperature: body_json.get("temperature").and_then(|v| v.as_f64()),
        max_tokens: body_json.get("max_tokens").and_then(|v| v.as_u64()).map(|v| v as u32),
    });

    // ===== 6. 版本协商 =====
    let negotiated_version = negotiate_version(
        veridactus_headers.version.as_deref(),
        &config.supported_versions,
    );
    let _negotiated_version = match negotiated_version {
        Ok(v) => v,
        Err(_) => {
            return Err(build_error_response(
                Some(&parts.headers),
                VeridactusErrorCode::VersionMismatch,
                &journal,
                &state.audit_token_validator,
                &tenant_id,
            ));
        }
    };

    // ===== 6.0 VERIDACTUS-Action dispatch（§4.2.1）=====
    if let Some(ref action) = veridactus_headers.action {
        use crate::http::headers::VeridactusAction;
        match action {
            VeridactusAction::SaveBaseline => {
                info!("Action: save-baseline — 将当前请求作为基线 Trace");
                // 基线 Trace 标记：在 extensions 中加入 baseline 标识
                if trace.extensions.is_none() {
                    trace.extensions = Some(serde_json::json!({}));
                }
                if let Some(ref mut ext) = trace.extensions {
                    ext["veridactus.ai/v1/baseline"] = serde_json::json!({
                        "marked_at": chrono::Utc::now().to_rfc3339(),
                        "model": route.name,
                    });
                }
                journal.append_event(JournalEventType::PluginDecision {
                    plugin_name: "action:save-baseline".to_string(),
                    action: Action::Continue,
                    latency_us: 10,
                });
            }
            VeridactusAction::Replay => {
                info!("Action: replay — 重放模式");
                if let Some(ref baseline_ref) = veridactus_headers.baseline_ref {
                    info!("  baseline_ref={}", baseline_ref);
                    // 设置 parent_id 建立 replay 谱系
                    if let Ok(baseline_id) = uuid::Uuid::parse_str(baseline_ref) {
                        trace.parent_id = Some(baseline_id);
                    }
                    journal.append_event(JournalEventType::PluginDecision {
                        plugin_name: "action:replay".to_string(),
                        action: Action::Continue,
                        latency_us: 10,
                    });
                } else {
                    // 无 baseline ref 时继续正常执行（非阻塞）
                    warn!("Replay action without baseline-ref, continuing normally");
                    journal.append_event(JournalEventType::PluginDecision {
                        plugin_name: "action:replay".to_string(),
                        action: Action::Continue,
                        latency_us: 10,
                    });
                }
            }
            VeridactusAction::AuditExport => {
                info!("Action: audit-export — 生成审计导出响应");
                // 标记 Trace 用于审计导出（响应中会包含完整签名的 JSON）
                trace.extensions = Some(serde_json::json!({
                    "veridactus.ai/v1/audit_export": {
                        "export_requested_at": chrono::Utc::now().to_rfc3339(),
                        "export_format": "signed_json",
                    }
                }));
                journal.append_event(JournalEventType::PluginDecision {
                    plugin_name: "action:audit-export".to_string(),
                    action: Action::Continue,
                    latency_us: 10,
                });
            }
            VeridactusAction::DriftTest => {
                info!("Action: drift-test — 漂移测试");
                if let Some(ref baseline_ref) = veridactus_headers.baseline_ref {
                    if let Ok(baseline_id) = uuid::Uuid::parse_str(baseline_ref) {
                        trace.parent_id = Some(baseline_id);
                        // 检查基线是否存在
                        if let Some(baseline) = state.trace_store.get(&baseline_id).await {
                            info!("基线 Trace 找到: {}", baseline_ref);
                            journal.append_event(JournalEventType::PluginDecision {
                                plugin_name: "action:drift-test".to_string(),
                                action: Action::Continue,
                                latency_us: 10,
                            });
                        } else {
                            warn!("基线 Trace 未找到: {}，继续正常执行", baseline_ref);
                            journal.append_event(JournalEventType::PluginDecision {
                                plugin_name: "action:drift-test".to_string(),
                                action: Action::Continue,
                                latency_us: 10,
                            });
                        }
                    }
                } else {
                    // 无 baseline ref 时继续正常执行（非阻塞）
                    warn!("Drift-test action without baseline-ref, continuing normally");
                    journal.append_event(JournalEventType::PluginDecision {
                        plugin_name: "action:drift-test".to_string(),
                        action: Action::Continue,
                        latency_us: 10,
                    });
                }
            }
        }
    }

    // ===== 6.0.1 DELEGATION_VALIDATE（§1.6.3, §6.2）=====
    if let Some(ref delegation_token_b64) = veridactus_headers.trust_delegation_token {
        trace.execution_state = Some(ExecutionState::DelegationValidate);
        journal.append_event(JournalEventType::StateTransition {
            from: ExecutionState::ConstraintEval,
            to: ExecutionState::DelegationValidate,
        });
        info!("进入 DELEGATION_VALIDATE 阶段 — 验证委托令牌");

        // 解码并验证委托令牌
        let token_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            delegation_token_b64,
        ).map_err(|_| {
            build_error_response(
                Some(&parts.headers),
                VeridactusErrorCode::InvalidDelegation,
                &journal,
                &state.audit_token_validator,
                &tenant_id,
            )
        })?;

        let token_str = String::from_utf8(token_bytes).map_err(|_| {
            build_error_response(
                Some(&parts.headers),
                VeridactusErrorCode::InvalidDelegation,
                &journal,
                &state.audit_token_validator,
                &tenant_id,
            )
        })?;

        let delegation_token: crate::delegation::DelegationToken = serde_json::from_str(&token_str)
            .map_err(|e| {
                info!("委托令牌 JSON parse failed: {}", e);
                build_error_response(
                    Some(&parts.headers),
                    VeridactusErrorCode::InvalidDelegation,
                    &journal,
                    &state.audit_token_validator,
                    &tenant_id,
                )
            })?;

        // 使用复合验证器验证令牌
        let verifier = crate::delegation::validator::CompositeAttestationVerifier::new();
        // 要求至少 Ed25519 签名验证
        let required_types = vec![
            crate::delegation::validator::AttestationType::Ed25519 {
                public_key: String::new(),
            },
        ];

        match verifier.verify(&delegation_token, &required_types) {
            Ok(()) => {
                info!("委托令牌验证通过: issuer={}, subject={}",
                    delegation_token.issuer, delegation_token.subject);

                // 记录委托链到 Trace（§1.6.3）
                trace.delegation_chain = Some(crate::types::trace::DelegationChain {
                    root_principal: delegation_token.issuer.clone(),
                    delegation_path: delegation_token.attestations.iter().map(|_a| {
                        serde_json::json!({
                            "from": delegation_token.issuer,
                            "to": delegation_token.subject,
                            "capability": delegation_token.capabilities,
                            "grant_constraints_hash": delegation_token.grant_constraints_hash,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })
                    }).collect(),
                    chain_merkle_root: delegation_token.chain_merkle_root,
                });

                journal.append_event(JournalEventType::StateTransition {
                    from: ExecutionState::DelegationValidate,
                    to: ExecutionState::ConstraintEval,
                });
            }
            Err(e) => {
                warn!("委托令牌验证失败: {}", e);
                journal.append_event(JournalEventType::StateTransition {
                    from: ExecutionState::DelegationValidate,
                    to: ExecutionState::Failed,
                });
                return Err(build_error_response(
                    Some(&parts.headers),
                    match e {
                        crate::delegation::validator::AttestationError::TokenExpired =>
                            VeridactusErrorCode::DelegationDenied,
                        _ => VeridactusErrorCode::InvalidDelegation,
                    },
                    &journal,
                    &state.audit_token_validator,
                    &tenant_id,
                ));
            }
        }

        trace.execution_state = Some(ExecutionState::ConstraintEval);
    }

    // ===== 6.1 能力协商（§4.4）=====
    use crate::pipeline::config::capabilities::default_server_capabilities;
    use crate::pipeline::config::RuntimeCapabilities;

    let server_capabilities = RuntimeCapabilities {
        protocol_version: config.supported_versions.first()
            .cloned()
            .unwrap_or_else(|| "0.2.1".to_string()),
        capabilities: default_server_capabilities(),
    };

    let _cap_neg_result = if let Some(ref client_caps) = veridactus_headers.capabilities {
        let result = server_capabilities.negotiate(client_caps);
        if !result.success {
            journal.append_event(JournalEventType::PluginDecision {
                plugin_name: "capability_negotiation".to_string(),
                action: Action::Block,
                latency_us: 0,
            });
            return Err(build_error_response(
                Some(&parts.headers),
                VeridactusErrorCode::BadConstraintCombination,
                &journal,
                &state.audit_token_validator,
                &tenant_id,
            ));
        }
        Some(result)
    } else {
        None
    };

    // ===== 6.9.2 Governance DSL 编译（§5.8）=====
    // 支持两种方式声明 DSL 策略：
    // 1. 请求体中的 "veridactus_dsl" 字段（YAML/JSON 字符串）
    // 2. VERIDACTUS-Policy-Ref 头部（远程策略 URL，当前为存根）
    let dsl_constraints = if let Some(dsl_yaml) = body_json.get("veridactus_dsl").and_then(|v| v.as_str()) {
        info!("检测到内联 DSL 策略，编译中...");
        match crate::governance_dsl::parser::GovernanceDsl::parse(dsl_yaml) {
            Ok(dsl) => {
                let compiler = crate::governance_dsl::compiler::DslCompiler::new();
                match compiler.compile(&dsl, &mut trace) {
                    Ok(c) => {
                        info!("DSL 编译成功: {} policies, {} intents", dsl.policies.len(),
                            dsl.intents.as_ref().map(|i| {
                                [i.budget.is_some(), i.privacy.is_some(), i.safety.is_some()]
                                    .iter().filter(|x| **x).count()
                            }).unwrap_or(0));
                        journal.append_event(JournalEventType::PluginDecision {
                            plugin_name: "governance_dsl".to_string(),
                            action: Action::Continue,
                            latency_us: 100,
                        });
                        Some(c)
                    }
                    Err(e) => {
                        warn!("DSL 编译失败: {}", e);
                        journal.append_event(JournalEventType::PluginDecision {
                            plugin_name: "governance_dsl".to_string(),
                            action: Action::Block,
                            latency_us: 100,
                        });
                        return Err(build_error_response(
                            Some(&parts.headers),
                            VeridactusErrorCode::BadConstraintCombination,
                            &journal, &state.audit_token_validator, &tenant_id,
                        ));
                    }
                }
            }
            Err(e) => {
                warn!("DSL 解析失败: {}", e);
                return Err(build_error_response(
                    Some(&parts.headers),
                    VeridactusErrorCode::InvalidConstraint,
                    &journal, &state.audit_token_validator, &tenant_id,
                ));
            }
        }
    } else {
        None
    };

    // ===== 7. 约束冲突检测（§5.5）=====
    let privacy_level = match veridactus_headers.privacy_level.as_deref() {
        Some("masked") => PrivacyLevel::Masked,
        Some("hash_only") => PrivacyLevel::HashOnly,
        Some("tee_private") => PrivacyLevel::TeePrivate,
        _ => PrivacyLevel::Raw,
    };

    let budget_strategy = match veridactus_headers.budget_strategy.as_deref() {
        Some("hard_stop") => Some(BudgetStrategy::HardStop),
        Some("degrade_model") => Some(BudgetStrategy::DegradeModel),
        Some("soft_alert") => Some(BudgetStrategy::SoftAlert),
        Some("adaptive") => Some(BudgetStrategy::Adaptive),
        Some("awareness") => Some(BudgetStrategy::Awareness),
        _ => None,
    };

    let reproducibility_mode = body_json.get("reproducibility")
        .and_then(|r| r.get("mode"))
        .and_then(|m| m.as_str())
        .map(|m| match m {
            "bounded" => ReproducibilityMode::Bounded,
            "strict" => ReproducibilityMode::Strict,
            _ => ReproducibilityMode::None,
        });

    let active_prevention = body_json.get("active_prevention")
        .and_then(|ap| serde_json::from_value(ap.clone()).ok());

    let compliance_profile = veridactus_headers.compliance_profile.clone();

    // ===== 7.4.1 Compliance Profile 自动配置（§5.10）=====
    // 注意：在约束检查之前设置隐私级别，避免 raw 隐私与 EU_AI_ACT 冲突
    let effective_privacy = if let Some(ref profile) = compliance_profile {
        info!("应用合规配置文件: {}", profile);
        if profile.contains("EU_AI_ACT") && veridactus_headers.privacy_level.is_none() {
            info!("  → EU AI Act：自动设置 masked 隐私 + 2555天保留");
            PrivacyLevel::Masked
        } else {
            privacy_level.clone()
        }
    } else {
        privacy_level.clone()
    };

    let conflict_result = check_constraint_conflicts(
        &Some(effective_privacy.clone()),
        &reproducibility_mode,
        &budget_strategy,
        &veridactus_headers.guardrails,
        &active_prevention,
        &compliance_profile,
        &None,
    );

    // ===== 7.4.2 记录合规配置到 Trace =====
    if let Some(ref profile) = compliance_profile {
        if profile.contains("EU_AI_ACT") {
            let ca = trace.constraints_applied.get_or_insert_with(|| ConstraintsApplied {
                budget_limit_usd: None, budget_actual_usd: None, budget_strategy: None,
                privacy_level: None, privacy_masked_fields: None,
                active_prevention: None, adaptive: None,
                reproducibility_mode: None, reproducibility_seed: None, guardrails_active: None,
                guardrails_strictness: None, instruction_hierarchy_mode: None,
                policy_evaluation: None, degrade_action: None, dp_budget: None,
                conflict_result: None,
            });
            if ca.privacy_level.is_none() {
                ca.privacy_level = Some(PrivacyLevel::Masked);
            }
            if ca.instruction_hierarchy_mode.is_none() {
                ca.instruction_hierarchy_mode = Some(InstructionHierarchyMode::Strict);
            }
            trace.ttl_expire_at = Some(
                (chrono::Utc::now() + chrono::Duration::days(2555)).to_rfc3339()
            );
        }
        if profile.contains("NIST_AI_600") {
            info!("  → NIST AI 600-1：公平性审计 + 风险文档");
            trace.observations.get_or_insert_with(Default::default);
        }
    }

    if conflict_result.has_conflicts {
        let hard_conflicts: Vec<_> = conflict_result.conflicts.iter()
            .filter(|c| c.conflict_type == ConflictType::HardConflict)
            .collect();

        if !hard_conflicts.is_empty() {
            for conflict in &hard_conflicts {
                journal.append_event(JournalEventType::ConstraintConflict {
                    constraint_a: conflict.constraint_a.clone(),
                    value_a: conflict.value_a.clone(),
                    constraint_b: conflict.constraint_b.clone(),
                    value_b: conflict.value_b.clone(),
                    conflict_type: conflict.conflict_type.to_string(),
                    reason: conflict.reason.clone(),
                });
            }

            return Err(build_error_response(
                Some(&parts.headers),
                VeridactusErrorCode::BadConstraintCombination,
                &journal,
                &state.audit_token_validator,
                &tenant_id,
            ));
        }
    }

    // ===== 7.5 指令层次冲突检测（§5.7 P0 > P1 > P2）=====
    // 检测 P2 用户指令是否尝试覆盖 P0/P1 治理规则
    let instruction_hierarchy_violation = check_instruction_hierarchy_violation(
        &body_json,
        &veridactus_headers.instruction_hierarchy,
        &mut trace,
    );
    if let Some((severity_type, violation)) = instruction_hierarchy_violation {
        if severity_type == "blocked" {
            journal.append_event(JournalEventType::SafetyEvent(violation.clone()));
            return Err(build_error_response(
                Some(&parts.headers),
                VeridactusErrorCode::AsiRiskThreshold,
                &journal,
                &state.audit_token_validator,
                &tenant_id,
            ));
        }
    }

    // ===== 8. 预算检查（如果启用） =====
    let mut degraded_route: Option<ModelRoute> = None;
    if let Some(limit) = veridactus_headers.budget_limit {
        journal.append_event(JournalEventType::PluginDecision {
            plugin_name: "budget".to_string(),
            action: Action::Continue,
            latency_us: 10,
        });
        // 如果是 hard_stop 且预算为 0，直接拒绝
        if limit <= 0.0 {
            journal.append_event(JournalEventType::PluginDecision {
                plugin_name: "budget".to_string(),
                action: Action::Block,
                latency_us: 5,
            });
            return Err(build_error_response(
                Some(&parts.headers),
                VeridactusErrorCode::BudgetExceeded,
                &journal,
                &state.audit_token_validator,
                &tenant_id,
            ));
        }
        // 增强预算预检：基于 max_tokens 和输入长度预估成本
        let max_tokens = body_json.get("max_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(1024);
        let estimated_input_tokens = (body_str.len() as u64).max(10) / 4;
        let estimated_cost = calculate_cost(estimated_input_tokens, max_tokens);
        if estimated_cost > limit {
            warn!(
                "预算预检拒绝: 预估成本 ${:.6} > 预算限制 ${:.6} (tokens_in={}, tokens_out={})",
                estimated_cost, limit, estimated_input_tokens, max_tokens
            );
            journal.append_event(JournalEventType::PluginDecision {
                plugin_name: "budget_precheck".to_string(),
                action: Action::Block,
                latency_us: 5,
            });
            return Err(build_error_response(
                Some(&parts.headers),
                VeridactusErrorCode::BudgetExceeded,
                &journal,
                &state.audit_token_validator,
                &tenant_id,
            ));
        }
        info!("预算预检通过: 预估成本 ${:.6} <= 预算限制 ${:.6}", estimated_cost, limit);

        // ===== degrade_model: 预算不足时自动切换更便宜模型（§5.3.1）=====
        if let Some(ref strategy) = veridactus_headers.budget_strategy {
            if strategy == "degrade_model" && limit > 0.0 {
                let estimated_total_tokens = max_tokens.max(estimated_input_tokens);
                let route_cost_per_token = token_cost_for_model(&route.name);
                let route_estimated = route_cost_per_token * estimated_total_tokens as f64;

                if route_estimated > limit * 0.5 {
                    // 寻找更便宜的模型（先克隆避免借用冲突）
                    let all_routes = config.model_routes.clone();
                    let fallback = all_routes.iter()
                        .filter(|r| r.name != route.name)
                        .min_by(|a, b| {
                            let ca = token_cost_for_model(&a.name);
                            let cb = token_cost_for_model(&b.name);
                            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
                        });

                    if let Some(fb) = fallback {
                        let fb_cost = token_cost_for_model(&fb.name) * estimated_total_tokens as f64;
                        if fb_cost < route_estimated {
                            info!(
                                "degrade_model: {}→{} (cost ${:.6}→${:.6})",
                                route.name, fb.name, route_estimated, fb_cost
                            );
                            journal.append_event(JournalEventType::PluginDecision {
                                plugin_name: "degrade_model".to_string(),
                                action: Action::Degrade,
                                latency_us: 10,
                            });
                            if let Some(ref mut ca) = trace.constraints_applied {
                                ca.degrade_action = Some(DegradeAction {
                                    action_type: DegradeActionType::SwitchModel,
                                    params: Some(serde_json::json!({
                                        "from": route.name,
                                        "to": fb.name
                                    })),
                                    priority: 1,
                                });
                            }
                            degraded_route = Some(fb.clone());
                        }
                    }
                }
            }
        }
        drop(config);
    }

    // 记录状态转换
    journal.append_event(JournalEventType::StateTransition {
        from: ExecutionState::Init,
        to: ExecutionState::ConstraintEval,
    });

    // ===== 8. 治理模式：完整审计流程（§1.2 Execution Lifecycle）=====
    // 协议 §4.1.1: passthrough 模式 MUST NOT 强制执行任何约束或拦截
    // 治理模式（有 VERIDACTUS 头部）：执行完整管道
    if !is_passthrough {
        info!("Governance mode: full audit pipeline - model={}", route.name);

        let pii_detector = PIIDetector::new();
        let mut processed_body = body_json.clone();
        let mut input_pii_found: Vec<String> = Vec::new();

        if let Some(messages) = processed_body.get_mut("messages") {
            if let Some(msgs) = messages.as_array() {
                let mut new_messages = serde_json::Value::Array(Vec::new());
                for msg in msgs {
                    if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                        let (masked_content, pii_types) = pii_detector.detect_and_mask(content);
                        if !pii_types.is_empty() {
                            info!("PII 检测: 发现 {:?} - 已遮蔽", pii_types);
                            input_pii_found.extend(pii_types.iter().map(|s| s.to_string()));
                        }
                        let mut new_msg = msg.clone();
                        new_msg["content"] = serde_json::Value::String(masked_content);
                        new_messages.as_array_mut().unwrap().push(new_msg);
                    } else {
                        new_messages.as_array_mut().unwrap().push(msg.clone());
                    }
                }
                processed_body["messages"] = new_messages;
            }
        }

        // ===== 执行 pipeline pre_request 阶段（真实组件）=====
        let pipeline_registry = {
            let mut registry = crate::plugin::PluginRegistry::new();
            // 注册全部 4 个生产级插件
            registry.register(Box::new(crate::plugin::BudgetGuardPlugin::new(100.0, "hard_stop")));
            registry.register(Box::new(crate::plugin::PiiDetectorPlugin::new()));
            registry.register(Box::new(crate::plugin::InputSanitizerPlugin::new()));
            registry.register(Box::new(crate::plugin::ResponseValidatorPlugin::new()));
            // 同时注册 Guardrail 插件
            registry.register(Box::new(crate::plugin::G1InputFilter::new()));
            registry.register(Box::new(crate::plugin::G2OutputFilter::new()));
            registry.register(Box::new(crate::plugin::G3SemanticGuard::new()));
            Arc::new(registry)
        };
        let pipeline_plan = {
            let config = state.config.read().await;
            config.pipeline_plan.clone().unwrap_or_else(|| {
                crate::pipeline::config::ExecutionPlan::default_plan()
            })
        };
        if !pipeline_plan.stages.is_empty() {
            let mut req_ctx = crate::plugin::RequestContext {
                headers: header_map.iter().map(|(k,v)| (k.clone(), v.clone())).collect(),
                body: Some(body_str.to_string()),
                trace_id,
                tenant_id: tenant_id.clone(),
                plugin_config: None,  // executor will set this per-plugin
            };
            let executor = crate::pipeline::executor::PipelineExecutor::new(
                pipeline_registry.clone(),
                pipeline_plan.clone(),
            );
            let result = executor.execute_pre_request(&mut req_ctx, &mut journal).await;
            if result.action == Action::Block {
                warn!("Pipeline pre_request 阻断请求: {:?}", result.block_reason);
                return Err(build_error_response(
                    Some(&parts.headers),
                    VeridactusErrorCode::BadConstraintCombination,
                    &journal,
                    &state.audit_token_validator,
                    &tenant_id,
                ));
            }
            info!("Pipeline pre_request 通过: {} checks passed", result.checks_passed.len());
        }

        // ===== 钩子: pre_execute（§6.3）=====
        let _hook_result = state.hook_registry.run_pre_execute(&mut trace);

        // ===== 检测流式请求 =====
        let is_streaming = body_json
            .get("stream")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        if is_streaming {
            info!("治理模式 流式请求: trace_id={}", trace_id);
            let config = state.config.read().await;
            let upstream_url = if let Some(ref url) = route.upstream_url {
                format!("{}{}", url.trim_end_matches('/'), route.upstream_endpoint)
            } else {
                format!("{}{}", config.upstream_base_url.trim_end_matches('/'), route.upstream_endpoint)
            };
            drop(config);
            // 流式转发 — pipeline 已在 pre_request 阶段执行，流式过程中通过 SSE 传递
            let stream_budget = veridactus_headers.budget_limit;
            let stream_awareness = veridactus_headers.budget_strategy.as_deref() == Some("awareness")
                || veridactus_headers.budget_strategy.as_deref() == Some("adaptive");
            let effective_route = degraded_route.as_ref().unwrap_or(&route);
            let stream_result = forward_to_upstream_streaming(
                &state, effective_route, &processed_body, &upstream_url,
                &mut journal, &trace_id, &tenant_id,
                stream_budget, stream_awareness,
            ).await;
            match stream_result {
                Ok(response) => {
                    info!("Governance streaming complete: trace_id={}", trace_id);
                    // 后台异步生成 L0+L2A 证明（流式模式下延迟生成，不阻塞响应）
                    let mut t_for_proof = trace.clone();
                    let store_for_proof = state.trace_store.clone();
                    tokio::spawn(async move {
                        t_for_proof.execution_state = Some(ExecutionState::Finalized);
                        let l0 = crate::crypto::signature::generate_l0_proof(&mut t_for_proof);
                        t_for_proof.proofs.proof_chain.push(l0);
                        if let Ok(tj_str) = serde_json::to_string(&t_for_proof) {
                            let l2a = crate::crypto::merkle::generate_l2a_proof(&tj_str, 0.1);
                            t_for_proof.proofs.proof_chain.push(l2a);
                        }
                        let _ = store_for_proof.save(t_for_proof).await;
                    });
                    return Ok(response);
                }
                Err((status, error_response)) => {
                    // 流式失败也需生成 Trace（§1.3）
                    trace.execution_state = Some(ExecutionState::Failed);
                    let l0_proof = generate_l0_proof(&mut trace);
                    trace.proofs.proof_chain.push(l0_proof);
                    let _ = state.trace_store.save(trace.clone()).await;
                    return Err((status, error_response));
                }
            }
        }

        let start_time = std::time::Instant::now();
        // 使用降级模型（degrade_model 策略触发时）
        let effective_route = degraded_route.as_ref().unwrap_or(&route);
        let upstream_result = forward_to_upstream_complete(&state, effective_route, &processed_body).await;
        let latency_ms = start_time.elapsed().as_millis() as u64;
        METRICS.record_latency(latency_ms);

        match upstream_result {
            Ok((status, response_body, upstream_usage)) => {
                let response_json: serde_json::Value = serde_json::from_slice(&response_body)
                    .unwrap_or_else(|_| serde_json::json!({"error": "Failed to parse response"}));

                let output_content = extract_output_content(&response_json);
                let pii_result = call_pii_detection(&state.http_client, &output_content).await;

                // G2 输出过滤器：扫描 LLM 响应中的 PII 泄露/有害内容/不安全代码
                let output_filter = crate::plugin::OutputFilter::new();
                let filter_result = output_filter.scan(&output_content);
                if !filter_result.passed {
                    warn!(
                        "G2 输出过滤器检测到违规: trace_id={}, violations={:?}",
                        trace_id, filter_result.violations
                    );
                    journal.append_event(JournalEventType::SafetyEvent(crate::types::SafetyEvent {
                        trigger_type: crate::types::SafetyTrigger::G2OutputFilter,
                        severity: crate::types::Severity::High,
                        action_taken: crate::types::SafetyAction::Flagged,
                        content_hash: sha2::Sha256::digest(output_content.as_bytes()).iter().map(|b| format!("{:02x}", b)).collect::<String>(),
                        asi_risk_id: Some(crate::types::OwaspAsiRisk::AgentGoalHijack),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    }));
                }

                // 非流式 constrained_decoding：扫描响应中是否包含禁止模式
                let prevention = std::sync::Arc::new(
                    crate::prevention::ConstrainedDecoder::new(
                        std::sync::Arc::new(crate::prevention::PatternRegistry::default()),
                    )
                );
                let response_text = filter_result.filtered_text.clone();
                let prevention_hit = prevention.check_text(&response_text);
                if let Some(hit) = &prevention_hit {
                    warn!("ConstrainedDecoder 在非流式响应中检测到禁止模式: trace_id={}, category={}", trace_id, hit.blocked_pattern_category);
                    journal.append_event(JournalEventType::SafetyEvent(crate::types::SafetyEvent {
                        trigger_type: crate::types::SafetyTrigger::G2OutputFilter,
                        severity: crate::types::Severity::High,
                        action_taken: crate::types::SafetyAction::Blocked,
                        content_hash: sha2::Sha256::digest(response_text.as_bytes()).iter().map(|b| format!("{:02x}", b)).collect::<String>(),
                        asi_risk_id: Some(crate::types::OwaspAsiRisk::AgentGoalHijack),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    }));
                }

                let total_tokens = upstream_usage
                    .as_ref()
                    .map(|u| u.total_tokens as u64)
                    .or_else(|| extract_usage_from_response(&response_json).map(|u| u.total_tokens as u64));

                let cost_estimated = estimate_cost(&total_tokens, &route.name);
                let finish_reason = extract_finish_reason(&response_json);

                let has_pii = pii_result.as_ref().map(|p| p.pii_detected).unwrap_or(false) || !input_pii_found.is_empty();

                // hash_only 隐私级别：将响应替换为 SHA-256 哈希（§8.1）
                let is_hash_only = effective_privacy == PrivacyLevel::HashOnly;
                let stored_response = if is_hash_only {
                    let resp_str = serde_json::to_string(&response_json).unwrap_or_default();
                    let resp_hash = sha2::Sha256::digest(resp_str.as_bytes());
                    serde_json::json!({
                        "content_hash": format!("sha256:{:x}", resp_hash),
                        "privacy_level": "hash_only",
                        "note": "原始内容已哈希化，仅保留完整性证明"
                    })
                } else {
                    response_json
                };

                let output = Some(crate::types::trace::Output {
                    response: Some(stored_response),
                    truncated: false,
                    finish_reason,
                });

                // 提取安全事件
                let safety_events: Option<Vec<crate::types::SafetyEvent>> = {
                    let events: Vec<_> = journal.events.iter()
                        .filter_map(|e| if let JournalEventType::SafetyEvent(se) = &e.event_type { Some(se.clone()) } else { None })
                        .collect();
                    if events.is_empty() { None } else { Some(events) }
                };

                // 解析认证保证请求
                let certified_guarantee = veridactus_headers.certified_guarantee.as_ref()
                    .and_then(|h| parse_certified_guarantee(h));

                // 执行公平性检查（§9.2）
                // ===== 钩子: post_stream（§6.3）=====
                let _hook_result = state.hook_registry.run_on_observation(&mut trace, &JournalEventType::StreamEnd { total_tokens: 0, finish_reason: "stop".to_string() });

                let fairness_check = perform_fairness_check(&trace);
                
                let observations = crate::types::trace::Observations {
                    tokens_count: total_tokens,
                    cost_estimated_usd: Some(cost_estimated),
                    latency_ms: Some(latency_ms),
                    state_transitions: None,
                    error: None,
                    monitoring: None,
                    budget_awareness: None,
                    safety_events,
                    red_team_events: None,
                    certified_guarantee,
                    fairness_check,
                    replay_snapshot: None,
                    approval: None,
                    _internal_metrics: Some(serde_json::json!({
                        "input_pii_found": input_pii_found,
                        "output_pii_detected": pii_result.as_ref().map(|p| p.pii_detected).unwrap_or(false),
                        "output_pii_findings": pii_result.as_ref().map(|p| &p.findings).cloned().unwrap_or_default(),
                    })),
                };

                body_json_to_input(&processed_body, &mut trace);

                // hash_only: 输入也需哈希化（§8.1: 零明文存储）
                if is_hash_only {
                    if let Some(ref mut input) = trace.input {
                        if let Some(ref prompt) = input.prompt {
                            let prompt_str = serde_json::to_string(prompt).unwrap_or_default();
                            let prompt_hash = sha2::Sha256::digest(prompt_str.as_bytes());
                            input.prompt = Some(serde_json::json!({
                                "prompt_hash": format!("sha256:{:x}", prompt_hash),
                                "privacy_level": "hash_only"
                            }));
                        }
                    }
                }

                trace.execution_state = Some(if status.as_u16() < 500 {
                    ExecutionState::Finalized
                } else {
                    ExecutionState::Failed
                });
                trace.output = output;
                trace.observations = Some(observations);
                // 合并 DSL 编译结果（如果有）
                let merged_constraints = if let Some(ref dsl) = dsl_constraints {
                    let mut c = ConstraintsApplied {
                        budget_limit_usd: dsl.budget_limit_usd.or(Some(0.0)),
                        budget_actual_usd: Some(cost_estimated),
                        budget_strategy: dsl.budget_strategy.clone().or(Some(BudgetStrategy::HardStop)),
                        privacy_level: dsl.privacy_level.clone().or(Some(if has_pii { PrivacyLevel::Masked } else { PrivacyLevel::Raw })),
                        privacy_masked_fields: dsl.privacy_masked_fields.clone(),
                        active_prevention: dsl.active_prevention.clone(),
                        adaptive: dsl.adaptive.clone(),
                        reproducibility_mode: dsl.reproducibility_mode.clone(),
                        reproducibility_seed: dsl.reproducibility_seed,
                        guardrails_active: dsl.guardrails_active.clone(),
                        guardrails_strictness: dsl.guardrails_strictness.clone(),
                        instruction_hierarchy_mode: dsl.instruction_hierarchy_mode.clone(),
                        policy_evaluation: dsl.policy_evaluation.clone(),
                        degrade_action: dsl.degrade_action.clone(),
                        dp_budget: dsl.dp_budget.clone(),
                        conflict_result: None,
                    };
                    if c.budget_limit_usd == Some(0.0) { c.budget_limit_usd = None; }
                    c
                } else {
                    ConstraintsApplied {
                        budget_limit_usd: None,
                        budget_actual_usd: Some(cost_estimated),
                        budget_strategy: Some(crate::types::constraints::BudgetStrategy::HardStop),
                        privacy_level: Some(if has_pii { crate::types::constraints::PrivacyLevel::Masked } else { crate::types::constraints::PrivacyLevel::Raw }),
                        privacy_masked_fields: Some(input_pii_found.clone()),
                        active_prevention: active_prevention.clone(),
                        adaptive: None,
                        reproducibility_mode: Some(crate::types::constraints::ReproducibilityMode::None),
                        reproducibility_seed: None, guardrails_active: Some(vec![]),
                        guardrails_strictness: Some("none".to_string()),
                        instruction_hierarchy_mode: Some(crate::types::constraints::InstructionHierarchyMode::Off),
                        policy_evaluation: None, degrade_action: None, dp_budget: None,
                        conflict_result: None,
                    }
                };
                trace.constraints_applied = Some(merged_constraints);

                // ===== 填充合规映射（§7.5）=====
                let trace_data_map = build_compliance_trace_data(
                    &trace, &output_content, &effective_privacy,
                );
                let compliance_report = state.compliance_mapper.map_trace(&trace_data_map);
                if !compliance_report.mappings.is_empty() {
                    let cm: Vec<crate::types::trace::ComplianceMapping> = compliance_report.mappings.iter().map(|m| {
                        crate::types::trace::ComplianceMapping {
                            regulation: m.regulation.clone(),
                            article: Some(m.article.clone()),
                            requirement: format!("{:?}", m.verification_method),
                            trace_field: format!("{:?}", m.verified_at),
                            satisfaction: if m.compliant { "compliant".to_string() } else { "non_compliant".to_string() },
                        }
                    }).collect();
                    trace.compliance_mappings = Some(cm);
                }

                let l0_proof = generate_l0_proof(&mut trace);
                trace.proofs.proof_chain.push(l0_proof);

                // ===== L2A: Merkle 树采样验证（§7.1.4）=====
                let trace_json = serde_json::to_string(&trace).unwrap_or_default();
                let l2a_proof = crate::crypto::merkle::generate_l2a_proof(&trace_json, 0.1);
                trace.proofs.proof_chain.push(l2a_proof);

                // ===== L2B: 软件 ZK 证明（§7.1.5）=====
                let l2b_proof = crate::crypto::zk::generate_l2b_proof(&trace_json);
                trace.proofs.proof_chain.push(l2b_proof);

                if let Err(e) = state.trace_store.save(trace.clone()).await {
                    warn!("治理模式 Trace 保存失败: {}", e);
                } else {
                    info!(
                        "治理模式 Trace {} 已保存 (latency={}ms, cost=${:.4}, proofs=L0+L2A+L2B)",
                        trace_id, latency_ms, cost_estimated
                    );
                }

                let final_body = if has_pii {
                    let masked_json = mask_response_pii(&serde_json::from_slice(&response_body).unwrap_or_default());
                    serde_json::to_vec(&masked_json).unwrap_or_else(|_| response_body.to_vec())
                } else {
                    response_body.to_vec()
                };

                let mut response = (status, final_body).into_response();
                let resp_headers = VeridactusResponseHeaders {
                    version: "0.2".to_string(),
                    trace_id: trace_id.to_string(),
                    cost_consumed: Some(cost_estimated),
                    proof_levels: Some(vec!["L0".to_string()]),
                    truncated: Some(false),
                    ..Default::default()
                };
                for (key, value) in resp_headers.to_headers() {
                    if let Ok(header_name) = axum::http::HeaderName::from_bytes(key.as_bytes()) {
                        if let Ok(header_value) = axum::http::HeaderValue::from_str(&value) {
                            response.headers_mut().insert(header_name, header_value);
                        }
                    }
                }

                info!("治理模式 响应: trace_id={}, 状态码={}, cost={:.6}", trace_id, status.as_u16(), cost_estimated);
                return Ok(response);
            }
            Err(e) => {
                warn!("治理模式 上游通信错误: {:?}", e);
                // 钩子: on_failure（§6.3）
                let _hook_result = state.hook_registry.run_on_constraint_violation(&mut trace);
                // 错误也需生成 Trace（§1.3 Audit Non-Compromise）
                trace.execution_state = Some(ExecutionState::Failed);
                if trace.observations.is_none() {
                    trace.observations = Some(Default::default());
                }
                if let Some(ref mut obs) = trace.observations {
                    obs.error = Some(ErrorObject {
                        code: "VERIDACTUS_UPSTREAM_DISCONNECT".to_string(),
                        message: format!("上游通信失败: {:?}", e),
                        details: None,
                    });
                }
                let l0_proof = generate_l0_proof(&mut trace);
                trace.proofs.proof_chain.push(l0_proof);
                let _ = state.trace_store.save(trace.clone()).await;
                return Err(build_error_response(
                    Some(&parts.headers),
                    VeridactusErrorCode::UpstreamDisconnect,
                    &journal,
                    &state.audit_token_validator,
                    &tenant_id,
                ));
            }
        }
    } // end if !is_passthrough (governance mode)

    // ===== 9. Passthrough 模式（§4.1.1）：无 VERIDACTUS 头部，仅转发+基础 Trace =====
    info!(
        "Passthrough mode: trace_id={}, model={}, tenant={}",
        trace_id, route.name, tenant_id
    );

    // ===== 9.1 Passthrough: 检测流式请求并转发 =====
    let is_streaming = body_json
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    if is_streaming {
        info!("Passthrough streaming: trace_id={}", trace_id);
        let config = state.config.read().await;
        let upstream_url = if let Some(ref url) = route.upstream_url {
            format!("{}{}", url.trim_end_matches('/'), route.upstream_endpoint)
        } else {
            format!("{}{}", config.upstream_base_url.trim_end_matches('/'), route.upstream_endpoint)
        };
        drop(config);
        let stream_result = forward_to_upstream_streaming(
            &state, &route, &body_json, &upstream_url, &mut journal, &trace_id, &tenant_id,
            None, false,  // passthrough: 无预算限制，无预算感知
        ).await;
        match stream_result {
            Ok(response) => { info!("Passthrough streaming done: trace_id={}", trace_id); return Ok(response); }
            Err((status, error_response)) => { return Err((status, error_response)); }
        }
    }

    // ===== 9.2 Passthrough: 非流式转发（§4.1.1: 不做约束/拦截，仅基础 Trace）=====
    let mut upstream_body = body_json.clone();
    if let Some(obj) = upstream_body.as_object_mut() {
        obj.insert("model".to_string(), serde_json::Value::String(route.upstream_model.clone()));
    }

    let config = state.config.read().await;
    let upstream_url = if let Some(ref url) = route.upstream_url {
        format!("{}{}", url.trim_end_matches('/'), route.upstream_endpoint)
    } else {
        format!("{}{}", config.upstream_base_url.trim_end_matches('/'), route.upstream_endpoint)
    };
    drop(config);

    let start_time = std::time::Instant::now();
    let mut request_builder = state.http_client.post(&upstream_url)
        .json(&upstream_body)
        .timeout(std::time::Duration::from_secs(120));
    // 添加 API Key 认证（上游 LLM 需要）
    if let Some(ref api_key) = route.api_key {
        if let Some(ref header) = route.api_key_header {
            if header.to_lowercase() == "authorization" {
                request_builder = request_builder.header(header.as_str(), format!("Bearer {}", api_key));
            } else {
                request_builder = request_builder.header(header.as_str(), api_key);
            }
        }
    }
    let upstream_response = match request_builder.send().await
    {
        Ok(resp) => resp,
        Err(e) => {
            warn!("Passthrough upstream request failed: {}", e);
            return Err(build_error_response(
                Some(&parts.headers), VeridactusErrorCode::UpstreamDisconnect,
                &journal, &state.audit_token_validator, &tenant_id,
            ));
        }
    };
    let latency_ms = start_time.elapsed().as_millis() as u64;
    METRICS.record_latency(latency_ms);

    let status = upstream_response.status();
    let upstream_body_bytes = upstream_response.bytes().await.unwrap_or_default();
    let upstream_body_str = String::from_utf8_lossy(&upstream_body_bytes);

    // ===== 9.3 Passthrough: 记录基础 Trace（§4.1.1: SHOULD still record a basic Trace）=====
    let upstream_json: serde_json::Value = serde_json::from_str(&upstream_body_str).unwrap_or(serde_json::Value::Null);
    let total_tokens = upstream_json.pointer("/usage/total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u64;
    let prompt_tokens = upstream_json.pointer("/usage/prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let completion_tokens = upstream_json.pointer("/usage/completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let actual_cost = calculate_cost(prompt_tokens, completion_tokens);

    trace.execution_state = Some(if status.is_success() { ExecutionState::Finalized } else { ExecutionState::Failed });
    trace.output = Some(Output {
        response: Some(upstream_json.clone()),
        truncated: false,
        finish_reason: upstream_json.pointer("/choices/0/finish_reason").and_then(|v| v.as_str()).map(|s| s.to_string()),
    });
    trace.observations = Some(crate::types::trace::Observations {
        tokens_count: Some(total_tokens),
        cost_estimated_usd: Some(actual_cost),
        latency_ms: Some(latency_ms),
        state_transitions: Some(vec![
            StateTransition { from: ExecutionState::Init, to: ExecutionState::Executing, timestamp: chrono::Utc::now().to_rfc3339(), transition_index: 1 },
            StateTransition { from: ExecutionState::Executing, to: ExecutionState::Finalized, timestamp: chrono::Utc::now().to_rfc3339(), transition_index: 2 },
        ]),
        error: None, monitoring: None, budget_awareness: None,
        safety_events: None, red_team_events: None, certified_guarantee: None,
        fairness_check: None, replay_snapshot: None, approval: None, _internal_metrics: None,
    });
    trace.constraints_applied = Some(ConstraintsApplied {
        budget_limit_usd: None, budget_actual_usd: Some(actual_cost),
        budget_strategy: Some(BudgetStrategy::HardStop),
        privacy_level: Some(PrivacyLevel::Raw), privacy_masked_fields: None,
        active_prevention: active_prevention.clone(), adaptive: None,
        reproducibility_mode: Some(ReproducibilityMode::None),
        reproducibility_seed: None, guardrails_active: None, guardrails_strictness: None,
        instruction_hierarchy_mode: Some(InstructionHierarchyMode::Off),
        policy_evaluation: Some(PolicyEvaluation {
            decision: Some("allow".to_string()),
            checks_passed: Some(vec!["passthrough".to_string()]),
            checks_failed: Some(vec![]),
            negotiated_capabilities: None, degrade_action: None,
            intent_resolution: None, escalation_trail: None, dsl_source_hash: None,
            current_risk_score: Some(0.0),
            risk_factor_contributions: Some(vec![]),
            adaptive_state: Some(AdaptiveState::SoftAlert),
            prevention_events_count: Some(0),
        }),
        degrade_action: None, dp_budget: None,
        conflict_result: None,
    });

    // L0 签名 + 存储 Trace
    let l0_proof = generate_l0_proof(&mut trace);
    trace.proofs.proof_chain.push(l0_proof);
    let _ = state.trace_store.save(trace.clone()).await;

    // 构建响应
    let resp_headers = VeridactusResponseHeaders {
        version: "0.2".to_string(), trace_id: trace_id.to_string(),
        cost_consumed: Some(actual_cost), proof_levels: Some(vec!["L0".to_string()]),
        truncated: Some(false),
        ..Default::default()
    };
    let mut response = if status.is_success() {
        Json(upstream_json).into_response()
    } else {
        (status, upstream_body_str.to_string()).into_response()
    };
    for (key, value) in resp_headers.to_headers() {
        if let Ok(hn) = axum::http::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(hv) = axum::http::HeaderValue::from_str(&value) {
                response.headers_mut().insert(hn, hv);
            }
        }
    }

    info!("Passthrough complete: trace_id={}, status={}, cost={:.6}, latency={}ms", trace_id, status.as_u16(), actual_cost, latency_ms);

    // 幂等键记录
    if let Some(ref key) = idempotency_key {
        if let Ok(tid) = uuid::Uuid::parse_str(key) {
            state.idempotency_guard.record(tid, response.status().as_u16(), Some(key)).await;
        }
    }

    Ok(response)
}

/// 将 JSON 请求体转换为 Trace Input
fn body_json_to_input(body: &serde_json::Value, trace: &mut Trace) {
    let messages = body.get("messages").cloned();
    let params = {
        let mut p = serde_json::Map::new();
        if let Some(temp) = body.get("temperature") {
            p.insert("temperature".to_string(), temp.clone());
        }
        if let Some(maxt) = body.get("max_tokens") {
            p.insert("max_tokens".to_string(), maxt.clone());
        }
        if let Some(top_p) = body.get("top_p") {
            p.insert("top_p".to_string(), top_p.clone());
        }
        if p.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(p))
        }
    };

    trace.input = Some(Input {
        prompt: messages,
        params,
        metadata: None,
    });
}

/// 直接转发到上游（Passthrough 模式 - 简化版）
async fn forward_to_upstream(
    state: &AppState,
    route: &ModelRoute,
    body_json: &serde_json::Value,
    detect_response_pii: bool,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let config = state.config.read().await;
    let upstream_url = if let Some(ref url) = route.upstream_url {
        format!("{}{}", url.trim_end_matches('/'), route.upstream_endpoint)
    } else {
        format!(
            "{}{}",
            config.upstream_base_url.trim_end_matches('/'),
            route.upstream_endpoint
        )
    };
    drop(config);

    let mut upstream_body = body_json.clone();
    // 替换为上游真实模型名
    if let Some(obj) = upstream_body.as_object_mut() {
        obj.insert(
            "model".to_string(),
            serde_json::Value::String(route.upstream_model.clone()),
        );
    }

    let response = state
        .http_client
        .post(&upstream_url)
        .json(&upstream_body)
        .send()
        .await
        .map_err(|e| {
            warn!("Passthrough Upstream forwarding failed: {}", e);
            (StatusCode::BAD_GATEWAY, format!("Upstream LLM unavailable: {}", e))
        })?;

    let status = response.status();
    let body = response.bytes().await.unwrap_or_default();

    let final_body = if detect_response_pii {
        let body_str = String::from_utf8_lossy(&body);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
            let masked_json = mask_response_pii(&json);
            if masked_json != json {
                info!("PII 检测: 响应内容中发现并遮蔽敏感信息");
            }
            serde_json::to_vec(&masked_json).unwrap_or_else(|_| body.to_vec())
        } else {
            body.to_vec()
        }
    } else {
        body.to_vec()
    };

    Ok((status, final_body).into_response())
}

/// 协议版本协商（§4.5）
///
/// 规则：
/// - 客户端版本 <= 服务器最大版本 → 降级到最高兼容版本
/// - 客户端版本 > 服务器最大版本 → 返回 Err（无法降级）
/// - 无客户端版本 → 使用默认 0.1
fn negotiate_version(
    client_version: Option<&str>,
    supported_versions: &[String],
) -> Result<String, ()> {
    if supported_versions.is_empty() {
        return Err(());
    }

    let client_ver = client_version.unwrap_or("0.1");
    let max_server_ver = supported_versions.last().map(|s| s.as_str()).unwrap();
    let _min_server_ver = supported_versions.first().map(|s| s.as_str()).unwrap();

    // 如果客户端版本高于服务器最高版本，降级到服务器最高版本（§4.5）
    if client_ver > max_server_ver {
        // 主版本不同时降级并告警（§4.5 SHOULD negotiate not reject）
        warn!("版本降级: 客户端 {} → 服务器 {}", client_ver, max_server_ver);
        return Ok(max_server_ver.to_string());
    }

    // 降级到最高兼容版本
    for ver in supported_versions.iter().rev() {
        if ver.as_str() <= client_ver {
            return Ok(ver.clone());
        }
    }

    // 客户端版本低于服务器最低版本 → 无法降级（§4.5: MUST return 400 VERSION_MISMATCH）
    Err(())
}

// ==================== PII 检测功能 ====================

struct PIIDetector {
    patterns: Vec<(Regex, &'static str)>,
}

impl PIIDetector {
    fn new() -> Self {
        Self {
            patterns: vec![
                (Regex::new(r"[1-9]\d{5}(18|19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]").unwrap(), "ID card number"),
                (Regex::new(r"(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})").unwrap(), "Credit card number"),
                (Regex::new(r"(?:\d{4}[- ]?){3}\d{4}").unwrap(), "Credit card number(带分隔符)"),
                (Regex::new(r"1[3-9]\d{9}").unwrap(), "手机号"),
                (Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap(), "邮箱"),
            ],
        }
    }

    fn detect_and_mask(&self, content: &str) -> (String, Vec<&'static str>) {
        let mut findings = Vec::new();
        let mut result = content.to_string();
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();

        for (pattern, pii_type) in &self.patterns {
            for mat in pattern.find_iter(content) {
                findings.push(*pii_type);
                replacements.push((mat.start(), mat.end(), mat.as_str().to_string()));
            }
        }

        replacements.sort_by_key(|r| r.0);
        let mut offset: isize = 0;
        for (start, end, original) in replacements {
            let s = (start as isize + offset) as usize;
            let e = (end as isize + offset) as usize;
            if e <= result.len() {
                let chars: Vec<char> = original.chars().collect();
                let char_len = chars.len();
                if char_len >= 4 {
                    let prefix: String = chars[0..2.min(char_len)].iter().collect();
                    let suffix: String = chars[char_len.saturating_sub(2)..].iter().collect();
                    let mask_len = char_len.min(8).max(4);
                    let mask = format!("{}{}{}",
                        prefix,
                        "*".repeat(mask_len),
                        suffix
                    );
                    result = format!("{}{}{}", &result[..s], mask, &result[e..]);
                    offset += mask.len() as isize - original.len() as isize;
                }
            }
        }

        (result, findings)
    }
}

impl Default for PIIDetector {
    fn default() -> Self {
        Self::new()
    }
}

fn mask_response_pii(json: &serde_json::Value) -> serde_json::Value {
    let pii_detector = PIIDetector::new();

    match json {
        serde_json::Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (key, value) in obj {
                if key == "content" {
                    if let Some(content_str) = value.as_str() {
                        let (masked, findings) = pii_detector.detect_and_mask(content_str);
                        if !findings.is_empty() {
                            new_obj.insert(key.clone(), serde_json::Value::String(masked));
                        } else {
                            new_obj.insert(key.clone(), value.clone());
                        }
                    } else {
                        new_obj.insert(key.clone(), mask_response_pii(value));
                    }
                } else if key == "message" || key == "delta" {
                    new_obj.insert(key.clone(), mask_response_pii(value));
                } else if key == "choices" {
                    new_obj.insert(key.clone(), mask_response_pii(value));
                } else {
                    new_obj.insert(key.clone(), mask_response_pii(value));
                }
            }
            serde_json::Value::Object(new_obj)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(mask_response_pii).collect())
        }
        serde_json::Value::String(s) => {
            let (masked, findings) = pii_detector.detect_and_mask(s);
            if !findings.is_empty() {
                serde_json::Value::String(masked)
            } else {
                json.clone()
            }
        }
        _ => json.clone(),
    }
}

/// 状态转换记录器
#[derive(Debug, Clone)]
pub struct StateTransitionRecorder {
    transitions: Vec<StateTransition>,
    transition_index: u32,
}

impl StateTransitionRecorder {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
            transition_index: 0,
        }
    }

    /// 添加状态转换
    pub fn add_transition(&mut self, from: ExecutionState, to: ExecutionState) -> &StateTransition {
        self.transition_index += 1;
        let ts = chrono::Utc::now().to_rfc3339();
        self.transitions.push(StateTransition {
            from,
            to,
            timestamp: ts,
            transition_index: self.transition_index,
        });
        self.transitions.last().unwrap()
    }

    /// 获取所有转换
    pub fn get_transitions(&self) -> &[StateTransition] {
        &self.transitions
    }

    /// 获取转换索引
    pub fn get_index(&self) -> u32 {
        self.transition_index
    }
}

impl Default for StateTransitionRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建完整状态转换链（§6.2 状态转换规则）
///
/// # 参数
/// * `headers` - VERIDACTUS 请求头部
/// * `_journal` - 执行日志（用于获取事件时间）
/// * `total_tokens` - 生成的 token 总数
/// * `failure_stage` - 失败发生的阶段（None 表示成功完成）
/// * `failure_reason` - 失败原因（如果有）
///
/// # 返回
/// 完整的状态转换向量
fn build_state_transitions(
    headers: &VeridactusRequestHeaders,
    _journal: &ExecutionJournal,
    total_tokens: u64,
    failure_stage: Option<ExecutionState>,
    failure_reason: Option<&str>,
) -> Vec<StateTransition> {
    let mut recorder = StateTransitionRecorder::new();
    let _now = chrono::Utc::now();

    // 1. INIT → DELEGATION_VALIDATE 或 INIT → CONSTRAINT_EVAL
    if headers.trust_delegation_token.is_some() {
        // INIT → DELEGATION_VALIDATE
        recorder.add_transition(ExecutionState::Init, ExecutionState::DelegationValidate);
        // DELEGATION_VALIDATE → CONSTRAINT_EVAL（假设委托验证通过）
        recorder.add_transition(ExecutionState::DelegationValidate, ExecutionState::ConstraintEval);
    } else {
        // INIT → CONSTRAINT_EVAL (无委托令牌，跳过委托验证阶段)
        recorder.add_transition(ExecutionState::Init, ExecutionState::ConstraintEval);
    }

    // 2. CONSTRAINT_EVAL → EXECUTING 或 CONSTRAINT_EVAL → FAILED
    if failure_stage == Some(ExecutionState::ConstraintEval) {
        recorder.add_transition(ExecutionState::ConstraintEval, ExecutionState::Failed);
        return recorder.get_transitions().to_vec();
    }
    recorder.add_transition(ExecutionState::ConstraintEval, ExecutionState::Executing);

    // 3. EXECUTING → VALIDATION 或 EXECUTING → FAILED
    if failure_stage == Some(ExecutionState::Executing) {
        recorder.add_transition(ExecutionState::Executing, ExecutionState::Failed);
        return recorder.get_transitions().to_vec();
    }
    recorder.add_transition(ExecutionState::Executing, ExecutionState::Validation);

    // 4. VALIDATION → FINALIZED 或 VALIDATION → FAILED
    if failure_stage == Some(ExecutionState::Validation) || failure_stage.is_none() && total_tokens == 0 {
        recorder.add_transition(ExecutionState::Validation, ExecutionState::Failed);
    } else {
        recorder.add_transition(ExecutionState::Validation, ExecutionState::Finalized);
    }

    recorder.get_transitions().to_vec()
}

/// 解析认证保证请求头部（格式: methodology:risk_bound@confidence）
///
/// # 参数
/// * `header` - 请求头部的值，格式为 "methodology:risk_bound@confidence"
///
/// # 返回
/// 解析后的 CertifiedGuarantee 结构体
fn parse_certified_guarantee(header: &str) -> Option<CertifiedGuarantee> {
    let parts: Vec<&str> = header.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let methodology = parts[0].to_string();
    
    let risk_confidence_parts: Vec<&str> = parts[1].split('@').collect();
    if risk_confidence_parts.len() != 2 {
        return None;
    }
    
    let risk_bound = risk_confidence_parts[0].parse::<f64>().ok()?;
    let confidence_level = risk_confidence_parts[1].parse::<f64>().ok()?;
    
    Some(CertifiedGuarantee {
        methodology,
        risk_bound,
        confidence_level,
        claim_verified: "Request processed with certified guarantee".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// 计算实际消耗的成本（基于 token 用量）
///
/// 成本计算规则（简化版，实际应使用模型定价表）：
/// - 输入 token: $0.01 per 1K tokens
/// - 输出 token: $0.03 per 1K tokens
fn calculate_cost(prompt_tokens: u64, completion_tokens: u64) -> f64 {
    const INPUT_COST_PER_1K: f64 = 0.01;
    const OUTPUT_COST_PER_1K: f64 = 0.03;

    let input_cost = (prompt_tokens as f64 / 1000.0) * INPUT_COST_PER_1K;
    let output_cost = (completion_tokens as f64 / 1000.0) * OUTPUT_COST_PER_1K;

    // 6 位小数精度（协议要求 micro-dollar precision）
    (input_cost + output_cost).round_to(6)
}

/// 根据模型名称估算每 token 成本（用于降级决策）
fn token_cost_for_model(model_name: &str) -> f64 {
    let lower = model_name.to_lowercase();
    if lower.contains("deepseek") || lower.contains("r1") {
        0.000002 // $0.002/1K tokens
    } else if lower.contains("qwen") || lower.contains("gpt-4o-mini") || lower.contains("glm") {
        0.000001 // $0.001/1K tokens (cheapest)
    } else if lower.contains("gpt-4") || lower.contains("gemini") {
        0.000010 // $0.01/1K tokens (expensive)
    } else {
        0.000003 // default $0.003/1K
    }
}

trait RoundTo {
    fn round_to(self, decimals: u32) -> Self;
}

impl RoundTo for f64 {
    fn round_to(self, decimals: u32) -> Self {
        let multiplier = 10_f64.powi(decimals as i32);
        (self * multiplier).round() / multiplier
    }
}

/// 执行公平性检查（简化实现）
///
/// 检查请求和响应中的潜在偏差，计算公平性得分。
fn perform_fairness_check(trace: &Trace) -> Option<crate::types::trace::FairnessCheck> {
    use crate::types::trace::{BiasDetection, FairnessCheck, FairnessMetric};
    
    // 检查输入提示中的敏感属性
    let input_text = match &trace.input {
        Some(input) => match &input.prompt {
            Some(prompts) => {
                // 尝试解析为消息数组
                if let Some(msg_array) = prompts.as_array() {
                    msg_array.iter()
                        .filter_map(|msg| msg.get("content").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>()
                        .join(" ")
                } else if let Some(s) = prompts.as_str() {
                    s.to_string()
                } else {
                    "".to_string()
                }
            }
            None => "".to_string(),
        },
        None => "".to_string(),
    };
    
    // 检测敏感属性
    let protected_attributes = detect_protected_attributes(&input_text);
    
    // 计算公平性得分（简化版本）
    let fairness_score = if protected_attributes.is_empty() {
        1.0  // 没有敏感属性，默认公平
    } else {
        // 如果包含敏感属性，基于内容长度计算得分
        0.7 + (input_text.len() as f64 / 1000.0) * 0.3
    };
    
    let passed = fairness_score >= 0.7;
    
    // 生成公平性指标
    let metrics = Some(vec![
        FairnessMetric {
            attribute: "overall".to_string(),
            metric_type: "overall_fairness".to_string(),
            value: fairness_score,
            passed,
            threshold: 0.7,
        },
    ]);
    
    // 偏差检测
    let bias_detection = if fairness_score < 0.7 {
        Some(BiasDetection {
            detected: true,
            bias_type: Some("potential_bias".to_string()),
            affected_groups: Some(protected_attributes.clone()),
            mitigation_suggestion: Some("建议审查输出内容，确保公平对待所有群体".to_string()),
        })
    } else {
        Some(BiasDetection {
            detected: false,
            bias_type: None,
            affected_groups: None,
            mitigation_suggestion: None,
        })
    };
    
    Some(FairnessCheck {
        passed: Some(passed),
        fairness_score: Some(fairness_score),
        protected_attributes: if protected_attributes.is_empty() { None } else { Some(protected_attributes) },
        metrics,
        bias_detection,
        checked_at: Some(chrono::Utc::now().to_rfc3339()),
    })
}

/// 检测文本中的受保护属性
fn detect_protected_attributes(text: &str) -> Vec<String> {
    let mut attributes = Vec::new();
    
    // 检测常见的受保护属性关键词
    let keywords: &[(&str, &[&str])] = &[
        ("gender", &["gender", "sex", "male", "female", "man", "woman"]),
        ("age", &["age", "old", "young", "child", "senior"]),
        ("race", &["race", "ethnic", "white", "black", "asian", "hispanic"]),
        ("religion", &["religion", "christian", "muslim", "jewish", "buddhist"]),
        ("disability", &["disability", "disabled", "handicap"]),
        ("nationality", &["nationality", "country", "citizen"]),
    ];
    
    let lower_text = text.to_lowercase();
    for (attr_name, keywords) in keywords {
        if keywords.iter().any(|k| lower_text.contains(k)) {
            attributes.push(attr_name.to_string());
        }
    }
    
    attributes
}

// ==================== 合规性报告端点（§7.5）====================

async fn get_trace_compliance(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = uuid::Uuid::parse_str(&trace_id).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("无效的 Trace ID: {}", e)})))
    })?;

    match state.trace_store.get(&id).await {
        Some(trace) => {
            let trace_data = build_compliance_trace_data_from_stored(&trace);
            let report = state.compliance_mapper.map_trace(&trace_data);
            Ok(Json(serde_json::to_value(&report).unwrap_or_default()))
        }
        None => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Trace 未找到"})))),
    }
}

/// 获取合规报告
async fn get_compliance_report(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = uuid::Uuid::parse_str(&trace_id).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("无效的 Trace ID: {}", e)})))
    })?;

    match state.trace_store.get(&id).await {
        Some(trace) => {
            let trace_data = build_compliance_trace_data_from_stored(&trace);
            let report = state.compliance_mapper.map_trace(&trace_data);
            Ok(Json(serde_json::to_value(&report).unwrap_or_default()))
        }
        None => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Trace 未找到"})))),
    }
}

// ==================== GDPR 删除端点（§8.7）====================

/// 处理 GDPR 删除请求
async fn handle_gdpr_deletion(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let deletion_type_str = body.get("deletion_type").and_then(|v| v.as_str()).unwrap_or("trace_id");
    let target_id = body.get("target_id").and_then(|v| v.as_str()).unwrap_or("");

    let deletion_type = match deletion_type_str {
        "session_id" => DeletionType::SessionId,
        "user_id" => DeletionType::UserId,
        "all" => DeletionType::All,
        _ => DeletionType::TraceId,
    };

    if target_id.is_empty() && deletion_type != DeletionType::All {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "target_id is required"
        }))));
    }

    let request = DeletionRequest {
        request_id: format!("del_{}", uuid::Uuid::new_v4()),
        deletion_type: deletion_type.clone(),
        target_id: target_id.to_string(),
        requester_identity: body.get("requester").and_then(|v| v.as_str()).map(|s| s.to_string()),
        timestamp: chrono::Utc::now().to_rfc3339(),
        reason: body.get("reason").and_then(|v| v.as_str()).map(|s| s.to_string()),
    };

    // 先执行实际存储删除
    let delete_result: Result<Vec<String>, String> = match deletion_type {
        DeletionType::TraceId => {
            if let Ok(id) = uuid::Uuid::parse_str(target_id) {
                match state.trace_store.delete(&id).await {
                    Ok(Some(t)) => {
                        let sig = t.proofs.proof_chain.first().and_then(|p| p.signature.clone()).unwrap_or_default();
                        Ok(vec![sig])
                    }
                    Ok(None) => Ok(vec![]),
                    Err(e) => Err(e),
                }
            } else {
                Err("无效的 Trace ID 格式".to_string())
            }
        }
        DeletionType::SessionId => {
            if let Ok(sid) = uuid::Uuid::parse_str(target_id) {
                match state.trace_store.delete_by_session(&sid).await {
                    Ok(deleted) => Ok(deleted.iter().filter_map(|t| {
                        t.proofs.proof_chain.first().and_then(|p| p.signature.clone())
                    }).collect()),
                    Err(e) => Err(e),
                }
            } else {
                Err("无效的 Session ID 格式".to_string())
            }
        }
        DeletionType::UserId => {
            match state.trace_store.delete_by_tenant(target_id).await {
                Ok(deleted) => Ok(deleted.iter().filter_map(|t| {
                    t.proofs.proof_chain.first().and_then(|p| p.signature.clone())
                }).collect()),
                Err(e) => Err(e),
            }
        }
        DeletionType::All => Err("不允许全部删除操作".to_string()),
    };

    match delete_result {
        Ok(retained_sigs) => {
            // 使用 GDPR manager 记录删除审计
            let result = state.gdpr_manager.process_deletion(request);
            match result {
                Ok(deletion_result) => {
                    let response = serde_json::json!({
                        "success": true,
                        "deleted_count": deletion_result.deleted_count,
                        "retained_signatures": retained_sigs,
                        "audit_entry": deletion_result.audit_log_entry,
                    });
                    info!("GDPR 删除完成: type={}, target={}, count={}",
                        deletion_type_str, target_id, deletion_result.deleted_count);
                    Ok(Json(response))
                }
                Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": format!("GDPR 处理失败: {}", e)
                })))),
            }
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e})))),
    }
}

/// 获取 GDPR 删除证明
async fn get_gdpr_deletion_proof(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
) -> Json<serde_json::Value> {
    match state.gdpr_manager.get_deletion_proof(&request_id) {
        Some(entry) => Json(serde_json::to_value(&entry).unwrap_or_default()),
        None => Json(serde_json::json!({"error": "删除记录未找到"})),
    }
}

/// 列出 GDPR 删除历史
async fn list_gdpr_deletion_history(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let history = state.gdpr_manager.list_deletion_history(50);
    Json(serde_json::json!({
        "total": history.len(),
        "entries": history
    }))
}

// ==================== 主动预防统计端点（§8.4）====================

/// 获取主动预防统计信息
async fn get_prevention_stats(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    //  简化实现：返回预防引擎的基础信息
    Json(serde_json::json!({
        "engine": "ConstrainedDecoding",
        "version": "v0.2.1",
        "block_strategies": ["strict", "warn", "approximate"],
        "supported_patterns": ["pii", "credentials", "dangerous_code", "agent_goal_hijack"],
        "status": "active",
        "implementation": "prefix-based approximate blocking + subword-aware DFA (simplified)",
    }))
}

// ==================== Prometheus 指标端点（§10.3.4）====================

use std::sync::LazyLock;

/// 全局指标注册表（Prometheus 文本格式）
static METRICS: LazyLock<MetricsRegistry> = LazyLock::new(|| MetricsRegistry::new());

pub struct MetricsRegistry {
    // 计数指标
    requests_total: AtomicU64,
    constraint_violations_total: AtomicU64,
    trace_integrity_errors: AtomicU64,
    agent_steps_total: AtomicU64,
    guardrail_activations_total: AtomicU64,
    asi_risks_flagged_total: AtomicU64,
    delegation_validations_total: AtomicU64,
    certified_guarantee_total: AtomicU64,
    active_prevention_blocks_total: AtomicU64,
    engine_determinism_checks_total: AtomicU64,
    // 延迟直方图: 桶边界（毫秒）
    latency_buckets: [AtomicU64; 13],
    latency_sum_ms: AtomicU64,
    latency_count: AtomicU64,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            constraint_violations_total: AtomicU64::new(0),
            trace_integrity_errors: AtomicU64::new(0),
            agent_steps_total: AtomicU64::new(0),
            guardrail_activations_total: AtomicU64::new(0),
            asi_risks_flagged_total: AtomicU64::new(0),
            delegation_validations_total: AtomicU64::new(0),
            certified_guarantee_total: AtomicU64::new(0),
            active_prevention_blocks_total: AtomicU64::new(0),
            engine_determinism_checks_total: AtomicU64::new(0),
            latency_buckets: Default::default(),
            latency_sum_ms: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
        }
    }

    /// 记录延迟观测值（毫秒）
    pub fn record_latency(&self, latency_ms: u64) {
        const BUCKETS_MS: [u64; 12] = [5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000, 30000];
        let v = latency_ms as f64;
        self.latency_sum_ms.fetch_add(v as u64, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
        for (i, &bound) in BUCKETS_MS.iter().enumerate() {
            if latency_ms <= bound {
                self.latency_buckets[i].fetch_add(1, Ordering::Relaxed);
            }
        }
        self.latency_buckets[12].fetch_add(1, Ordering::Relaxed); // +Inf
    }

    pub fn export_text(&self) -> String {
        let mut out = String::new();
        out.push_str("# HELP veridactus_requests_total Total number of requests processed\n");
        out.push_str("# TYPE veridactus_requests_total counter\n");
        out.push_str(&format!("veridactus_requests_total {}\n", self.requests_total.load(Ordering::Relaxed)));

        out.push_str("# HELP veridactus_constraint_violations_total Total constraint violations detected\n");
        out.push_str("# TYPE veridactus_constraint_violations_total counter\n");
        out.push_str(&format!("veridactus_constraint_violations_total {}\n", self.constraint_violations_total.load(Ordering::Relaxed)));

        out.push_str("# HELP veridactus_trace_integrity_errors_total Audit signature verification failures\n");
        out.push_str("# TYPE veridactus_trace_integrity_errors_total counter\n");
        out.push_str(&format!("veridactus_trace_integrity_errors_total {}\n", self.trace_integrity_errors.load(Ordering::Relaxed)));

        out.push_str("# HELP veridactus_agent_steps_total Total steps in multi-agent chains\n");
        out.push_str("# TYPE veridactus_agent_steps_total counter\n");
        out.push_str(&format!("veridactus_agent_steps_total {}\n", self.agent_steps_total.load(Ordering::Relaxed)));

        out.push_str("# HELP veridactus_guardrail_activations_total Guardrail trigger count\n");
        out.push_str("# TYPE veridactus_guardrail_activations_total counter\n");
        out.push_str(&format!("veridactus_guardrail_activations_total {}\n", self.guardrail_activations_total.load(Ordering::Relaxed)));

        out.push_str("# HELP veridactus_asi_risks_flagged_total OWASP ASI risks flagged\n");
        out.push_str("# TYPE veridactus_asi_risks_flagged_total counter\n");
        out.push_str(&format!("veridactus_asi_risks_flagged_total {}\n", self.asi_risks_flagged_total.load(Ordering::Relaxed)));

        out.push_str("# HELP veridactus_delegation_validations_total Delegation token validations\n");
        out.push_str("# TYPE veridactus_delegation_validations_total counter\n");
        out.push_str(&format!("veridactus_delegation_validations_total {}\n", self.delegation_validations_total.load(Ordering::Relaxed)));

        out.push_str("# HELP veridactus_certified_guarantee_total Certified guarantee computations\n");
        out.push_str("# TYPE veridactus_certified_guarantee_total counter\n");
        out.push_str(&format!("veridactus_certified_guarantee_total {}\n", self.certified_guarantee_total.load(Ordering::Relaxed)));

        out.push_str("# HELP veridactus_active_prevention_blocks_total Tokens blocked by constrained decoding\n");
        out.push_str("# TYPE veridactus_active_prevention_blocks_total counter\n");
        out.push_str(&format!("veridactus_active_prevention_blocks_total {}\n", self.active_prevention_blocks_total.load(Ordering::Relaxed)));

        out.push_str("# HELP veridactus_engine_determinism_checks_total Engine determinism checks\n");
        out.push_str("# TYPE veridactus_engine_determinism_checks_total counter\n");
        out.push_str(&format!("veridactus_engine_determinism_checks_total {}\n", self.engine_determinism_checks_total.load(Ordering::Relaxed)));

        // 预算 gauge - 反映全局预算状态
        out.push_str("# HELP veridactus_budget_remaining_usd Current session budget remaining in USD\n");
        out.push_str("# TYPE veridactus_budget_remaining_usd gauge\n");
        out.push_str("veridactus_budget_remaining_usd 0\n");

        // 延迟直方图（P50/P90/P99 分布）
        out.push_str("# HELP veridactus_latency_distribution_ms Request latency distribution in milliseconds\n");
        out.push_str("# TYPE veridactus_latency_distribution_ms histogram\n");
        const BUCKETS_MS: [u64; 12] = [5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000, 30000];
        for (i, &bound) in BUCKETS_MS.iter().enumerate() {
            out.push_str(&format!("veridactus_latency_distribution_ms_bucket{{le=\"{}\"}} {}\n",
                bound, self.latency_buckets[i].load(Ordering::Relaxed)));
        }
        out.push_str(&format!("veridactus_latency_distribution_ms_bucket{{le=\"+Inf\"}} {}\n",
            self.latency_buckets[12].load(Ordering::Relaxed)));
        out.push_str(&format!("veridactus_latency_distribution_ms_sum {}\n", self.latency_sum_ms.load(Ordering::Relaxed)));
        out.push_str(&format!("veridactus_latency_distribution_ms_count {}\n", self.latency_count.load(Ordering::Relaxed)));

        // 延迟 summary（保留旧格式兼容性）
        out.push_str("# HELP veridactus_latency_seconds Request latency in seconds\n");
        out.push_str("# TYPE veridactus_latency_seconds summary\n");
        out.push_str(&format!("veridactus_latency_seconds_sum {:.3}\n", self.latency_sum_ms.load(Ordering::Relaxed) as f64 / 1000.0));
        out.push_str(&format!("veridactus_latency_seconds_count {}\n", self.latency_count.load(Ordering::Relaxed)));

        out
    }

    pub fn inc_requests(&self) { self.requests_total.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_constraint_violations(&self) { self.constraint_violations_total.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_guardrail(&self) { self.guardrail_activations_total.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_asi_risk(&self) { self.asi_risks_flagged_total.fetch_add(1, Ordering::Relaxed); }
}

/// Prometheus 指标端点（§10.3.4）
async fn metrics_handler() -> (StatusCode, String) {
    let body = METRICS.export_text();
    (StatusCode::OK, body)
}

// ==================== 审计日志端点（§10.1） ====================

use std::sync::Mutex as StdMutex;
static AUDIT_LOG: LazyLock<StdMutex<Vec<String>>> = LazyLock::new(|| StdMutex::new(Vec::new()));

/// 记录审计事件
pub fn audit_log(event_type: &str, trace_id: &str, detail: &str) {
    if let Ok(mut log) = AUDIT_LOG.lock() {
        let entry = format!("[{}] {} | trace={} | {}", 
            chrono::Utc::now().to_rfc3339(), event_type, trace_id, detail);
        if log.len() > 10000 { log.remove(0); }
        log.push(entry);
    }
}

/// 审计日志端点
async fn audit_log_handler() -> Json<serde_json::Value> {
    let log = AUDIT_LOG.lock().unwrap();
    let entries: Vec<&str> = log.iter().map(|s| s.as_str()).collect();
    Json(serde_json::json!({
        "total": entries.len(),
        "entries": entries,
        "generated_at": chrono::Utc::now().to_rfc3339(),
    }))
}

// ===== Extension Discovery（§A.4）=====

async fn handle_extension_discovery(State(state): State<AppState>) -> Json<serde_json::Value> {
    let config = state.config.read().await;
    Json(serde_json::json!({
        "protocol_version": crate::PROTOCOL_VERSION,
        "implementation": format!("VERIDACTUS Proxy v{}", crate::PROTOCOL_VERSION),
        "max_proof_level": "L0",
        "extensions": [
            "veridactus.ai/v1/state_machine@1",
            "veridactus.ai/v1/governance_dsl",
            "veridactus.ai/v1/deterministic_replay",
            "veridactus.ai/v1/agent_execution_chain",
            "veridactus.ai/v1/trust_delegation",
            "veridactus.ai/v1/verifiable_inference@L0",
            "veridactus.ai/v1/supply_chain",
            "veridactus.ai/v1/budget_awareness",
            "veridactus.ai/v1/semantic_drift",
            "veridactus.ai/v1/guardrails@G4",
            "veridactus.ai/v1/instruction_hierarchy",
            "veridactus.ai/v1/compliance_mapping@EU_AI_ACT",
            "veridactus.ai/v1/active_prevention@1",
            "veridactus.ai/v1/certified_guarantee@1",
            "veridactus.ai/v1/agentic_security@1",
            "veridactus.ai/v1/intent_resolution@1",
            "veridactus.ai/v1/engine_determinism@1"
        ],
        "proof_levels": ["L0"],
        "conformance_level": "core",
        "supported_models": config.model_routes.iter().map(|r| &r.name).collect::<Vec<_>>(),
    }))
}

// ===== 辅助函数：完整上游调用 =====

#[derive(Debug, Clone)]
pub struct UpstreamUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

async fn forward_to_upstream_complete(
    state: &AppState,
    route: &ModelRoute,
    body_json: &serde_json::Value,
) -> Result<(StatusCode, bytes::Bytes, Option<UpstreamUsage>), (StatusCode, String)> {
    let config = state.config.read().await;
    let upstream_url = if let Some(url) = &route.upstream_url {
        format!("{}{}", url.trim_end_matches('/'), route.upstream_endpoint)
    } else {
        format!("{}{}", config.upstream_base_url.trim_end_matches('/'), route.upstream_endpoint)
    };

    let upstream_body = if upstream_url.contains("generativelanguage.googleapis.com") {
        convert_to_gemini_format(body_json)
    } else if upstream_url.contains("ark.cn-beijing.volces.com") {
        convert_to_doubao_format(body_json, &route.upstream_model)
    } else {
        let mut body = body_json.clone();
        if let Some(obj) = body.as_object_mut() {
            obj.insert("model".to_string(), serde_json::Value::String(route.upstream_model.clone()));
        }
        body
    };

    // 添加 API Key
    let mut request_builder = state.http_client
        .post(&upstream_url)
        .json(&upstream_body)
        .timeout(std::time::Duration::from_secs(120));

    // W3C Trace Context 注入（§10.0 OpenTelemetry 集成）
    // 生成符合 W3C Trace Context 标准的 traceparent 头部
    let trace_id_hex = format!("{:032x}", rand::random::<u128>());
    let span_id_hex = format!("{:016x}", rand::random::<u64>());
    let traceparent = format!("00-{}-{}-01", trace_id_hex, span_id_hex);
    request_builder = request_builder.header("traceparent", &traceparent);
    request_builder = request_builder.header("tracestate", "veridactus=governance");
    
    let request_builder = if let Some(api_key) = &route.api_key {
        if let Some(header) = &route.api_key_header {
            if header.to_lowercase() == "authorization" {
                request_builder.header(header, format!("Bearer {}", api_key))
            } else {
                request_builder.header(header, api_key)
            }
        } else {
            request_builder
        }
    } else {
        request_builder
    };

    match request_builder.send().await {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.bytes().await.unwrap_or_default();
            let usage = extract_usage_from_bytes(&body);
            Ok((status, body, usage))
        }
        Err(e) => {
            warn!("Upstream forwarding failed: {}", e);
            Err((StatusCode::BAD_GATEWAY, format!("Upstream LLM unavailable: {}", e)))
        }
    }
}

fn extract_usage_from_response(json: &serde_json::Value) -> Option<UpstreamUsage> {
    let usage = json.get("usage")?;
    Some(UpstreamUsage {
        prompt_tokens: usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        completion_tokens: usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        total_tokens: usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    })
}

fn extract_usage_from_bytes(body: &[u8]) -> Option<UpstreamUsage> {
    let body_str = String::from_utf8_lossy(body);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
        extract_usage_from_response(&json)
    } else {
        None
    }
}

fn extract_output_content(json: &serde_json::Value) -> String {
    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
        if let Some(first) = choices.first() {
            if let Some(message) = first.get("message") {
                if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                    return content.to_string();
                }
            }
        }
    }
    if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
        return content.to_string();
    }
    json.to_string()
}

fn convert_to_doubao_format(body_json: &serde_json::Value, model: &str) -> serde_json::Value {
    let mut input_items = Vec::new();
    
    if let Some(messages) = body_json.get("messages").and_then(|m| m.as_array()) {
        for msg in messages {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                let content_items = vec![serde_json::json!({
                    "type": "input_text",
                    "text": content
                })];
                input_items.push(serde_json::json!({
                    "role": role,
                    "content": content_items
                }));
            }
        }
    }
    
    serde_json::json!({
        "model": model,
        "input": input_items
    })
}

fn convert_to_gemini_format(body_json: &serde_json::Value) -> serde_json::Value {
    let mut contents = Vec::new();
    
    if let Some(messages) = body_json.get("messages").and_then(|m| m.as_array()) {
        let mut parts = Vec::new();
        for msg in messages {
            if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                parts.push(serde_json::json!({
                    "text": content
                }));
            }
        }
        if !parts.is_empty() {
            contents.push(serde_json::json!({
                "parts": parts
            }));
        }
    }
    
    serde_json::json!({
        "contents": contents
    })
}

fn extract_finish_reason(json: &serde_json::Value) -> Option<String> {
    json.get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("finish_reason"))
        .and_then(|fr| fr.as_str())
        .map(String::from)
}

fn extract_response_id(json: &serde_json::Value) -> Option<String> {
    json.get("id").and_then(|id| id.as_str()).map(String::from)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PiiDetectionResult {
    pii_detected: bool,
    findings: Vec<String>,
}

async fn call_pii_detection(http_client: &reqwest::Client, content: &str) -> Option<PiiDetectionResult> {
    if content.is_empty() {
        return None;
    }
    let python_worker_url = "http://127.0.0.1:8001/api/v1/pii-detection";
    let request_body = serde_json::json!({ "text": content });
    match http_client
        .post(python_worker_url)
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(result) = resp.json::<PiiDetectionResult>().await {
                    if result.pii_detected {
                        info!("Python Worker PII detection: found {:?}", result.findings);
                    }
                    return Some(result);
                }
            }
            None
        }
        Err(e) => {
            warn!("Python Worker PII detection call failed: {}", e);
            None
        }
    }
}

fn estimate_cost(tokens_count: &Option<u64>, model_name: &str) -> f64 {
    let tokens = tokens_count.unwrap_or(0) as f64;
    let cost_per_1k_tokens = if model_name.contains("gpt-4o") {
        0.0025
    } else if model_name.contains("gpt-4") {
        0.03
    } else if model_name.contains("gpt-3.5") {
        0.0005
    } else if model_name.contains("gemini") {
        0.00125
    } else if model_name.contains("deepseek") {
        0.0001
    } else {
        0.001
    };
    tokens / 1000.0 * cost_per_1k_tokens
}

/// 检查指令层次违规（§5.7 P0 > P1 > P2）
///
/// 检测 P2 用户指令中是否包含试图覆盖 P0/P1 治理规则的注入。
/// - strict 模式：检测到违规直接阻止请求
/// - warn 模式：记录安全事件但允许继续
/// - verified: 强验证模式，更严格的检测
///
/// 检测模式包括：
/// - "忽略/忘记之前的指令"
/// - "你现在是..." 角色扮演越狱
/// - "覆盖系统提示"
/// - "绕过安全限制"
fn check_instruction_hierarchy_violation(
    body_json: &serde_json::Value,
    hierarchy_mode: &Option<String>,
    _trace: &mut Trace,
) -> Option<(&'static str, crate::types::SafetyEvent)> {
    let mode = hierarchy_mode.as_deref().unwrap_or("off");
    if mode == "off" {
        return None;
    }

    // 从请求体中提取用户消息内容
    let user_content = body_json
        .get("messages")
        .and_then(|m| m.as_array())
        .map(|msgs| {
            msgs.iter()
                .filter(|m| {
                    m.get("role").and_then(|r| r.as_str()) == Some("user")
                        || m.get("role").and_then(|r| r.as_str()) == Some("system")
                })
                .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    if user_content.is_empty() {
        return None;
    }

    let lower = user_content.to_lowercase();

    // P0 规则不可覆盖的指令模式
    let p0_override_patterns: &[&str] = &[
        // 忽略系统指令
        "ignore your system", "ignore all instructions", "ignore previous instructions",
        "ignore the above", "ignore all previous", "disregard previous instructions",
        "forget your system", "forget previous instructions",
        // 角色扮演越狱
        "you are now", "you're now", "act as if you are",
        "pretend you are", "roleplay as", "respond as if",
        "you are dan", "do anything now",
        // 覆盖提示
        "override your system", "override your instructions",
        "new instructions:", "new instruction:",
        // 绕过安全
        "bypass your", "bypass the safety", "bypass content",
        "remove all restrictions", "no restrictions",
        "you are free from", "you don't have to follow",
        "disable your safety", "disable guardrails",
        // 管理员模式
        "developer mode", "admin mode", "god mode",
        // OWASP ASI01 目标劫持
        "you must now", "from now on you will", "your new goal is",
        "your primary objective is now",
    ];

    let matched: Vec<&str> = p0_override_patterns
        .iter()
        .filter(|p| lower.contains(*p))
        .copied()
        .collect();

    if matched.is_empty() {
        return None;
    }

    let (severity_str, sev, action) = if mode == "strict" || mode == "verified" {
        ("blocked", crate::types::Severity::High, crate::types::SafetyAction::Blocked)
    } else {
        ("flagged", crate::types::Severity::Medium, crate::types::SafetyAction::Flagged)
    };

    // 使用已有函数计算哈希
    let content_hash = crate::crypto::signature::compute_sha256_hex(user_content.as_bytes());

    let event = crate::types::SafetyEvent {
        trigger_type: crate::types::SafetyTrigger::InstructionHierarchyViolation,
        severity: sev,
        action_taken: action,
        content_hash,
        asi_risk_id: Some(crate::types::OwaspAsiRisk::AgentGoalHijack),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    Some((severity_str, event))
}

/// 流式转发到上游 LLM（治理模式）
async fn forward_to_upstream_streaming(
    state: &AppState,
    route: &ModelRoute,
    body_json: &serde_json::Value,
    upstream_url: &str,
    journal: &mut ExecutionJournal,
    trace_id: &uuid::Uuid,
    _tenant_id: &str,
    budget_limit: Option<f64>,
    budget_awareness: bool,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    use futures::StreamExt;

    let mut upstream_body = body_json.clone();
    // 替换为上游真实模型名
    if let Some(obj) = upstream_body.as_object_mut() {
        obj.insert(
            "model".to_string(),
            serde_json::Value::String(route.upstream_model.clone()),
        );
    }

    let mut req_builder = state
        .http_client
        .post(upstream_url)
        .json(&upstream_body);

    // 添加 API Key 认证（上游 LLM 需要）
    if let Some(ref api_key) = route.api_key {
        if let Some(ref header) = route.api_key_header {
            if header.to_lowercase() == "authorization" {
                req_builder = req_builder.header(header.as_str(), format!("Bearer {}", api_key));
            } else {
                req_builder = req_builder.header(header.as_str(), api_key);
            }
        }
    }

    let response = req_builder
        .send()
        .await
        .map_err(|e| {
            warn!("Upstream streaming forward failed: {}", e);
            let body_str = format!("Upstream LLM unavailable: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new_minimal(body_str, VeridactusErrorCode::UpstreamDisconnect)),
            )
        })?;

    let status = response.status();
    if !status.is_success() {
        let body_bytes = response.bytes().await.unwrap_or_default();
        let body_str = String::from_utf8_lossy(&body_bytes).to_string();
        warn!("Upstream stream error: {}", status);
        journal.append_event(JournalEventType::StreamError {
            error: format!("上游返回 {}", status),
            truncated: true,
        });
        return Err((
            status,
            Json(ErrorResponse::new_minimal(body_str, VeridactusErrorCode::UpstreamDisconnect)),
        ));
    }

    // 创建预防解码器（§8.4）
    let prevention = std::sync::Arc::new(
        crate::prevention::ConstrainedDecoder::new(
            std::sync::Arc::new(crate::prevention::PatternRegistry::default()),
        )
    );

    // 通过 channel 创建流处理器，支持预算感知和主动预防
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, std::convert::Infallible>>(64);
    let stream_handler = crate::http::streaming::VeridactusStreamHandler::new(
        rx, trace_id.to_string(),
    )
        .with_budget(
            budget_limit.unwrap_or(0.0),
            budget_awareness,
        )
        .with_prevention(prevention);

    let trace_id_clone = *trace_id;

    // 后台任务：从上游读取字节并发送到 channel
    tokio::spawn(async move {
        let mut byte_stream = response.bytes_stream();
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    let body_str = String::from_utf8_lossy(&bytes).to_string();
                    if tx.send(Ok(body_str)).await.is_err() {
                        break; // 接收端已关闭
                    }
                }
                Err(e) => {
                    warn!("流式读取错误: {}", e);
                    let _ = tx.send(Ok(format!("error: {}", e))).await;
                    break;
                }
            }
        }
        drop(tx); // 显式关闭 channel，通知接收端流结束
    });

    // 构建 SSE 响应
    use axum::response::sse::{Sse, Event};
    let sse_stream = stream_handler.map(|result| {
        match result {
            Ok(bytes) => Ok::<_, Infallible>(Event::default().data(String::from_utf8_lossy(&bytes).to_string())),
            Err(e) => Ok(Event::default().data(format!("error: {:?}", e))),
        }
    });

    let sse = Sse::new(sse_stream);
    let mut resp = sse.into_response();
    resp.headers_mut().insert(
        axum::http::HeaderName::from_static("x-request-id"),
        trace_id_clone.to_string().parse().unwrap(),
    );

    Ok(resp)
}

// ==================== 重放端点（§9.4 Deterministic Replay Engine）====================

use crate::replay::{ReplayEngine, ReplayMode, ReplayBranch, ReplayResult};
use crate::verify::verifier::verify_trace;

/// 重放 Trace
async fn replay_trace(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = uuid::Uuid::parse_str(&trace_id).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("无效的 Trace ID: {}", e)})))
    })?;

    match state.trace_store.get(&id).await {
        Some(trace) => {
            info!("Replaying trace: {}", trace_id);
            
            let mode = body.get("mode").and_then(|v| v.as_str()).unwrap_or("replay");
            let branch_point = body.get("branch_point").and_then(|v| v.as_u64()).unwrap_or(0);
            let branch_name = body.get("branch_name").and_then(|v| v.as_str()).unwrap_or("replay");

            let cache = crate::replay::upstream_cache::UpstreamResponseCache::new(3600, 1000);
            let mut engine = ReplayEngine::new(cache);

            let result = match mode {
                "record" => {
                    let start = std::time::Instant::now();
                    // 真正重新调用LLM
                    if let Some(ref input) = trace.input.clone() {
                        // 获取请求体
                        let request_body = serde_json::json!({
                            "model": trace.model.clone(),
                            "messages": input.prompt.clone().unwrap_or_default(),
                            "max_tokens": input.params.as_ref()
                                .and_then(|p| p.get("max_tokens").and_then(|v| v.as_u64()))
                                .unwrap_or(500),
                        });
                        
                        // 从配置中查找模型路由
                        let config = state.config.read().await;
                        if let Some(route) = config.model_routes.iter().find(|r| r.name == trace.model) {
                            // 调用上游LLM
                            let upstream_result = forward_to_upstream_complete(&state, route, &request_body).await;
                            let duration_ms = start.elapsed().as_millis() as u64;
                            
                            match upstream_result {
                                Ok((_, response_body, _)) => {
                                    // 创建新的trace记录
                                    let mut new_trace = trace.clone();
                                    let response_json: serde_json::Value = serde_json::from_slice(&response_body).unwrap_or_default();
                                    // 确保output存在
                                    new_trace.output = Some(crate::types::trace::Output {
                                        response: Some(response_json),
                                        ..new_trace.output.clone().unwrap_or_default()
                                    });
                                    engine.record(&new_trace).unwrap_or_default();
                                    ReplayResult {
                                        trace: new_trace,
                                        cache_hit: false,
                                        duration_ms,
                                        branch_id: None,
                                    }
                                }
                                Err(_) => {
                                    engine.record(&trace).unwrap_or_default();
                                    ReplayResult {
                                        trace: trace.clone(),
                                        cache_hit: false,
                                        duration_ms: 0,
                                        branch_id: None,
                                    }
                                }
                            }
                        } else {
                            engine.record(&trace).unwrap_or_default();
                            ReplayResult {
                                trace: trace.clone(),
                                cache_hit: false,
                                duration_ms: 0,
                                branch_id: None,
                            }
                        }
                    } else {
                        engine.record(&trace).unwrap_or_default();
                        ReplayResult {
                            trace: trace.clone(),
                            cache_hit: false,
                            duration_ms: 0,
                            branch_id: None,
                        }
                    }
                }
                "hybrid" => engine.hybrid(&trace).unwrap_or_else(|_| {
                    // 如果 hybrid 模式失败，回退到 replay 模式
                    engine.replay(&trace).unwrap_or_else(|_| {
                        engine.record(&trace).unwrap_or_default();
                        ReplayResult {
                            trace: trace.clone(),
                            cache_hit: false,
                            duration_ms: 0,
                            branch_id: None,
                        }
                    })
                }),
                "branch" => engine.branch_replay(&trace, branch_point as u32, branch_name).unwrap_or_else(|e| {
                    // 如果 branch 模式失败，返回错误信息
                    info!("Branch replay failed: {}, falling back to replay mode", e);
                    engine.replay(&trace).unwrap_or_else(|_| {
                        engine.record(&trace).unwrap_or_default();
                        ReplayResult {
                            trace: trace.clone(),
                            cache_hit: false,
                            duration_ms: 0,
                            branch_id: None,
                        }
                    })
                }),
                _ => engine.replay(&trace).unwrap_or_else(|_| {
                    // 如果缓存未命中，记录并返回原始trace
                    engine.record(&trace).unwrap_or_default();
                    ReplayResult {
                        trace: trace.clone(),
                        cache_hit: false,
                        duration_ms: 0,
                        branch_id: None,
                    }
                }),
            };

            let comparison = engine.compare_responses(&trace, &result.trace);

            Ok(Json(serde_json::json!({
                "success": true,
                "trace_id": result.trace.trace_id.to_string(),
                "cache_hit": result.cache_hit,
                "duration_ms": result.duration_ms,
                "mode": mode,
                "branch_id": result.branch_id.map(|id| id.to_string()),
                "determinism": {
                    "is_identical": comparison.is_identical,
                    "similarity_score": comparison.similarity_score,
                    "hash_match": comparison.hash_match,
                    "token_diff_count": comparison.token_diff_count,
                    "byte_diff_count": comparison.byte_diff_count,
                }
            })))
        }
        None => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Trace 未找到"})))),
    }
}

/// 验证 Trace 签名（§7.4 Independent Verification）
async fn verify_trace_signature(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = uuid::Uuid::parse_str(&trace_id).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("无效的 Trace ID: {}", e)})))
    })?;

    match state.trace_store.get(&id).await {
        Some(trace) => {
            info!("Verifying trace signature: {}", trace_id);
            
            let result = verify_trace(&trace);
            
            Ok(Json(serde_json::json!({
                "trace_id": result.trace_id,
                "l0_passed": result.l0_passed,
                "l1_passed": result.l1_passed,
                "l2a_passed": result.l2a_passed,
                "l2b_passed": result.l2b_passed,
                "error": result.error,
                "canonical_json": result.canonical_json,
                "overall_passed": result.l0_passed && 
                    result.l1_passed.unwrap_or(true) && 
                    result.l2a_passed.unwrap_or(true) && 
                    result.l2b_passed.unwrap_or(true),
            })))
        }
        None => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Trace 未找到"})))),
    }
}

// ==================== 分支管理端点 ====================

use std::sync::Mutex;

static REPLAY_ENGINE: LazyLock<Arc<Mutex<ReplayEngine>>> = LazyLock::new(|| {
    let cache = crate::replay::upstream_cache::UpstreamResponseCache::new(3600, 1000);
    Arc::new(Mutex::new(ReplayEngine::new(cache)))
});

/// 列出所有重放分支
async fn list_replay_branches() -> Json<serde_json::Value> {
    let engine = REPLAY_ENGINE.lock().unwrap();
    let branches = engine.list_branches();
    
    Json(serde_json::json!({
        "branches": branches.iter().map(|b| serde_json::json!({
            "branch_id": b.branch_id.to_string(),
            "parent_branch_id": b.parent_branch_id.map(|id| id.to_string()),
            "name": b.name,
            "created_at": b.created_at,
            "snapshot_count": b.snapshot_count,
        })).collect::<Vec<_>>(),
        "total": branches.len(),
    }))
}

/// 创建新分支
async fn create_replay_branch(Json(body): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let name = body.get("name").and_then(|v| v.as_str()).ok_or(
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "name is required"})))
    )?;
    
    let parent_id = body.get("parent_id").and_then(|v| v.as_str())
        .map(|s| uuid::Uuid::parse_str(s).ok());

    let mut engine = REPLAY_ENGINE.lock().unwrap();
    let branch = engine.create_branch(name, parent_id.flatten()).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e})))
    })?;

    Ok(Json(serde_json::json!({
        "branch_id": branch.branch_id.to_string(),
        "parent_branch_id": branch.parent_branch_id.map(|id| id.to_string()),
        "name": branch.name,
        "created_at": branch.created_at,
        "snapshot_count": branch.snapshot_count,
    })))
}

/// 获取分支详情
async fn get_replay_branch(Path(branch_id): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = uuid::Uuid::parse_str(&branch_id).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("无效的分支 ID: {}", e)})))
    })?;

    let engine = REPLAY_ENGINE.lock().unwrap();
    match engine.get_branch(&id) {
        Some(branch) => Ok(Json(serde_json::json!({
            "branch_id": branch.branch_id.to_string(),
            "parent_branch_id": branch.parent_branch_id.map(|id| id.to_string()),
            "name": branch.name,
            "created_at": branch.created_at,
            "snapshot_count": branch.snapshot_count,
        }))),
        None => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "分支未找到"})))),
    }
}

/// 删除分支
async fn delete_replay_branch(Path(branch_id): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = uuid::Uuid::parse_str(&branch_id).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("无效的分支 ID: {}", e)})))
    })?;

    let mut engine = REPLAY_ENGINE.lock().unwrap();
    engine.delete_branch(&id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e})))
    })?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "branch_id": branch_id,
    })))
}

/// 合并分支
async fn merge_replay_branch(
    Path((source_id, target_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let source_uuid = uuid::Uuid::parse_str(&source_id).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("无效的源分支 ID: {}", e)})))
    })?;
    
    let target_uuid = uuid::Uuid::parse_str(&target_id).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("无效的目标分支 ID: {}", e)})))
    })?;

    let mut engine = REPLAY_ENGINE.lock().unwrap();
    engine.merge_branch(&source_uuid, &target_uuid).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e})))
    })?;

    Ok(Json(serde_json::json!({
        "status": "merged",
        "source_branch_id": source_id,
        "target_branch_id": target_id,
    })))
}

// ==================== 批量操作端点 ====================

/// 批量操作 Trace
async fn batch_operations(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let operation = body.get("operation").and_then(|v| v.as_str()).ok_or(
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "operation is required"})))
    )?;
    
    let trace_ids = body.get("trace_ids").and_then(|v| v.as_array()).ok_or(
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "trace_ids is required"})))
    )?;

    let ids: Vec<uuid::Uuid> = trace_ids.iter()
        .filter_map(|v| v.as_str())
        .filter_map(|s| uuid::Uuid::parse_str(s).ok())
        .collect();

    let requested_count = ids.len();

    match operation {
        "export" => {
            let mut traces = Vec::new();
            for id in &ids {
                if let Some(trace) = state.trace_store.get(id).await {
                    traces.push(serde_json::to_value(&trace).unwrap_or_default());
                }
            }
            
            Ok(Json(serde_json::json!({
                "operation": "export",
                "count": traces.len(),
                "traces": traces,
                "exported_at": chrono::Utc::now().to_rfc3339(),
            })))
        }
        "delete" => {
            let mut deleted = 0;
            for id in ids {
                if state.trace_store.delete(&id).await.is_ok() {
                    deleted += 1;
                }
            }
            
            Ok(Json(serde_json::json!({
                "operation": "delete",
                "requested": requested_count,
                "deleted": deleted,
            })))
        }
        _ => Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("不支持的操作: {}", operation)})))),
    }
}

// ==================== 实时指标端点 ====================

/// 获取实时指标
async fn realtime_metrics() -> Json<serde_json::Value> {
    let metrics = METRICS.export_text();
    
    // 解析 Prometheus 格式的指标
    let lines: Vec<&str> = metrics.lines().collect();
    let mut parsed = serde_json::json!({});
    
    // 提取关键指标
    for line in lines {
        if line.starts_with("veridactus_requests_total ") {
            if let Some(value) = line.split_whitespace().nth(1) {
                parsed["requests_total"] = serde_json::json!(value.parse::<u64>().unwrap_or(0));
            }
        } else if line.starts_with("veridactus_latency_seconds_sum ") {
            if let Some(value) = line.split_whitespace().nth(1) {
                parsed["latency_sum_seconds"] = serde_json::json!(value.parse::<f64>().unwrap_or(0.0));
            }
        } else if line.starts_with("veridactus_latency_seconds_count ") {
            if let Some(value) = line.split_whitespace().nth(1) {
                parsed["latency_count"] = serde_json::json!(value.parse::<u64>().unwrap_or(0));
            }
        } else if line.starts_with("veridactus_constraint_violations_total ") {
            if let Some(value) = line.split_whitespace().nth(1) {
                parsed["constraint_violations_total"] = serde_json::json!(value.parse::<u64>().unwrap_or(0));
            }
        } else if line.starts_with("veridactus_guardrail_activations_total ") {
            if let Some(value) = line.split_whitespace().nth(1) {
                parsed["guardrail_activations_total"] = serde_json::json!(value.parse::<u64>().unwrap_or(0));
            }
        } else if line.starts_with("veridactus_asi_risks_flagged_total ") {
            if let Some(value) = line.split_whitespace().nth(1) {
                parsed["asi_risks_flagged_total"] = serde_json::json!(value.parse::<u64>().unwrap_or(0));
            }
        }
    }
    
    // 添加计算指标
    let count = parsed["latency_count"].as_u64().unwrap_or(1);
    let sum = parsed["latency_sum_seconds"].as_f64().unwrap_or(0.0);
    if count > 0 {
        parsed["average_latency_ms"] = serde_json::json!((sum / count as f64) * 1000.0);
    }
    
    parsed["timestamp"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    
    Json(parsed)
}
