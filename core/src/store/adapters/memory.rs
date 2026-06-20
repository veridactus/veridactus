//! # 内存存储适配器
//!
//! 用于开发和测试的内存存储实现。

use crate::store::traits::{CacheStore, ObjectStore, TraceStore};
use crate::types::trace::Trace;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

pub struct InMemoryTraceStoreAdapter {
    traces: RwLock<HashMap<Uuid, Trace>>,
}

impl InMemoryTraceStoreAdapter {
    pub fn new() -> Self {
        Self {
            traces: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryTraceStoreAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for InMemoryTraceStoreAdapter {
    fn clone(&self) -> Self {
        Self {
            traces: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl TraceStore for InMemoryTraceStoreAdapter {
    async fn save(&self, trace: Trace) -> Result<(), String> {
        let mut traces = self.traces.write().map_err(|e| e.to_string())?;
        traces.insert(trace.trace_id, trace);
        Ok(())
    }

    async fn get(&self, trace_id: &Uuid) -> Option<Trace> {
        let traces = self.traces.read().ok()?;
        traces.get(trace_id).cloned()
    }

    async fn list(&self, tenant_id: Option<&str>, _limit: usize, _offset: usize) -> Vec<Trace> {
        let traces = self.traces.read().unwrap_or_else(|e| e.into_inner());
        traces
            .values()
            .filter(|t| tenant_id.map_or(true, |tid| t.tenant_id.as_deref() == Some(tid)))
            .cloned()
            .collect()
    }

    async fn count(&self, tenant_id: Option<&str>) -> usize {
        let traces = self.traces.read().unwrap_or_else(|e| e.into_inner());
        if let Some(tid) = tenant_id {
            traces
                .values()
                .filter(|t| t.tenant_id.as_deref() == Some(tid))
                .count()
        } else {
            traces.len()
        }
    }

    async fn delete(&self, trace_id: &Uuid) -> Result<Option<Trace>, String> {
        let mut traces = self.traces.write().map_err(|e| e.to_string())?;
        Ok(traces.remove(trace_id))
    }

    async fn delete_by_session(&self, session_id: &Uuid) -> Result<Vec<Trace>, String> {
        let mut traces = self.traces.write().map_err(|e| e.to_string())?;
        let mut deleted = Vec::new();
        traces.retain(|_, t| {
            if t.session_id.as_ref() == Some(session_id) {
                deleted.push(t.clone());
                false
            } else {
                true
            }
        });
        Ok(deleted)
    }

    async fn delete_by_tenant(&self, tenant_id: &str) -> Result<Vec<Trace>, String> {
        let mut traces = self.traces.write().map_err(|e| e.to_string())?;
        let mut deleted = Vec::new();
        traces.retain(|_, t| {
            if t.tenant_id.as_deref() == Some(tenant_id) {
                deleted.push(t.clone());
                false
            } else {
                true
            }
        });
        Ok(deleted)
    }
}

/// 最大存储容量（默认 10000 pipelines Trace，防止 OOM）
const DEFAULT_MAX_CAPACITY: usize = 10000;

#[derive(Clone)]
pub struct InMemoryTraceStore {
    inner: InMemoryTraceStoreAdapter,
    max_capacity: usize,
}

impl InMemoryTraceStore {
    pub fn new() -> Self {
        Self {
            inner: InMemoryTraceStoreAdapter::new(),
            max_capacity: DEFAULT_MAX_CAPACITY,
        }
    }

    /// 设置最大存储容量（0 = 无限制）
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.max_capacity = capacity;
        self
    }
}

impl Default for InMemoryTraceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TraceStore for InMemoryTraceStore {
    async fn save(&self, trace: Trace) -> Result<(), String> {
        // LRU 淘汰：超过最大容量时移除最旧的 25%
        if self.max_capacity > 0 {
            let count = self.inner.count(None).await;
            if count >= self.max_capacity {
                let all = self.inner.list(None, self.max_capacity, 0).await;
                // 按 created_at 排序，删除最旧的 25%
                let to_remove = (self.max_capacity / 4).max(1);
                let mut sorted: Vec<_> = all.iter().collect();
                sorted.sort_by_key(|t| &t.created_at);
                for t in sorted.iter().take(to_remove) {
                    let _ = self.inner.delete(&t.trace_id).await;
                }
            }
        }
        self.inner.save(trace).await
    }

    async fn get(&self, trace_id: &Uuid) -> Option<Trace> {
        self.inner.get(trace_id).await
    }

    async fn list(&self, tenant_id: Option<&str>, limit: usize, offset: usize) -> Vec<Trace> {
        self.inner.list(tenant_id, limit, offset).await
    }

    async fn count(&self, tenant_id: Option<&str>) -> usize {
        self.inner.count(tenant_id).await
    }

    async fn delete(&self, trace_id: &Uuid) -> Result<Option<Trace>, String> {
        self.inner.delete(trace_id).await
    }

    async fn delete_by_session(&self, session_id: &Uuid) -> Result<Vec<Trace>, String> {
        self.inner.delete_by_session(session_id).await
    }

    async fn delete_by_tenant(&self, tenant_id: &str) -> Result<Vec<Trace>, String> {
        self.inner.delete_by_tenant(tenant_id).await
    }
}

pub struct InMemoryCacheStore {
    cache: RwLock<HashMap<String, (String, Option<u64>)>>,
}

impl InMemoryCacheStore {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCacheStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CacheStore for InMemoryCacheStore {
    async fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().ok()?;
        cache.get(key).map(|(v, _)| v.clone())
    }

    async fn set(&self, key: &str, value: &str, _ttl_secs: Option<u64>) -> Result<(), String> {
        let mut cache = self.cache.write().map_err(|e| e.to_string())?;
        cache.insert(key.to_string(), (value.to_string(), None));
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let mut cache = self.cache.write().map_err(|e| e.to_string())?;
        cache.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> bool {
        let cache = self.cache.read().unwrap_or_else(|e| e.into_inner());
        cache.contains_key(key)
    }
}

pub struct LocalFileStore {
    base_path: std::path::PathBuf,
}

impl LocalFileStore {
    pub fn new(base_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }
}

#[async_trait]
impl ObjectStore for LocalFileStore {
    async fn put(&self, bucket: &str, key: &str, data: &[u8]) -> Result<(), String> {
        let path = self.base_path.join(bucket).join(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get(&self, bucket: &str, key: &str) -> Option<Vec<u8>> {
        let path = self.base_path.join(bucket).join(key);
        tokio::fs::read(&path).await.ok()
    }

    async fn delete(&self, bucket: &str, key: &str) -> Result<(), String> {
        let path = self.base_path.join(bucket).join(key);
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| e.to_string())
    }

    async fn list(&self, bucket: &str, prefix: &str) -> Vec<String> {
        let path = self.base_path.join(bucket).join(prefix);
        let mut results = Vec::new();

        if let Ok(mut entries) = tokio::fs::read_dir(path.parent().unwrap_or(&path)).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(name) = entry.file_name().into_string() {
                    results.push(name);
                }
            }
        }

        results
    }
}
