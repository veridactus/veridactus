//! # 存储接口定义
//!
//! 定义所有存储抽象的核心 trait 接口。

use crate::pipeline::config::ExecutionPlan;
use crate::types::trace::Trace;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait TraceStore: Send + Sync {
    async fn save(&self, trace: Trace) -> Result<(), String>;
    async fn get(&self, trace_id: &Uuid) -> Option<Trace>;
    async fn list(&self, tenant_id: Option<&str>, limit: usize, offset: usize) -> Vec<Trace>;
    async fn count(&self, tenant_id: Option<&str>) -> usize;
    async fn delete(&self, trace_id: &Uuid) -> Result<Option<Trace>, String>;
    async fn delete_by_session(&self, session_id: &Uuid) -> Result<Vec<Trace>, String>;
    async fn delete_by_tenant(&self, tenant_id: &str) -> Result<Vec<Trace>, String>;
}

#[async_trait]
pub trait ConfigStore: Send + Sync {
    async fn get_pipeline(&self, tenant_id: &str) -> Option<ExecutionPlan>;
    async fn list_pipelines(&self) -> Vec<ExecutionPlan>;
    async fn save_pipeline(&self, plan: &ExecutionPlan) -> Result<(), String>;
    async fn delete_pipeline(&self, plan_id: &str) -> Result<(), String>;
    async fn get_config_version(&self) -> ConfigVersions;
    async fn notify_config_change(&self, change_type: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, Default)]
pub struct ConfigVersions {
    pub pipeline_version: u64,
    pub policy_version: u64,
    pub plugin_version: u64,
}

#[async_trait]
pub trait BudgetStore: Send + Sync {
    async fn get_remaining(&self, tenant_id: &str) -> Option<f64>;
    async fn reserve(&self, tenant_id: &str, amount: f64) -> Result<f64, String>;
    async fn release(&self, tenant_id: &str, amount: f64) -> Result<f64, String>;
    async fn set_limit(&self, tenant_id: &str, limit: f64) -> Result<(), String>;
    async fn reset(&self, tenant_id: &str) -> Result<(), String>;
}

#[async_trait]
pub trait CacheStore: Send + Sync {
    async fn get(&self, key: &str) -> Option<String>;
    async fn set(&self, key: &str, value: &str, ttl_secs: Option<u64>) -> Result<(), String>;
    async fn delete(&self, key: &str) -> Result<(), String>;
    async fn exists(&self, key: &str) -> bool;
}

#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(&self, bucket: &str, key: &str, data: &[u8]) -> Result<(), String>;
    async fn get(&self, bucket: &str, key: &str) -> Option<Vec<u8>>;
    async fn delete(&self, bucket: &str, key: &str) -> Result<(), String>;
    async fn list(&self, bucket: &str, prefix: &str) -> Vec<String>;
}

pub trait AsyncTraceWriter: Send + Sync {
    fn enqueue(&self, trace: Trace) -> Result<(), String>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}
