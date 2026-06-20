//! # Execution Journal 执行日志
//!
//! 严格遵循 AI.md §4.0 Execution Journal 详细设计。
//! Journal 是 VERIDACTUS 的核心可信组件，通过事件溯源 + 哈希链实现不可篡改。

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

use super::Action;
use super::trace::ExecutionState;
use super::SafetyEvent;

/// Journal 事件类型枚举（覆盖协议关键节点）
///
/// 参考 AI.md §4.1 数据结构与哈希链
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalEventType {
    /// 请求到达事件
    RequestReceived {
        method: String,
        path: String,
        headers: BTreeMap<String, String>,
        body_hash: String,
    },
    /// 请求解析完成
    RequestParsed {
        model: String,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
    },
    /// 状态转换
    StateTransition {
        from: ExecutionState,
        to: ExecutionState,
    },
    /// 插件决策事件
    PluginDecision {
        plugin_name: String,
        action: Action,
        latency_us: u64,
    },
    /// 上游模型选择事件
    UpstreamSelected {
        model: String,
        endpoint: String,
    },
    /// 流式 chunk 投递事件
    StreamChunkDelivered {
        seq: u64,
        chunk_hash: String,
        client_ack: bool,
    },
    /// 流结束事件
    StreamEnd {
        total_tokens: u32,
        finish_reason: String,
    },
    /// 流错误事件
    StreamError {
        error: String,
        truncated: bool,
    },
    /// 异步任务分发事件
    AsyncTaskDispatched {
        task_type: String,
        task_id: String,
    },
    /// 异步任务结果事件
    AsyncTaskResult {
        task_type: String,
        result_hash: String,
        signature: String,
        success: bool,
    },
    /// Trace 最终化事件（含 L0 签名）
    TraceFinalized {
        signature: String,
    },
    /// 安全事件（ASI 相关）
    SafetyEvent(SafetyEvent),
    /// 约束冲突事件（§5.5）
    ConstraintConflict {
        constraint_a: String,
        value_a: String,
        constraint_b: String,
        value_b: String,
        conflict_type: String,
        reason: String,
    },
}

/// 单个 Journal 事件
///
/// 每个事件包含前一个事件的哈希，形成不可篡改的哈希链。
/// hash = SHA-256(prev_hash + seq + canonical(event_type))
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEvent {
    /// 事件序号（严格递增）
    pub seq: u64,
    /// 事件时间戳
    pub timestamp: String,
    /// 事件类型
    pub event_type: JournalEventType,
    /// 前一个事件的 SHA-256 哈希（首事件为 64 个 '0'）
    pub prev_hash: String,
    /// 当前事件的 SHA-256 哈希
    /// hash = SHA-256(prev_hash || seq || canonical(event_type))
    pub hash: String,
}

impl JournalEvent {
    /// 计算事件的哈希值
    ///
    /// 哈希公式: SHA-256(prev_hash || seq || canonical_json(event_type))
    fn compute_hash(event: &JournalEventType, prev_hash: &str, seq: u64) -> String {
        let canonical = serde_json::to_string(event).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(seq.to_le_bytes());
        hasher.update(canonical.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Execution Journal - 可信事件日志
///
/// 核心可信组件，使用事件溯源模式记录 LLM 执行的完整生命周期。
/// 支持哈希链防篡改、确定性重放和密码学证明。
///
/// 参考 AI.md §4.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionJournal {
    /// 关联的 Trace ID
    pub trace_id: Uuid,
    /// 租户 ID
    pub tenant_id: String,
    /// 创建时间
    pub created_at: String,
    /// 事件列表
    pub events: Vec<JournalEvent>,
    /// 当前链头哈希
    pub head_hash: String,
}

impl ExecutionJournal {
    /// 创建一个新的 Execution Journal
    ///
    /// 初始化时设置首事件的前一个哈希为 64 个 '0'。
    pub fn new(trace_id: Uuid, tenant_id: impl Into<String>) -> Self {
        Self {
            trace_id,
            tenant_id: tenant_id.into(),
            created_at: Utc::now().to_rfc3339(),
            events: Vec::with_capacity(64), // 预分配空间
            head_hash: "0".repeat(64),       // 初始哈希为 64 个 '0'
        }
    }

    /// 追加一个新事件到 Journal
    ///
    /// # 参数
    /// * `event_type` - 事件类型
    ///
    /// # 返回
    /// 追加的事件的序号
    ///
    /// # 哈希链保证
    /// 自动计算 prev_hash 和当前事件哈希，确保链的完整性。
    pub fn append_event(&mut self, event_type: JournalEventType) -> u64 {
        let seq = self.events.len() as u64 + 1;
        let prev_hash = self.head_hash.clone();

        // 计算当前事件的哈希
        let hash = JournalEvent::compute_hash(&event_type, &prev_hash, seq);

        let event = JournalEvent {
            seq,
            timestamp: Utc::now().to_rfc3339(),
            event_type,
            prev_hash,
            hash: hash.clone(),
        };

        self.head_hash = hash;
        self.events.push(event);
        seq
    }

    /// 验证 Journal 哈希链的完整性
    ///
    /// 遍历所有事件，重新计算哈希并与存储的值比对。
    ///
    /// # 返回
    /// * `Ok(())` - 链完整
    /// * `Err(String)` - 链断裂，返回具体的失败信息
    pub fn verify_chain(&self) -> Result<(), String> {
        let mut expected_prev = "0".repeat(64);

        for event in &self.events {
            // 验证 prev_hash 匹配
            if event.prev_hash != expected_prev {
                return Err(format!(
                    "哈希链断裂于 seq {}: 期望 prev_hash={}, 实际 prev_hash={}",
                    event.seq, expected_prev, event.prev_hash
                ));
            }

            // 重新计算哈希并验证
            let recomputed = JournalEvent::compute_hash(&event.event_type, &event.prev_hash, event.seq);
            if recomputed != event.hash {
                return Err(format!(
                    "事件 {} 哈希不匹配: 期望 hash={}, 重算 hash={}",
                    event.seq, event.hash, recomputed
                ));
            }

            expected_prev = event.hash.clone();
        }

        // 验证 head_hash
        if self.head_hash != expected_prev {
            return Err(format!(
                "head_hash 不匹配: 期望={}, 实际={}",
                expected_prev, self.head_hash
            ));
        }

        Ok(())
    }

    /// 获取最新事件的哈希（链头）
    pub fn head_hash(&self) -> &str {
        &self.head_hash
    }

    /// 获取事件数量
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// 检查 Journal 是否为空
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// 测试 Journal 创建
    #[test]
    fn test_journal_creation() {
        let trace_id = Uuid::new_v4();
        let journal = ExecutionJournal::new(trace_id, "test-tenant");
        assert!(journal.is_empty());
        assert_eq!(journal.head_hash, "0".repeat(64));
        assert_eq!(journal.trace_id, trace_id);
    }

    /// 测试事件追加和哈希链
    #[test]
    fn test_event_append_and_hash_chain() {
        let trace_id = Uuid::new_v4();
        let mut journal = ExecutionJournal::new(trace_id, "test-tenant");

        // 追加请求到达事件
        let mut headers = BTreeMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let seq1 = journal.append_event(JournalEventType::RequestReceived {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers,
            body_hash: "abc123".to_string(),
        });
        assert_eq!(seq1, 1);
        assert_eq!(journal.event_count(), 1);

        // 追加插件决策事件
        let seq2 = journal.append_event(JournalEventType::PluginDecision {
            plugin_name: "budget".to_string(),
            action: Action::Continue,
            latency_us: 15,
        });
        assert_eq!(seq2, 2);
        assert_eq!(journal.event_count(), 2);

        // 验证链完整
        assert!(journal.verify_chain().is_ok());

        // 验证第二个事件的 prev_hash 等于第一个事件的 hash
        assert_eq!(journal.events[1].prev_hash, journal.events[0].hash);
    }

    /// 测试哈希链篡改检测
    #[test]
    fn test_tamper_detection() {
        let trace_id = Uuid::new_v4();
        let mut journal = ExecutionJournal::new(trace_id, "test-tenant");

        journal.append_event(JournalEventType::RequestReceived {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers: BTreeMap::new(),
            body_hash: "abc".to_string(),
        });

        journal.append_event(JournalEventType::PluginDecision {
            plugin_name: "auth".to_string(),
            action: Action::Continue,
            latency_us: 10,
        });

        // 验证原始链完整
        assert!(journal.verify_chain().is_ok());

        // 篡改: 修改第一个事件的哈希
        journal.events[0].hash = "fake_hash".to_string();

        // 验证链断裂
        assert!(journal.verify_chain().is_err());
    }

    /// 测试空 Journal 验证
    #[test]
    fn test_empty_journal_verification() {
        let trace_id = Uuid::new_v4();
        let journal = ExecutionJournal::new(trace_id, "test-tenant");
        assert!(journal.verify_chain().is_ok());
    }
}
