// VERIDACTUS 控制平面 — SQLite 存储实现
// 适配 StoreFacade 接口，保留原有单机部署能力
package store

import (
	"context"
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log"
	"os"

	_ "github.com/mattn/go-sqlite3"
	"github.com/google/uuid"
	"github.com/veridactus/control-plane/internal/model"
)

// SQLiteStore SQLite 存储实现
type SQLiteStore struct {
	db *sql.DB
}

// NewSQLiteStore 创建 SQLite 存储
func NewSQLiteStore(cfg StoreConfig) (StoreFacade, error) {
	dbPath := cfg.DBPath
	if dbPath == "" {
		dbPath = "./veridactus.db"
	}

	db, err := sql.Open("sqlite3", dbPath)
	if err != nil {
		return nil, fmt.Errorf("open sqlite: %w", err)
	}

	// WAL 模式 + 外键约束
	if _, err := db.Exec("PRAGMA journal_mode=WAL;"); err != nil {
		return nil, fmt.Errorf("enable WAL: %w", err)
	}
	if _, err := db.Exec("PRAGMA foreign_keys=ON;"); err != nil {
		return nil, fmt.Errorf("enable foreign keys: %w", err)
	}

	store := &SQLiteStore{db: db}

	// 运行迁移
	if err := store.RunMigrations(context.Background()); err != nil {
		return nil, fmt.Errorf("run migrations: %w", err)
	}

	// 初始化种子数据
	if err := store.seedDefaultData(context.Background()); err != nil {
		log.Printf("WARN: seed data init: %v", err)
	}

	return store, nil
}

// ==================== 数据库管理 ====================

func (s *SQLiteStore) RunMigrations(ctx context.Context) error {
	// 使用嵌入的 SQL 迁移
	migration, err := os.ReadFile("internal/store/migrations/001_initial.sql")
	if err != nil {
		// 如果找不到文件，使用内联迁移（适用于编译后的二进制）
		if err := s.runInlineMigration(ctx); err != nil { return err }
	} else {
		if err := s.execSQLStatements(ctx, string(migration)); err != nil { return err }
	}
	// 幂等迁移: 添加 phone 列（ignore error if already exists）
	s.db.ExecContext(ctx, "ALTER TABLE users ADD COLUMN phone TEXT")
	return nil
}

func (s *SQLiteStore) runInlineMigration(ctx context.Context) error {
	queries := []string{
		// 组织
		`CREATE TABLE IF NOT EXISTS organizations (id TEXT PRIMARY KEY, name TEXT NOT NULL, slug TEXT NOT NULL UNIQUE, plan TEXT NOT NULL DEFAULT 'free', logo_url TEXT, primary_color TEXT DEFAULT '#6c5ce7', settings TEXT DEFAULT '{}', created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')));`,
		// 工作空间
		`CREATE TABLE IF NOT EXISTS workspaces (id TEXT PRIMARY KEY, org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE, name TEXT NOT NULL, slug TEXT NOT NULL, description TEXT, settings TEXT DEFAULT '{}', created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')), UNIQUE(org_id, slug));`,
		// 用户
		`CREATE TABLE IF NOT EXISTS users (
		phone TEXT,id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, display_name TEXT, avatar_url TEXT, auth_provider TEXT NOT NULL, auth_provider_id TEXT, password_hash TEXT, settings TEXT DEFAULT '{}', last_login_at TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')));`,
		// 迁移: 添加 phone 列 (ignore error if column exists)
		// 成员
		`CREATE TABLE IF NOT EXISTS workspace_members (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, role TEXT NOT NULL DEFAULT 'developer', invited_by TEXT REFERENCES users(id), invited_at TEXT, joined_at TEXT NOT NULL DEFAULT (datetime('now')), UNIQUE(workspace_id, user_id));`,
		// 刷新令牌
		`CREATE TABLE IF NOT EXISTS refresh_tokens (id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, token_hash TEXT NOT NULL UNIQUE, expires_at TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')));`,
		// 虚拟密钥
		`CREATE TABLE IF NOT EXISTS virtual_keys (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE, name TEXT NOT NULL, key_prefix TEXT NOT NULL, key_hash TEXT NOT NULL UNIQUE, type TEXT NOT NULL DEFAULT 'platform', provider_key_encrypted TEXT, allowed_models TEXT DEFAULT '[]', rate_limit_rpm INTEGER DEFAULT 60, rate_limit_tpm INTEGER DEFAULT 100000, spend_limit_usd_micro INTEGER DEFAULT 0, status TEXT NOT NULL DEFAULT 'active', last_used_at TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), created_by TEXT NOT NULL REFERENCES users(id));`,
		// 钱包
		`CREATE TABLE IF NOT EXISTS wallets (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL UNIQUE REFERENCES workspaces(id), balance_usd_micro INTEGER NOT NULL DEFAULT 0, overdraft_limit_micro INTEGER NOT NULL DEFAULT 0, last_credit_at TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')));`,
		// 交易
		`CREATE TABLE IF NOT EXISTS transactions (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id), wallet_id TEXT NOT NULL REFERENCES wallets(id), type TEXT NOT NULL, amount_usd_micro INTEGER NOT NULL, balance_after_micro INTEGER NOT NULL, description TEXT, trace_id TEXT, metadata TEXT DEFAULT '{}', created_at TEXT NOT NULL DEFAULT (datetime('now')));`,
		// 流水线
		`CREATE TABLE IF NOT EXISTS pipelines (plan_id TEXT PRIMARY KEY, org_id TEXT, workspace_id TEXT, name TEXT NOT NULL DEFAULT '', description TEXT NOT NULL DEFAULT '', tenant TEXT NOT NULL, stages TEXT NOT NULL, created_at TEXT NOT NULL);`,
		// 插件
		`CREATE TABLE IF NOT EXISTS plugins (id TEXT PRIMARY KEY, org_id TEXT, workspace_id TEXT, name TEXT NOT NULL, type TEXT NOT NULL, version TEXT, description TEXT, config TEXT DEFAULT '{}');`,
		// 策略
		`CREATE TABLE IF NOT EXISTS policies (id TEXT PRIMARY KEY, name TEXT NOT NULL, type TEXT NOT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL);`,
		// API Key
		`CREATE TABLE IF NOT EXISTS apikeys (id TEXT PRIMARY KEY, org_id TEXT, workspace_id TEXT, name TEXT NOT NULL, key TEXT NOT NULL UNIQUE, tenant_id TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, last_used TEXT);`,
		// 模型
		`CREATE TABLE IF NOT EXISTS models (id TEXT PRIMARY KEY, org_id TEXT, workspace_id TEXT, name TEXT NOT NULL UNIQUE, upstream_url TEXT NOT NULL, upstream_model TEXT NOT NULL, api_key TEXT, api_key_header TEXT DEFAULT 'Authorization', use_proxy INTEGER NOT NULL DEFAULT 0, proxy_url TEXT, is_default INTEGER NOT NULL DEFAULT 0, supported_versions TEXT, status TEXT NOT NULL DEFAULT 'active');`,
		// Trace
		`CREATE TABLE IF NOT EXISTS traces (trace_id TEXT PRIMARY KEY, model TEXT NOT NULL, tenant_id TEXT NOT NULL, execution_state TEXT NOT NULL, created_at TEXT NOT NULL, signature TEXT);`,
		// 配置版本
		`CREATE TABLE IF NOT EXISTS config_versions (key TEXT PRIMARY KEY, value INTEGER NOT NULL DEFAULT 0);`,
		// 数据面配置
		`CREATE TABLE IF NOT EXISTS data_plane_configs (id TEXT PRIMARY KEY, key TEXT NOT NULL, value TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')));`,
		// 设置
		`CREATE TABLE IF NOT EXISTS settings (key TEXT NOT NULL, workspace_id TEXT NOT NULL DEFAULT 'default', value TEXT NOT NULL, PRIMARY KEY (key, workspace_id));`,
		// 种子版本号
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('pipeline_version', 0);`,
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('policy_version', 0);`,
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('plugin_version', 0);`,
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('storage_version', 0);`,
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('model_version', 0);`,
	}
	return s.execSQLStatements(ctx, queries...)
}

func (s *SQLiteStore) execSQLStatements(ctx context.Context, statements ...string) error {
	for _, stmt := range statements {
		if _, err := s.db.ExecContext(ctx, stmt); err != nil {
			return fmt.Errorf("exec: %s: %w", stmt[:min(len(stmt), 60)], err)
		}
	}
	return nil
}

func (s *SQLiteStore) HealthCheck(ctx context.Context) error {
	var count int
	return s.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM config_versions").Scan(&count)
}

func (s *SQLiteStore) Close() error {
	return s.db.Close()
}

// ==================== 种子数据 ====================

func (s *SQLiteStore) seedDefaultData(ctx context.Context) error {
	var count int
	s.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM apikeys").Scan(&count)
	if count > 0 {
		return nil
	}

	// 创建默认组织 + 工作空间 + 用户 + 钱包
	orgID := uuid.New().String()
	wsID := uuid.New().String()
	adminUserID := uuid.New().String()
	walletID := uuid.New().String()

	s.db.ExecContext(ctx, `INSERT INTO organizations (id, name, slug, plan) VALUES (?, 'Default Organization', 'default', 'free')`, orgID)
	s.db.ExecContext(ctx, `INSERT INTO workspaces (id, org_id, name, slug) VALUES (?, ?, 'Default Workspace', 'default')`, wsID, orgID)

	// 创建默认管理员用户（供 Virtual Key 等需要 created_by 的操作使用）
	s.db.ExecContext(ctx, `INSERT INTO users (id, email, display_name, auth_provider, auth_provider_id) VALUES (?, 'admin@veridactus.local', 'Platform Admin', 'local', 'admin')`, adminUserID)

	// 添加管理员为 workspace 成员
	memberID := uuid.New().String()
	s.db.ExecContext(ctx, `INSERT INTO workspace_members (id, workspace_id, user_id, role) VALUES (?, ?, ?, 'platform_admin')`, memberID, wsID, adminUserID)

	// 创建钱包
	s.db.ExecContext(ctx, `INSERT INTO wallets (id, workspace_id, balance_usd_micro, overdraft_limit_micro) VALUES (?, ?, 10000000, 0)`, walletID, wsID)

	// 创建默认 API Keys
	randomKey := func() string {
		b := make([]byte, 32)
		rand.Read(b)
		return "vd-" + hex.EncodeToString(b)[:32]
	}
	defaultKeys := []struct{ name, status string }{
		{"Production API Key", "active"},
		{"Staging API Key", "active"},
		{"CI/CD Pipeline Key", "rotated"},
	}
	for _, dk := range defaultKeys {
		k := uuid.New().String()
		key := randomKey()
		s.db.ExecContext(ctx,
			`INSERT INTO apikeys (id, org_id, workspace_id, name, key, tenant_id, status, created_at) VALUES (?,?,?,?,?,?,?,datetime('now'))`,
			k, orgID, wsID, dk.name, key, "acme-corp", dk.status,
		)
	}

	// 初始化默认插件
	plugins := []struct{ name, ptype, version, desc, config string }{
		{"PII Detector", "native", "0.2.1", "PII检测与脱敏插件", `{"enabled":true,"action_on_detect":"mask"}`},
		{"Budget Guard", "native", "1.0.0", "预算守卫插件", `{"limit_usd":10.0,"window":"daily"}`},
		{"Auth Validator", "native", "1.0.0", "认证验证器", `{}`},
		{"Trace Finalizer", "native", "1.0.0", "Trace终结器", `{}`},
	}
	for _, p := range plugins {
		s.db.ExecContext(ctx,
			`INSERT INTO plugins (id, org_id, workspace_id, name, type, version, description, config) VALUES (?,?,?,?,?,?,?,?)`,
			uuid.New().String(), orgID, wsID, p.name, p.ptype, p.version, p.desc, p.config,
		)
	}

	log.Println("SQLite: default data seeded")
	return nil
}

// ==================== 组织 ====================

func (s *SQLiteStore) CreateOrganization(ctx context.Context, org *model.Organization) error {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO organizations (id, name, slug, plan, logo_url, primary_color, settings) VALUES (?,?,?,?,?,?,?)`,
		org.ID, org.Name, org.Slug, org.Plan, org.LogoURL, org.PrimaryColor, org.Settings)
	return err
}

func (s *SQLiteStore) GetOrganization(ctx context.Context, id string) (*model.Organization, error) {
	var org model.Organization
	var createdAt, updatedAt string
	err := s.db.QueryRowContext(ctx,
		`SELECT id, name, slug, plan, COALESCE(logo_url,''), COALESCE(primary_color,'#6c5ce7'), COALESCE(settings,'{}'), created_at, updated_at FROM organizations WHERE id=?`, id,
	).Scan(&org.ID, &org.Name, &org.Slug, &org.Plan, &org.LogoURL, &org.PrimaryColor, &org.Settings, &createdAt, &updatedAt)
	if err != nil {
		return nil, err
	}
	return &org, nil
}

func (s *SQLiteStore) GetOrganizationBySlug(ctx context.Context, slug string) (*model.Organization, error) {
	var org model.Organization
	err := s.db.QueryRowContext(ctx,
		`SELECT id, name, slug, plan, COALESCE(logo_url,''), COALESCE(primary_color,'#6c5ce7'), COALESCE(settings,'{}'), created_at, updated_at FROM organizations WHERE slug=?`, slug,
	).Scan(&org.ID, &org.Name, &org.Slug, &org.Plan, &org.LogoURL, &org.PrimaryColor, &org.Settings, &org.CreatedAt, &org.UpdatedAt)
	if err != nil {
		return nil, err
	}
	return &org, nil
}

func (s *SQLiteStore) ListOrganizations(ctx context.Context) ([]model.Organization, error) {
	rows, err := s.db.QueryContext(ctx, `SELECT id, name, slug, plan, COALESCE(logo_url,''), COALESCE(primary_color,'#6c5ce7'), COALESCE(settings,'{}'), created_at, updated_at FROM organizations ORDER BY created_at`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var orgs []model.Organization
	for rows.Next() {
		var org model.Organization
		if err := rows.Scan(&org.ID, &org.Name, &org.Slug, &org.Plan, &org.LogoURL, &org.PrimaryColor, &org.Settings, &org.CreatedAt, &org.UpdatedAt); err != nil {
			return nil, err
		}
		orgs = append(orgs, org)
	}
	return orgs, nil
}

func (s *SQLiteStore) ListOrganizationsByUser(ctx context.Context, userID string) ([]model.Organization, error) {
	rows, err := s.db.QueryContext(ctx,
		`SELECT DISTINCT o.id, o.name, o.slug, o.plan, COALESCE(o.logo_url,''), COALESCE(o.primary_color,'#6c5ce7'), COALESCE(o.settings,'{}'), o.created_at, o.updated_at
		 FROM organizations o
		 JOIN workspaces w ON w.org_id = o.id
		 JOIN workspace_members wm ON wm.workspace_id = w.id
		 WHERE wm.user_id = ?
		 ORDER BY o.created_at`, userID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var orgs []model.Organization
	for rows.Next() {
		var org model.Organization
		if err := rows.Scan(&org.ID, &org.Name, &org.Slug, &org.Plan, &org.LogoURL, &org.PrimaryColor, &org.Settings, &org.CreatedAt, &org.UpdatedAt); err != nil {
			return nil, err
		}
		orgs = append(orgs, org)
	}
	return orgs, nil
}

func (s *SQLiteStore) UpdateOrganization(ctx context.Context, id string, updates map[string]interface{}) error {
	setClause, args := buildSetClause(updates, 2)
	if setClause == "" {
		return nil
	}
	args = append([]interface{}{"NOW()"}, args...)
	args = append(args, id)
	_, err := s.db.ExecContext(ctx,
		`UPDATE organizations SET updated_at=?, `+setClause+` WHERE id=?`, args...)
	return err
}

func (s *SQLiteStore) DeleteOrganization(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM organizations WHERE id=?`, id)
	return err
}

// ==================== 工作空间 ====================

func (s *SQLiteStore) CreateWorkspace(ctx context.Context, ws *model.Workspace) error {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO workspaces (id, org_id, name, slug, description, settings) VALUES (?,?,?,?,?,?)`,
		ws.ID, ws.OrgID, ws.Name, ws.Slug, ws.Description, ws.Settings)
	if err != nil {
		return err
	}
	// 自动创建钱包
	walletID := uuid.New().String()
	s.db.ExecContext(ctx, `INSERT INTO wallets (id, workspace_id) VALUES (?,?)`, walletID, ws.ID)
	return nil
}

func (s *SQLiteStore) GetWorkspace(ctx context.Context, id string) (*model.Workspace, error) {
	var ws model.Workspace
	err := s.db.QueryRowContext(ctx,
		`SELECT id, org_id, name, slug, COALESCE(description,''), COALESCE(settings,'{}'), created_at, updated_at FROM workspaces WHERE id=?`, id,
	).Scan(&ws.ID, &ws.OrgID, &ws.Name, &ws.Slug, &ws.Description, &ws.Settings, &ws.CreatedAt, &ws.UpdatedAt)
	if err != nil {
		return nil, err
	}
	return &ws, nil
}

func (s *SQLiteStore) GetWorkspaceBySlug(ctx context.Context, orgID, slug string) (*model.Workspace, error) {
	var ws model.Workspace
	err := s.db.QueryRowContext(ctx,
		`SELECT id, org_id, name, slug, COALESCE(description,''), COALESCE(settings,'{}'), created_at, updated_at FROM workspaces WHERE org_id=? AND slug=?`, orgID, slug,
	).Scan(&ws.ID, &ws.OrgID, &ws.Name, &ws.Slug, &ws.Description, &ws.Settings, &ws.CreatedAt, &ws.UpdatedAt)
	if err != nil {
		return nil, err
	}
	return &ws, nil
}

func (s *SQLiteStore) ListWorkspaces(ctx context.Context, orgID string) ([]model.Workspace, error) {
	rows, err := s.db.QueryContext(ctx,
		`SELECT id, org_id, name, slug, COALESCE(description,''), COALESCE(settings,'{}'), created_at, updated_at FROM workspaces WHERE org_id=? ORDER BY created_at`, orgID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var wss []model.Workspace
	for rows.Next() {
		var ws model.Workspace
		if err := rows.Scan(&ws.ID, &ws.OrgID, &ws.Name, &ws.Slug, &ws.Description, &ws.Settings, &ws.CreatedAt, &ws.UpdatedAt); err != nil {
			return nil, err
		}
		wss = append(wss, ws)
	}
	return wss, nil
}

func (s *SQLiteStore) UpdateWorkspace(ctx context.Context, id string, updates map[string]interface{}) error {
	setClause, args := buildSetClause(updates, 2)
	if setClause == "" {
		return nil
	}
	args = append([]interface{}{"NOW()"}, args...)
	args = append(args, id)
	_, err := s.db.ExecContext(ctx,
		`UPDATE workspaces SET updated_at=?, `+setClause+` WHERE id=?`, args...)
	return err
}

func (s *SQLiteStore) DeleteWorkspace(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM workspaces WHERE id=?`, id)
	return err
}

// ==================== 用户 ====================

func (s *SQLiteStore) CreateUser(ctx context.Context, user *model.User) error {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO users (id, email, display_name, avatar_url, auth_provider, auth_provider_id, password_hash, settings) VALUES (?,?,?,?,?,?,?,?)`,
		user.ID, user.Email, user.DisplayName, user.AvatarURL, user.AuthProvider, user.AuthProviderID, user.PasswordHash, user.Settings)
	return err
}

func (s *SQLiteStore) GetUser(ctx context.Context, id string) (*model.User, error) {
	var user model.User
	var lastLogin sql.NullString
	err := s.db.QueryRowContext(ctx,
		`SELECT id, email, COALESCE(display_name,''), COALESCE(avatar_url,''), auth_provider, COALESCE(auth_provider_id,''), COALESCE(password_hash,''), COALESCE(settings,'{}'), last_login_at, created_at, updated_at FROM users WHERE id=?`, id,
	).Scan(&user.ID, &user.Email, &user.DisplayName, &user.AvatarURL, &user.AuthProvider, &user.AuthProviderID, &user.PasswordHash, &user.Settings, &lastLogin, &user.CreatedAt, &user.UpdatedAt)
	if err != nil {
		return nil, err
	}
	if lastLogin.Valid {
		user.LastLoginAt = lastLogin.String
	}
	return &user, nil
}

func (s *SQLiteStore) GetUserByEmail(ctx context.Context, email string) (*model.User, error) {
	var user model.User
	var lastLogin sql.NullString
	err := s.db.QueryRowContext(ctx,
		`SELECT id, email, COALESCE(phone,''), COALESCE(display_name,''), COALESCE(avatar_url,''), auth_provider, COALESCE(auth_provider_id,''), COALESCE(password_hash,''), COALESCE(settings,'{}'), last_login_at, created_at, updated_at FROM users WHERE email=?`, email,
	).Scan(&user.ID, &user.Email, &user.Phone, &user.DisplayName, &user.AvatarURL, &user.AuthProvider, &user.AuthProviderID, &user.PasswordHash, &user.Settings, &lastLogin, &user.CreatedAt, &user.UpdatedAt)
	if err != nil {
		return nil, err
	}
	if lastLogin.Valid { user.LastLoginAt = lastLogin.String }
	return &user, nil
}

func (s *SQLiteStore) GetUserByPhone(ctx context.Context, phone string) (*model.User, error) {
	var user model.User
	var lastLogin sql.NullString
	err := s.db.QueryRowContext(ctx,
		`SELECT id, email, COALESCE(phone,''), COALESCE(display_name,''), COALESCE(avatar_url,''), auth_provider, COALESCE(auth_provider_id,''), COALESCE(password_hash,''), COALESCE(settings,'{}'), last_login_at, created_at, updated_at FROM users WHERE phone=? OR auth_provider_id=?`, phone, phone,
	).Scan(&user.ID, &user.Email, &user.Phone, &user.DisplayName, &user.AvatarURL, &user.AuthProvider, &user.AuthProviderID, &user.PasswordHash, &user.Settings, &lastLogin, &user.CreatedAt, &user.UpdatedAt)
	if err != nil { return nil, err }
	if lastLogin.Valid { user.LastLoginAt = lastLogin.String }
	return &user, nil
}

func (s *SQLiteStore) GetUserByProvider(ctx context.Context, provider, providerID string) (*model.User, error) {
	var user model.User
	var lastLogin sql.NullString
	err := s.db.QueryRowContext(ctx,
		`SELECT id, email, COALESCE(display_name,''), COALESCE(avatar_url,''), auth_provider, COALESCE(auth_provider_id,''), COALESCE(password_hash,''), COALESCE(settings,'{}'), last_login_at, created_at, updated_at FROM users WHERE auth_provider=? AND auth_provider_id=?`,
		provider, providerID,
	).Scan(&user.ID, &user.Email, &user.DisplayName, &user.AvatarURL, &user.AuthProvider, &user.AuthProviderID, &user.PasswordHash, &user.Settings, &lastLogin, &user.CreatedAt, &user.UpdatedAt)
	if err != nil {
		return nil, err
	}
	if lastLogin.Valid {
		user.LastLoginAt = lastLogin.String
	}
	return &user, nil
}

func (s *SQLiteStore) UpdateUser(ctx context.Context, id string, updates map[string]interface{}) error {
	setClause, args := buildSetClause(updates, 2)
	if setClause == "" {
		return nil
	}
	args = append([]interface{}{"NOW()"}, args...)
	args = append(args, id)
	_, err := s.db.ExecContext(ctx,
		`UPDATE users SET updated_at=?, `+setClause+` WHERE id=?`, args...)
	return err
}

func (s *SQLiteStore) DeleteUser(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM users WHERE id=?`, id)
	return err
}

// ==================== 成员 ====================

func (s *SQLiteStore) AddMember(ctx context.Context, member *model.WorkspaceMember) error {
	var invitedBy interface{}
	if member.InvitedBy != "" {
		invitedBy = member.InvitedBy
	}
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO workspace_members (id, workspace_id, user_id, role, invited_by, invited_at, joined_at) VALUES (?,?,?,?,?,?,datetime('now'))`,
		member.ID, member.WorkspaceID, member.UserID, member.Role, invitedBy, member.InvitedAt)
	return err
}

func (s *SQLiteStore) GetMember(ctx context.Context, workspaceID, userID string) (*model.WorkspaceMember, error) {
	var m model.WorkspaceMember
	err := s.db.QueryRowContext(ctx,
		`SELECT id, workspace_id, user_id, role, COALESCE(invited_by,''), invited_at, joined_at FROM workspace_members WHERE workspace_id=? AND user_id=?`,
		workspaceID, userID,
	).Scan(&m.ID, &m.WorkspaceID, &m.UserID, &m.Role, &m.InvitedBy, &m.InvitedAt, &m.JoinedAt)
	if err != nil {
		return nil, err
	}
	return &m, nil
}

func (s *SQLiteStore) ListMembers(ctx context.Context, workspaceID string) ([]model.WorkspaceMember, error) {
	rows, err := s.db.QueryContext(ctx,
		`SELECT m.id, m.workspace_id, m.user_id, m.role, COALESCE(m.invited_by,''), COALESCE(m.invited_at,''), m.joined_at, COALESCE(u.display_name,''), COALESCE(u.email,'') FROM workspace_members m LEFT JOIN users u ON m.user_id=u.id WHERE m.workspace_id=? ORDER BY m.joined_at`,
		workspaceID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var members []model.WorkspaceMember
	for rows.Next() {
		var m model.WorkspaceMember
		if err := rows.Scan(&m.ID, &m.WorkspaceID, &m.UserID, &m.Role, &m.InvitedBy, &m.InvitedAt, &m.JoinedAt, &m.UserName, &m.UserEmail); err != nil {
			return nil, err
		}
		members = append(members, m)
	}
	return members, nil
}

func (s *SQLiteStore) UpdateMemberRole(ctx context.Context, id string, role string) error {
	_, err := s.db.ExecContext(ctx, `UPDATE workspace_members SET role=? WHERE id=?`, role, id)
	return err
}

func (s *SQLiteStore) RemoveMember(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM workspace_members WHERE id=?`, id)
	return err
}

// ==================== 刷新令牌 ====================

func (s *SQLiteStore) CreateRefreshToken(ctx context.Context, userID string, tokenHash string, expiresAt string) (string, error) {
	id := uuid.New().String()
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES (?,?,?,?)`,
		id, userID, tokenHash, expiresAt)
	return id, err
}

func (s *SQLiteStore) GetRefreshToken(ctx context.Context, tokenHash string) (*model.RefreshToken, error) {
	var rt model.RefreshToken
	err := s.db.QueryRowContext(ctx,
		`SELECT id, user_id, token_hash, expires_at, created_at FROM refresh_tokens WHERE token_hash=?`,
		tokenHash,
	).Scan(&rt.ID, &rt.UserID, &rt.TokenHash, &rt.ExpiresAt, &rt.CreatedAt)
	if err != nil {
		return nil, err
	}
	return &rt, nil
}

func (s *SQLiteStore) RevokeRefreshToken(ctx context.Context, tokenHash string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM refresh_tokens WHERE token_hash=?`, tokenHash)
	return err
}

func (s *SQLiteStore) RevokeUserRefreshTokens(ctx context.Context, userID string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM refresh_tokens WHERE user_id=?`, userID)
	return err
}

// ==================== Pipeline ====================

func (s *SQLiteStore) ListPipelines(ctx context.Context, workspaceID string) ([]model.Pipeline, error) {
	rows, err := s.db.QueryContext(ctx,
		`SELECT plan_id, COALESCE(org_id,''), COALESCE(workspace_id,''), COALESCE(name,''), COALESCE(description,''), tenant, stages, created_at FROM pipelines WHERE workspace_id=? OR workspace_id IS NULL OR workspace_id=''`, workspaceID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var pipelines []model.Pipeline
	for rows.Next() {
		var p model.Pipeline
		var stagesJSON string
		if err := rows.Scan(&p.PlanID, &p.OrgID, &p.WorkspaceID, &p.Name, &p.Description, &p.Tenant, &stagesJSON, &p.Created); err != nil {
			return nil, err
		}
		p.ID = p.PlanID
		json.Unmarshal([]byte(stagesJSON), &p.Stages)
		pipelines = append(pipelines, p)
	}
	return pipelines, nil
}

func (s *SQLiteStore) GetPipeline(ctx context.Context, id string) (*model.Pipeline, error) {
	var p model.Pipeline
	var stagesJSON string
	err := s.db.QueryRowContext(ctx,
		`SELECT plan_id, COALESCE(org_id,''), COALESCE(workspace_id,''), COALESCE(name,''), COALESCE(description,''), tenant, stages, created_at FROM pipelines WHERE plan_id=?`, id,
	).Scan(&p.PlanID, &p.OrgID, &p.WorkspaceID, &p.Name, &p.Description, &p.Tenant, &stagesJSON, &p.Created)
	if err != nil {
		return nil, err
	}
	p.ID = p.PlanID
	json.Unmarshal([]byte(stagesJSON), &p.Stages)
	return &p, nil
}

func (s *SQLiteStore) CreatePipeline(ctx context.Context, p *model.Pipeline) error {
	stagesJSON, _ := json.Marshal(p.Stages)
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO pipelines (plan_id, org_id, workspace_id, name, description, tenant, stages, created_at) VALUES (?,?,?,?,?,?,?,?)`,
		p.PlanID, p.OrgID, p.WorkspaceID, p.Name, p.Description, p.Tenant, string(stagesJSON), p.Created)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion(ctx, "pipeline")
}

func (s *SQLiteStore) UpdatePipeline(ctx context.Context, id string, p *model.Pipeline) error {
	stagesJSON, _ := json.Marshal(p.Stages)
	_, err := s.db.ExecContext(ctx,
		`UPDATE pipelines SET name=?, description=?, tenant=?, stages=?, workspace_id=? WHERE plan_id=?`,
		p.Name, p.Description, p.Tenant, string(stagesJSON), p.WorkspaceID, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion(ctx, "pipeline")
}

func (s *SQLiteStore) DeletePipeline(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM pipelines WHERE plan_id=?`, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion(ctx, "pipeline")
}

// ==================== Plugin ====================

func (s *SQLiteStore) ListPlugins(ctx context.Context, workspaceID string) ([]model.PluginMeta, error) {
	rows, err := s.db.QueryContext(ctx,
		`SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, type, COALESCE(version,''), COALESCE(description,''), COALESCE(config,'{}') FROM plugins WHERE workspace_id=? OR workspace_id IS NULL OR workspace_id=''`, workspaceID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var plugins []model.PluginMeta
	for rows.Next() {
		var p model.PluginMeta
		if err := rows.Scan(&p.ID, &p.OrgID, &p.WorkspaceID, &p.Name, &p.Type, &p.Version, &p.Description, &p.Config); err != nil {
			return nil, err
		}
		plugins = append(plugins, p)
	}
	return plugins, nil
}

func (s *SQLiteStore) CreatePlugin(ctx context.Context, p *model.PluginMeta) error {
	if p.Config == "" {
		p.Config = "{}"
	}
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO plugins (id, org_id, workspace_id, name, type, version, description, config) VALUES (?,?,?,?,?,?,?,?)`,
		p.ID, p.OrgID, p.WorkspaceID, p.Name, p.Type, p.Version, p.Description, p.Config)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion(ctx, "plugin")
}

func (s *SQLiteStore) UpdatePlugin(ctx context.Context, id string, p *model.PluginMeta) error {
	if p.Config == "" {
		p.Config = "{}"
	}
	_, err := s.db.ExecContext(ctx,
		`UPDATE plugins SET name=?, type=?, version=?, description=?, config=? WHERE id=?`,
		p.Name, p.Type, p.Version, p.Description, p.Config, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion(ctx, "plugin")
}

func (s *SQLiteStore) DeletePlugin(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM plugins WHERE id=?`, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion(ctx, "plugin")
}

// ==================== Policy ====================

func (s *SQLiteStore) ListPolicies(ctx context.Context, workspaceID string) ([]model.Policy, error) {
	var rows *sql.Rows
	var err error
	if workspaceID != "" {
		rows, err = s.db.QueryContext(ctx,
			`SELECT id, name, type, content, created_at FROM policies WHERE workspace_id=? OR (org_id IS NOT NULL AND workspace_id IS NULL) ORDER BY created_at`, workspaceID)
	} else {
		rows, err = s.db.QueryContext(ctx,
			`SELECT id, name, type, content, created_at FROM policies WHERE workspace_id IS NULL ORDER BY created_at`)
	}
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var policies []model.Policy
	for rows.Next() {
		var p model.Policy
		if err := rows.Scan(&p.ID, &p.Name, &p.Type, &p.Content, &p.CreatedAt); err != nil {
			return nil, err
		}
		policies = append(policies, p)
	}
	return policies, nil
}

func (s *SQLiteStore) CreatePolicy(ctx context.Context, p *model.Policy) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO policies (id, name, type, content, created_at) VALUES (?,?,?,?,?)`,
		p.ID, p.Name, p.Type, p.Content, p.CreatedAt)
	return err
}

func (s *SQLiteStore) DeletePolicy(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM policies WHERE id=?`, id)
	return err
}

// ==================== API Key ====================

func (s *SQLiteStore) ListApiKeys(ctx context.Context, workspaceID string) ([]model.ApiKey, error) {
	rows, err := s.db.QueryContext(ctx,
		`SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, key, tenant_id, status, created_at, COALESCE(last_used,'') FROM apikeys WHERE workspace_id=? OR workspace_id IS NULL OR workspace_id=''`, workspaceID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var keys []model.ApiKey
	for rows.Next() {
		var k model.ApiKey
		if err := rows.Scan(&k.ID, &k.OrgID, &k.WorkspaceID, &k.Name, &k.Key, &k.TenantID, &k.Status, &k.CreatedAt, &k.LastUsed); err != nil {
			return nil, err
		}
		keys = append(keys, k)
	}
	return keys, nil
}

func (s *SQLiteStore) GetApiKey(ctx context.Context, id string) (*model.ApiKey, error) {
	var k model.ApiKey
	err := s.db.QueryRowContext(ctx,
		`SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, key, tenant_id, status, created_at, COALESCE(last_used,'') FROM apikeys WHERE id=?`, id,
	).Scan(&k.ID, &k.OrgID, &k.WorkspaceID, &k.Name, &k.Key, &k.TenantID, &k.Status, &k.CreatedAt, &k.LastUsed)
	if err != nil {
		return nil, err
	}
	return &k, nil
}

func (s *SQLiteStore) CreateApiKey(ctx context.Context, k *model.ApiKey) error {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO apikeys (id, org_id, workspace_id, name, key, tenant_id, status, created_at, last_used) VALUES (?,?,?,?,?,?,?,?,?)`,
		k.ID, k.OrgID, k.WorkspaceID, k.Name, k.Key, k.TenantID, k.Status, k.CreatedAt, k.LastUsed)
	return err
}

func (s *SQLiteStore) UpdateApiKey(ctx context.Context, k *model.ApiKey) error {
	_, err := s.db.ExecContext(ctx,
		`UPDATE apikeys SET name=?, key=?, tenant_id=?, status=?, last_used=? WHERE id=?`,
		k.Name, k.Key, k.TenantID, k.Status, k.LastUsed, k.ID)
	return err
}

func (s *SQLiteStore) DeleteApiKey(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM apikeys WHERE id=?`, id)
	return err
}

// ==================== Model ====================

func (s *SQLiteStore) ListModels(ctx context.Context, workspaceID string) ([]model.ModelConfig, error) {
	rows, err := s.db.QueryContext(ctx,
		`SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, upstream_url, upstream_model, COALESCE(api_key,''), COALESCE(api_key_header,'Authorization'), use_proxy, COALESCE(proxy_url,''), is_default, COALESCE(supported_versions,'[]'), status FROM models WHERE workspace_id=? OR workspace_id IS NULL OR workspace_id=''`, workspaceID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var models []model.ModelConfig
	for rows.Next() {
		var m model.ModelConfig
		var versionsJSON string
		var useProxy, isDefault int
		if err := rows.Scan(&m.ID, &m.OrgID, &m.WorkspaceID, &m.Name, &m.UpstreamURL, &m.UpstreamModel, &m.ApiKey, &m.ApiKeyHeader, &useProxy, &m.ProxyURL, &isDefault, &versionsJSON, &m.Status); err != nil {
			return nil, err
		}
		m.UseProxy = useProxy != 0
		m.IsDefault = isDefault != 0
		json.Unmarshal([]byte(versionsJSON), &m.SupportedVersions)
		models = append(models, m)
	}
	return models, nil
}

func (s *SQLiteStore) GetModel(ctx context.Context, id string) (*model.ModelConfig, error) {
	var m model.ModelConfig
	var versionsJSON string
	var useProxy, isDefault int
	err := s.db.QueryRowContext(ctx,
		`SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, upstream_url, upstream_model, COALESCE(api_key,''), COALESCE(api_key_header,'Authorization'), use_proxy, COALESCE(proxy_url,''), is_default, COALESCE(supported_versions,'[]'), status FROM models WHERE id=?`, id,
	).Scan(&m.ID, &m.OrgID, &m.WorkspaceID, &m.Name, &m.UpstreamURL, &m.UpstreamModel, &m.ApiKey, &m.ApiKeyHeader, &useProxy, &m.ProxyURL, &isDefault, &versionsJSON, &m.Status)
	if err != nil {
		return nil, err
	}
	m.UseProxy = useProxy != 0
	m.IsDefault = isDefault != 0
	json.Unmarshal([]byte(versionsJSON), &m.SupportedVersions)
	return &m, nil
}

func (s *SQLiteStore) CreateModel(ctx context.Context, m *model.ModelConfig) error {
	versionsJSON, _ := json.Marshal(m.SupportedVersions)
	useProxy := 0
	if m.UseProxy { useProxy = 1 }
	isDefault := 0
	if m.IsDefault { isDefault = 1 }
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO models (id, org_id, workspace_id, name, upstream_url, upstream_model, api_key, api_key_header, use_proxy, proxy_url, is_default, supported_versions, status) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)`,
		m.ID, m.OrgID, m.WorkspaceID, m.Name, m.UpstreamURL, m.UpstreamModel, m.ApiKey, m.ApiKeyHeader, useProxy, m.ProxyURL, isDefault, string(versionsJSON), m.Status)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion(ctx, "model")
}

func (s *SQLiteStore) UpdateModel(ctx context.Context, id string, m *model.ModelConfig) error {
	versionsJSON, _ := json.Marshal(m.SupportedVersions)
	useProxy := 0
	if m.UseProxy { useProxy = 1 }
	isDefault := 0
	if m.IsDefault { isDefault = 1 }
	_, err := s.db.ExecContext(ctx,
		`UPDATE models SET name=?, upstream_url=?, upstream_model=?, api_key=?, api_key_header=?, use_proxy=?, proxy_url=?, is_default=?, supported_versions=?, status=? WHERE id=?`,
		m.Name, m.UpstreamURL, m.UpstreamModel, m.ApiKey, m.ApiKeyHeader, useProxy, m.ProxyURL, isDefault, string(versionsJSON), m.Status, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion(ctx, "model")
}

func (s *SQLiteStore) DeleteModel(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM models WHERE id=?`, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion(ctx, "model")
}

// ==================== Virtual Key ====================

func (s *SQLiteStore) CreateVirtualKey(ctx context.Context, vk *model.VirtualKey) (*model.VirtualKey, error) {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO virtual_keys (id, workspace_id, name, key_prefix, key_hash, type, provider_key_encrypted, allowed_models, rate_limit_rpm, rate_limit_tpm, spend_limit_usd_micro, status, created_by) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)`,
		vk.ID, vk.WorkspaceID, vk.Name, vk.KeyPrefix, vk.KeyHash, vk.Type, vk.ProviderKeyEncrypted, vk.AllowedModels, vk.RateLimitRPM, vk.RateLimitTPM, vk.SpendLimitUSDMicro, vk.Status, vk.CreatedBy)
	return vk, err
}

func (s *SQLiteStore) GetVirtualKey(ctx context.Context, id string) (*model.VirtualKey, error) {
	var vk model.VirtualKey
	var lastUsed sql.NullString
	err := s.db.QueryRowContext(ctx,
		`SELECT id, workspace_id, name, key_prefix, key_hash, type, COALESCE(provider_key_encrypted,''), COALESCE(allowed_models,'[]'), rate_limit_rpm, rate_limit_tpm, spend_limit_usd_micro, status, last_used_at, created_at, created_by FROM virtual_keys WHERE id=?`, id,
	).Scan(&vk.ID, &vk.WorkspaceID, &vk.Name, &vk.KeyPrefix, &vk.KeyHash, &vk.Type, &vk.ProviderKeyEncrypted, &vk.AllowedModels, &vk.RateLimitRPM, &vk.RateLimitTPM, &vk.SpendLimitUSDMicro, &vk.Status, &lastUsed, &vk.CreatedAt, &vk.CreatedBy)
	if err != nil {
		return nil, err
	}
	if lastUsed.Valid {
		lastUsedStr := lastUsed.String
		vk.LastUsedAt = lastUsedStr
	}
	return &vk, nil
}

func (s *SQLiteStore) GetVirtualKeyByHash(ctx context.Context, keyHash string) (*model.VirtualKey, error) {
	var vk model.VirtualKey
	var lastUsed sql.NullString
	err := s.db.QueryRowContext(ctx,
		`SELECT id, workspace_id, name, key_prefix, key_hash, type, COALESCE(provider_key_encrypted,''), COALESCE(allowed_models,'[]'), rate_limit_rpm, rate_limit_tpm, spend_limit_usd_micro, status, last_used_at, created_at, created_by FROM virtual_keys WHERE key_hash=?`, keyHash,
	).Scan(&vk.ID, &vk.WorkspaceID, &vk.Name, &vk.KeyPrefix, &vk.KeyHash, &vk.Type, &vk.ProviderKeyEncrypted, &vk.AllowedModels, &vk.RateLimitRPM, &vk.RateLimitTPM, &vk.SpendLimitUSDMicro, &vk.Status, &lastUsed, &vk.CreatedAt, &vk.CreatedBy)
	if err != nil {
		return nil, err
	}
	if lastUsed.Valid {
		lastUsedStr := lastUsed.String
		vk.LastUsedAt = lastUsedStr
	}
	return &vk, nil
}

func (s *SQLiteStore) ListVirtualKeys(ctx context.Context, workspaceID string) ([]model.VirtualKey, error) {
	rows, err := s.db.QueryContext(ctx,
		`SELECT id, workspace_id, name, key_prefix, key_hash, type, COALESCE(provider_key_encrypted,''), COALESCE(allowed_models,'[]'), rate_limit_rpm, rate_limit_tpm, spend_limit_usd_micro, status, last_used_at, created_at, created_by FROM virtual_keys WHERE workspace_id=? ORDER BY created_at`, workspaceID)
	if err != nil { return nil, err }
	defer rows.Close()
	var keys []model.VirtualKey
	for rows.Next() {
		var vk model.VirtualKey
		var lastUsed sql.NullString
		if err := rows.Scan(&vk.ID, &vk.WorkspaceID, &vk.Name, &vk.KeyPrefix, &vk.KeyHash, &vk.Type, &vk.ProviderKeyEncrypted, &vk.AllowedModels, &vk.RateLimitRPM, &vk.RateLimitTPM, &vk.SpendLimitUSDMicro, &vk.Status, &lastUsed, &vk.CreatedAt, &vk.CreatedBy); err != nil {
			return nil, err
		}
		if lastUsed.Valid {
			lastUsedStr := lastUsed.String
			vk.LastUsedAt = lastUsedStr
		}
		keys = append(keys, vk)
	}
	return keys, nil
}

func (s *SQLiteStore) UpdateVirtualKey(ctx context.Context, id string, updates map[string]interface{}) error {
	setClause, args := buildSetClause(updates, 1)
	if setClause == "" { return nil }
	args = append(args, id)
	_, err := s.db.ExecContext(ctx, `UPDATE virtual_keys SET `+setClause+` WHERE id=?`, args...)
	return err
}

func (s *SQLiteStore) RevokeVirtualKey(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `UPDATE virtual_keys SET status='revoked' WHERE id=?`, id)
	return err
}

// ==================== Wallet ====================

func (s *SQLiteStore) GetWallet(ctx context.Context, workspaceID string) (*model.Wallet, error) {
	var w model.Wallet
	var lastCredit sql.NullString
	err := s.db.QueryRowContext(ctx,
		`SELECT id, workspace_id, balance_usd_micro, overdraft_limit_micro, last_credit_at, created_at, updated_at FROM wallets WHERE workspace_id=?`, workspaceID,
	).Scan(&w.ID, &w.WorkspaceID, &w.BalanceUSDMicro, &w.OverdraftLimitMicro, &lastCredit, &w.CreatedAt, &w.UpdatedAt)
	if err != nil { return nil, err }
	if lastCredit.Valid {
		lc := lastCredit.String
		w.LastCreditAt = &lc
	}
	return &w, nil
}

func (s *SQLiteStore) CreateWallet(ctx context.Context, w *model.Wallet) error {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO wallets (id, workspace_id, balance_usd_micro, overdraft_limit_micro) VALUES (?,?,?,?)`,
		w.ID, w.WorkspaceID, w.BalanceUSDMicro, w.OverdraftLimitMicro)
	return err
}

func (s *SQLiteStore) UpdateWalletBalance(ctx context.Context, workspaceID string, amountMicro int64) error {
	_, err := s.db.ExecContext(ctx,
		`UPDATE wallets SET balance_usd_micro=balance_usd_micro+?, updated_at=CURRENT_TIMESTAMP WHERE workspace_id=?`,
		amountMicro, workspaceID)
	return err
}

func (s *SQLiteStore) ListTransactions(ctx context.Context, workspaceID string, limit, offset int) ([]model.Transaction, error) {
	rows, err := s.db.QueryContext(ctx,
		`SELECT id, workspace_id, wallet_id, type, amount_usd_micro, balance_after_micro, COALESCE(description,''), COALESCE(trace_id,''), COALESCE(metadata,'{}'), created_at FROM transactions WHERE workspace_id=? ORDER BY created_at DESC LIMIT ? OFFSET ?`,
		workspaceID, limit, offset)
	if err != nil { return nil, err }
	defer rows.Close()
	var txs []model.Transaction
	for rows.Next() {
		var t model.Transaction
		if err := rows.Scan(&t.ID, &t.WorkspaceID, &t.WalletID, &t.Type, &t.AmountUSDMicro, &t.BalanceAfterMicro, &t.Description, &t.TraceID, &t.Metadata, &t.CreatedAt); err != nil {
			return nil, err
		}
		txs = append(txs, t)
	}
	return txs, nil
}

func (s *SQLiteStore) CreateTransaction(ctx context.Context, t *model.Transaction) error {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO transactions (id, workspace_id, wallet_id, type, amount_usd_micro, balance_after_micro, description, trace_id, metadata) VALUES (?,?,?,?,?,?,?,?,?)`,
		t.ID, t.WorkspaceID, t.WalletID, t.Type, t.AmountUSDMicro, t.BalanceAfterMicro, t.Description, t.TraceID, t.Metadata)
	return err
}

// ==================== 配置版本 ====================

func (s *SQLiteStore) GetConfigVersions(ctx context.Context) (model.ConfigVersion, error) {
	var v model.ConfigVersion
	m := map[string]*int{
		"pipeline_version": &v.PipelineVersion,
		"policy_version":   &v.PolicyVersion,
		"plugin_version":   &v.PluginVersion,
		"storage_version":  &v.StorageVersion,
		"model_version":    &v.ModelVersion,
	}
	for key, ptr := range m {
		var val int
		err := s.db.QueryRowContext(ctx, `SELECT value FROM config_versions WHERE key=?`, key).Scan(&val)
		if err != nil {
			*ptr = 0
		} else {
			*ptr = val
		}
	}
	return v, nil
}

func (s *SQLiteStore) IncrementConfigVersion(ctx context.Context, key string) error {
	versionKey := key + "_version"
	_, err := s.db.ExecContext(ctx, `UPDATE config_versions SET value=value+1 WHERE key=?`, versionKey)
	return err
}

// ==================== 数据面配置 ====================

func (s *SQLiteStore) ListDataPlaneConfigs(ctx context.Context) ([]model.DataPlaneConfig, error) {
	rows, err := s.db.QueryContext(ctx, `SELECT id, key, value, created_at FROM data_plane_configs`)
	if err != nil { return nil, err }
	defer rows.Close()
	var configs []model.DataPlaneConfig
	for rows.Next() {
		var c model.DataPlaneConfig
		if err := rows.Scan(&c.ID, &c.Key, &c.Value, &c.CreatedAt); err != nil { return nil, err }
		configs = append(configs, c)
	}
	return configs, nil
}

func (s *SQLiteStore) CreateDataPlaneConfig(ctx context.Context, c *model.DataPlaneConfig) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO data_plane_configs (id, key, value, created_at) VALUES (?,?,?,datetime('now'))`, c.ID, c.Key, c.Value)
	return err
}

func (s *SQLiteStore) UpdateDataPlaneConfig(ctx context.Context, id string, c *model.DataPlaneConfig) error {
	_, err := s.db.ExecContext(ctx, `UPDATE data_plane_configs SET key=?, value=? WHERE id=?`, c.Key, c.Value, id)
	return err
}

func (s *SQLiteStore) DeleteDataPlaneConfig(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM data_plane_configs WHERE id=?`, id)
	return err
}

// ==================== 设置 ====================

func (s *SQLiteStore) GetSettings(ctx context.Context, workspaceID string) (map[string]string, error) {
	rows, err := s.db.QueryContext(ctx, `SELECT key, value FROM settings WHERE workspace_id=?`, workspaceID)
	if err != nil { return nil, err }
	defer rows.Close()
	settings := make(map[string]string)
	for rows.Next() {
		var k, v string
		if err := rows.Scan(&k, &v); err != nil { return nil, err }
		settings[k] = v
	}
	return settings, nil
}

func (s *SQLiteStore) UpdateSettings(ctx context.Context, workspaceID string, settings map[string]string) error {
	for k, v := range settings {
		_, err := s.db.ExecContext(ctx,
			`INSERT INTO settings (key, workspace_id, value) VALUES (?,?,?) ON CONFLICT(key, workspace_id) DO UPDATE SET value=excluded.value`,
			k, workspaceID, v)
		if err != nil { return err }
	}
	return nil
}

// ==================== 审计事件 ====================

func (s *SQLiteStore) GetAuditEvents(ctx context.Context, workspaceID string, period string) ([]map[string]interface{}, error) {
	// 基于现有 traces 和 apikeys 数据聚合审计指标
	// 生产环境应使用 ClickHouse 专用审计事件表
	events := []map[string]interface{}{}

	var traceCount int
	s.db.QueryRowContext(ctx, `SELECT COUNT(*) FROM traces WHERE tenant_id=? OR tenant_id=''`, workspaceID).Scan(&traceCount)
	events = append(events, map[string]interface{}{
		"type": "total_traces", "label": "总执行次数", "value": traceCount, "icon": "Activity",
	})

	var apiKeyCount int
	s.db.QueryRowContext(ctx, `SELECT COUNT(*) FROM apikeys WHERE workspace_id=?`, workspaceID).Scan(&apiKeyCount)
	events = append(events, map[string]interface{}{
		"type": "active_keys", "label": "活跃 API Key", "value": apiKeyCount, "icon": "Key",
	})

	// 模拟安全事件统计（生产环境从专用审计表读取）
	var pipelineCount int
	s.db.QueryRowContext(ctx, `SELECT COUNT(*) FROM pipelines WHERE workspace_id=? OR workspace_id IS NULL`, workspaceID).Scan(&pipelineCount)
	events = append(events, map[string]interface{}{
		"type": "guardrail_blocks", "label": "Guardrail 拦截次数", "value": 0, "icon": "Shield",
	})
	events = append(events, map[string]interface{}{
		"type": "pii_detections", "label": "PII 检测次数", "value": 0, "icon": "Eye",
	})
	events = append(events, map[string]interface{}{
		"type": "budget_exceeded", "label": "预算熔断次数", "value": 0, "icon": "DollarSign",
	})
	events = append(events, map[string]interface{}{
		"type": "pipelines", "label": "活跃流水线", "value": pipelineCount, "icon": "GitBranch",
	})

	// 按风险级别分布
	events = append(events, map[string]interface{}{
		"type": "risk_distribution",
		"label": "风险分布",
		"levels": []map[string]interface{}{
			{"level": "critical", "count": 0, "color": "#ff7675"},
			{"level": "high", "count": 0, "color": "#fdcb6e"},
			{"level": "medium", "count": 0, "color": "#74b9ff"},
			{"level": "low", "count": traceCount, "color": "#00d4aa"},
		},
		"icon": "BarChart",
	})

	return events, nil
}

// ==================== 工具函数 ====================

func buildSetClause(updates map[string]interface{}, startIdx int) (string, []interface{}) {
	var clause string
	var args []interface{}
	i := startIdx
	for col, val := range updates {
		if clause != "" { clause += ", " }
		clause += fmt.Sprintf("%s=?", col)
		args = append(args, val)
		i++
	}
	return clause, args
}

func min(a, b int) int {
	if a < b { return a }
	return b
}

func (s *SQLiteStore) ListUserMemberships(ctx context.Context, userID string) ([]model.WorkspaceMember, error) {
	rows, err := s.db.QueryContext(ctx, `SELECT id, workspace_id, user_id, role, COALESCE(invited_by,''), COALESCE(invited_at,''), joined_at FROM workspace_members WHERE user_id=?`, userID)
	if err != nil { return nil, err }
	defer rows.Close()
	var members []model.WorkspaceMember
	for rows.Next() {
		var m model.WorkspaceMember
		if err := rows.Scan(&m.ID, &m.WorkspaceID, &m.UserID, &m.Role, &m.InvitedBy, &m.InvitedAt, &m.JoinedAt); err != nil { return nil, err }
		members = append(members, m)
	}
	return members, nil
}

// ==================== 聊天会话 (SQLite 精简实现，仅供开发环境) ====================
func (s *SQLiteStore) ListConversations(ctx context.Context, userID string) ([]model.Conversation, error) {
	return nil, fmt.Errorf("conversations not supported in SQLite dev mode, use postgres")
}
func (s *SQLiteStore) CreateConversation(ctx context.Context, conv *model.Conversation) error {
	return fmt.Errorf("conversations not supported in SQLite dev mode, use postgres")
}
func (s *SQLiteStore) GetConversation(ctx context.Context, id string) (*model.Conversation, error) {
	return nil, fmt.Errorf("conversations not supported in SQLite dev mode, use postgres")
}
func (s *SQLiteStore) UpdateConversation(ctx context.Context, id string, updates map[string]interface{}) error {
	return fmt.Errorf("conversations not supported in SQLite dev mode, use postgres")
}
func (s *SQLiteStore) DeleteConversation(ctx context.Context, id string) error {
	return fmt.Errorf("conversations not supported in SQLite dev mode, use postgres")
}
func (s *SQLiteStore) ListMessages(ctx context.Context, conversationID string, limit int) ([]model.Message, error) {
	return nil, fmt.Errorf("conversations not supported in SQLite dev mode, use postgres")
}
func (s *SQLiteStore) CreateMessage(ctx context.Context, msg *model.Message) error {
	return fmt.Errorf("conversations not supported in SQLite dev mode, use postgres")
}
func (s *SQLiteStore) DeleteMessagesByConversation(ctx context.Context, conversationID string) error {
	return fmt.Errorf("conversations not supported in SQLite dev mode, use postgres")
}
func (s *SQLiteStore) ListDpTraces(ctx context.Context, limit int) ([]map[string]interface{}, error) { return nil, nil }
