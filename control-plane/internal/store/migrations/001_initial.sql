-- VERIDACTUS Phase 1: 多租户基础设施迁移
-- 从 v0.2.1 单租户 SQLite → v1.0 多租户 PostgreSQL/SQLite

-- ============================================
-- 组织表
-- ============================================
CREATE TABLE IF NOT EXISTS organizations (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL UNIQUE,
    plan        TEXT NOT NULL DEFAULT 'free',
    logo_url    TEXT,
    primary_color TEXT DEFAULT '#6c5ce7',
    settings    TEXT DEFAULT '{}',
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================
-- 工作空间表
-- ============================================
CREATE TABLE IF NOT EXISTS workspaces (
    id          TEXT PRIMARY KEY,
    org_id      TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL,
    description TEXT,
    settings    TEXT DEFAULT '{}',
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(org_id, slug)
);

-- ============================================
-- 用户表
-- ============================================
CREATE TABLE IF NOT EXISTS users (
    id              TEXT PRIMARY KEY,
    email           TEXT NOT NULL UNIQUE,
    display_name    TEXT,
    avatar_url      TEXT,
    auth_provider   TEXT NOT NULL,
    auth_provider_id TEXT,
    password_hash   TEXT,
    settings        TEXT DEFAULT '{}',
    last_login_at   TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================
-- 工作空间成员表
-- ============================================
CREATE TABLE IF NOT EXISTS workspace_members (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role         TEXT NOT NULL DEFAULT 'developer',
    invited_by   TEXT REFERENCES users(id),
    invited_at   TIMESTAMP,
    joined_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(workspace_id, user_id)
);

-- ============================================
-- 刷新令牌表
-- ============================================
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMP NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user ON refresh_tokens(user_id);

-- ============================================
-- 虚拟密钥表
-- ============================================
CREATE TABLE IF NOT EXISTS virtual_keys (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    key_prefix      TEXT NOT NULL,
    key_hash        TEXT NOT NULL UNIQUE,
    type            TEXT NOT NULL DEFAULT 'platform',
    provider_key_encrypted TEXT,
    provider_key_kms_id    TEXT,
    allowed_models  TEXT DEFAULT '[]',
    rate_limit_rpm  INTEGER DEFAULT 60,
    rate_limit_tpm  INTEGER DEFAULT 100000,
    spend_limit_usd_micro INTEGER DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'active',
    last_used_at    TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT NOT NULL REFERENCES users(id)
);

-- ============================================
-- 钱包表
-- ============================================
CREATE TABLE IF NOT EXISTS wallets (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL UNIQUE REFERENCES workspaces(id),
    balance_usd_micro       INTEGER NOT NULL DEFAULT 0,
    overdraft_limit_micro   INTEGER NOT NULL DEFAULT 0,
    last_credit_at  TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================
-- 交易记录表
-- ============================================
CREATE TABLE IF NOT EXISTS transactions (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id),
    wallet_id       TEXT NOT NULL REFERENCES wallets(id),
    type            TEXT NOT NULL,
    amount_usd_micro INTEGER NOT NULL,
    balance_after_micro INTEGER NOT NULL,
    description     TEXT,
    trace_id        TEXT,
    metadata        TEXT DEFAULT '{}',
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_transactions_workspace ON transactions(workspace_id, created_at DESC);

-- ============================================
-- 流水线 (多租户增强)
-- ============================================
CREATE TABLE IF NOT EXISTS pipelines (
    plan_id      TEXT PRIMARY KEY,
    org_id       TEXT REFERENCES organizations(id),
    workspace_id TEXT REFERENCES workspaces(id),
    name         TEXT NOT NULL DEFAULT '',
    description  TEXT NOT NULL DEFAULT '',
    tenant       TEXT NOT NULL,
    stages       TEXT NOT NULL,
    created_at   TEXT NOT NULL
);

-- ============================================
-- 插件 (多租户增强)
-- ============================================
CREATE TABLE IF NOT EXISTS plugins (
    id          TEXT PRIMARY KEY,
    org_id      TEXT,
    workspace_id TEXT,
    name        TEXT NOT NULL,
    type        TEXT NOT NULL,
    version     TEXT,
    description TEXT,
    config      TEXT DEFAULT '{}'
);

-- ============================================
-- 策略表
-- ============================================
CREATE TABLE IF NOT EXISTS policies (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    type        TEXT NOT NULL,
    content     TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

-- ============================================
-- API 密钥 (多租户增强)
-- ============================================
CREATE TABLE IF NOT EXISTS apikeys (
    id          TEXT PRIMARY KEY,
    org_id      TEXT,
    workspace_id TEXT,
    name        TEXT NOT NULL,
    key         TEXT NOT NULL UNIQUE,
    tenant_id   TEXT NOT NULL,
    status      TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    last_used   TEXT
);

-- ============================================
-- 模型配置 (多租户增强)
-- ============================================
CREATE TABLE IF NOT EXISTS models (
    id                TEXT PRIMARY KEY,
    org_id            TEXT,
    workspace_id      TEXT,
    name              TEXT NOT NULL UNIQUE,
    upstream_url      TEXT NOT NULL,
    upstream_model    TEXT NOT NULL,
    api_key           TEXT,
    api_key_header    TEXT DEFAULT 'Authorization',
    use_proxy         INTEGER NOT NULL DEFAULT 0,
    proxy_url         TEXT,
    is_default        INTEGER NOT NULL DEFAULT 0,
    supported_versions TEXT,
    status            TEXT NOT NULL DEFAULT 'active'
);

-- ============================================
-- Trace 引用表
-- ============================================
CREATE TABLE IF NOT EXISTS traces (
    trace_id        TEXT PRIMARY KEY,
    model           TEXT NOT NULL,
    tenant_id       TEXT NOT NULL,
    execution_state TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    signature       TEXT
);

-- ============================================
-- 配置版本表
-- ============================================
CREATE TABLE IF NOT EXISTS config_versions (
    key   TEXT PRIMARY KEY,
    value INTEGER NOT NULL DEFAULT 0
);

-- ============================================
-- 数据面存储配置表
-- ============================================
CREATE TABLE IF NOT EXISTS data_plane_configs (
    id         TEXT PRIMARY KEY,
    key        TEXT NOT NULL,
    value      TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================
-- 系统设置表
-- ============================================
CREATE TABLE IF NOT EXISTS settings (
    key          TEXT NOT NULL,
    workspace_id TEXT NOT NULL DEFAULT 'default',
    value        TEXT NOT NULL,
    PRIMARY KEY (key, workspace_id)
);
