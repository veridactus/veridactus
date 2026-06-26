// VERIDACTUS 控制平面 — 存储后端抽象接口
// 支持 PostgreSQL (生产) 和 SQLite (单机开发) 双实现
package store

import (
	"context"

	"github.com/veridactus/control-plane/internal/model"
)

// StoreFacade 存储后端统一接口
// 所有写操作通过此接口，支持透明切换 PG/SQLite
type StoreFacade interface {
	// ==================== 数据库管理 ====================
	RunMigrations(ctx context.Context) error
	HealthCheck(ctx context.Context) error
	Close() error

	// ==================== 组织 ====================
	CreateOrganization(ctx context.Context, org *model.Organization) error
	GetOrganization(ctx context.Context, id string) (*model.Organization, error)
	GetOrganizationBySlug(ctx context.Context, slug string) (*model.Organization, error)
	ListOrganizations(ctx context.Context) ([]model.Organization, error)
	ListOrganizationsByUser(ctx context.Context, userID string) ([]model.Organization, error)
	UpdateOrganization(ctx context.Context, id string, updates map[string]interface{}) error
	DeleteOrganization(ctx context.Context, id string) error

	// ==================== 工作空间 ====================
	CreateWorkspace(ctx context.Context, ws *model.Workspace) error
	GetWorkspace(ctx context.Context, id string) (*model.Workspace, error)
	GetWorkspaceBySlug(ctx context.Context, orgID, slug string) (*model.Workspace, error)
	ListWorkspaces(ctx context.Context, orgID string) ([]model.Workspace, error)
	UpdateWorkspace(ctx context.Context, id string, updates map[string]interface{}) error
	DeleteWorkspace(ctx context.Context, id string) error

	// ==================== 用户 ====================
	CreateUser(ctx context.Context, user *model.User) error
	GetUser(ctx context.Context, id string) (*model.User, error)
	GetUserByEmail(ctx context.Context, email string) (*model.User, error)
	GetUserByPhone(ctx context.Context, phone string) (*model.User, error)
	GetUserByProvider(ctx context.Context, provider, providerID string) (*model.User, error)
	UpdateUser(ctx context.Context, id string, updates map[string]interface{}) error
	DeleteUser(ctx context.Context, id string) error

	// ==================== 成员 ====================
	AddMember(ctx context.Context, member *model.WorkspaceMember) error
	GetMember(ctx context.Context, workspaceID, userID string) (*model.WorkspaceMember, error)
	ListMembers(ctx context.Context, workspaceID string) ([]model.WorkspaceMember, error)
	ListUserMemberships(ctx context.Context, userID string) ([]model.WorkspaceMember, error)
	UpdateMemberRole(ctx context.Context, id string, role string) error
	RemoveMember(ctx context.Context, id string) error

	// ==================== 刷新令牌 ====================
	CreateRefreshToken(ctx context.Context, userID string, tokenHash string, expiresAt string) (string, error)
	GetRefreshToken(ctx context.Context, tokenHash string) (*model.RefreshToken, error)
	RevokeRefreshToken(ctx context.Context, tokenHash string) error
	RevokeUserRefreshTokens(ctx context.Context, userID string) error

	// ==================== Pipeline (多租户) ====================
	ListPipelines(ctx context.Context, workspaceID string) ([]model.Pipeline, error)
	GetPipeline(ctx context.Context, id string) (*model.Pipeline, error)
	CreatePipeline(ctx context.Context, p *model.Pipeline) error
	UpdatePipeline(ctx context.Context, id string, p *model.Pipeline) error
	DeletePipeline(ctx context.Context, id string) error

	// ==================== Plugin ====================
	ListPlugins(ctx context.Context, workspaceID string) ([]model.PluginMeta, error)
	CreatePlugin(ctx context.Context, p *model.PluginMeta) error
	UpdatePlugin(ctx context.Context, id string, p *model.PluginMeta) error
	DeletePlugin(ctx context.Context, id string) error

	// ==================== Policy ====================
	ListPolicies(ctx context.Context, workspaceID string) ([]model.Policy, error)
	CreatePolicy(ctx context.Context, p *model.Policy) error
	DeletePolicy(ctx context.Context, id string) error

	// ==================== API Key ====================
	ListApiKeys(ctx context.Context, workspaceID string) ([]model.ApiKey, error)
	GetApiKey(ctx context.Context, id string) (*model.ApiKey, error)
	CreateApiKey(ctx context.Context, k *model.ApiKey) error
	UpdateApiKey(ctx context.Context, k *model.ApiKey) error
	DeleteApiKey(ctx context.Context, id string) error

	// ==================== Model ====================
	ListModels(ctx context.Context, workspaceID string) ([]model.ModelConfig, error)
	GetModel(ctx context.Context, id string) (*model.ModelConfig, error)
	CreateModel(ctx context.Context, m *model.ModelConfig) error
	UpdateModel(ctx context.Context, id string, m *model.ModelConfig) error
	DeleteModel(ctx context.Context, id string) error

	// ==================== Virtual Key ====================
	CreateVirtualKey(ctx context.Context, vk *model.VirtualKey) (*model.VirtualKey, error)
	GetVirtualKey(ctx context.Context, id string) (*model.VirtualKey, error)
	GetVirtualKeyByHash(ctx context.Context, keyHash string) (*model.VirtualKey, error)
	ListVirtualKeys(ctx context.Context, workspaceID string) ([]model.VirtualKey, error)
	UpdateVirtualKey(ctx context.Context, id string, updates map[string]interface{}) error
	RevokeVirtualKey(ctx context.Context, id string) error

	// ==================== Wallet ====================
	GetWallet(ctx context.Context, workspaceID string) (*model.Wallet, error)
	CreateWallet(ctx context.Context, w *model.Wallet) error
	UpdateWalletBalance(ctx context.Context, workspaceID string, amountMicro int64) error
	ListTransactions(ctx context.Context, workspaceID string, limit, offset int) ([]model.Transaction, error)
	CreateTransaction(ctx context.Context, t *model.Transaction) error

	// ==================== 配置版本 ====================
	GetConfigVersions(ctx context.Context) (model.ConfigVersion, error)
	IncrementConfigVersion(ctx context.Context, key string) error

	// ==================== 数据面存储配置 ====================
	ListDataPlaneConfigs(ctx context.Context) ([]model.DataPlaneConfig, error)
	CreateDataPlaneConfig(ctx context.Context, c *model.DataPlaneConfig) error
	UpdateDataPlaneConfig(ctx context.Context, id string, c *model.DataPlaneConfig) error
	DeleteDataPlaneConfig(ctx context.Context, id string) error

	// ==================== 设置 ====================
	GetSettings(ctx context.Context, workspaceID string) (map[string]string, error)
	UpdateSettings(ctx context.Context, workspaceID string, settings map[string]string) error

	// ==================== 审计 ====================
	GetAuditEvents(ctx context.Context, workspaceID string, period string) ([]map[string]interface{}, error)
}

// StoreConfig 存储配置
type StoreConfig struct {
	Backend  string // "postgres" | "sqlite"
	DBPath   string // SQLite 文件路径
	PGHost   string
	PGPort   int
	PGUser   string
	PGPass   string
	PGDBName string
	PGSSLMode string
}

// NewStore 工厂函数：根据配置创建对应存储实现
func NewStore(cfg StoreConfig) (StoreFacade, error) {
	switch cfg.Backend {
	case "postgres":
		return NewPostgresStore(cfg)
	case "sqlite":
		return NewSQLiteStore(cfg)
	default:
		// 默认 SQLite (开发模式)
		return NewSQLiteStore(cfg)
	}
}
