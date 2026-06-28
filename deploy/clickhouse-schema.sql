-- VERIDACTUS ClickHouse OLAP Schema (Phase P2)
-- 用于 FinOps 报表秒级聚合 + 审计事件分析

CREATE DATABASE IF NOT EXISTS veridactus;

-- ============================================
-- 审计事件表 (按月份分区)
-- ============================================
CREATE TABLE IF NOT EXISTS veridactus.audit_events (
    event_id        UUID,
    org_id          String,
    workspace_id    String,
    event_type      LowCardinality(String),   -- pii_detected|injection_blocked|budget_exceeded|guardrail_triggered|l0_verified
    severity        LowCardinality(String),   -- low|medium|high|critical
    trace_id        String,
    user_id         String,
    model           String,
    cost_usd_micro  Int64,
    tokens_count    Int32,
    latency_ms      Int32,
    asi_risk_id     LowCardinality(String),   -- ASI01-ASI10
    metadata        String,                    -- JSON
    created_at      DateTime64(3)
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (org_id, workspace_id, event_type, created_at)
TTL created_at + INTERVAL 365 DAY;

-- ============================================
-- Trace 聚合表 (用于 FinOps 报表)
-- ============================================
CREATE TABLE IF NOT EXISTS veridactus.traces_agg (
    trace_id        String,
    org_id          String,
    workspace_id    String,
    user_id         String,
    model           String,
    provider        LowCardinality(String),
    tokens_count    Int32,
    cost_usd_micro  Int64,
    latency_ms      Int32,
    execution_state LowCardinality(String),
    safety_status   LowCardinality(String),  -- safe|flagged|blocked
    proof_levels    Array(String),           -- [L0, L2A, L2B]
    created_at      DateTime64(3)
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (org_id, workspace_id, created_at)
TTL created_at + INTERVAL 730 DAY;

-- ============================================
-- 预算消耗物化视图 (每小时聚合)
-- ============================================
CREATE MATERIALIZED VIEW IF NOT EXISTS veridactus.budget_hourly_mv
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(hour)
ORDER BY (org_id, workspace_id, hour)
AS SELECT
    org_id,
    workspace_id,
    toStartOfHour(created_at) AS hour,
    sum(cost_usd_micro) AS total_cost_micro,
    sum(tokens_count) AS total_tokens,
    count() AS request_count
FROM veridactus.traces_agg
GROUP BY org_id, workspace_id, hour;

-- ============================================
-- 安全事件物化视图 (按天聚合)
-- ============================================
CREATE MATERIALIZED VIEW IF NOT EXISTS veridactus.safety_daily_mv
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(day)
ORDER BY (org_id, workspace_id, event_type, day)
AS SELECT
    org_id,
    workspace_id,
    event_type,
    toStartOfDay(created_at) AS day,
    count() AS event_count
FROM veridactus.audit_events
GROUP BY org_id, workspace_id, event_type, day;
