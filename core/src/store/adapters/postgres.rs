//! # PostgreSQL 存储适配器
//!
//! 生产环境使用的 PostgreSQL Trace 存储实现。

use crate::store::traits::TraceStore;
use crate::types::trace::Trace;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

pub struct PostgresTraceStore {
    pool: Arc<sqlx::PgPool>,
}

impl PostgresTraceStore {
    pub fn new(pool: Arc<sqlx::PgPool>) -> Self {
        Self { pool }
    }

    pub async fn init_schema(pool: &sqlx::PgPool) -> Result<(), String> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS dp_traces (
                trace_id UUID PRIMARY KEY,
                tenant_id VARCHAR(64) NOT NULL,
                session_id UUID,
                trace_data JSONB NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
                expires_at TIMESTAMP WITH TIME ZONE
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_dp_traces_tenant ON dp_traces(tenant_id)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_dp_traces_timestamp ON dp_traces(created_at)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_dp_traces_session ON dp_traces(session_id)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[async_trait]
impl TraceStore for PostgresTraceStore {
    async fn save(&self, trace: Trace) -> Result<(), String> {
        let trace_json = serde_json::to_value(&trace).map_err(|e| e.to_string())?;

        sqlx::query(
            r#"
            INSERT INTO dp_traces (
                trace_id, tenant_id, session_id, trace_data, created_at
            ) VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (trace_id) DO UPDATE SET
                trace_data = EXCLUDED.trace_data
            "#,
        )
        .bind(&trace.trace_id)
        .bind(trace.tenant_id.as_deref().unwrap_or("default"))
        .bind(trace.session_id)
        .bind(trace_json)
        .bind(chrono::Utc::now())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn get(&self, trace_id: &Uuid) -> Option<Trace> {
        let row: Option<(Value,)> =
            sqlx::query_as("SELECT trace_data FROM dp_traces WHERE trace_id = $1")
                .bind(trace_id)
                .fetch_optional(self.pool.as_ref())
                .await
                .ok()?;

        row.and_then(|(data,)| serde_json::from_value(data).ok())
    }

    async fn list(&self, tenant_id: Option<&str>, limit: usize, offset: usize) -> Vec<Trace> {
        let rows: Vec<(Value,)> = if let Some(tid) = tenant_id {
            sqlx::query_as(
                "SELECT trace_data FROM dp_traces WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(tid)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(self.pool.as_ref())
            .await
            .unwrap_or_default()
        } else {
            sqlx::query_as(
                "SELECT trace_data FROM dp_traces ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            )
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(self.pool.as_ref())
            .await
            .unwrap_or_default()
        };

        rows.into_iter()
            .filter_map(|(data,)| serde_json::from_value(data).ok())
            .collect()
    }

    async fn count(&self, tenant_id: Option<&str>) -> usize {
        if let Some(tid) = tenant_id {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM dp_traces WHERE tenant_id = $1")
                .bind(tid)
                .fetch_one(self.pool.as_ref())
                .await
                .unwrap_or(0) as usize
        } else {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM dp_traces")
                .fetch_one(self.pool.as_ref())
                .await
                .unwrap_or(0) as usize
        }
    }

    async fn delete(&self, trace_id: &Uuid) -> Result<Option<Trace>, String> {
        let row: Option<(Value,)> =
            sqlx::query_as("DELETE FROM dp_traces WHERE trace_id = $1 RETURNING trace_data")
                .bind(trace_id)
                .fetch_optional(self.pool.as_ref())
                .await
                .map_err(|e| e.to_string())?;

        Ok(row.and_then(|(data,)| serde_json::from_value(data).ok()))
    }

    async fn delete_by_session(&self, session_id: &Uuid) -> Result<Vec<Trace>, String> {
        let rows: Vec<(Value,)> =
            sqlx::query_as("DELETE FROM dp_traces WHERE session_id = $1 RETURNING trace_data")
                .bind(session_id)
                .fetch_all(self.pool.as_ref())
                .await
                .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .filter_map(|(data,)| serde_json::from_value(data).ok())
            .collect())
    }

    async fn delete_by_tenant(&self, tenant_id: &str) -> Result<Vec<Trace>, String> {
        let rows: Vec<(Value,)> =
            sqlx::query_as("DELETE FROM dp_traces WHERE tenant_id = $1 RETURNING trace_data")
                .bind(tenant_id)
                .fetch_all(self.pool.as_ref())
                .await
                .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .filter_map(|(data,)| serde_json::from_value(data).ok())
            .collect())
    }
}
