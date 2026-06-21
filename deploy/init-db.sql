-- =============================================================================
-- VERIDACTUS PostgreSQL 初始化脚本
-- 创建 traces 表和索引
--
-- Project: VERIDACTUS - Trusted AI Execution Governance
-- License: Apache-2.0
-- =============================================================================

-- 创建 traces 表（如果不存在）
CREATE TABLE IF NOT EXISTS traces (
    trace_id UUID PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    session_id UUID,
    trace_data JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP WITH TIME ZONE
);

-- 创建索引以优化查询性能
CREATE INDEX IF NOT EXISTS idx_traces_tenant ON traces(tenant_id);
CREATE INDEX IF NOT EXISTS idx_traces_timestamp ON traces(created_at);
CREATE INDEX IF NOT EXISTS idx_traces_session ON traces(session_id);

-- 创建清理过期 traces 的函数
CREATE OR REPLACE FUNCTION cleanup_expired_traces() RETURNS void AS $$
BEGIN
    DELETE FROM traces WHERE expires_at < CURRENT_TIMESTAMP;
END;
$$ LANGUAGE plpgsql;

-- 创建清理过期 traces 的定时任务扩展（需要 pg_cron 扩展）
-- 注意：pg_cron 需要在 PostgreSQL 配置中启用
-- SELECT cron.schedule('cleanup_traces', '0 0 * * *', 'SELECT cleanup_expired_traces()');

-- 创建审计日志表（可选）
CREATE TABLE IF NOT EXISTS audit_logs (
    id SERIAL PRIMARY KEY,
    action VARCHAR(64) NOT NULL,
    actor VARCHAR(128),
    resource_type VARCHAR(64),
    resource_id VARCHAR(128),
    details JSONB,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建审计日志索引
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(created_at);

-- 创建预算跟踪表（可选）
CREATE TABLE IF NOT EXISTS budget_usage (
    id SERIAL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    model VARCHAR(128),
    cost_usd DECIMAL(10, 6) NOT NULL DEFAULT 0,
    request_count INTEGER NOT NULL DEFAULT 0,
    period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建预算索引
CREATE INDEX IF NOT EXISTS idx_budget_tenant ON budget_usage(tenant_id);
CREATE INDEX IF NOT EXISTS idx_budget_period ON budget_usage(period_start, period_end);

-- 授权 veridactus 用户访问所有表
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO veridactus;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO veridactus;

-- 初始化完成提示
DO $$
BEGIN
    RAISE NOTICE 'VERIDACTUS PostgreSQL schema initialized successfully';
END
$$;