//! VERIDACTUS 数据平面 — 启动入口
//!
//! 启动 HTTP/SSE 代理服务器，监听 :8080。
//! 上游 LLM 地址通过配置或环境变量指定。

use futures::executor;
use futures::Future;
use reqwest;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use veridactus_core::agent_chain::AgentExecutionChainManager;
use veridactus_core::audit::token::AuditTokenValidator;
use veridactus_core::auth::keys::ApiKeyManager;
use veridactus_core::compliance::ComplianceMapper;
use veridactus_core::configsync::{ConfigPullClient, ModelConfig};
use veridactus_core::gdpr::{DeletionStorage, GdprErasureManager};
use veridactus_core::http::server::{create_router, AppState, ProxyConfig};
use veridactus_core::store::facade::ConfigStoreAdapter;
use veridactus_core::store::{create_trace_store, StoreBackend, TraceStore};

/// GDPR 存储包装器（适配 dyn TraceStore 到 GdprErasureManager）
struct GdprStorageWrapper {
    store: Arc<dyn TraceStore>,
}

impl DeletionStorage for GdprStorageWrapper {
    fn delete_by_trace_id(
        &self,
        trace_id: &str,
    ) -> Result<veridactus_core::gdpr::DeletionResult, veridactus_core::gdpr::DeletionError> {
        let id = Uuid::parse_str(trace_id).map_err(|_| {
            veridactus_core::gdpr::DeletionError::ValidationError("invalid trace_id".into())
        })?;
        let sigs = executor::block_on(self.store.delete(&id))
            .map_err(|e| veridactus_core::gdpr::DeletionError::StorageError(e))?
            .map(|t| {
                t.proofs
                    .proof_chain
                    .first()
                    .and_then(|p| p.signature.clone())
                    .unwrap_or_default()
            })
            .map(|s| vec![s])
            .unwrap_or_default();

        Ok(veridactus_core::gdpr::DeletionResult {
            request_id: format!("del_{}", Uuid::new_v4()),
            success: true,
            deleted_count: if sigs.is_empty() { 0 } else { 1 },
            retained_signatures: sigs,
            audit_log_entry: veridactus_core::gdpr::DeletionAuditEntry {
                audit_id: format!("audit_{}", Uuid::new_v4()),
                request_id: format!("del_{}", Uuid::new_v4()),
                deletion_type: veridactus_core::gdpr::DeletionType::TraceId,
                target_id: trace_id.to_string(),
                deleted_count: 1,
                retained_signature_hashes: Vec::new(),
                deleted_at: chrono::Utc::now().to_rfc3339(),
                deleted_by: None,
                compliance_evidence: veridactus_core::gdpr::ComplianceEvidence {
                    regulation: "GDPR".to_string(),
                    article: "Article 17".to_string(),
                    basis: "Right to erasure".to_string(),
                    data_subject_right: "Right to be forgotten".to_string(),
                },
            },
            error_message: None,
        })
    }

    fn delete_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<veridactus_core::gdpr::DeletionResult, veridactus_core::gdpr::DeletionError> {
        let id = Uuid::parse_str(session_id).map_err(|_| {
            veridactus_core::gdpr::DeletionError::ValidationError("invalid session_id".into())
        })?;
        let deleted = executor::block_on(self.store.delete_by_session(&id))
            .map_err(|e| veridactus_core::gdpr::DeletionError::StorageError(e))?;
        let sigs: Vec<String> = deleted
            .iter()
            .filter_map(|t| {
                t.proofs
                    .proof_chain
                    .first()
                    .and_then(|p| p.signature.clone())
            })
            .collect();

        Ok(veridactus_core::gdpr::DeletionResult {
            request_id: format!("del_{}", Uuid::new_v4()),
            success: true,
            deleted_count: deleted.len(),
            retained_signatures: sigs,
            audit_log_entry: veridactus_core::gdpr::DeletionAuditEntry {
                audit_id: format!("audit_{}", Uuid::new_v4()),
                request_id: format!("del_{}", Uuid::new_v4()),
                deletion_type: veridactus_core::gdpr::DeletionType::SessionId,
                target_id: session_id.to_string(),
                deleted_count: deleted.len(),
                retained_signature_hashes: Vec::new(),
                deleted_at: chrono::Utc::now().to_rfc3339(),
                deleted_by: None,
                compliance_evidence: veridactus_core::gdpr::ComplianceEvidence {
                    regulation: "GDPR".to_string(),
                    article: "Article 17".to_string(),
                    basis: "Right to erasure".to_string(),
                    data_subject_right: "Right to be forgotten".to_string(),
                },
            },
            error_message: None,
        })
    }

    fn delete_by_user_id(
        &self,
        user_id: &str,
    ) -> Result<veridactus_core::gdpr::DeletionResult, veridactus_core::gdpr::DeletionError> {
        let deleted = executor::block_on(self.store.delete_by_tenant(user_id))
            .map_err(|e| veridactus_core::gdpr::DeletionError::StorageError(e))?;
        let sigs: Vec<String> = deleted
            .iter()
            .filter_map(|t| {
                t.proofs
                    .proof_chain
                    .first()
                    .and_then(|p| p.signature.clone())
            })
            .collect();

        Ok(veridactus_core::gdpr::DeletionResult {
            request_id: format!("del_{}", Uuid::new_v4()),
            success: true,
            deleted_count: deleted.len(),
            retained_signatures: sigs,
            audit_log_entry: veridactus_core::gdpr::DeletionAuditEntry {
                audit_id: format!("audit_{}", Uuid::new_v4()),
                request_id: format!("del_{}", Uuid::new_v4()),
                deletion_type: veridactus_core::gdpr::DeletionType::UserId,
                target_id: user_id.to_string(),
                deleted_count: deleted.len(),
                retained_signature_hashes: Vec::new(),
                deleted_at: chrono::Utc::now().to_rfc3339(),
                deleted_by: None,
                compliance_evidence: veridactus_core::gdpr::ComplianceEvidence {
                    regulation: "GDPR".to_string(),
                    article: "Article 17".to_string(),
                    basis: "Right to erasure".to_string(),
                    data_subject_right: "Right to be forgotten".to_string(),
                },
            },
            error_message: None,
        })
    }

    fn retain_signature(
        &self,
        _trace_id: &str,
        _audit_signature: &str,
    ) -> Result<(), veridactus_core::gdpr::DeletionError> {
        Ok(())
    }

    fn get_deletion_log(
        &self,
        _request_id: &str,
    ) -> Option<veridactus_core::gdpr::DeletionAuditEntry> {
        None
    }

    fn list_deletion_logs(&self, _limit: usize) -> Vec<veridactus_core::gdpr::DeletionAuditEntry> {
        Vec::new()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("VERIDACTUS Data Plane starting...");

    // 从环境变量获取配置
    let upstream_url =
        std::env::var("UPSTREAM_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let upstream_api_key = std::env::var("UPSTREAM_API_KEY").unwrap_or_default();
    let control_plane_url =
        std::env::var("CONTROL_PLANE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let admin_key = std::env::var("VERIDACTUS_ADMIN_KEY").unwrap_or_default();

    // 初始化存储（自动检测后端：memory | file | postgres）
    let backend = StoreBackend::detect();
    let trace_store = create_trace_store(&backend).await;

    // 初始化配置存储适配器
    let config_store = Arc::new(ConfigStoreAdapter::new());

    // 初始化 API Key 管理器
    let api_key_manager = Arc::new(std::sync::Mutex::new(ApiKeyManager::new(upstream_api_key)));

    // 注册静态 API Keys（通过环境变量 VERIDACTUS_STATIC_API_KEYS=name:key,name:key）
    if let Ok(static_keys) = std::env::var("VERIDACTUS_STATIC_API_KEYS") {
        let mut mgr = api_key_manager.lock().unwrap();
        for pair in static_keys.split(',') {
            let parts: Vec<&str> = pair.splitn(2, ':').collect();
            if parts.len() == 2 {
                mgr.add_static_key(parts[0].trim(), parts[1].trim());
                info!("Static API Key registered: {}", parts[0].trim());
            }
        }
    }

    // 初始化审计令牌验证器
    let audit_token_validator = Arc::new(AuditTokenValidator::new(Vec::new()));

    // 初始化合规映射器
    let compliance_mapper = Arc::new(ComplianceMapper::new());

    // 初始化 HTTP 客户端
    let http_client = reqwest::Client::new();

    // 初始化代理配置（使用 RwLock 支持动态更新）
    // 所有 API 密钥必须通过环境变量提供，严禁硬编码
    let zhipu_api_key = std::env::var("ZHIPU_API_KEY").ok();
    let mut model_routes = Vec::new();

    // 智谱 AI GLM-5.1（仅当 ZHIPU_API_KEY 环境变量已设置时启用）
    if let Some(ref key) = zhipu_api_key {
        model_routes.push(veridactus_core::http::server::ModelRoute {
            name: "glm-5.1".to_string(),
            upstream_model: "glm-5.1".to_string(),
            upstream_endpoint: "/api/paas/v4/chat/completions".to_string(),
            is_default: true,
            upstream_url: Some("https://open.bigmodel.cn".to_string()),
            api_key: Some(key.clone()),
            api_key_header: Some("Authorization".to_string()),
            use_proxy: false,
            proxy_url: None,
        });
        info!("Zhipu GLM-5.1 model route configured (key from ZHIPU_API_KEY env)");
    } else {
        warn!("ZHIPU_API_KEY not set, GLM-5.1 model route disabled");
    }

    // Ollama 本地模型（回退选项，无需 API 密钥）
    model_routes.push(veridactus_core::http::server::ModelRoute {
        name: "deepseek-r1:14b".to_string(),
        upstream_model: "deepseek-r1:14b".to_string(),
        upstream_endpoint: "/v1/chat/completions".to_string(),
        is_default: zhipu_api_key.is_none(),
        upstream_url: Some(upstream_url.clone()),
        api_key: None,
        api_key_header: None,
        use_proxy: false,
        proxy_url: None,
    });

    if model_routes.is_empty() {
        warn!(
            "No model routes configured. Set ZHIPU_API_KEY or configure models via control plane."
        );
    }

    let default_model = model_routes
        .iter()
        .find(|r| r.is_default)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "deepseek-r1:14b".to_string());

    let proxy_config = Arc::new(tokio::sync::RwLock::new(ProxyConfig {
        upstream_base_url: upstream_url.clone(),
        default_model,
        model_routes,
        supported_versions: vec!["0.1".to_string(), "0.2".to_string()],
        detailed_errors: true,
        pipeline_plans: HashMap::new(),
    }));

    // 创建模型配置更新回调
    let proxy_config_clone = proxy_config.clone();
    let model_updater =
        move |models: Vec<ModelConfig>| -> Pin<Box<dyn Future<Output = ()> + Send>> {
            let proxy_config_clone = proxy_config_clone.clone();
            Box::pin(async move {
                let mut config = proxy_config_clone.write().await;
                let mut routes = Vec::new();
                let mut default_model = "deepseek-r1:14b".to_string();

                for model in models {
                    if model.status != "active" {
                        continue;
                    }

                    // 根据模型类型确定端点
                    let endpoint = if model
                        .upstream_url
                        .contains("generativelanguage.googleapis.com")
                    {
                        format!("/{}:generateContent", model.upstream_model)
                    } else if model.upstream_url.contains("models.inference.ai.azure.com") {
                        "/chat/completions".to_string()
                    } else if model.upstream_url.contains("open.bigmodel.cn") {
                        "/api/paas/v4/chat/completions".to_string()
                    } else if model.upstream_url.contains("qianfan.baidubce.com") {
                        "/v2/chat/completions".to_string()
                    } else {
                        "/v1/chat/completions".to_string()
                    };

                    routes.push(veridactus_core::http::server::ModelRoute {
                        name: model.name.clone(),
                        upstream_url: Some(model.upstream_url.clone()),
                        upstream_model: model.upstream_model,
                        upstream_endpoint: endpoint,
                        api_key: model.api_key,
                        api_key_header: model.api_key_header,
                        use_proxy: model.use_proxy,
                        proxy_url: model.proxy_url,
                        is_default: model.is_default,
                    });

                    if model.is_default {
                        default_model = model.name;
                    }
                }

                config.model_routes = routes;
                config.default_model = default_model;
                info!(
                    "Model config updated: {} models, default: {}",
                    config.model_routes.len(),
                    config.default_model
                );
            })
        };

    // 初始化 Agent 执行链管理器
    let agent_chain_manager = Arc::new(AgentExecutionChainManager::new());

    // 初始化 GDPR 删除管理器
    let gdpr_storage = Box::new(GdprStorageWrapper {
        store: trace_store.clone(),
    });
    let gdpr_manager = Arc::new(GdprErasureManager::new(gdpr_storage));

    // 创建应用状态
    let idempotency_guard = Arc::new(veridactus_core::middleware::IdempotencyGuard::new(
        3600, 10000,
    ));
    let app_state = AppState {
        trace_store,
        api_key_manager,
        audit_token_validator,
        compliance_mapper,
        http_client,
        config: proxy_config,
        idempotency_guard,
        agent_chain_manager,
        gdpr_manager,
        hook_registry: Arc::new(veridactus_core::hooks::registry::HookRegistry::new()),
        control_plane_url: control_plane_url.clone(),
        admin_key: admin_key.clone(),
    };

    // 创建路由
    let router = create_router(app_state);

    // 启动配置同步客户端（带模型更新回调）
    let config_pull_client =
        ConfigPullClient::new(&control_plane_url, config_store).with_model_updater(model_updater);

    // 立即拉取一次配置
    if let Ok(Some(payload)) = config_pull_client.pull().await {
        info!(
            "Initial config fetch success: change_type={}",
            payload.change_type
        );
    } else {
        warn!("Initial config fetch failed");
    }

    let _ = config_pull_client.start_poll_loop();
    info!("Config sync client started, CP URL: {}", control_plane_url);

    // 启动服务器（支持优雅关闭）
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("VERIDACTUS Data Plane ready, awaiting requests...");

    // 优雅关闭：捕获 SIGTERM/SIGINT，等待活跃请求完成
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            info!("Shutdown signal received, waiting for active requests...");
        })
        .await?;

    info!("VERIDACTUS Data Plane shutdown complete");
    Ok(())
}
