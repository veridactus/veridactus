// VERIDACTUS ClickHouse Trace Writer — OLAP 可观测性存储
use crate::types::proof::ProofLevel;
use crate::types::trace::Trace;
use chrono::Utc;
use clickhouse::{Client, Row};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Row)]
struct CHAuditEvent {
    event_id: String,
    org_id: String,
    workspace_id: String,
    event_type: String,
    severity: String,
    trace_id: String,
    user_id: String,
    model: String,
    cost_usd_micro: i64,
    tokens_count: i64,
    latency_ms: i64,
    asi_risk_id: String,
    metadata: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Row)]
struct CHTraceAgg {
    trace_id: String,
    org_id: String,
    workspace_id: String,
    user_id: String,
    model: String,
    provider: String,
    tokens_count: i64,
    cost_usd_micro: i64,
    latency_ms: i64,
    execution_state: String,
    safety_status: String,
    proof_levels: String,
    created_at: String,
}

pub struct ClickHouseTraceStore {
    client: Client,
    enabled: bool,
}

impl ClickHouseTraceStore {
    pub async fn new(ch_url: &str, database: &str) -> Self {
        let client = Client::default()
            .with_url(ch_url)
            .with_database(database)
            .with_option("async_insert", "1")
            .with_option("wait_for_async_insert", "0");

        let enabled = match client.query("SELECT 1").fetch_one::<u8>().await {
            Ok(_) => { tracing::info!("ClickHouse connected: {}", ch_url); true }
            Err(e) => { tracing::warn!("ClickHouse unavailable ({}), CH writes disabled", e); false }
        };
        Self { client, enabled }
    }

    /// 写入审计事件
    pub async fn write_audit_event(&self, trace: &Trace) {
        if !self.enabled { return; }
        let obs = trace.observations.as_ref();
        let event = CHAuditEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            org_id: trace.tenant_id.clone().unwrap_or_default(),
            workspace_id: trace.tenant_id.clone().unwrap_or_default(),
            event_type: "trace_finalized".to_string(),
            severity: "info".to_string(),
            trace_id: trace.trace_id.to_string(),
            user_id: String::new(),
            model: trace.model.clone(),
            cost_usd_micro: obs.and_then(|o| o.cost_estimated_usd.map(|c| (c * 1_000_000.0) as i64)).unwrap_or(0),
            tokens_count: obs.and_then(|o| o.tokens_count.map(|t| t as i64)).unwrap_or(0),
            latency_ms: obs.and_then(|o| o.latency_ms.map(|l| l as i64)).unwrap_or(0),
            asi_risk_id: String::new(),
            metadata: serde_json::to_string(&trace.proofs.proof_chain).unwrap_or_default(),
            created_at: Utc::now().to_rfc3339(),
        };
        if let Err(e) = self.insert_audit(&event).await {
            tracing::warn!("CH audit event write failed: {}", e);
        }
    }

    /// 写入聚合 Trace
    pub async fn write_trace_agg(&self, trace: &Trace) {
        if !self.enabled { return; }
        let obs = trace.observations.as_ref();
        let proof_levels: Vec<&str> = {
            let mut lv = vec!["L0"];
            if trace.proofs.proof_chain.iter().any(|e| matches!(e.level, ProofLevel::L2A)) { lv.push("L2A"); }
            if trace.proofs.proof_chain.iter().any(|e| matches!(e.level, ProofLevel::L2B)) { lv.push("L2B"); }
            lv
        };
        let agg = CHTraceAgg {
            trace_id: trace.trace_id.to_string(),
            org_id: trace.tenant_id.clone().unwrap_or_default(),
            workspace_id: trace.tenant_id.clone().unwrap_or_default(),
            user_id: String::new(),
            model: trace.model.clone(),
            provider: trace
                .model
                .split('/')
                .next()
                .unwrap_or("unknown")
                .to_string(),
            tokens_count: obs
                .and_then(|o| o.tokens_count.map(|t| t as i64))
                .unwrap_or(0),
            cost_usd_micro: obs
                .and_then(|o| o.cost_estimated_usd.map(|c| (c * 1_000_000.0) as i64))
                .unwrap_or(0),
            latency_ms: obs
                .and_then(|o| o.latency_ms.map(|l| l as i64))
                .unwrap_or(0),
            execution_state: trace
                .execution_state
                .as_ref()
                .map(|s| format!("{:?}", s))
                .unwrap_or_default(),
            safety_status: "safe".to_string(),
            proof_levels: proof_levels.join(","),
            created_at: Utc::now().to_rfc3339(),
        };
        if let Err(e) = self.insert_agg(&agg).await {
            tracing::warn!("CH trace agg write failed: {}", e);
        }
    }

    async fn insert_audit(&self, event: &CHAuditEvent) -> Result<(), clickhouse::error::Error> {
        let mut insert = self.client.insert::<CHAuditEvent>("audit_events").await?;
        insert.write(event).await?;
        insert.end().await
    }

    async fn insert_agg(&self, agg: &CHTraceAgg) -> Result<(), clickhouse::error::Error> {
        let mut insert = self.client.insert::<CHTraceAgg>("traces_agg").await?;
        insert.write(agg).await?;
        insert.end().await
    }
}

// OpenTelemetry 分布式追踪初始化（Phase 2 接入指引）
// 接入方式：cargo add opentelemetry_sdk --features "trace,rt-tokio"
// use opentelemetry_sdk::trace::SdkTracerProvider;
// use opentelemetry_sdk::Resource;
// use tracing_opentelemetry::OpenTelemetryLayer;
//
// let resource = Resource::builder().with_service_name(service_name).build();
// let provider = SdkTracerProvider::builder().with_resource(resource).build();
// let tracer = provider.tracer("veridactus-core");
// tracing::subscriber::set_global_default(
//     tracing_subscriber::Registry::default()
//         .with(OpenTelemetryLayer::new(tracer))
// ).expect("OTel init failed");
