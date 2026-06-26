//! # VERIDACTUS 流式预算熔断守卫 (Phase 2)
//!
//! 在 SSE 流式输出过程中实时扣减 Redis 预算。
//! 每 N 个 token 发起一次 Redis Lua 原子扣减，超限时立即切断流。
//!
//! 参考: SPECIFICATION.md §2.2.2-2.2.3

use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use std::time::Instant;
use tracing::{info, warn};

/// 预算检查状态
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetStatus {
    /// 预算充足，继续
    Ok { remaining_micro: i64 },
    /// 预算不足，已熔断
    Exceeded { reason: String, remaining_micro: i64 },
    /// Redis 不可用，降级到内存模式
    Degraded,
    /// 内部错误
    Error(String),
}

/// 流式预算熔断守卫
///
/// 绑定到一个 workspace 的 Redis 预算 key。
/// 每次 SSE chunk 到达后调用 `check_and_decr`，
/// 内部按 `check_interval` 批量提交到 Redis。
pub struct StreamBudgetGuard {
    redis: Option<MultiplexedConnection>,
    workspace_id: String,
    /// 每间隔多少个 token 检查一次预算
    check_interval: usize,
    /// 上次检查后累积的 token 数
    tokens_since_check: usize,
    /// 每个 token 的预估成本（微美元）
    cost_per_token_micro: u64,
    /// 日预算上限（微美元，0=无限）
    daily_limit_micro: i64,
    /// 请求 ID（用于日志）
    request_id: String,
    /// 创建时间（用于日志）
    started_at: Instant,
    /// 总已消费 token（用于最终持久化）
    total_tokens: u64,
    /// 总已消费微美元
    total_cost_micro: u64,
}

impl StreamBudgetGuard {
    /// 创建预算守卫
    pub fn new(
        redis: Option<MultiplexedConnection>,
        workspace_id: String,
        cost_per_token_micro: u64,
        daily_limit_micro: i64,
        request_id: String,
    ) -> Self {
        Self {
            redis,
            workspace_id,
            check_interval: 10, // 每 10 个 token 检查一次
            tokens_since_check: 0,
            cost_per_token_micro,
            daily_limit_micro,
            request_id,
            started_at: Instant::now(),
            total_tokens: 0,
            total_cost_micro: 0,
        }
    }

    /// 设置检查间隔（默认 10 token）
    pub fn with_check_interval(mut self, interval: usize) -> Self {
        self.check_interval = interval;
        self
    }

    /// 记录一个 token 消耗，累积到阈值后触发 Redis 扣减
    pub async fn record_token(&mut self) -> Result<BudgetStatus, String> {
        self.total_tokens += 1;
        self.tokens_since_check += 1;

        if self.tokens_since_check < self.check_interval {
            return Ok(BudgetStatus::Degraded); // 未到检查点，跳过
        }

        let cost = self.tokens_since_check as u64 * self.cost_per_token_micro;
        self.total_cost_micro += cost;
        self.tokens_since_check = 0;

        self.check_budget(cost as i64).await
    }

    /// 直接扣除指定金额（用于非流式场景或最终结算）
    pub async fn deduct(&mut self, amount_micro: i64) -> Result<BudgetStatus, String> {
        self.total_cost_micro += amount_micro as u64;
        self.check_budget(amount_micro).await
    }

    /// 执行 Redis Lua 脚本扣减
    async fn check_budget(&self, amount_micro: i64) -> Result<BudgetStatus, String> {
        let redis = match &self.redis {
            Some(r) => r.clone(),
            None => {
                // Redis 不可用，降级到内存模式（允许通过但不持久化扣减）
                warn!(
                    "Redis not available for budget check, running in degraded mode. ws={}, cost={}",
                    self.workspace_id, amount_micro
                );
                return Ok(BudgetStatus::Degraded);
            }
        };

        let budget_key = format!("workspace:{}:budget", self.workspace_id);
        let daily_key = format!("workspace:{}:budget:daily", self.workspace_id);

        // 加载并执行 Lua 脚本
        let script = redis::Script::new(include_str!("../../../scripts/redis/budget_decr.lua"));

        let result: Vec<String> = script
            .key(budget_key)
            .key(daily_key)
            .arg(amount_micro)
            .arg(self.daily_limit_micro)
            .arg(&self.request_id)
            .invoke_async(&mut redis.clone())
            .await
            .map_err(|e| format!("Redis budget script error: {}", e))?;

        let status: i32 = result
            .get(0)
            .and_then(|s| s.parse().ok())
            .unwrap_or(-1);
        let reason = result.get(1).cloned().unwrap_or_default();
        let remaining: i64 = result
            .get(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        match status {
            1 => {
                info!(
                    "Budget deducted: ws={}, amount={}, remaining={}, total_tokens={}",
                    self.workspace_id, amount_micro, remaining, self.total_tokens
                );
                Ok(BudgetStatus::Ok { remaining_micro: remaining })
            }
            0 => {
                warn!(
                    "Budget EXCEEDED: ws={}, reason={}, remaining={}",
                    self.workspace_id, reason, remaining
                );
                Ok(BudgetStatus::Exceeded {
                    reason,
                    remaining_micro: remaining,
                })
            }
            _ => Ok(BudgetStatus::Error(format!("Unknown Redis response: {:?}", result))),
        }
    }

    /// 获取统计摘要
    pub fn summary(&self) -> BudgetSummary {
        BudgetSummary {
            workspace_id: self.workspace_id.clone(),
            total_tokens: self.total_tokens,
            total_cost_micro: self.total_cost_micro,
            elapsed_ms: self.started_at.elapsed().as_millis() as u64,
            request_id: self.request_id.clone(),
        }
    }
}

/// 预算统计摘要
#[derive(Debug, Clone)]
pub struct BudgetSummary {
    pub workspace_id: String,
    pub total_tokens: u64,
    pub total_cost_micro: u64,
    pub elapsed_ms: u64,
    pub request_id: String,
}

impl BudgetSummary {
    /// 总成本（美元）
    pub fn cost_usd(&self) -> f64 {
        self.total_cost_micro as f64 / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_status_comparison() {
        let ok = BudgetStatus::Ok { remaining_micro: 100 };
        let exceeded = BudgetStatus::Exceeded { reason: "test".into(), remaining_micro: 0 };
        assert_ne!(ok, BudgetStatus::Exceeded { reason: "test".into(), remaining_micro: 0 });
        assert_eq!(ok, BudgetStatus::Ok { remaining_micro: 100 });
        assert_eq!(exceeded, exceeded);
    }

    #[test]
    fn test_budget_summary_dollar_conversion() {
        let s = BudgetSummary {
            workspace_id: "ws1".into(),
            total_tokens: 1000,
            total_cost_micro: 1_500_000, // $1.50
            elapsed_ms: 500,
            request_id: "req1".into(),
        };
        assert!((s.cost_usd() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_guard_without_redis() {
        // 无 Redis 连接时自动降级
        let guard = StreamBudgetGuard::new(
            None,
            "ws-test".into(),
            1, // $0.000001 per token
            0,
            "req-test".into(),
        );
        assert_eq!(guard.check_interval, 10);
        assert_eq!(guard.cost_per_token_micro, 1);
    }
}
