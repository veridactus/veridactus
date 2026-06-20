//! # 存储管理器
//!
//! 提供统一的存储访问入口，协调多种存储后端。

use std::sync::Arc;
use uuid::Uuid;
use crate::types::trace::Trace;
use crate::pipeline::config::ExecutionPlan;
use crate::store::traits::{
    TraceStore, ConfigStore, BudgetStore, CacheStore, ObjectStore,
    ConfigVersions,
};

pub struct StoreManager {
    trace_store: Arc<dyn TraceStore>,
    config_store: Arc<dyn ConfigStore>,
    budget_store: Arc<dyn BudgetStore>,
    cache_store: Arc<dyn CacheStore>,
    object_store: Arc<dyn ObjectStore>,
}

impl StoreManager {
    pub fn new(
        trace_store: Arc<dyn TraceStore>,
        config_store: Arc<dyn ConfigStore>,
        budget_store: Arc<dyn BudgetStore>,
        cache_store: Arc<dyn CacheStore>,
        object_store: Arc<dyn ObjectStore>,
    ) -> Self {
        Self {
            trace_store,
            config_store,
            budget_store,
            cache_store,
            object_store,
        }
    }

    pub fn trace_store(&self) -> Arc<dyn TraceStore> {
        self.trace_store.clone()
    }

    pub fn config_store(&self) -> Arc<dyn ConfigStore> {
        self.config_store.clone()
    }

    pub fn budget_store(&self) -> Arc<dyn BudgetStore> {
        self.budget_store.clone()
    }

    pub fn cache_store(&self) -> Arc<dyn CacheStore> {
        self.cache_store.clone()
    }

    pub fn object_store(&self) -> Arc<dyn ObjectStore> {
        self.object_store.clone()
    }

    pub async fn get_trace(&self, trace_id: &Uuid) -> Option<Trace> {
        self.trace_store.get(trace_id).await
    }

    pub async fn list_traces(
        &self,
        tenant_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Vec<Trace> {
        self.trace_store.list(tenant_id, limit, offset).await
    }

    pub async fn count_traces(&self, tenant_id: Option<&str>) -> usize {
        self.trace_store.count(tenant_id).await
    }

    pub async fn save_trace(&self, trace: Trace) -> Result<(), String> {
        self.trace_store.save(trace).await
    }

    pub async fn get_pipeline(&self, tenant_id: &str) -> Option<ExecutionPlan> {
        if let Some(cached) = self.cache_store.get(&format!("pipeline:{}", tenant_id)).await {
            if let Ok(plan) = serde_json::from_str::<ExecutionPlan>(&cached) {
                return Some(plan);
            }
        }
        
        if let Some(plan) = self.config_store.get_pipeline(tenant_id).await {
            if let Ok(json) = serde_json::to_string(&plan) {
                let _ = self.cache_store.set(&format!("pipeline:{}", tenant_id), &json, Some(300)).await;
            }
            return Some(plan);
        }
        
        None
    }

    pub async fn get_budget(&self, tenant_id: &str) -> Option<f64> {
        self.budget_store.get_remaining(tenant_id).await
    }

    pub async fn reserve_budget(&self, tenant_id: &str, amount: f64) -> Result<f64, String> {
        self.budget_store.reserve(tenant_id, amount).await
    }

    pub async fn release_budget(&self, tenant_id: &str, amount: f64) -> Result<f64, String> {
        self.budget_store.release(tenant_id, amount).await
    }

    pub async fn set_budget_limit(&self, tenant_id: &str, limit: f64) -> Result<(), String> {
        self.budget_store.set_limit(tenant_id, limit).await
    }

    pub async fn get_config_versions(&self) -> ConfigVersions {
        self.config_store.get_config_version().await
    }
}

impl Clone for StoreManager {
    fn clone(&self) -> Self {
        Self {
            trace_store: self.trace_store.clone(),
            config_store: self.config_store.clone(),
            budget_store: self.budget_store.clone(),
            cache_store: self.cache_store.clone(),
            object_store: self.object_store.clone(),
        }
    }
}

pub struct ConfigStoreAdapter {
    pipelines: std::sync::RwLock<std::collections::HashMap<String, ExecutionPlan>>,
    versions: std::sync::RwLock<ConfigVersions>,
}

impl ConfigStoreAdapter {
    pub fn new() -> Self {
        Self {
            pipelines: std::sync::RwLock::new(std::collections::HashMap::new()),
            versions: std::sync::RwLock::new(ConfigVersions::default()),
        }
    }

    pub fn set_pipeline(&self, tenant: &str, plan: ExecutionPlan) {
        let mut pipelines = self.pipelines.write().unwrap();
        pipelines.insert(tenant.to_string(), plan);
    }
}

impl Default for ConfigStoreAdapter {
    fn default() -> Self {
        Self::new()
    }
}

use async_trait::async_trait;

#[async_trait]
impl ConfigStore for ConfigStoreAdapter {
    async fn get_pipeline(&self, tenant_id: &str) -> Option<ExecutionPlan> {
        let pipelines = self.pipelines.read().unwrap();
        pipelines.get(tenant_id).cloned()
    }

    async fn list_pipelines(&self) -> Vec<ExecutionPlan> {
        let pipelines = self.pipelines.read().unwrap();
        pipelines.values().cloned().collect()
    }

    async fn save_pipeline(&self, plan: &ExecutionPlan) -> Result<(), String> {
        let mut pipelines = self.pipelines.write().unwrap();
        pipelines.insert(plan.tenant.clone().unwrap_or_default(), plan.clone());
        
        let mut versions = self.versions.write().unwrap();
        versions.pipeline_version += 1;
        
        Ok(())
    }

    async fn delete_pipeline(&self, plan_id: &str) -> Result<(), String> {
        let mut pipelines = self.pipelines.write().unwrap();
        pipelines.retain(|_, p| p.plan_id != plan_id);
        
        let mut versions = self.versions.write().unwrap();
        versions.pipeline_version += 1;
        
        Ok(())
    }

    async fn get_config_version(&self) -> ConfigVersions {
        let versions = self.versions.read().unwrap();
        versions.clone()
    }

    async fn notify_config_change(&self, change_type: &str) -> Result<(), String> {
        let mut versions = self.versions.write().unwrap();
        match change_type {
            "pipeline" => versions.pipeline_version += 1,
            "policy" => versions.policy_version += 1,
            "plugin" => versions.plugin_version += 1,
            _ => {}
        }
        Ok(())
    }
}
