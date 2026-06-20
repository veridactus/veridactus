//! # 长周期 Trace 管理器
//!
//! 严格遵循 AI.md §8.2 LongRunningTraceManager。
//! 支持分段哈希存储（每 1000 事件一个段）和异步结果聚合。

use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;

use crate::types::journal::{ExecutionJournal, JournalEvent, JournalEventType};
use crate::types::trace::Trace;

/// 异步任务回调
type Callback = Box<dyn FnOnce() -> Result<serde_json::Value, String> + Send>;

/// 任务 ID
type TaskId = String;

/// 长周期 Trace 管理器（AI.md §8.2）
pub struct LongRunningTraceManager {
    /// 分段哈希间隔
    segment_hash_interval: u64,
    /// 当前段的事件
    current_segment: Vec<JournalEvent>,
    /// 已完成的段哈希
    final_segments: Vec<String>,
    /// 等待中的异步结果
    pending_async_results: DashMap<TaskId, Callback>,
    /// 已完成的异步结果
    async_results: Vec<serde_json::Value>,
}

impl LongRunningTraceManager {
    /// 创建新的管理器
    pub fn new(segment_hash_interval: u64) -> Self {
        Self {
            segment_hash_interval,
            current_segment: Vec::new(),
            final_segments: Vec::new(),
            pending_async_results: DashMap::new(),
            async_results: Vec::new(),
        }
    }

    /// 追加事件到当前段（AI.md §8.2 append_event_with_segment）
    ///
    /// 段满时计算段哈希并开启新段。
    pub fn append_event_with_segment(&mut self, event: JournalEvent) {
        self.current_segment.push(event);

        if self.current_segment.len() >= self.segment_hash_interval as usize {
            let segment_hash = compute_segment_hash(&self.current_segment);
            self.final_segments.push(segment_hash);
            let seg_count = self.final_segments.len();
            info!(
                "段已完成: 段#{}, 事件数={}",
                seg_count,
                self.current_segment.len()
            );
            self.current_segment.clear();
        }
    }

    /// 注册异步任务回调
    pub fn register_async_result(&self, task_id: TaskId, callback: Callback) {
        self.pending_async_results.insert(task_id, callback);
    }

    /// 最终化并等待异步结果（AI.md §8.2 finalize_with_async_results）
    ///
    /// 等待所有异步任务完成或超时。
    pub async fn finalize_with_async_results(
        &mut self,
        _journal: ExecutionJournal,
        timeout_secs: u64,
    ) -> Result<Trace, String> {
        // 完成最后一段
        if !self.current_segment.is_empty() {
            let segment_hash = compute_segment_hash(&self.current_segment);
            self.final_segments.push(segment_hash);
            self.current_segment.clear();
        }

        // 等待异步结果
        let wait_timeout = Duration::from_secs(timeout_secs);
        let pending_ids: Vec<TaskId> = self
            .pending_async_results
            .iter()
            .map(|e| e.key().clone())
            .collect();

        for task_id in pending_ids {
            if let Some((_, callback)) = self.pending_async_results.remove(&task_id) {
                match timeout(wait_timeout, async { callback() }).await {
                    Ok(Ok(result)) => {
                        self.async_results.push(result);
                    }
                    Ok(Err(e)) => {
                        info!("异步任务 {} 失败: {}", task_id, e);
                    }
                    Err(_) => {
                        info!("异步任务 {} 超时", task_id);
                    }
                }
            }
        }

        // 计算最终聚合哈希
        let final_hash = aggregate_hashes(&self.final_segments, &self.async_results)?;

        // 构建 Trace（简化版本）
        let mut trace = Trace::new("long-running");
        trace.extensions = Some(serde_json::json!({
            "segment_count": self.final_segments.len(),
            "async_result_count": self.async_results.len(),
            "aggregated_hash": final_hash,
        }));

        info!(
            "长周期 Trace 最终化: {} 个段, {} 个异步结果",
            self.final_segments.len(),
            self.async_results.len()
        );

        Ok(trace)
    }

    /// 获取段数量
    pub fn segment_count(&self) -> usize {
        self.final_segments.len()
    }

    /// 获取当前待处理事件数
    pub fn pending_event_count(&self) -> usize {
        self.current_segment.len()
    }
}

/// 计算段哈希
fn compute_segment_hash(events: &[JournalEvent]) -> String {
    let mut hasher = Sha256::new();
    for event in events {
        hasher.update(event.hash.as_bytes());
    }
    format!("seg:{}", hex::encode(hasher.finalize()))
}

/// 聚合所有段哈希和异步结果
fn aggregate_hashes(
    segment_hashes: &[String],
    async_results: &[serde_json::Value],
) -> Result<String, String> {
    let mut hasher = Sha256::new();
    for sh in segment_hashes {
        hasher.update(sh.as_bytes());
    }
    for ar in async_results {
        let json = serde_json::to_string(ar).map_err(|e| e.to_string())?;
        hasher.update(json.as_bytes());
    }
    Ok(format!("agg:{}", hex::encode(hasher.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_segment_creation() {
        let mut mgr = LongRunningTraceManager::new(5); // 每5个事件一个段

        for i in 0..12u64 {
            let event = JournalEvent {
                seq: i + 1,
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: JournalEventType::PluginDecision {
                    plugin_name: "test".to_string(),
                    action: crate::types::Action::Continue,
                    latency_us: 10,
                },
                prev_hash: "0".repeat(64),
                hash: format!("hash_{}", i),
            };
            mgr.append_event_with_segment(event);
        }

        // 12 个事件 → 2 个完整段 + 2 个剩余事件
        assert_eq!(mgr.segment_count(), 2, "应有2个完整段");
        assert_eq!(mgr.pending_event_count(), 2, "应有2个待处理事件");
    }

    #[test]
    fn test_no_segment_for_small_count() {
        let mut mgr = LongRunningTraceManager::new(100);
        let event = JournalEvent {
            seq: 1,
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: JournalEventType::TraceFinalized {
                signature: "test".to_string(),
            },
            prev_hash: "0".repeat(64),
            hash: "hash_1".to_string(),
        };
        mgr.append_event_with_segment(event);
        assert_eq!(mgr.segment_count(), 0);
        assert_eq!(mgr.pending_event_count(), 1);
    }
}
