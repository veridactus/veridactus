// VERIDACTUS 控制平面 — PostgreSQL 存储实现
// 生产环境使用，支持完整多租户隔离
package store

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/lib/pq"
	_ "github.com/lib/pq"
	"github.com/veridactus/control-plane/internal/model"
)

// PostgresStore PostgreSQL 存储实现
type PostgresStore struct {
	db *sql.DB
}

// NewPostgresStore 创建 PostgreSQL 存储
func NewPostgresStore(cfg StoreConfig) (StoreFacade, error) {
	dsn := fmt.Sprintf(
		"host=%s port=%d user=%s password=%s dbname=%s sslmode=%s",
		cfg.PGHost, cfg.PGPort, cfg.PGUser, cfg.PGPass, cfg.PGDBName, cfg.PGSSLMode,
	)

	db, err := sql.Open("postgres", dsn)
	if err != nil {
		return nil, fmt.Errorf("open postgres: %w", err)
	}

	// 配置连接池
	db.SetMaxOpenConns(25)
	db.SetMaxIdleConns(10)
	db.SetConnMaxLifetime(5 * time.Minute)

	// 验证连接
	if err := db.PingContext(context.Background()); err != nil {
		return nil, fmt.Errorf("ping postgres: %w", err)
	}

	store := &PostgresStore{db: db}

	if err := store.RunMigrations(context.Background()); err != nil {
		return nil, fmt.Errorf("run migrations: %w", err)
	}

	if err := store.seedDefaultData(context.Background()); err != nil {
		log.Printf("WARN: seed data: %v", err)
	}

	return store, nil
}

func (s *PostgresStore) RunMigrations(ctx context.Context) error {
	// PostgreSQL 始终使用自己的迁移（不读 SQLite 文件）
	return s.runInlineMigration(ctx)
}

func (s *PostgresStore) runInlineMigration(ctx context.Context) error {
	// 生产环境 PostgreSQL 迁移
	queries := getPostgresMigrations()
	for _, q := range queries {
		if _, err := s.db.ExecContext(ctx, q); err != nil {
			return fmt.Errorf("migrate: %w", err)
		}
	}
	return nil
}

func (s *PostgresStore) execSQL(ctx context.Context, sql []byte) error {
	_, err := s.db.ExecContext(ctx, string(sql))
	return err
}

func (s *PostgresStore) seedDefaultData(ctx context.Context) error {
	var count int
	s.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM apikeys").Scan(&count)
	if count > 0 {
		return nil
	}

	orgID := uuid.New().String()
	wsID := uuid.New().String()
	walletID := uuid.New().String()

	s.db.ExecContext(ctx, `INSERT INTO organizations (id, name, slug, plan) VALUES ($1, 'Default Organization', 'default', 'free')`, orgID)
	s.db.ExecContext(ctx, `INSERT INTO workspaces (id, org_id, name, slug) VALUES ($1, $2, 'Default Workspace', 'default')`, wsID, orgID)
	s.db.ExecContext(ctx, `INSERT INTO wallets (id, workspace_id) VALUES ($1, $2)`, walletID, wsID)

	// 默认插件
	plugins := []struct{ name, ptype, version, desc, config string }{
		{"PII Detector", "native", "0.2.1", "PII检测与脱敏插件", `{"enabled":true}`},
		{"Budget Guard", "native", "1.0.0", "预算守卫插件", `{"limit_usd":10.0}`},
		{"Auth Validator", "native", "1.0.0", "认证验证器", `{}`},
		{"Trace Finalizer", "native", "1.0.0", "Trace终结器", `{}`},
	}
	for _, p := range plugins {
		s.db.ExecContext(ctx,
			`INSERT INTO plugins (id, org_id, workspace_id, name, type, version, description, config) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)`,
			uuid.New().String(), orgID, wsID, p.name, p.ptype, p.version, p.desc, p.config)
	}

	log.Println("PostgreSQL: default data seeded")
	return nil
}

// 实现所有 StoreFacade 方法 (与 SQLiteStore 类似，使用 $N 参数占位符)
// 以下为关键方法示例，完整实现在 Phase 1 后续迭代中补充

func (s *PostgresStore) HealthCheck(ctx context.Context) error {
	return s.db.PingContext(ctx)
}

func (s *PostgresStore) Close() error {
	return s.db.Close()
}

// ==================== 组织 ====================

func (s *PostgresStore) CreateOrganization(ctx context.Context, org *model.Organization) error {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO organizations (id, name, slug, plan, logo_url, primary_color, settings) VALUES ($1,$2,$3,$4,$5,$6,$7)`,
		org.ID, org.Name, org.Slug, org.Plan, org.LogoURL, org.PrimaryColor, org.Settings)
	return err
}

func (s *PostgresStore) GetOrganization(ctx context.Context, id string) (*model.Organization, error) {
	var org model.Organization
	err := s.db.QueryRowContext(ctx,
		`SELECT id, name, slug, plan, COALESCE(logo_url,''), COALESCE(primary_color,'#6c5ce7'), COALESCE(settings,'{}'), created_at, updated_at FROM organizations WHERE id=$1`, id,
	).Scan(&org.ID, &org.Name, &org.Slug, &org.Plan, &org.LogoURL, &org.PrimaryColor, &org.Settings, &org.CreatedAt, &org.UpdatedAt)
	if err != nil { return nil, err }
	return &org, nil
}

func (s *PostgresStore) GetOrganizationBySlug(ctx context.Context, slug string) (*model.Organization, error) {
	var org model.Organization
	err := s.db.QueryRowContext(ctx,
		`SELECT id, name, slug, plan, COALESCE(logo_url,''), COALESCE(primary_color,'#6c5ce7'), COALESCE(settings,'{}'), created_at, updated_at FROM organizations WHERE slug=$1`, slug,
	).Scan(&org.ID, &org.Name, &org.Slug, &org.Plan, &org.LogoURL, &org.PrimaryColor, &org.Settings, &org.CreatedAt, &org.UpdatedAt)
	if err != nil { return nil, err }
	return &org, nil
}

func (s *PostgresStore) ListOrganizations(ctx context.Context) ([]model.Organization, error) {
	return listHelper[model.Organization](ctx, s.db,
		`SELECT id, name, slug, plan, COALESCE(logo_url,''), COALESCE(primary_color,'#6c5ce7'), COALESCE(settings,'{}'), created_at, updated_at FROM organizations ORDER BY created_at`,
		func(org *model.Organization, scan func(...interface{}) error) error {
			return scan(&org.ID, &org.Name, &org.Slug, &org.Plan, &org.LogoURL, &org.PrimaryColor, &org.Settings, &org.CreatedAt, &org.UpdatedAt)
		})
}

// ListOrganizationsByUser 返回用户所属的所有组织（通过 workspace_members 关联）
func (s *PostgresStore) ListOrganizationsByUser(ctx context.Context, userID string) ([]model.Organization, error) {
	return listHelper[model.Organization](ctx, s.db,
		`SELECT DISTINCT o.id, o.name, o.slug, o.plan, COALESCE(o.logo_url,''), COALESCE(o.primary_color,'#6c5ce7'), COALESCE(o.settings,'{}'), o.created_at, o.updated_at
		 FROM organizations o
		 JOIN workspaces w ON w.org_id = o.id
		 JOIN workspace_members wm ON wm.workspace_id = w.id
		 WHERE wm.user_id = $1
		 ORDER BY o.created_at`,
		func(org *model.Organization, scan func(...interface{}) error) error {
			return scan(&org.ID, &org.Name, &org.Slug, &org.Plan, &org.LogoURL, &org.PrimaryColor, &org.Settings, &org.CreatedAt, &org.UpdatedAt)
		}, userID)
}

func (s *PostgresStore) UpdateOrganization(ctx context.Context, id string, updates map[string]interface{}) error {
	return updateHelper(ctx, s.db, "organizations", id, updates)
}

func (s *PostgresStore) DeleteOrganization(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM organizations WHERE id=$1`, id)
	return err
}

// ==================== 工作空间 ====================

func (s *PostgresStore) CreateWorkspace(ctx context.Context, ws *model.Workspace) error {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO workspaces (id, org_id, name, slug, description, settings) VALUES ($1,$2,$3,$4,$5,$6)`,
		ws.ID, ws.OrgID, ws.Name, ws.Slug, ws.Description, ws.Settings)
	if err != nil { return err }
	// 自动创建钱包
	walletID := uuid.New().String()
	s.db.ExecContext(ctx, `INSERT INTO wallets (id, workspace_id) VALUES ($1,$2)`, walletID, ws.ID)
	return nil
}

func (s *PostgresStore) GetWorkspace(ctx context.Context, id string) (*model.Workspace, error) {
	var ws model.Workspace
	err := s.db.QueryRowContext(ctx,
		`SELECT id, org_id, name, slug, COALESCE(description,''), COALESCE(settings,'{}'), created_at, updated_at FROM workspaces WHERE id=$1`, id,
	).Scan(&ws.ID, &ws.OrgID, &ws.Name, &ws.Slug, &ws.Description, &ws.Settings, &ws.CreatedAt, &ws.UpdatedAt)
	if err != nil { return nil, err }
	return &ws, nil
}

func (s *PostgresStore) GetWorkspaceBySlug(ctx context.Context, orgID, slug string) (*model.Workspace, error) {
	var ws model.Workspace
	err := s.db.QueryRowContext(ctx,
		`SELECT id, org_id, name, slug, COALESCE(description,''), COALESCE(settings,'{}'), created_at, updated_at FROM workspaces WHERE org_id=$1 AND slug=$2`, orgID, slug,
	).Scan(&ws.ID, &ws.OrgID, &ws.Name, &ws.Slug, &ws.Description, &ws.Settings, &ws.CreatedAt, &ws.UpdatedAt)
	if err != nil { return nil, err }
	return &ws, nil
}

func (s *PostgresStore) ListWorkspaces(ctx context.Context, orgID string) ([]model.Workspace, error) {
	return listHelper[model.Workspace](ctx, s.db,
		`SELECT id, org_id, name, slug, COALESCE(description,''), COALESCE(settings,'{}'), created_at, updated_at FROM workspaces WHERE org_id=$1 ORDER BY created_at`,
		func(ws *model.Workspace, scan func(...interface{}) error) error {
			return scan(&ws.ID, &ws.OrgID, &ws.Name, &ws.Slug, &ws.Description, &ws.Settings, &ws.CreatedAt, &ws.UpdatedAt)
		}, orgID)
}

func (s *PostgresStore) UpdateWorkspace(ctx context.Context, id string, updates map[string]interface{}) error {
	return updateHelper(ctx, s.db, "workspaces", id, updates)
}

func (s *PostgresStore) DeleteWorkspace(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM workspaces WHERE id=$1`, id)
	return err
}

// ==================== 用户 ====================

func (s *PostgresStore) CreateUser(ctx context.Context, user *model.User) error {
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO users (id, email, phone, display_name, avatar_url, auth_provider, auth_provider_id, password_hash, settings) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)`,
		user.ID, user.Email, user.Phone, user.DisplayName, user.AvatarURL, user.AuthProvider, user.AuthProviderID, user.PasswordHash, user.Settings)
	return err
}

func (s *PostgresStore) GetUser(ctx context.Context, id string) (*model.User, error) {
	var user model.User
	err := s.db.QueryRowContext(ctx,
		`SELECT id, email, COALESCE(phone,''), COALESCE(display_name,''), COALESCE(avatar_url,''), auth_provider, COALESCE(auth_provider_id,''), COALESCE(password_hash,''), COALESCE(settings,'{}'), COALESCE(last_login_at::text,''), created_at, updated_at FROM users WHERE id=$1`, id,
	).Scan(&user.ID, &user.Email, &user.Phone, &user.DisplayName, &user.AvatarURL, &user.AuthProvider, &user.AuthProviderID, &user.PasswordHash, &user.Settings, &user.LastLoginAt, &user.CreatedAt, &user.UpdatedAt)
	if err != nil { return nil, err }
	return &user, nil
}

func (s *PostgresStore) GetUserByEmail(ctx context.Context, email string) (*model.User, error) {
	var user model.User
	err := s.db.QueryRowContext(ctx,
		`SELECT id, email, COALESCE(phone,''), COALESCE(display_name,''), COALESCE(avatar_url,''), auth_provider, COALESCE(auth_provider_id,''), COALESCE(password_hash,''), COALESCE(settings,'{}'), COALESCE(last_login_at::text,''), created_at, updated_at FROM users WHERE email=$1`, email,
	).Scan(&user.ID, &user.Email, &user.Phone, &user.DisplayName, &user.AvatarURL, &user.AuthProvider, &user.AuthProviderID, &user.PasswordHash, &user.Settings, &user.LastLoginAt, &user.CreatedAt, &user.UpdatedAt)
	if err != nil { return nil, err }
	return &user, nil
}

func (s *PostgresStore) GetUserByProvider(ctx context.Context, provider, providerID string) (*model.User, error) {
	var user model.User
	err := s.db.QueryRowContext(ctx,
		`SELECT id, email, COALESCE(phone,''), COALESCE(display_name,''), COALESCE(avatar_url,''), auth_provider, COALESCE(auth_provider_id,''), COALESCE(password_hash,''), COALESCE(settings,'{}'), COALESCE(last_login_at::text,''), created_at, updated_at FROM users WHERE auth_provider=$1 AND auth_provider_id=$2`, provider, providerID,
	).Scan(&user.ID, &user.Email, &user.Phone, &user.DisplayName, &user.AvatarURL, &user.AuthProvider, &user.AuthProviderID, &user.PasswordHash, &user.Settings, &user.LastLoginAt, &user.CreatedAt, &user.UpdatedAt)
	if err != nil { return nil, err }
	return &user, nil
}

func (s *PostgresStore) UpdateUser(ctx context.Context, id string, updates map[string]interface{}) error {
	return updateHelper(ctx, s.db, "users", id, updates)
}

func (s *PostgresStore) DeleteUser(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM users WHERE id=$1`, id)
	return err
}

// ==================== 成员 ====================

func (s *PostgresStore) AddMember(ctx context.Context, m *model.WorkspaceMember) error {
	invitedBy := toNullString(m.InvitedBy)
	_, err := s.db.ExecContext(ctx,
		`INSERT INTO workspace_members (id, workspace_id, user_id, role, invited_by) VALUES ($1,$2,$3,$4,$5)`,
		m.ID, m.WorkspaceID, m.UserID, m.Role, invitedBy)
	return err
}

func toNullString(s string) interface{} {
	if s == "" { return nil }
	return s
}

func (s *PostgresStore) GetMember(ctx context.Context, workspaceID, userID string) (*model.WorkspaceMember, error) {
	var m model.WorkspaceMember
	err := s.db.QueryRowContext(ctx,
		`SELECT id, workspace_id, user_id, role, COALESCE(invited_by,''), COALESCE(invited_at::text,''), COALESCE(joined_at::text,'') FROM workspace_members WHERE workspace_id=$1 AND user_id=$2`, workspaceID, userID,
	).Scan(&m.ID, &m.WorkspaceID, &m.UserID, &m.Role, &m.InvitedBy, &m.InvitedAt, &m.JoinedAt)
	if err != nil { return nil, err }
	return &m, nil
}

func (s *PostgresStore) ListMembers(ctx context.Context, workspaceID string) ([]model.WorkspaceMember, error) {
	rows, err := s.db.QueryContext(ctx, `SELECT id, workspace_id, user_id, role, COALESCE(invited_by,''), COALESCE(invited_at::text,''), COALESCE(joined_at::text,'') FROM workspace_members WHERE workspace_id=$1`, workspaceID)
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

func (s *PostgresStore) UpdateMemberRole(ctx context.Context, id string, role string) error {
	_, err := s.db.ExecContext(ctx, `UPDATE workspace_members SET role=$1 WHERE id=$2`, role, id)
	return err
}

func (s *PostgresStore) RemoveMember(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM workspace_members WHERE id=$1`, id)
	return err
}

// ==================== 简化的 Pipeline/ApiKey/Model/VirtualKey/Wallet/Config (PG 版本) ====================
// 以下为核心方法，完整实现在 Phase 1 最终版本中补充
// 此处使用与 SQLite 相同的逻辑，参数占位符改为 $N

func (s *PostgresStore) CreateRefreshToken(ctx context.Context, uid, hash string, exp string) (string, error) {
	id := uuid.New().String()
	_, err := s.db.ExecContext(ctx, `INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1,$2,$3,$4)`, id, uid, hash, exp)
	return id, err
}
func (s *PostgresStore) GetRefreshToken(ctx context.Context, hash string) (*model.RefreshToken, error) {
	var rt model.RefreshToken
	err := s.db.QueryRowContext(ctx, `SELECT id, user_id, token_hash, expires_at, created_at FROM refresh_tokens WHERE token_hash=$1`, hash).Scan(&rt.ID, &rt.UserID, &rt.TokenHash, &rt.ExpiresAt, &rt.CreatedAt)
	if err != nil { return nil, err }
	return &rt, nil
}
func (s *PostgresStore) RevokeRefreshToken(ctx context.Context, hash string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM refresh_tokens WHERE token_hash=$1`, hash)
	return err
}
func (s *PostgresStore) RevokeUserRefreshTokens(ctx context.Context, uid string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM refresh_tokens WHERE user_id=$1`, uid)
	return err
}

// Pipeline/Plugin/Policy/ApiKey/Model - 委托给通用实现
func (s *PostgresStore) ListPipelines(ctx context.Context, wsID string) ([]model.Pipeline, error) {
	// 空 wsID → 返回所有 pipeline（用于 config/poll 全局同步）
	if wsID == "" {
		return listHelper[model.Pipeline](ctx, s.db,
			`SELECT plan_id, COALESCE(org_id,''), COALESCE(workspace_id,''), COALESCE(name,''), COALESCE(description,''), tenant, stages, COALESCE(status,'draft'), created_at FROM pipelines`,
			func(p *model.Pipeline, scan func(...interface{}) error) error {
				var stagesJSON string
				err := scan(&p.PlanID, &p.OrgID, &p.WorkspaceID, &p.Name, &p.Description, &p.Tenant, &stagesJSON, &p.Status, &p.Created)
				if err == nil {
					p.ID = p.PlanID
					json.Unmarshal([]byte(stagesJSON), &p.Stages)
				}
				return err
			})
	}
	return listHelper[model.Pipeline](ctx, s.db,
		`SELECT plan_id, COALESCE(org_id,''), COALESCE(workspace_id,''), COALESCE(name,''), COALESCE(description,''), tenant, stages, COALESCE(status,'draft'), created_at FROM pipelines WHERE workspace_id=$1`,
		func(p *model.Pipeline, scan func(...interface{}) error) error {
			var stagesJSON string
			err := scan(&p.PlanID, &p.OrgID, &p.WorkspaceID, &p.Name, &p.Description, &p.Tenant, &stagesJSON, &p.Status, &p.Created)
			if err == nil {
				p.ID = p.PlanID
				json.Unmarshal([]byte(stagesJSON), &p.Stages)
			}
			return err
		}, wsID)
}
func (s *PostgresStore) GetPipeline(ctx context.Context, id string) (*model.Pipeline, error) {
	var p model.Pipeline
	var stagesJSON string
	err := s.db.QueryRowContext(ctx, `SELECT plan_id, COALESCE(org_id,''), COALESCE(workspace_id,''), COALESCE(name,''), COALESCE(description,''), tenant, stages, COALESCE(status,'draft'), created_at FROM pipelines WHERE plan_id=$1`, id).
		Scan(&p.PlanID, &p.OrgID, &p.WorkspaceID, &p.Name, &p.Description, &p.Tenant, &stagesJSON, &p.Status, &p.Created)
	if err != nil { return nil, err }
	p.ID = p.PlanID
	json.Unmarshal([]byte(stagesJSON), &p.Stages)
	return &p, nil
}
func (s *PostgresStore) CreatePipeline(ctx context.Context, p *model.Pipeline) error {
	stagesJSON, _ := json.Marshal(p.Stages)
	if p.Status == "" { p.Status = "draft" }
	_, err := s.db.ExecContext(ctx, `INSERT INTO pipelines (plan_id, org_id, workspace_id, name, description, tenant, stages, status, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)`,
		p.PlanID, p.OrgID, p.WorkspaceID, p.Name, p.Description, p.Tenant, string(stagesJSON), p.Status, p.Created)
	if err != nil { return err }
	return s.IncrementConfigVersion(ctx, "pipeline")
}
func (s *PostgresStore) UpdatePipeline(ctx context.Context, id string, p *model.Pipeline) error {
	stagesJSON, _ := json.Marshal(p.Stages)
	stagesStr := string(stagesJSON)
	// stages 保护: nil 或无效 JSON → 保留原值不覆盖
	if stagesStr == "null" || stagesStr == "" || stagesStr == "[null]" {
		stagesStr = "" // 用 COALESCE(NULLIF('',''),stages) 保护
	}
	_, err := s.db.ExecContext(ctx, `UPDATE pipelines SET name=COALESCE(NULLIF($1,''),name), description=COALESCE(NULLIF($2,''),description), tenant=COALESCE(NULLIF($3,''),tenant), stages=COALESCE(NULLIF($4,''),stages), workspace_id=COALESCE(NULLIF($5,''),workspace_id), status=COALESCE(NULLIF($6,''),status) WHERE plan_id=$7`,
		p.Name, p.Description, p.Tenant, stagesStr, p.WorkspaceID, p.Status, id)
	if err != nil { return err }
	return s.IncrementConfigVersion(ctx, "pipeline")
}
func (s *PostgresStore) DeletePipeline(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM pipelines WHERE plan_id=$1`, id)
	if err != nil { return err }
	return s.IncrementConfigVersion(ctx, "pipeline")
}

func (s *PostgresStore) ListPlugins(ctx context.Context, wsID string) ([]model.PluginMeta, error) {
	return listHelper[model.PluginMeta](ctx, s.db,
		`SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, type, COALESCE(version,''), COALESCE(description,''), COALESCE(config,'{}') FROM plugins WHERE workspace_id=$1 OR workspace_id IS NULL OR workspace_id=''`,
		func(p *model.PluginMeta, scan func(...interface{}) error) error {
			return scan(&p.ID, &p.OrgID, &p.WorkspaceID, &p.Name, &p.Type, &p.Version, &p.Description, &p.Config)
		}, wsID)
}
func (s *PostgresStore) CreatePlugin(ctx context.Context, p *model.PluginMeta) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO plugins (id, org_id, workspace_id, name, type, version, description, config) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)`,
		p.ID, p.OrgID, p.WorkspaceID, p.Name, p.Type, p.Version, p.Description, p.Config)
	if err != nil { return err }
	return s.IncrementConfigVersion(ctx, "plugin")
}
func (s *PostgresStore) UpdatePlugin(ctx context.Context, id string, p *model.PluginMeta) error {
	_, err := s.db.ExecContext(ctx, `UPDATE plugins SET name=$1, type=$2, version=$3, description=$4, config=$5 WHERE id=$6`,
		p.Name, p.Type, p.Version, p.Description, p.Config, id)
	if err != nil { return err }
	return s.IncrementConfigVersion(ctx, "plugin")
}
func (s *PostgresStore) DeletePlugin(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM plugins WHERE id=$1`, id)
	if err != nil { return err }
	return s.IncrementConfigVersion(ctx, "plugin")
}

func (s *PostgresStore) ListPolicies(ctx context.Context, workspaceID string) ([]model.Policy, error) {
	// 若指定 workspaceID，仅返回该工作空间及其组织级策略
	if workspaceID != "" {
		return listHelper[model.Policy](ctx, s.db,
			`SELECT id, name, type, content, created_at FROM policies WHERE workspace_id=$1 OR (org_id IS NOT NULL AND workspace_id IS NULL) ORDER BY created_at`,
			func(p *model.Policy, scan func(...interface{}) error) error {
				return scan(&p.ID, &p.Name, &p.Type, &p.Content, &p.CreatedAt)
			}, workspaceID)
	}
	// 无 workspaceID 时返回全局策略（仅 admin 使用）
	return listHelper[model.Policy](ctx, s.db,
		`SELECT id, name, type, content, created_at FROM policies WHERE workspace_id IS NULL ORDER BY created_at`,
		func(p *model.Policy, scan func(...interface{}) error) error {
			return scan(&p.ID, &p.Name, &p.Type, &p.Content, &p.CreatedAt)
		})
}
func (s *PostgresStore) CreatePolicy(ctx context.Context, p *model.Policy) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO policies (id, org_id, workspace_id, name, type, content, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)`,
		p.ID, nilSafeString(p.OrgID), nilSafeString(p.WorkspaceID), p.Name, p.Type, p.Content, p.CreatedAt)
	return err
}
func (s *PostgresStore) DeletePolicy(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM policies WHERE id=$1`, id)
	return err
}

func (s *PostgresStore) ListApiKeys(ctx context.Context, wsID string) ([]model.ApiKey, error) {
	return listHelper[model.ApiKey](ctx, s.db,
		`SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, key, tenant_id, status, created_at, COALESCE(last_used,'') FROM apikeys WHERE workspace_id=$1 OR workspace_id IS NULL OR workspace_id=''`,
		func(k *model.ApiKey, scan func(...interface{}) error) error {
			return scan(&k.ID, &k.OrgID, &k.WorkspaceID, &k.Name, &k.Key, &k.TenantID, &k.Status, &k.CreatedAt, &k.LastUsed)
		}, wsID)
}
func (s *PostgresStore) GetApiKey(ctx context.Context, id string) (*model.ApiKey, error) {
	var k model.ApiKey
	err := s.db.QueryRowContext(ctx, `SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, key, tenant_id, status, created_at, COALESCE(last_used,'') FROM apikeys WHERE id=$1`, id).
		Scan(&k.ID, &k.OrgID, &k.WorkspaceID, &k.Name, &k.Key, &k.TenantID, &k.Status, &k.CreatedAt, &k.LastUsed)
	if err != nil { return nil, err }
	return &k, nil
}
func (s *PostgresStore) CreateApiKey(ctx context.Context, k *model.ApiKey) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO apikeys (id, org_id, workspace_id, name, key, tenant_id, status, created_at, last_used) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)`,
		k.ID, k.OrgID, k.WorkspaceID, k.Name, k.Key, k.TenantID, k.Status, k.CreatedAt, k.LastUsed)
	return err
}
func (s *PostgresStore) UpdateApiKey(ctx context.Context, k *model.ApiKey) error {
	_, err := s.db.ExecContext(ctx, `UPDATE apikeys SET name=$1, key=$2, tenant_id=$3, status=$4, last_used=$5 WHERE id=$6`, k.Name, k.Key, k.TenantID, k.Status, k.LastUsed, k.ID)
	return err
}
func (s *PostgresStore) DeleteApiKey(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM apikeys WHERE id=$1`, id)
	return err
}

func (s *PostgresStore) ListModels(ctx context.Context, wsID string) ([]model.ModelConfig, error) {
	return listHelper[model.ModelConfig](ctx, s.db,
		`SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, upstream_url, upstream_model, COALESCE(api_key,''), COALESCE(api_key_header,'Authorization'), use_proxy, COALESCE(proxy_url,''), is_default, COALESCE(supported_versions,'[]'), status FROM models WHERE workspace_id=$1 OR workspace_id IS NULL OR workspace_id=''`,
		func(m *model.ModelConfig, scan func(...interface{}) error) error {
			var versionsJSON string
			var useProxy, isDefault int
			err := scan(&m.ID, &m.OrgID, &m.WorkspaceID, &m.Name, &m.UpstreamURL, &m.UpstreamModel, &m.ApiKey, &m.ApiKeyHeader, &useProxy, &m.ProxyURL, &isDefault, &versionsJSON, &m.Status)
			m.UseProxy = useProxy != 0
			m.IsDefault = isDefault != 0
			json.Unmarshal([]byte(versionsJSON), &m.SupportedVersions)
			return err
		}, wsID)
}
func (s *PostgresStore) GetModel(ctx context.Context, id string) (*model.ModelConfig, error) {
	var m model.ModelConfig
	var versionsJSON string
	var useProxy, isDefault int
	err := s.db.QueryRowContext(ctx, `SELECT id, COALESCE(org_id,''), COALESCE(workspace_id,''), name, upstream_url, upstream_model, COALESCE(api_key,''), COALESCE(api_key_header,'Authorization'), use_proxy, COALESCE(proxy_url,''), is_default, COALESCE(supported_versions,'[]'), status FROM models WHERE id=$1`, id).
		Scan(&m.ID, &m.OrgID, &m.WorkspaceID, &m.Name, &m.UpstreamURL, &m.UpstreamModel, &m.ApiKey, &m.ApiKeyHeader, &useProxy, &m.ProxyURL, &isDefault, &versionsJSON, &m.Status)
	m.UseProxy = useProxy != 0
	m.IsDefault = isDefault != 0
	json.Unmarshal([]byte(versionsJSON), &m.SupportedVersions)
	if err != nil { return nil, err }
	return &m, nil
}
func (s *PostgresStore) CreateModel(ctx context.Context, m *model.ModelConfig) error {
	versionsJSON, _ := json.Marshal(m.SupportedVersions)
	up, idf := 0, 0
	if m.UseProxy { up = 1 }
	if m.IsDefault { idf = 1 }
	_, err := s.db.ExecContext(ctx, `INSERT INTO models (id, org_id, workspace_id, name, upstream_url, upstream_model, api_key, api_key_header, use_proxy, proxy_url, is_default, supported_versions, status) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)`,
		m.ID, m.OrgID, m.WorkspaceID, m.Name, m.UpstreamURL, m.UpstreamModel, m.ApiKey, m.ApiKeyHeader, up, m.ProxyURL, idf, string(versionsJSON), m.Status)
	if err != nil { return err }
	return s.IncrementConfigVersion(ctx, "model")
}
func (s *PostgresStore) UpdateModel(ctx context.Context, id string, m *model.ModelConfig) error {
	versionsJSON, _ := json.Marshal(m.SupportedVersions)
	up, idf := 0, 0
	if m.UseProxy { up = 1 }
	if m.IsDefault { idf = 1 }
	_, err := s.db.ExecContext(ctx, `UPDATE models SET name=$1, upstream_url=$2, upstream_model=$3, api_key=$4, api_key_header=$5, use_proxy=$6, proxy_url=$7, is_default=$8, supported_versions=$9, status=$10 WHERE id=$11`,
		m.Name, m.UpstreamURL, m.UpstreamModel, m.ApiKey, m.ApiKeyHeader, up, m.ProxyURL, idf, string(versionsJSON), m.Status, id)
	if err != nil { return err }
	return s.IncrementConfigVersion(ctx, "model")
}
func (s *PostgresStore) DeleteModel(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM models WHERE id=$1`, id)
	if err != nil { return err }
	return s.IncrementConfigVersion(ctx, "model")
}

func (s *PostgresStore) CreateVirtualKey(ctx context.Context, vk *model.VirtualKey) (*model.VirtualKey, error) {
	_, err := s.db.ExecContext(ctx, `INSERT INTO virtual_keys (id, workspace_id, name, key_prefix, key_hash, type, provider_key_encrypted, allowed_models, rate_limit_rpm, rate_limit_tpm, spend_limit_usd_micro, status, created_by) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)`,
		vk.ID, vk.WorkspaceID, vk.Name, vk.KeyPrefix, vk.KeyHash, vk.Type, vk.ProviderKeyEncrypted, vk.AllowedModels, vk.RateLimitRPM, vk.RateLimitTPM, vk.SpendLimitUSDMicro, vk.Status, vk.CreatedBy)
	return vk, err
}
func (s *PostgresStore) GetVirtualKey(ctx context.Context, id string) (*model.VirtualKey, error) {
	var vk model.VirtualKey
	err := s.db.QueryRowContext(ctx, `SELECT id, workspace_id, name, key_prefix, key_hash, type, COALESCE(provider_key_encrypted,''), COALESCE(allowed_models,'[]'), rate_limit_rpm, rate_limit_tpm, spend_limit_usd_micro, status, last_used_at, created_at, created_by FROM virtual_keys WHERE id=$1`, id).
		Scan(&vk.ID, &vk.WorkspaceID, &vk.Name, &vk.KeyPrefix, &vk.KeyHash, &vk.Type, &vk.ProviderKeyEncrypted, &vk.AllowedModels, &vk.RateLimitRPM, &vk.RateLimitTPM, &vk.SpendLimitUSDMicro, &vk.Status, &vk.LastUsedAt, &vk.CreatedAt, &vk.CreatedBy)
	if err != nil { return nil, err }
	return &vk, nil
}
func (s *PostgresStore) GetVirtualKeyByHash(ctx context.Context, hash string) (*model.VirtualKey, error) {
	var vk model.VirtualKey
	err := s.db.QueryRowContext(ctx, `SELECT id, workspace_id, name, key_prefix, key_hash, type, COALESCE(provider_key_encrypted,''), COALESCE(allowed_models,'[]'), rate_limit_rpm, rate_limit_tpm, spend_limit_usd_micro, status, last_used_at, created_at, created_by FROM virtual_keys WHERE key_hash=$1`, hash).
		Scan(&vk.ID, &vk.WorkspaceID, &vk.Name, &vk.KeyPrefix, &vk.KeyHash, &vk.Type, &vk.ProviderKeyEncrypted, &vk.AllowedModels, &vk.RateLimitRPM, &vk.RateLimitTPM, &vk.SpendLimitUSDMicro, &vk.Status, &vk.LastUsedAt, &vk.CreatedAt, &vk.CreatedBy)
	if err != nil { return nil, err }
	return &vk, nil
}
func (s *PostgresStore) ListVirtualKeys(ctx context.Context, wsID string) ([]model.VirtualKey, error) {
	return listHelper[model.VirtualKey](ctx, s.db,
		`SELECT id, workspace_id, name, key_prefix, key_hash, type, COALESCE(provider_key_encrypted,''), COALESCE(allowed_models,'[]'), rate_limit_rpm, rate_limit_tpm, spend_limit_usd_micro, status, last_used_at, created_at, created_by FROM virtual_keys WHERE workspace_id=$1 ORDER BY created_at`,
		func(vk *model.VirtualKey, scan func(...interface{}) error) error {
			return scan(&vk.ID, &vk.WorkspaceID, &vk.Name, &vk.KeyPrefix, &vk.KeyHash, &vk.Type, &vk.ProviderKeyEncrypted, &vk.AllowedModels, &vk.RateLimitRPM, &vk.RateLimitTPM, &vk.SpendLimitUSDMicro, &vk.Status, &vk.LastUsedAt, &vk.CreatedAt, &vk.CreatedBy)
		}, wsID)
}
func (s *PostgresStore) UpdateVirtualKey(ctx context.Context, id string, updates map[string]interface{}) error {
	return updateHelper(ctx, s.db, "virtual_keys", id, updates)
}
func (s *PostgresStore) RevokeVirtualKey(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `UPDATE virtual_keys SET status='revoked' WHERE id=$1`, id)
	return err
}

func (s *PostgresStore) GetWallet(ctx context.Context, wsID string) (*model.Wallet, error) {
	var w model.Wallet
	err := s.db.QueryRowContext(ctx, `SELECT id, workspace_id, balance_usd_micro, overdraft_limit_micro, last_credit_at, created_at, updated_at FROM wallets WHERE workspace_id=$1`, wsID).
		Scan(&w.ID, &w.WorkspaceID, &w.BalanceUSDMicro, &w.OverdraftLimitMicro, &w.LastCreditAt, &w.CreatedAt, &w.UpdatedAt)
	if err != nil { return nil, err }
	return &w, nil
}
func (s *PostgresStore) CreateWallet(ctx context.Context, w *model.Wallet) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO wallets (id, workspace_id, balance_usd_micro, overdraft_limit_micro) VALUES ($1,$2,$3,$4)`, w.ID, w.WorkspaceID, w.BalanceUSDMicro, w.OverdraftLimitMicro)
	return err
}
func (s *PostgresStore) UpdateWalletBalance(ctx context.Context, wsID string, amount int64) error {
	_, err := s.db.ExecContext(ctx, `UPDATE wallets SET balance_usd_micro=balance_usd_micro+$1, updated_at=NOW() WHERE workspace_id=$2`, amount, wsID)
	return err
}
func (s *PostgresStore) ListTransactions(ctx context.Context, wsID string, limit, offset int) ([]model.Transaction, error) {
	return listHelper[model.Transaction](ctx, s.db,
		`SELECT id, workspace_id, wallet_id, type, amount_usd_micro, balance_after_micro, COALESCE(description,''), COALESCE(trace_id,''), COALESCE(metadata,'{}'), created_at FROM transactions WHERE workspace_id=$1 ORDER BY created_at DESC LIMIT $2 OFFSET $3`,
		func(t *model.Transaction, scan func(...interface{}) error) error {
			return scan(&t.ID, &t.WorkspaceID, &t.WalletID, &t.Type, &t.AmountUSDMicro, &t.BalanceAfterMicro, &t.Description, &t.TraceID, &t.Metadata, &t.CreatedAt)
		}, wsID, limit, offset)
}
func (s *PostgresStore) CreateTransaction(ctx context.Context, t *model.Transaction) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO transactions (id, workspace_id, wallet_id, type, amount_usd_micro, balance_after_micro, description, trace_id, metadata) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)`,
		t.ID, t.WorkspaceID, t.WalletID, t.Type, t.AmountUSDMicro, t.BalanceAfterMicro, t.Description, t.TraceID, t.Metadata)
	return err
}

func (s *PostgresStore) GetConfigVersions(ctx context.Context) (model.ConfigVersion, error) {
	var v model.ConfigVersion
	m := map[string]*int{"pipeline_version": &v.PipelineVersion, "policy_version": &v.PolicyVersion, "plugin_version": &v.PluginVersion, "storage_version": &v.StorageVersion, "model_version": &v.ModelVersion}
	for key, ptr := range m {
		s.db.QueryRowContext(ctx, `SELECT value FROM config_versions WHERE key=$1`, key).Scan(ptr)
	}
	return v, nil
}
func (s *PostgresStore) IncrementConfigVersion(ctx context.Context, key string) error {
	_, err := s.db.ExecContext(ctx, `UPDATE config_versions SET value=value+1 WHERE key=$1`, key+"_version")
	return err
}
func (s *PostgresStore) ListDataPlaneConfigs(ctx context.Context) ([]model.DataPlaneConfig, error) {
	return listHelper[model.DataPlaneConfig](ctx, s.db, `SELECT id, key, value, created_at FROM data_plane_configs`,
		func(c *model.DataPlaneConfig, scan func(...interface{}) error) error { return scan(&c.ID, &c.Key, &c.Value, &c.CreatedAt) })
}
func (s *PostgresStore) CreateDataPlaneConfig(ctx context.Context, c *model.DataPlaneConfig) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO data_plane_configs (id, key, value) VALUES ($1,$2,$3)`, c.ID, c.Key, c.Value)
	return err
}
func (s *PostgresStore) UpdateDataPlaneConfig(ctx context.Context, id string, c *model.DataPlaneConfig) error {
	_, err := s.db.ExecContext(ctx, `UPDATE data_plane_configs SET key=$1, value=$2 WHERE id=$3`, c.Key, c.Value, id)
	return err
}
func (s *PostgresStore) DeleteDataPlaneConfig(ctx context.Context, id string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM data_plane_configs WHERE id=$1`, id)
	return err
}
func (s *PostgresStore) GetSettings(ctx context.Context, wsID string) (map[string]string, error) {
	rows, err := s.db.QueryContext(ctx, `SELECT key, value FROM settings WHERE workspace_id=$1`, wsID)
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
func (s *PostgresStore) UpdateSettings(ctx context.Context, wsID string, settings map[string]string) error {
	for k, v := range settings {
		_, err := s.db.ExecContext(ctx, `INSERT INTO settings (key, workspace_id, value) VALUES ($1,$2,$3) ON CONFLICT(key, workspace_id) DO UPDATE SET value=EXCLUDED.value`, k, wsID, v)
		if err != nil { return err }
	}
	return nil
}

// ==================== 通用辅助函数 ====================

// ==================== 审计事件 ====================

func (s *PostgresStore) GetAuditEvents(ctx context.Context, workspaceID string, period string) ([]map[string]interface{}, error) {
	var traceCount int
	s.db.QueryRowContext(ctx, `SELECT COUNT(*) FROM traces WHERE tenant_id=$1`, workspaceID).Scan(&traceCount)
	var keyCount int
	s.db.QueryRowContext(ctx, `SELECT COUNT(*) FROM apikeys WHERE workspace_id=$1`, workspaceID).Scan(&keyCount)
	return []map[string]interface{}{
		{"type": "total_traces", "label": "总执行次数", "value": traceCount, "icon": "Activity"},
		{"type": "active_keys", "label": "活跃 API Key", "value": keyCount, "icon": "Key"},
		{"type": "guardrail_blocks", "label": "Guardrail 拦截", "value": 0, "icon": "Shield"},
		{"type": "pii_detections", "label": "PII 检测", "value": 0, "icon": "Eye"},
		{"type": "risk_distribution", "label": "风险分布", "levels": []map[string]interface{}{
			{"level": "low", "count": traceCount, "color": "#00d4aa"},
		}, "icon": "BarChart"},
	}, nil
}

// nilSafeString 将空字符串转为 nil，用于可选外键字段
func nilSafeString(s string) interface{} {
	if s == "" {
		return nil
	}
	return s
}

func listHelper[T any](ctx context.Context, db *sql.DB, query string, scanFn func(*T, func(...interface{}) error) error, args ...interface{}) ([]T, error) {
	rows, err := db.QueryContext(ctx, query, args...)
	if err != nil { return nil, err }
	defer rows.Close()
	var results []T
	for rows.Next() {
		var item T
		if err := scanFn(&item, rows.Scan); err != nil { return nil, err }
		results = append(results, item)
	}
	return results, nil
}

func updateHelper(ctx context.Context, db *sql.DB, table, id string, updates map[string]interface{}) error {
	var setClause string
	var args []interface{}
	i := 1
	for col, val := range updates {
		if setClause != "" { setClause += ", " }
		setClause += fmt.Sprintf("%s=$%d", col, i)
		args = append(args, val)
		i++
	}
	if setClause == "" { return nil }
	args = append(args, id)
	_, err := db.ExecContext(ctx, fmt.Sprintf("UPDATE %s SET %s WHERE id=$%d", table, setClause, i), args...)
	return err
}

func getPostgresMigrations() []string {
	return []string{
		`CREATE TABLE IF NOT EXISTS organizations (id TEXT PRIMARY KEY, name TEXT NOT NULL, slug TEXT NOT NULL UNIQUE, plan TEXT NOT NULL DEFAULT 'free', logo_url TEXT, primary_color TEXT DEFAULT '#6c5ce7', settings TEXT DEFAULT '{}', created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())`,
		`CREATE TABLE IF NOT EXISTS workspaces (id TEXT PRIMARY KEY, org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE, name TEXT NOT NULL, slug TEXT NOT NULL, description TEXT, settings TEXT DEFAULT '{}', created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), UNIQUE(org_id, slug))`,
		`CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, phone TEXT, display_name TEXT, avatar_url TEXT, auth_provider TEXT NOT NULL, auth_provider_id TEXT, password_hash TEXT, org_id TEXT REFERENCES organizations(id), settings TEXT DEFAULT '{}', last_login_at TIMESTAMPTZ, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())`,
		// migrate: add org_id to existing users table if column missing
		`DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='users' AND column_name='org_id') THEN ALTER TABLE users ADD COLUMN org_id TEXT REFERENCES organizations(id); END IF; END $$`,
		`CREATE TABLE IF NOT EXISTS workspace_members (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, role TEXT NOT NULL DEFAULT 'developer', invited_by TEXT REFERENCES users(id), invited_at TIMESTAMPTZ, joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), UNIQUE(workspace_id, user_id))`,
		`CREATE TABLE IF NOT EXISTS refresh_tokens (id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, token_hash TEXT NOT NULL UNIQUE, expires_at TIMESTAMPTZ NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())`,
		`CREATE TABLE IF NOT EXISTS virtual_keys (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE, name TEXT NOT NULL, key_prefix TEXT NOT NULL, key_hash TEXT NOT NULL UNIQUE, type TEXT NOT NULL DEFAULT 'platform', provider_key_encrypted TEXT, allowed_models TEXT DEFAULT '[]', rate_limit_rpm INTEGER DEFAULT 60, rate_limit_tpm INTEGER DEFAULT 100000, spend_limit_usd_micro BIGINT DEFAULT 0, status TEXT NOT NULL DEFAULT 'active', last_used_at TIMESTAMPTZ, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), created_by TEXT NOT NULL REFERENCES users(id))`,
		`CREATE TABLE IF NOT EXISTS wallets (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL UNIQUE REFERENCES workspaces(id), balance_usd_micro BIGINT NOT NULL DEFAULT 0, overdraft_limit_micro BIGINT NOT NULL DEFAULT 0, last_credit_at TIMESTAMPTZ, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())`,
		`CREATE TABLE IF NOT EXISTS transactions (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id), wallet_id TEXT NOT NULL REFERENCES wallets(id), type TEXT NOT NULL, amount_usd_micro BIGINT NOT NULL, balance_after_micro BIGINT NOT NULL, description TEXT, trace_id TEXT, metadata TEXT DEFAULT '{}', created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())`,
		`CREATE TABLE IF NOT EXISTS pipelines (plan_id TEXT PRIMARY KEY, org_id TEXT REFERENCES organizations(id), workspace_id TEXT REFERENCES workspaces(id), name TEXT NOT NULL DEFAULT '', description TEXT NOT NULL DEFAULT '', tenant TEXT NOT NULL, stages TEXT NOT NULL, created_at TEXT NOT NULL)`,
		`DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='pipelines' AND column_name='status') THEN ALTER TABLE pipelines ADD COLUMN status TEXT NOT NULL DEFAULT 'draft'; END IF; END $$`,
		`CREATE TABLE IF NOT EXISTS plugins (id TEXT PRIMARY KEY, org_id TEXT, workspace_id TEXT, name TEXT NOT NULL, type TEXT NOT NULL, version TEXT, description TEXT, config TEXT DEFAULT '{}')`,
		`CREATE TABLE IF NOT EXISTS policies (id TEXT PRIMARY KEY, org_id TEXT REFERENCES organizations(id), workspace_id TEXT REFERENCES workspaces(id), name TEXT NOT NULL, type TEXT NOT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL)`,
		`CREATE TABLE IF NOT EXISTS apikeys (id TEXT PRIMARY KEY, org_id TEXT, workspace_id TEXT, name TEXT NOT NULL, key TEXT NOT NULL UNIQUE, tenant_id TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, last_used TEXT)`,
		`CREATE TABLE IF NOT EXISTS models (id TEXT PRIMARY KEY, org_id TEXT, workspace_id TEXT, name TEXT NOT NULL UNIQUE, upstream_url TEXT NOT NULL, upstream_model TEXT NOT NULL, api_key TEXT, api_key_header TEXT DEFAULT 'Authorization', use_proxy INTEGER NOT NULL DEFAULT 0, proxy_url TEXT, is_default INTEGER NOT NULL DEFAULT 0, supported_versions TEXT, status TEXT NOT NULL DEFAULT 'active')`,
		`CREATE TABLE IF NOT EXISTS traces (trace_id TEXT PRIMARY KEY, org_id TEXT REFERENCES organizations(id), workspace_id TEXT REFERENCES workspaces(id), model TEXT NOT NULL, tenant_id TEXT NOT NULL, execution_state TEXT NOT NULL, created_at TEXT NOT NULL, signature TEXT)`,
		// migrate: add org_id/workspace_id to existing traces table if columns missing
		`DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='traces' AND column_name='org_id') THEN ALTER TABLE traces ADD COLUMN org_id TEXT REFERENCES organizations(id); END IF; END $$`,
		`DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='traces' AND column_name='workspace_id') THEN ALTER TABLE traces ADD COLUMN workspace_id TEXT REFERENCES workspaces(id); END IF; END $$`,
		`CREATE TABLE IF NOT EXISTS config_versions (key TEXT PRIMARY KEY, value INTEGER NOT NULL DEFAULT 0)`,
		`CREATE TABLE IF NOT EXISTS data_plane_configs (id TEXT PRIMARY KEY, key TEXT NOT NULL, value TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT NOW())`,
		`CREATE TABLE IF NOT EXISTS settings (key TEXT NOT NULL, workspace_id TEXT NOT NULL DEFAULT 'default', value TEXT NOT NULL, PRIMARY KEY (key, workspace_id))`,
		`CREATE TABLE IF NOT EXISTS invoices (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id), org_id TEXT REFERENCES organizations(id), stripe_invoice_id TEXT, stripe_payment_intent_id TEXT, amount_usd_micro BIGINT NOT NULL DEFAULT 0, currency TEXT NOT NULL DEFAULT 'usd', status TEXT NOT NULL DEFAULT 'draft', description TEXT, period_start TIMESTAMPTZ, period_end TIMESTAMPTZ, paid_at TIMESTAMPTZ, metadata TEXT DEFAULT '{}', created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())`,
		// ==================== 聊天会话 ====================
		`CREATE TABLE IF NOT EXISTS conversations (id TEXT PRIMARY KEY, workspace_id TEXT, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, title TEXT NOT NULL DEFAULT '新对话', model TEXT NOT NULL DEFAULT '', created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())`,
		`CREATE TABLE IF NOT EXISTS messages (id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, role TEXT NOT NULL, content TEXT NOT NULL, model TEXT DEFAULT '', tokens INTEGER DEFAULT 0, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())`,
		`CREATE INDEX IF NOT EXISTS idx_conversations_user_id ON conversations(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_conversations_updated_at ON conversations(updated_at DESC)`,
		`CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id, created_at)`,
	}
}

func (s *PostgresStore) ListUserMemberships(ctx context.Context, userID string) ([]model.WorkspaceMember, error) {
	rows, err := s.db.QueryContext(ctx, `SELECT id, workspace_id, user_id, role, COALESCE(invited_by,''), COALESCE(invited_at::text,''), COALESCE(joined_at::text,'') FROM workspace_members WHERE user_id=$1`, userID)
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
func (s *PostgresStore) GetUserByPhone(ctx context.Context, phone string) (*model.User, error) { return nil, nil }

// ==================== 聊天会话 ====================

func (s *PostgresStore) ListConversations(ctx context.Context, userID string) ([]model.Conversation, error) {
	return listHelper[model.Conversation](ctx, s.db,
		`SELECT id, COALESCE(workspace_id,''), user_id, title, model, created_at::text, updated_at::text FROM conversations WHERE user_id=$1 ORDER BY updated_at DESC`,
		func(c *model.Conversation, scan func(...interface{}) error) error { return scan(&c.ID, &c.WorkspaceID, &c.UserID, &c.Title, &c.Model, &c.CreatedAt, &c.UpdatedAt) },
		userID)
}
func (s *PostgresStore) CreateConversation(ctx context.Context, conv *model.Conversation) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO conversations (id, workspace_id, user_id, title, model, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7)`,
		conv.ID, conv.WorkspaceID, conv.UserID, conv.Title, conv.Model, conv.CreatedAt, conv.UpdatedAt)
	return err
}
func (s *PostgresStore) GetConversation(ctx context.Context, id string) (*model.Conversation, error) {
	var c model.Conversation
	err := s.db.QueryRowContext(ctx, `SELECT id, COALESCE(workspace_id,''), user_id, title, model, created_at::text, updated_at::text FROM conversations WHERE id=$1`, id).
		Scan(&c.ID, &c.WorkspaceID, &c.UserID, &c.Title, &c.Model, &c.CreatedAt, &c.UpdatedAt)
	if err != nil { return nil, err }
	return &c, nil
}
func (s *PostgresStore) UpdateConversation(ctx context.Context, id string, updates map[string]interface{}) error {
	sets, args := []string{}, []interface{}{}
	i := 1
	for k, v := range updates {
		sets = append(sets, fmt.Sprintf("%s=$%d", k, i)); args = append(args, v); i++
	}
	args = append(args, id)
	_, err := s.db.ExecContext(ctx, fmt.Sprintf(`UPDATE conversations SET %s, updated_at=NOW() WHERE id=$%d`, strings.Join(sets, ","), i), args...)
	return err
}
func (s *PostgresStore) DeleteConversation(ctx context.Context, id string) error {
	_, _ = s.db.ExecContext(ctx, `DELETE FROM messages WHERE conversation_id=$1`, id)
	_, err := s.db.ExecContext(ctx, `DELETE FROM conversations WHERE id=$1`, id)
	return err
}
func (s *PostgresStore) ListMessages(ctx context.Context, conversationID string, limit int) ([]model.Message, error) {
	if limit <= 0 { limit = 100 }
	return listHelper[model.Message](ctx, s.db,
		fmt.Sprintf(`SELECT id, conversation_id, role, content, COALESCE(model,''), tokens, created_at::text FROM messages WHERE conversation_id=$1 ORDER BY created_at ASC LIMIT %d`, limit),
		func(m *model.Message, scan func(...interface{}) error) error { return scan(&m.ID, &m.ConversationID, &m.Role, &m.Content, &m.Model, &m.Tokens, &m.CreatedAt) },
		conversationID)
}
func (s *PostgresStore) CreateMessage(ctx context.Context, msg *model.Message) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO messages (id, conversation_id, role, content, model, tokens, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)`,
		msg.ID, msg.ConversationID, msg.Role, msg.Content, msg.Model, msg.Tokens, msg.CreatedAt)
	return err
}
func (s *PostgresStore) DeleteMessagesByConversation(ctx context.Context, conversationID string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM messages WHERE conversation_id=$1`, conversationID)
	return err
}

// ListDpTraces 从 Rust DP 的 dp_traces 表读取 trace 记录（按 workspace/tenant 隔离，返回完整摘要）
func (s *PostgresStore) ListDpTraces(ctx context.Context, workspaceID string, limit int) ([]map[string]interface{}, error) {
	if limit <= 0 { limit = 50 }
	rows, err := s.db.QueryContext(ctx, `SELECT trace_id, tenant_id, trace_data->>'model' as model,
		trace_data->>'created_at' as created_at,
		trace_data->>'execution_state' as execution_state,
		trace_data->'proof_levels' as proof_levels,
		trace_data->>'signature' as signature,
		trace_data->>'session_id' as session_id,
		COALESCE((trace_data->'observations'->>'cost_estimated_usd')::float, 0) as cost_usd,
		COALESCE((trace_data->'observations'->>'tokens_count')::int, 0) as tokens_count,
		trace_data->'output'->>'safety' as safety
		FROM dp_traces WHERE tenant_id=$1 ORDER BY created_at DESC LIMIT $2`, workspaceID, limit)
	if err != nil { return nil, err }
	defer rows.Close()
	var traces []map[string]interface{}
	for rows.Next() {
		var traceID, tenantID, model, createdAt string
		var executionState, proofLevels, signature, sessionID, safety interface{}
		var costUsd float64
		var tokensCount int
		if err := rows.Scan(&traceID, &tenantID, &model, &createdAt, &executionState, &proofLevels, &signature, &sessionID, &costUsd, &tokensCount, &safety); err != nil { continue }
		traces = append(traces, map[string]interface{}{
			"trace_id": traceID, "tenant_id": tenantID, "model": model,
			"created_at": createdAt, "execution_state": executionState,
			"proof_levels": proofLevels, "signature": signature,
			"session_id": sessionID, "cost_estimated_usd": costUsd,
			"tokens_count": tokensCount, "safety": safety,
		})
	}
	return traces, nil
}

// ListDpTracesByWorkspaces 按多个 workspace/tenant 查询 traces（企业级：同一组织下所有 workspace）
func (s *PostgresStore) ListDpTracesByWorkspaces(ctx context.Context, workspaceIDs []string, limit int) ([]map[string]interface{}, error) {
	if limit <= 0 { limit = 50 }
	if len(workspaceIDs) == 0 { return []map[string]interface{}{}, nil }
	rows, err := s.db.QueryContext(ctx, `SELECT trace_id, tenant_id, trace_data->>'model' as model,
		trace_data->>'created_at' as created_at,
		trace_data->>'execution_state' as execution_state,
		trace_data->'proof_levels' as proof_levels,
		trace_data->>'signature' as signature,
		trace_data->>'session_id' as session_id,
		COALESCE((trace_data->'observations'->>'cost_estimated_usd')::float, 0) as cost_usd,
		COALESCE((trace_data->'observations'->>'tokens_count')::int, 0) as tokens_count,
		trace_data->'output'->>'safety' as safety
		FROM dp_traces WHERE tenant_id = ANY($1) ORDER BY created_at DESC LIMIT $2`, pq.Array(workspaceIDs), limit)
	if err != nil { return nil, err }
	defer rows.Close()
	var traces []map[string]interface{}
	for rows.Next() {
		var traceID, tenantID, model, createdAt string
		var executionState, proofLevels, signature, sessionID, safety interface{}
		var costUsd float64
		var tokensCount int
		if err := rows.Scan(&traceID, &tenantID, &model, &createdAt, &executionState, &proofLevels, &signature, &sessionID, &costUsd, &tokensCount, &safety); err != nil { continue }
		traces = append(traces, map[string]interface{}{
			"trace_id": traceID, "tenant_id": tenantID, "model": model,
			"created_at": createdAt, "execution_state": executionState,
			"proof_levels": proofLevels, "signature": signature,
			"session_id": sessionID, "cost_estimated_usd": costUsd,
			"tokens_count": tokensCount, "safety": safety,
		})
	}
	return traces, nil
}
