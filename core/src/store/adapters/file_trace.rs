//! # 本地文件系统 Trace 存储适配器
//!
//! 将 Trace 持久化为 JSON 文件，适用于单机部署、边缘设备和开发环境。
//! 支持按租户/TraceID 的分层目录结构。

use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;
use uuid::Uuid;
use tracing::{info, warn};
use crate::types::trace::Trace;
use crate::store::traits::TraceStore;

/// 本地文件 Trace 存储
///
/// ## 目录结构
/// ```text
/// data/traces/
///   ├── tenant_a/
///   │   ├── 550e8400-*.json
///   │   └── ...
///   ├── tenant_b/
///   │   └── ...
///   └── _index.json        # 全局索引（trace_id → file_path）
/// ```
pub struct FileTraceStore {
    base_path: PathBuf,
    /// 内存索引：trace_id → (tenant_id, file_name)
    index: RwLock<HashMap<Uuid, (String, String)>>,
    /// 租户目录缓存
    tenant_dirs: RwLock<HashMap<String, PathBuf>>,
}

impl FileTraceStore {
    /// 创建新的文件存储实例
    /// - `base_path`: 存储根目录，默认 "data/traces"
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        let bp = base_path.into();
        std::fs::create_dir_all(&bp).ok();

        let store = Self {
            base_path: bp,
            index: RwLock::new(HashMap::new()),
            tenant_dirs: RwLock::new(HashMap::new()),
        };

        // 启动时重建索引
        store.rebuild_index();
        store
    }

    /// 获取租户目录路径，自动创建
    fn tenant_dir(&self, tenant_id: &str) -> PathBuf {
        {
            let dirs = self.tenant_dirs.read().unwrap();
            if let Some(dir) = dirs.get(tenant_id) {
                return dir.clone();
            }
        }
        let dir = self.base_path.join(tenant_id);
        std::fs::create_dir_all(&dir).ok();
        let mut dirs = self.tenant_dirs.write().unwrap();
        dirs.insert(tenant_id.to_string(), dir.clone());
        dir
    }

    /// 获取 Trace 文件路径
    fn trace_path(&self, tenant_id: &str, trace_id: &Uuid) -> PathBuf {
        self.tenant_dir(tenant_id)
            .join(format!("{}.json", trace_id))
    }

    /// 扫描磁盘重建内存索引
    fn rebuild_index(&self) {
        let mut index = self.index.write().unwrap();
        index.clear();
        let mut count = 0usize;

        if let Ok(entries) = std::fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let tenant = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    if let Ok(files) = std::fs::read_dir(&path) {
                        for file in files.flatten() {
                            let name = file.file_name();
                            let name_str = name.to_string_lossy();
                            if name_str.ends_with(".json") {
                                let id_str = name_str.trim_end_matches(".json");
                                if let Ok(tid) = Uuid::parse_str(id_str) {
                                    index.insert(tid, (tenant.clone(), name_str.to_string()));
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        info!("FileTraceStore index rebuilt: {} traces", count);
    }

    /// 序列化 Trace 为 JSON 并写入文件
    async fn write_trace_file(&self, path: &PathBuf, trace: &Trace) -> Result<(), String> {
        let json = serde_json::to_string_pretty(trace).map_err(|e| e.to_string())?;
        tokio::fs::write(path, json.as_bytes())
            .await
            .map_err(|e| format!("File write failed: {}", e))?;
        Ok(())
    }

    /// 从文件读取 Trace
    async fn read_trace_file(&self, path: &PathBuf) -> Option<Trace> {
        let data = tokio::fs::read(path).await.ok()?;
        serde_json::from_slice(&data).ok()
    }
}

#[async_trait]
impl TraceStore for FileTraceStore {
    async fn save(&self, trace: Trace) -> Result<(), String> {
        let tenant = trace.tenant_id.as_deref().unwrap_or("default");
        let path = self.trace_path(tenant, &trace.trace_id);

        self.write_trace_file(&path, &trace).await?;

        // 更新内存索引
        let mut index = self.index.write().unwrap();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        index.insert(trace.trace_id, (tenant.to_string(), file_name));

        Ok(())
    }

    async fn get(&self, trace_id: &Uuid) -> Option<Trace> {
        // 1. 查内存索引
        let (tenant, _) = {
            let index = self.index.read().unwrap();
            index.get(trace_id).cloned()?
        };

        // 2. 读文件
        let path = self.trace_path(&tenant, trace_id);
        self.read_trace_file(&path).await
    }

    async fn list(&self, tenant_id: Option<&str>, limit: usize, offset: usize) -> Vec<Trace> {
        let index = self.index.read().unwrap();
        let mut entries: Vec<_> = index.iter()
            .filter(|(_, (t, _))| tenant_id.map_or(true, |tid| t == tid))
            .collect();

        // 按 trace_id 排序（近似时间排序，UUID v4 时间戳在前）
        entries.sort_by_key(|(id, _)| id.to_string());

        let results: Vec<Trace> = entries
            .iter()
            .skip(offset)
            .take(if limit == 0 { usize::MAX } else { limit })
            .filter_map(|(id, (tenant, _))| {
                let path = self.trace_path(tenant, id);
                // 注意：这里使用同步读取以避免 async 闭包问题
                std::fs::read_to_string(&path).ok()
                    .and_then(|data| serde_json::from_str(&data).ok())
            })
            .collect();

        results
    }

    async fn count(&self, tenant_id: Option<&str>) -> usize {
        let index = self.index.read().unwrap();
        if let Some(tid) = tenant_id {
            index.values().filter(|(t, _)| t == tid).count()
        } else {
            index.len()
        }
    }

    async fn delete(&self, trace_id: &Uuid) -> Result<Option<Trace>, String> {
        let (tenant, _) = {
            let index = self.index.read().unwrap();
            index.get(trace_id).cloned().ok_or("Trace not found".to_string())?
        };

        let path = self.trace_path(&tenant, trace_id);
        let trace = self.read_trace_file(&path).await;

        // 删除文件
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| format!("File delete failed: {}", e))?;

        // 从索引移除
        let mut index = self.index.write().unwrap();
        index.remove(trace_id);

        Ok(trace)
    }

    async fn delete_by_session(&self, session_id: &Uuid) -> Result<Vec<Trace>, String> {
        // 扫描所有文件查找匹配的 session_id
        let all = self.list(None, 0, 0).await;
        let mut deleted = Vec::new();

        for trace in &all {
            if trace.session_id.as_ref() == Some(session_id) {
                if let Ok(Some(t)) = self.delete(&trace.trace_id).await {
                    deleted.push(t);
                }
            }
        }
        Ok(deleted)
    }

    async fn delete_by_tenant(&self, tenant_id: &str) -> Result<Vec<Trace>, String> {
        let all = self.list(Some(tenant_id), 0, 0).await;
        let mut deleted = Vec::new();

        for trace in &all {
            if let Ok(Some(t)) = self.delete(&trace.trace_id).await {
                deleted.push(t);
            }
        }

        // 尝试删除空目录
        let dir = self.tenant_dir(tenant_id);
        tokio::fs::remove_dir(&dir).await.ok();

        Ok(deleted)
    }
}
