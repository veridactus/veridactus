//! # 异步写入队列
//!
//! 提供 Trace 的异步写入功能，支持后台批量处理。

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Notify};
use tracing::{error, info};
use uuid::Uuid;

use crate::types::trace::Trace;
use crate::store::traits::{TraceStore, ObjectStore, AsyncTraceWriter};

pub struct AsyncWriteQueue {
    sender: mpsc::Sender<Trace>,
    queue: Arc<RwLock<Vec<Trace>>>,
    pending_count: Arc<RwLock<usize>>,
    /// 关闭通知（drop 时触发，确保所有待处理 Trace 被写入）
    shutdown: Arc<Notify>,
}

impl AsyncWriteQueue {
    pub fn new<S: TraceStore + ObjectStore + 'static>(
        store: Arc<S>,
        object_store: Arc<S>,
        batch_size: usize,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<Trace>(10000);
        let queue = Arc::new(RwLock::new(Vec::with_capacity(batch_size)));
        let pending_count = Arc::new(RwLock::new(0));
        let queue_clone = queue.clone();
        let pending_clone = pending_count.clone();
        let store_clone = store.clone();
        let object_store_clone = object_store.clone();
        let shutdown = Arc::new(Notify::new());
        let shutdown_clone = shutdown.clone();

        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(batch_size);
            
            loop {
                tokio::select! {
                    Some(trace) = rx.recv() => {
                        batch.push(trace);
                        *pending_clone.write().await += 1;
                        
                        if batch.len() >= batch_size {
                            Self::flush_batch(&store_clone, &object_store_clone, &mut batch).await;
                            *pending_clone.write().await -= batch.len();
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                        if !batch.is_empty() {
                            Self::flush_batch(&store_clone, &object_store_clone, &mut batch).await;
                            *pending_clone.write().await -= batch.len();
                        }
                    }
                    _ = shutdown_clone.notified() => {
                        // 优雅关闭：刷出剩余批次
                        if !batch.is_empty() {
                            info!("AsyncWriteQueue 关闭: 刷出最后 {} 个 Trace", batch.len());
                            Self::flush_batch(&store_clone, &object_store_clone, &mut batch).await;
                        }
                        break;
                    }
                }
            }
        });

        Self {
            sender: tx,
            queue: queue_clone,
            pending_count,
            shutdown,
        }
    }

    /// 触发优雅关闭，刷出所有待处理 Trace
    pub fn shutdown(&self) {
        self.shutdown.notify_one();
    }

    async fn flush_batch<S: TraceStore + ObjectStore>(
        store: &Arc<S>,
        _object_store: &Arc<S>,
        batch: &mut Vec<Trace>,
    ) {
        if batch.is_empty() {
            return;
        }

        info!("异步写入 {} 个 Trace", batch.len());

        for trace in batch.drain(..) {
            match store.save(trace).await {
                Ok(_) => {}
                Err(e) => {
                    error!("Trace write failed: {}", e);
                }
            }
        }
    }

    pub async fn pending(&self) -> usize {
        *self.pending_count.read().await
    }
}

impl AsyncTraceWriter for AsyncWriteQueue {
    fn enqueue(&self, trace: Trace) -> Result<(), String> {
        self.sender
            .try_send(trace)
            .map_err(|e| e.to_string())
    }

    fn len(&self) -> usize {
        self.sender.capacity()
    }

    fn is_empty(&self) -> bool {
        self.sender.capacity() == 0
    }
}

pub struct HybridTraceStore<S: TraceStore, O: ObjectStore> {
    memory_store: Arc<RwLock<Vec<Trace>>>,
    persistent_store: S,
    object_store: O,
    memory_capacity: usize,
}

impl<S: TraceStore, O: ObjectStore> HybridTraceStore<S, O> {
    pub fn new(
        persistent_store: S,
        object_store: O,
        memory_capacity: usize,
    ) -> Self {
        Self {
            memory_store: Arc::new(RwLock::new(Vec::new())),
            persistent_store,
            object_store,
            memory_capacity,
        }
    }

    pub async fn save(&self, trace: Trace) -> Result<(), String> {
        let trace_size = serde_json::to_string(&trace)
            .map(|s| s.len())
            .unwrap_or(0);

        let trace_for_memory = trace.clone();

        if trace_size < 100_000 {
            // 小 Trace: 直接存入 PG
            self.persistent_store.save(trace.clone()).await?;
        } else {
            // 大 Trace: S3 存储全量，PG 仅存元数据+引用
            let trace_id = trace.trace_id.to_string();
            let tenant_id = trace.tenant_id.as_deref().unwrap_or("default");
            let key = format!("traces/{}/{}.json", tenant_id, trace_id);

            let trace_json = serde_json::to_vec(&trace).map_err(|e| e.to_string())?;
            self.object_store.put("veridactus", &key, &trace_json).await?;

            // PG 存储精简版（仅元数据+S3引用，不包含完整trace_data）
            let mut trace_meta = trace;
            trace_meta.extensions = Some(serde_json::json!({
                "s3_path": key,
                "storage_tier": "s3",
                "original_size_bytes": trace_size,
            }));
            self.persistent_store.save(trace_meta).await?;
        }

        let mut memory = self.memory_store.write().await;
        memory.push(trace_for_memory);
        if memory.len() > self.memory_capacity {
            memory.remove(0);
        }

        Ok(())
    }

    pub async fn get(&self, trace_id: &Uuid) -> Option<Trace> {
        let memory = self.memory_store.read().await;
        if let Some(trace) = memory.iter().find(|t| &t.trace_id == trace_id) {
            return Some(trace.clone());
        }

        self.persistent_store.get(trace_id).await
    }

    pub async fn list(&self, tenant_id: Option<&str>, limit: usize, offset: usize) -> Vec<Trace> {
        self.persistent_store.list(tenant_id, limit, offset).await
    }

    pub async fn count(&self, tenant_id: Option<&str>) -> usize {
        self.persistent_store.count(tenant_id).await
    }
}
