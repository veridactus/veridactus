// VERIDACTUS 控制平面 — 统一数据模型 v2.0
// 支持多租户 (Organization → Workspace 三级隔离)
package model

import (
	"encoding/json"
)

// ==================== 组织与多租户 ====================

// Organization 组织（企业/个人）
type Organization struct {
	ID           string    `json:"id"`
	Name         string    `json:"name"`
	Slug         string    `json:"slug"`
	Plan         string    `json:"plan"` // free | pro | enterprise
	LogoURL      string    `json:"logo_url,omitempty"`
	PrimaryColor string    `json:"primary_color,omitempty"`
	Settings     string    `json:"settings,omitempty"` // JSON
	CreatedAt    string `json:"created_at"`
	UpdatedAt    string `json:"updated_at"`
}

// Workspace 工作空间
type Workspace struct {
	ID          string    `json:"id"`
	OrgID       string    `json:"org_id"`
	Name        string    `json:"name"`
	Slug        string    `json:"slug"`
	Description string    `json:"description,omitempty"`
	Settings    string    `json:"settings,omitempty"` // JSON
	CreatedAt   string `json:"created_at"`
	UpdatedAt   string `json:"updated_at"`
}

// ==================== 用户与认证 ====================

// User 用户
type User struct {
	ID             string    `json:"id"`
	Email          string    `json:"email"`
	Phone          string    `json:"phone,omitempty"`   // 手机号
	DisplayName    string    `json:"display_name,omitempty"`
	AvatarURL      string    `json:"avatar_url,omitempty"`
	AuthProvider   string    `json:"auth_provider"` // github | google | email | phone | sso
	AuthProviderID string    `json:"auth_provider_id,omitempty"`
	PasswordHash   string    `json:"-"` // 仅 email 注册时使用, JSON 不导出
	Settings       string    `json:"settings,omitempty"` // JSON
	LastLoginAt    string `json:"last_login_at,omitempty"`
	CreatedAt      string `json:"created_at"`
	UpdatedAt      string `json:"updated_at"`
}

// WorkspaceMember 工作空间成员（User ↔ Workspace M:N）
type WorkspaceMember struct {
	ID           string    `json:"id"`
	WorkspaceID  string    `json:"workspace_id"`
	UserID       string    `json:"user_id"`
	Role         string    `json:"role"` // platform_admin|org_admin|workspace_admin|developer|auditor
	InvitedBy    string    `json:"invited_by,omitempty"`
	InvitedAt    string `json:"invited_at,omitempty"`
	JoinedAt     string `json:"joined_at"`
	// 联表字段（列表查询时填充）
	UserName  string `json:"user_name,omitempty"`
	UserEmail string `json:"user_email,omitempty"`
}

// RefreshToken 刷新令牌
type RefreshToken struct {
	ID        string    `json:"id"`
	UserID    string    `json:"user_id"`
	TokenHash string    `json:"-"` // SHA-256(token), JSON 不导出
	ExpiresAt string `json:"expires_at"`
	CreatedAt string `json:"created_at"`
}

// ==================== 密钥与计费 ====================

// VirtualKey 虚拟密钥
type VirtualKey struct {
	ID                    string    `json:"id"`
	WorkspaceID           string    `json:"workspace_id"`
	Name                  string    `json:"name"`
	KeyPrefix             string    `json:"key_prefix"`            // "vd-xxxx"
	KeyHash               string    `json:"-"`                     // SHA-256(完整key), 不导出
	Type                  string    `json:"type"`                  // byok | platform
	ProviderKeyEncrypted  string    `json:"-"`                     // AES-256-GCM 加密, 不导出
	ProviderKeyKMSID      string    `json:"provider_key_kms_id,omitempty"`
	AllowedModels         string    `json:"allowed_models,omitempty"` // JSON array
	RateLimitRPM          int       `json:"rate_limit_rpm"`
	RateLimitTPM          int       `json:"rate_limit_tpm"`
	SpendLimitUSDMicro    int64     `json:"spend_limit_usd_micro"`
	Status                string    `json:"status"` // active|revoked|expired
	LastUsedAt            string `json:"last_used_at,omitempty"`
	CreatedAt             string `json:"created_at"`
	CreatedBy             string    `json:"created_by"`
}

// Wallet 钱包
type Wallet struct {
	ID                  string    `json:"id"`
	WorkspaceID         string    `json:"workspace_id"`
	BalanceUSDMicro     int64     `json:"balance_usd_micro"`
	OverdraftLimitMicro int64     `json:"overdraft_limit_micro"`
	LastCreditAt        *string `json:"last_credit_at,omitempty"`
	CreatedAt           string `json:"created_at"`
	UpdatedAt           string `json:"updated_at"`
}

// Transaction 交易记录
type Transaction struct {
	ID                string    `json:"id"`
	WorkspaceID       string    `json:"workspace_id"`
	WalletID          string    `json:"wallet_id"`
	Type              string    `json:"type"` // credit|debit|refund|correction
	AmountUSDMicro    int64     `json:"amount_usd_micro"`
	BalanceAfterMicro int64     `json:"balance_after_micro"`
	Description       string    `json:"description,omitempty"`
	TraceID           string    `json:"trace_id,omitempty"`
	Metadata          string    `json:"metadata,omitempty"` // JSON
	CreatedAt         string `json:"created_at"`
}

// ==================== 业务实体 (带多租户) ====================

// Pipeline 治理流水线
type Pipeline struct {
	ID          string       `json:"id"`
	PlanID      string       `json:"plan_id"`
	OrgID       string       `json:"org_id"`
	WorkspaceID string       `json:"workspace_id"`
	Name        string       `json:"name"`
	Description string       `json:"description"`
	Tenant      string       `json:"tenant"`
	Stages      []StageConfig `json:"stages"`
	Status      string       `json:"status"` // draft | published | active
	Created     string       `json:"created_at"`
}

// StageConfig 流水线阶段配置
type StageConfig struct {
	Placement string       `json:"placement"`
	Parallel  bool         `json:"parallel"`
	Plugins   []PluginConf `json:"plugins"`
}

// PluginConf 阶段内插件配置
type PluginConf struct {
	Name    string `json:"name"`
	Type    string `json:"type"`
	Config  string `json:"config"`
	Enabled bool   `json:"enabled"`
}

// PluginMeta 插件元数据
type PluginMeta struct {
	ID          string `json:"id"`
	OrgID       string `json:"org_id,omitempty"`
	WorkspaceID string `json:"workspace_id,omitempty"`
	Name        string `json:"name"`
	Type        string `json:"type"`
	Version     string `json:"version"`
	Description string `json:"description"`
	Config      string `json:"config,omitempty"`
}

// Policy 治理策略
type Policy struct {
	ID          string `json:"id"`
	OrgID       string `json:"org_id,omitempty"`
	WorkspaceID string `json:"workspace_id,omitempty"`
	Name        string `json:"name"`
	Type        string `json:"type"`
	Content     string `json:"content"`
	CreatedAt   string `json:"created_at"`
}

// ApiKey API 密钥
type ApiKey struct {
	ID          string `json:"id"`
	OrgID       string `json:"org_id,omitempty"`
	WorkspaceID string `json:"workspace_id,omitempty"`
	Name        string `json:"name"`
	Key         string `json:"key"`
	TenantID    string `json:"tenant_id"`
	Status      string `json:"status"`
	CreatedAt   string `json:"created_at"`
	LastUsed    string `json:"last_used,omitempty"`
}

// ModelConfig 模型路由配置
type ModelConfig struct {
	ID                string   `json:"id"`
	OrgID             string   `json:"org_id,omitempty"`
	WorkspaceID       string   `json:"workspace_id,omitempty"`
	Name              string   `json:"name"`
	UpstreamURL       string   `json:"upstream_url"`
	UpstreamModel     string   `json:"upstream_model"`
	ApiKey            string   `json:"api_key,omitempty"`
	ApiKeyHeader      string   `json:"api_key_header,omitempty"`
	UseProxy          bool     `json:"use_proxy"`
	ProxyURL          string   `json:"proxy_url,omitempty"`
	IsDefault         bool     `json:"is_default"`
	SupportedVersions []string `json:"supported_versions,omitempty"`
	Status            string   `json:"status"`
}

// DataPlaneConfig 数据面存储配置
type DataPlaneConfig struct {
	ID        string `json:"id"`
	Key       string `json:"key"`
	Value     string `json:"value"`
	CreatedAt string `json:"created_at"`
}

// ==================== 配置与版本 ====================

// ConfigVersion 配置版本号
type ConfigVersion struct {
	PipelineVersion int `json:"pipeline_version"`
	PolicyVersion   int `json:"policy_version"`
	PluginVersion   int `json:"plugin_version"`
	StorageVersion  int `json:"storage_version"`
	ModelVersion    int `json:"model_version"`
}

// ConfigChangePayload 配置变更负载
type ConfigChangePayload struct {
	ChangeType string          `json:"change_type"`
	Data       json.RawMessage `json:"data"`
	Version    ConfigVersion   `json:"version"`
}

// ==================== 请求/响应 DTO ====================

// CreateWorkspaceRequest 创建工作空间请求
type CreateWorkspaceRequest struct {
	OrgID       string `json:"org_id"`
	Name        string `json:"name"`
	Slug        string `json:"slug,omitempty"`
	Description string `json:"description,omitempty"`
}

// InviteMemberRequest 邀请成员请求
type InviteMemberRequest struct {
	Email string `json:"email"`
	Role  string `json:"role"`
}

// UpdateMemberRequest 更新成员角色请求
type UpdateMemberRequest struct {
	Role string `json:"role"`
}

// CreateVirtualKeyRequest 创建虚拟密钥请求
type CreateVirtualKeyRequest struct {
	Name         string `json:"name"`
	Type         string `json:"type"` // byok | platform
	ProviderKey  string `json:"provider_key,omitempty"` // 仅 BYOK
	AllowedModels []string `json:"allowed_models,omitempty"`
	RateLimitRPM int    `json:"rate_limit_rpm"`
	RateLimitTPM int    `json:"rate_limit_tpm"`
}

// OAuthCallbackRequest OAuth 回调
type OAuthCallbackRequest struct {
	Code  string `json:"code"`
	State string `json:"state"`
}

// AuthResponse 认证响应
type AuthResponse struct {
	AccessToken  string `json:"access_token"`
	RefreshToken string `json:"refresh_token"`
	TokenType    string `json:"token_type"`
	ExpiresIn    int    `json:"expires_in"`
	User         *User  `json:"user"`
}

// RefreshTokenRequest 刷新令牌请求
type RefreshTokenRequest struct {
	RefreshToken string `json:"refresh_token"`
}

// KeyResolveRequest 内部密钥解析请求 (数据面→控制面)
type KeyResolveRequest struct {
	VirtualKeyHash string `json:"virtual_key_hash"`
	Model          string `json:"model"`
}

// KeyResolveResponse 内部密钥解析响应
type KeyResolveResponse struct {
	Resolved              bool   `json:"resolved"`
	Provider              string `json:"provider,omitempty"`
	ProviderKey           string `json:"provider_key,omitempty"`
	UpstreamURL           string `json:"upstream_url,omitempty"`
	RateLimitRemaining    int    `json:"rate_limit_remaining,omitempty"`
	BudgetRemainingMicro  int64  `json:"budget_remaining_micro,omitempty"`
	Error                 string `json:"error,omitempty"`
}

// ListResponse 通用分页列表响应
type ListResponse struct {
	Total int `json:"total"`
	Items any `json:"items"`
}

// ErrorDetail API 错误详情
type ErrorDetail struct {
	Code      string `json:"code"`
	Message   string `json:"message"`
	RequestID string `json:"request_id,omitempty"`
	Details   any    `json:"details,omitempty"`
}

// Conversation 聊天会话
type Conversation struct {
	ID          string `json:"id"`
	WorkspaceID string `json:"workspace_id,omitempty"`
	UserID      string `json:"user_id"`
	Title       string `json:"title"`
	Model       string `json:"model"`
	CreatedAt   string `json:"created_at"`
	UpdatedAt   string `json:"updated_at"`
}

// Message 聊天消息
type Message struct {
	ID             string `json:"id"`
	ConversationID string `json:"conversation_id"`
	Role           string `json:"role"`
	Content        string `json:"content"`
	Model          string `json:"model,omitempty"`
	Tokens         int    `json:"tokens"`
	CreatedAt      string `json:"created_at"`
}
