//! # Redis Stream 异步任务分发器
//!
//! 严格遵循 AI.md §2.1 架构图：AsyncQueue → Redis Stream → Workers。
//! 将跟踪证明、漂移检测等异步任务推入 Redis Stream。
//!
//! 数据结构：
//! - Stream key: `veridactus:tasks`
//! - 消费者组: `python-workers`
//! - Python Worker 已在 app/main.py 中实现消费者

use redis::AsyncCommands;
use serde_json::Value;
use tracing::info;

use crate::types::journal::{ExecutionJournal, JournalEventType};

/// Redis Stream 键名
const STREAM_KEY: &str = "veridactus:tasks";

/// 异步任务类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AsyncTaskType {
    /// 嵌入漂移检测
    #[serde(rename = "embedding_drift")]
    EmbeddingDrift,
    /// 认证保证计算 (C-SafeGen)
    #[serde(rename = "certified_guarantee")]
    CertifiedGuarantee,
    /// 语义分析
    #[serde(rename = "semantic_analysis")]
    SemanticAnalysis,
}

/// 异步任务载荷
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AsyncTask {
    /// 关联 Trace ID
    pub trace_id: String,
    /// 任务类型
    pub task_type: String,
    /// 任务参数
    pub params: Value,
    /// 时间戳
    pub timestamp: String,
    /// 回调 URL (Python Worker 完成后回传)
    pub callback_url: Option<String>,
}

/// Redis 异步分发器
#[derive(Clone)]
pub struct AsyncDispatcher {
    /// Redis 连接 URL
    redis_url: String,
}

impl AsyncDispatcher {
    /// 创建新的分发器
    pub fn new(redis_url: impl Into<String>) -> Self {
        Self {
            redis_url: redis_url.into(),
        }
    }

    /// 分发异步任务到 Redis Stream
    ///
    /// 1. 构建任务载荷
    /// 2. 推入 Redis Stream
    /// 3. 记录 Journal 事件
    ///
    /// # 参数
    /// * `task_type` - 任务类型
    /// * `trace_id` - 关联 Trace
    /// * `params` - 任务参数
    /// * `journal` - 用于记录分发事件的 Journal
    /// * `callback_url` - 可选回调地址
    pub async fn dispatch(
        &self,
        task_type: &str,
        trace_id: &str,
        params: Value,
        journal: &mut ExecutionJournal,
        callback_url: Option<String>,
    ) -> Result<String, String> {
        let client = redis::Client::open(self.redis_url.as_str())
            .map_err(|e| format!("Redis connection failed: {}", e))?;

        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection failed: {}", e))?;

        // 构建任务
        let task_id = uuid::Uuid::new_v4().to_string();
        let task = AsyncTask {
            trace_id: trace_id.to_string(),
            task_type: task_type.to_string(),
            params,
            timestamp: chrono::Utc::now().to_rfc3339(),
            callback_url,
        };

        let task_json = serde_json::to_string(&task)
            .map_err(|e| format!("Task serialization failed: {}", e))?;

        // 推入 Redis Stream
        let _: String = conn
            .xadd(STREAM_KEY, "*", &[("task", &task_json)])
            .await
            .map_err(|e| format!("Redis XADD failed: {}", e))?;

        // 记录 Journal 事件
        journal.append_event(JournalEventType::AsyncTaskDispatched {
            task_type: task_type.to_string(),
            task_id: task_id.clone(),
        });

        info!(
            "异步任务已分发: type={}, trace_id={}, task_id={}",
            task_type, trace_id, task_id
        );

        Ok(task_id)
    }

    /// 检查 Redis 连接是否正常
    pub async fn health_check(&self) -> Result<(), String> {
        let client = redis::Client::open(self.redis_url.as_str())
            .map_err(|e| format!("Invalid Redis URL: {}", e))?;
        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection failed: {}", e))?;

        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| format!("Redis PING failed: {}", e))?;

        Ok(())
    }

    /// 获取默认分发器 (Redis localhost:6379)
    pub fn default() -> Self {
        Self::new("redis://127.0.0.1:6379")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_serialization() {
        let task = AsyncTask {
            trace_id: "test-trace".to_string(),
            task_type: "embedding_drift".to_string(),
            params: serde_json::json!({"prompt": "hello", "response": "world"}),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            callback_url: Some("http://localhost:8081/api/v1/traces/update".to_string()),
        };
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("embedding_drift"));
        assert!(json.contains("test-trace"));

        // 反序列化回来
        let deserialized: AsyncTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.trace_id, "test-trace");
    }
}
