//! # SSE 流处理 + Budget Awareness + Constrained Decoding
//!
//! 实现 VERIDACTUS 协议 §4.3 SSE 截断规范、§4.6 预算感知 SSE 事件、
//! §8.4 主动预防（constrained decoding）。

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use bytes::Bytes;
use futures::Stream;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::crypto::signature::compute_sha256_hex;
use crate::prevention::{ConstrainedDecoder, PreventionEvent};
use crate::types::journal::{ExecutionJournal, JournalEventType};

/// SSE 截断错误代码（§4.3）
const SSE_ERROR_BUDGET_EXCEEDED: &str = r#"data: {"error": {"code": "VERIDACTUS_BUDGET_EXCEEDED", "message": "Request terminated due to budget limit", "trace_id": "CURRENT_TRACE_ID"}}
data: [DONE]
"#;

/// 每 chunk 的近似 token 成本（简化模型：~4 chars/token）
const CHARS_PER_TOKEN: f64 = 4.0;
const COST_PER_TOKEN_USD: f64 = 0.000003; // $0.003/1K tokens (micro-dollar)

/// 预算感知注入事件（§4.6）
const BUDGET_EVENT_TEMPLATE: &str = r#"data: {"veridactus": {"budget_remaining": BUDGET_REMAINING, "budget_low": BUDGET_LOW, "budget_pct": BUDGET_PCT}}
"#;

/// VeridactusStreamHandler 包装上游字节流，支持预算感知和主动预防
pub struct VeridactusStreamHandler {
    rx: mpsc::Receiver<Result<String, Infallible>>,
    journal: Option<ExecutionJournal>,
    /// 关联的 trace_id（用于错误消息）
    trace_id: String,
    chunks_delivered: u64,
    total_content: String,
    blocked: bool,
    blocked_reason: Option<String>,
    /// 预算限制（美元，0 表示无限制）
    budget_limit_usd: f64,
    /// 累计成本
    accumulated_cost: f64,
    /// 是否启用预算感知 SSE 事件
    budget_awareness: bool,
    /// 上次注入预算事件时的百分比（避免过于频繁注入）
    last_budget_pct_injected: u32,
    /// 主动预防解码器
    prevention: Option<Arc<ConstrainedDecoder>>,
    /// 预防事件记录
    prevention_events: Vec<PreventionEvent>,
}

impl VeridactusStreamHandler {
    pub fn new(rx: mpsc::Receiver<Result<String, Infallible>>, trace_id: String) -> Self {
        Self {
            rx,
            journal: None,
            trace_id,
            chunks_delivered: 0,
            total_content: String::new(),
            blocked: false,
            blocked_reason: None,
            budget_limit_usd: 0.0,
            accumulated_cost: 0.0,
            budget_awareness: false,
            last_budget_pct_injected: 100,
            prevention: None,
            prevention_events: Vec::new(),
        }
    }

    pub fn with_journal(mut self, journal: ExecutionJournal) -> Self {
        self.journal = Some(journal);
        self
    }

    /// 设置预算限制和预算感知（§4.6, §5.9）
    pub fn with_budget(mut self, limit_usd: f64, awareness: bool) -> Self {
        self.budget_limit_usd = limit_usd;
        self.budget_awareness = awareness && limit_usd > 0.0;
        self
    }

    /// 设置主动预防解码器（§8.4）
    pub fn with_prevention(mut self, decoder: Arc<ConstrainedDecoder>) -> Self {
        self.prevention = Some(decoder);
        self
    }

    pub fn total_chunks(&self) -> u64 { self.chunks_delivered }
    pub fn total_content(&self) -> &str { &self.total_content }
    pub fn is_blocked(&self) -> bool { self.blocked }
    pub fn blocked_reason(&self) -> Option<&str> { self.blocked_reason.as_deref() }
    pub fn accumulated_cost(&self) -> f64 { self.accumulated_cost }
    pub fn prevention_events(&self) -> &[PreventionEvent] { &self.prevention_events }

    /// 计算当前预算百分比
    fn budget_pct(&self) -> f64 {
        if self.budget_limit_usd > 0.0 {
            ((self.budget_limit_usd - self.accumulated_cost) / self.budget_limit_usd * 100.0)
                .max(0.0)
        } else {
            100.0
        }
    }

    /// 生成预算感知 SSE 事件
    fn build_budget_event(&self) -> String {
        let remaining = (self.budget_limit_usd - self.accumulated_cost).max(0.0);
        let pct = self.budget_pct();
        let low = pct < 20.0;
        BUDGET_EVENT_TEMPLATE
            .replace("BUDGET_REMAINING", &format!("{:.6}", remaining))
            .replace("BUDGET_LOW", if low { "true" } else { "false" })
            .replace("BUDGET_PCT", &format!("{:.1}", pct))
    }
}

impl Stream for VeridactusStreamHandler {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let chunk_fut = self.rx.poll_recv(cx);
        match chunk_fut {
            Poll::Ready(Some(Ok(chunk))) => {
                let me = &mut *self;
                me.chunks_delivered += 1;
                me.total_content.push_str(&chunk);

                // 更新累计成本（基于字符数近似估算 tokens）
                let est_tokens = chunk.chars().count() as f64 / CHARS_PER_TOKEN;
                me.accumulated_cost += est_tokens * COST_PER_TOKEN_USD;

                let seq = me.chunks_delivered;
                let content_hash = compute_sha256_hex(chunk.as_bytes());

                // Journal 记录
                if let Some(ref mut journal) = me.journal {
                    journal.append_event(JournalEventType::StreamChunkDelivered {
                        seq,
                        chunk_hash: content_hash,
                        client_ack: true,
                    });
                }

                // 主动预防检查（§8.4 constraint decoding）
                if let Some(ref prevention) = me.prevention {
                    // 提取 SSE data 中的实际文本内容
                    let text = if chunk.starts_with("data: ") {
                        &chunk[6..]
                    } else {
                        &chunk
                    };
                    if let Some(event) = prevention.check_text(text) {
                        me.prevention_events.push(event.clone());
                        warn!(
                            "主动预防阻断: category={}, tokens={}",
                            event.blocked_pattern_category, event.blocked_tokens.len()
                        );
                        me.blocked = true;
                        me.blocked_reason = Some(format!(
                            "主动预防阻断: {} (类别: {})",
                            event.blocked_tokens.join(", "),
                            event.blocked_pattern_category
                        ));
                        // 发送 SSE 错误块
                        let error_sse = format!(
                            "data: {{\"error\": {{\"code\": \"VERIDACTUS_ACTIVE_PREVENTION_BLOCKED\", \"message\": \"Active prevention blocked generation: {}\", \"trace_id\": \"N/A\"}}}}\n\ndata: [DONE]\n",
                            event.blocked_pattern_category
                        );
                        return Poll::Ready(Some(Ok(Bytes::from(error_sse))));
                    }
                }

                // 构造输出
                let mut output = format!("data: {}\n\n", chunk);

                // 预算感知 SSE 事件注入（§4.6: 每 20% 变化注入一次）
                if me.budget_awareness {
                    let current_pct = me.budget_pct() as u32;
                    let pct_threshold = (current_pct / 20) * 20; // 向下取整到 20% 步长
                    let last_threshold = (me.last_budget_pct_injected / 20) * 20;
                    if pct_threshold != last_threshold {
                        me.last_budget_pct_injected = current_pct;
                        let budget_event = me.build_budget_event();
                        output = format!("{}{}", budget_event, output);
                    }
                }

                // 预算耗尽检查（§4.3 hard stop）
                if me.budget_limit_usd > 0.0 && me.accumulated_cost >= me.budget_limit_usd {
                    warn!(
                        "预算耗尽: accumulated={:.6}, limit={:.6}",
                        me.accumulated_cost, me.budget_limit_usd
                    );
                    me.blocked = true;
                    me.blocked_reason = Some("预算耗尽".to_string());
                    let error_sse = build_budget_exceeded_sse(&me.trace_id);
                    output.push_str(&error_sse);
                }

                Poll::Ready(Some(Ok(Bytes::from(output))))
            }
            Poll::Ready(Some(Err(_))) => {
                Poll::Ready(None)
            }
            Poll::Ready(None) => {
                let tokens = self.chunks_delivered as u32;
                let is_blocked = self.blocked;
                let accumulated_cost = self.accumulated_cost;
                let finish = if is_blocked { "truncated" } else { "stop" };
                if let Some(ref mut journal) = self.journal {
                    journal.append_event(JournalEventType::StreamEnd {
                        total_tokens: tokens,
                        finish_reason: finish.to_string(),
                    });
                }
                info!(
                    chunks = tokens,
                    cost = accumulated_cost,
                    blocked = is_blocked,
                    "SSE 流结束"
                );
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// 构建预算耗尽 SSE 错误块（§4.3）
pub fn build_budget_exceeded_sse(trace_id: &str) -> String {
    SSE_ERROR_BUDGET_EXCEEDED.replace("CURRENT_TRACE_ID", trace_id)
}
