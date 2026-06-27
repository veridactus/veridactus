// Email/Password + Phone 注册认证模块
package auth

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"time"

	"github.com/google/uuid"
	"golang.org/x/crypto/bcrypt"
	"github.com/veridactus/control-plane/internal/crypto"
	"github.com/veridactus/control-plane/internal/model"
	"github.com/veridactus/control-plane/internal/store"
)

// EmailAuthService 邮箱/手机号认证服务
type EmailAuthService struct {
	store store.StoreFacade
}

func NewEmailAuthService(s store.StoreFacade) *EmailAuthService {
	return &EmailAuthService{store: s}
}

// RegisterRequest 注册请求
type RegisterRequest struct {
	Email       string `json:"email"`
	Password    string `json:"password"`
	DisplayName string `json:"display_name"`
	Phone       string `json:"phone,omitempty"`
	Plan        string `json:"plan,omitempty"`  // "personal" | "enterprise"
	OrgName     string `json:"org_name,omitempty"` // 企业版：组织名称
}

// RegisterResult 注册/登录结果
type RegisterResult struct {
	User          *model.User         `json:"user"`
	Org           *model.Organization `json:"org"`
	Workspace     *model.Workspace    `json:"workspace"`
	Token         string              `json:"access_token"`
	RefreshToken  string              `json:"refresh_token"`
	NeedBindPhone bool                `json:"need_bind_phone,omitempty"` // 微信登录后提示绑定手机号
}

// Register 邮箱注册 (自动创建组织+工作空间, 支持个人版/企业版)
func (s *EmailAuthService) Register(ctx context.Context, req RegisterRequest) (*RegisterResult, error) {
	if req.Email == "" {
		return nil, fmt.Errorf("邮箱不能为空")
	}

	// 密码强度验证
	if err := ValidatePassword(req.Password, true); err != nil {
		return nil, err
	}

	// 检查邮箱是否已注册
	existing, _ := s.store.GetUserByEmail(ctx, req.Email)
	if existing != nil {
		return nil, fmt.Errorf("该邮箱已注册，请直接登录")
	}

	// 如果提供了手机号，验证是否已绑定
	if req.Phone != "" {
		phoneUser, _ := s.store.GetUserByPhone(ctx, req.Phone)
		if phoneUser != nil {
			return nil, fmt.Errorf("该手机号已被绑定")
		}
	}

	// hash 密码 (bcrypt cost 12 更安全)
	hash, err := bcrypt.GenerateFromPassword([]byte(req.Password), 12)
	if err != nil {
		return nil, fmt.Errorf("安全加密失败，请重试")
	}

	// 默认 plan
	plan := req.Plan
	if plan == "" { plan = "personal" }
	orgName := req.OrgName
	if orgName == "" {
		if req.DisplayName != "" {
			orgName = req.DisplayName + "'s Workspace"
		} else {
			orgName = "My Workspace"
		}
	}

	// 创建用户
	user := &model.User{
		ID:           uuid.New().String(),
		Email:        req.Email,
		DisplayName:  req.DisplayName,
		Phone:        req.Phone,
		AuthProvider: "email",
		PasswordHash: string(hash),
	}
	if req.DisplayName == "" {
		user.DisplayName = req.Email
	}
	if err := s.store.CreateUser(ctx, user); err != nil {
		return nil, fmt.Errorf("创建用户失败: %w", err)
	}

	// 创建组织 (企业版plan不同)
	org := &model.Organization{
		ID:   uuid.New().String(),
		Name: orgName,
		Slug: sanitizeForSlug(orgName),
		Plan: plan,
	}
	if err := s.store.CreateOrganization(ctx, org); err != nil {
		return nil, fmt.Errorf("创建组织失败: %w", err)
	}

	ws := &model.Workspace{
		ID:    uuid.New().String(),
		OrgID: org.ID,
		Name:  "Default",
		Slug:  "default",
	}
	if err := s.store.CreateWorkspace(ctx, ws); err != nil {
		return nil, fmt.Errorf("创建工作空间失败: %w", err)
	}

	// 添加为 workspace_admin
	member := &model.WorkspaceMember{
		ID:          uuid.New().String(),
		WorkspaceID: ws.ID,
		UserID:      user.ID,
		Role:        "workspace_admin",
	}
	s.store.AddMember(ctx, member)

	// 签发 JWT (含 plan 信息)
	token, err := GenerateAccessTokenWithPlan(user.ID, user.Email, org.ID, ws.ID, "workspace_admin", plan)
	if err != nil {
		return nil, fmt.Errorf("token 签发失败: %w", err)
	}

	// 生成刷新令牌
	refreshToken, _ := crypto.GenerateRefreshToken()
	h := sha256.Sum256([]byte(refreshToken))
	refreshHash := hex.EncodeToString(h[:])
	s.store.CreateRefreshToken(ctx, user.ID, refreshHash, time.Now().Add(30*24*time.Hour).UTC().Format(time.RFC3339))

	return &RegisterResult{
		User:         user,
		Org:          org,
		Workspace:    ws,
		Token:        token,
		RefreshToken: refreshToken,
	}, nil
}

// Login 邮箱密码登录 (含账户锁定检查)
func (s *EmailAuthService) Login(ctx context.Context, email, password string) (*RegisterResult, error) {
	// 1. 检查账户锁定
	if locked, msg := IsAccountLocked(ctx, s.store, email); locked {
		return nil, fmt.Errorf(msg)
	}

	// 2. 查找用户
	user, err := s.store.GetUserByEmail(ctx, email)
	if err != nil {
		return nil, fmt.Errorf("邮箱或密码错误") // 模糊错误消息
	}
	if user.AuthProvider != "email" {
		return nil, fmt.Errorf("该账户使用 %s 登录，请使用对应的登录方式", providerLabel(user.AuthProvider))
	}
	if user.PasswordHash == "" {
		return nil, fmt.Errorf("该账户未设置密码，请使用其他方式登录")
	}

	// 3. 验证密码
	if err := bcrypt.CompareHashAndPassword([]byte(user.PasswordHash), []byte(password)); err != nil {
		failCount := RecordLoginFailure(ctx, s.store, email)
		if failCount >= 5 {
			return nil, fmt.Errorf("密码错误次数过多，账户已锁定15分钟")
		}
		remaining := 5 - failCount
		return nil, fmt.Errorf("邮箱或密码错误 (剩余尝试次数: %d)", remaining)
	}

	// 4. 清除失败记录
	ClearLoginFailures(ctx, s.store, email)

	// 5. 查找用户所属的组织和工作空间
	orgID, wsID := "", ""
	members, _ := s.store.ListUserMemberships(ctx, user.ID)
	if len(members) > 0 {
		wsID = members[0].WorkspaceID
		ws, _ := s.store.GetWorkspace(ctx, wsID)
		if ws != nil { orgID = ws.OrgID }
	}
	// Fallback: use first available
	if orgID == "" {
		orgs, _ := s.store.ListOrganizations(ctx)
		if len(orgs) > 0 { orgID = orgs[0].ID }
	}
	if wsID == "" {
		wss, _ := s.store.ListWorkspaces(ctx, orgID)
		if len(wss) > 0 { wsID = wss[0].ID }
	}
	if orgID == "" || wsID == "" {
		return nil, fmt.Errorf("账户数据异常，请联系管理员")
	}

	// 6. 签发 JWT
	org, _ := s.store.GetOrganization(ctx, orgID)
	plan := "personal"
	if org != nil { plan = org.Plan }
	token, err := GenerateAccessTokenWithPlan(user.ID, user.Email, orgID, wsID, "workspace_admin", plan)
	if err != nil {
		return nil, fmt.Errorf("认证失败，请重试")
	}

	// 7. 生成刷新令牌
	refreshToken, _ := crypto.GenerateRefreshToken()
	h := sha256.Sum256([]byte(refreshToken))
	refreshHash := hex.EncodeToString(h[:])
	s.store.CreateRefreshToken(ctx, user.ID, refreshHash, time.Now().Add(30*24*time.Hour).UTC().Format(time.RFC3339))

	// 8. 更新最后登录时间
	now := time.Now().UTC().Format(time.RFC3339)
	s.store.UpdateUser(ctx, user.ID, map[string]interface{}{"last_login_at": now})

	finalWs, _ := s.store.GetWorkspace(ctx, wsID)
	return &RegisterResult{
		User: user, Workspace: finalWs,
		Org:          org,
		Token:        token,
		RefreshToken: refreshToken,
	}, nil
}

func providerLabel(provider string) string {
	switch provider {
	case "github": return "GitHub"
	case "phone": return "手机号"
	case "wechat": return "微信"
	default: return provider
	}
}

// SendPhoneCode / VerifyPhoneCode 已移至 sms_provider.go (纯生产实现)
// generateVerificationCode 保留在此供 sms_provider.go 使用

func generateVerificationCode() string {
	b := make([]byte, 3)
	rand.Read(b)
	return fmt.Sprintf("%06d", int(b[0])<<16|int(b[1])<<8|int(b[2]))
}

func sanitizeForSlug(name string) string {
	slug := ""
	for _, r := range name {
		if (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') || r == '-' || r == '_' {
			slug += string(r)
		} else if r == ' ' || r == '.' || r == '@' {
			slug += "-"
		}
	}
	if len(slug) > 50 {
		slug = slug[:50]
	}
	if slug == "" {
		slug = "user-" + hex.EncodeToString([]byte(name))[:8]
	}
	return slug
}
