// VERIDACTUS 控制平面 — OAuth 认证 (GitHub)
package auth

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/veridactus/control-plane/internal/model"
	"github.com/veridactus/control-plane/internal/store"
)

// OAuthProvider OAuth Provider 接口
type OAuthProvider interface {
	Name() string
	GetAuthURL(state string) string
	ExchangeCode(ctx context.Context, code string) (*OAuthUserInfo, error)
}

// OAuthUserInfo OAuth 返回的用户信息
type OAuthUserInfo struct {
	Provider     string `json:"provider"`
	ProviderID   string `json:"provider_id"`
	Email        string `json:"email"`
	DisplayName  string `json:"display_name"`
	AvatarURL    string `json:"avatar_url"`
}

// OAuthService OAuth 认证服务
type OAuthService struct {
	store      store.StoreFacade
	jwtSecret  string
}

// NewOAuthService 创建 OAuth 服务
func NewOAuthService(s store.StoreFacade, jwtSecret string) *OAuthService {
	InitJWT(jwtSecret)
	return &OAuthService{store: s, jwtSecret: jwtSecret}
}

// GitHubProvider GitHub OAuth Provider
type GitHubProvider struct {
	clientID     string
	clientSecret string
	redirectURI  string
}

// NewGitHubProvider 创建 GitHub OAuth Provider
func NewGitHubProvider() *GitHubProvider {
	return &GitHubProvider{
		clientID:     os.Getenv("GITHUB_CLIENT_ID"),
		clientSecret: os.Getenv("GITHUB_CLIENT_SECRET"),
		redirectURI:  os.Getenv("GITHUB_REDIRECT_URI"),
	}
}

// IsConfigured 检查 OAuth 是否已正确配置
func (p *GitHubProvider) IsConfigured() bool {
	return p.clientID != "" && p.clientSecret != "" && p.redirectURI != ""
}

func (p *GitHubProvider) Name() string { return "github" }

func (p *GitHubProvider) GetAuthURL(state string) string {
	u, _ := url.Parse("https://github.com/login/oauth/authorize")
	q := u.Query()
	q.Set("client_id", p.clientID)
	q.Set("redirect_uri", p.redirectURI)
	q.Set("scope", "user:email")
	q.Set("state", state)
	u.RawQuery = q.Encode()
	return u.String()
}

func (p *GitHubProvider) ExchangeCode(ctx context.Context, code string) (*OAuthUserInfo, error) {
	// 1. 用 code 换 access_token
	tokenURL := "https://github.com/login/oauth/access_token"
	body := fmt.Sprintf("client_id=%s&client_secret=%s&code=%s&redirect_uri=%s",
		p.clientID, p.clientSecret, code, p.redirectURI)

	req, _ := http.NewRequestWithContext(ctx, "POST", tokenURL, strings.NewReader(body))
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("exchange token: %w", err)
	}
	defer resp.Body.Close()

	var tokenResp struct {
		AccessToken string `json:"access_token"`
		Error       string `json:"error"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&tokenResp); err != nil {
		return nil, fmt.Errorf("decode token response: %w", err)
	}
	if tokenResp.Error != "" {
		return nil, fmt.Errorf("oauth error: %s", tokenResp.Error)
	}

	// 2. 用 access_token 获取用户信息
	userReq, _ := http.NewRequestWithContext(ctx, "GET", "https://api.github.com/user", nil)
	userReq.Header.Set("Authorization", "Bearer "+tokenResp.AccessToken)
	userReq.Header.Set("Accept", "application/vnd.github.v3+json")

	userResp, err := http.DefaultClient.Do(userReq)
	if err != nil {
		return nil, fmt.Errorf("get user: %w", err)
	}
	defer userResp.Body.Close()

	var ghUser struct {
		ID        int    `json:"id"`
		Login     string `json:"login"`
		Name      string `json:"name"`
		Email     string `json:"email"`
		AvatarURL string `json:"avatar_url"`
	}
	if err := json.NewDecoder(userResp.Body).Decode(&ghUser); err != nil {
		return nil, fmt.Errorf("decode user: %w", err)
	}

	// 3. 获取邮箱（如果 primary email 未公开）
	email := ghUser.Email
	if email == "" {
		emailReq, _ := http.NewRequestWithContext(ctx, "GET", "https://api.github.com/user/emails", nil)
		emailReq.Header.Set("Authorization", "Bearer "+tokenResp.AccessToken)
		emailResp, err := http.DefaultClient.Do(emailReq)
		if err == nil {
			defer emailResp.Body.Close()
			var emails []struct {
				Email    string `json:"email"`
				Primary  bool   `json:"primary"`
				Verified bool   `json:"verified"`
			}
			if json.NewDecoder(emailResp.Body).Decode(&emails) == nil {
				for _, e := range emails {
					if e.Primary && e.Verified {
						email = e.Email
						break
					}
				}
			}
		}
	}
	if email == "" {
		email = fmt.Sprintf("%s@github.users", ghUser.Login)
	}

	displayName := ghUser.Name
	if displayName == "" {
		displayName = ghUser.Login
	}

	return &OAuthUserInfo{
		Provider:    "github",
		ProviderID:  fmt.Sprintf("%d", ghUser.ID),
		Email:       email,
		DisplayName: displayName,
		AvatarURL:   ghUser.AvatarURL,
	}, nil
}

// LoginOrCreate 登录或创建用户（OAuth 自动注册）
func (s *OAuthService) LoginOrCreate(ctx context.Context, info *OAuthUserInfo) (*model.User, *model.Organization, *model.Workspace, error) {
	// 1. 查找已有用户
	user, err := s.store.GetUserByProvider(ctx, info.Provider, info.ProviderID)
	if err == nil && user != nil {
		// 用户已存在，更新最后登录时间
		now := timeNow()
		s.store.UpdateUser(ctx, user.ID, map[string]interface{}{"last_login_at": &now})
	}

	// 2. 新用户 → 自动创建
	if user == nil {
		user = &model.User{
			ID:             uuid.New().String(),
			Email:          info.Email,
			DisplayName:    info.DisplayName,
			AvatarURL:      info.AvatarURL,
			AuthProvider:   info.Provider,
			AuthProviderID: info.ProviderID,
		}
		if err := s.store.CreateUser(ctx, user); err != nil {
			return nil, nil, nil, fmt.Errorf("create user: %w", err)
		}
	}

	// 3. 确保有默认组织和工作空间
	orgs, _ := s.store.ListOrganizations(ctx)
	var org *model.Organization
	var ws *model.Workspace

	if len(orgs) == 0 {
		// 首个用户 → 自动创建默认组织和工作空间
		orgID := uuid.New().String()
		org = &model.Organization{
			ID:   orgID,
			Name: info.DisplayName + "'s Organization",
			Slug: sanitizeSlug(info.DisplayName),
			Plan: "free",
		}
		s.store.CreateOrganization(ctx, org)

		wsID := uuid.New().String()
		ws = &model.Workspace{
			ID:    wsID,
			OrgID: orgID,
			Name:  "Default Workspace",
			Slug:  "default",
		}
		s.store.CreateWorkspace(ctx, ws)

		// 添加为 workspace_admin
		member := &model.WorkspaceMember{
			ID:          uuid.New().String(),
			WorkspaceID: wsID,
			UserID:      user.ID,
			Role:        "workspace_admin",
		}
		s.store.AddMember(ctx, member)
	} else {
		org = &orgs[0]
		wss, _ := s.store.ListWorkspaces(ctx, org.ID)
		if len(wss) > 0 {
			ws = &wss[0]
		}
		// 检查是否已是成员
		if ws != nil {
			member, err := s.store.GetMember(ctx, ws.ID, user.ID)
			if err != nil || member == nil {
				newMember := &model.WorkspaceMember{
					ID:          uuid.New().String(),
					WorkspaceID: ws.ID,
					UserID:      user.ID,
					Role:        "developer",
				}
				s.store.AddMember(ctx, newMember)
			}
		}
	}

	// 更新登录时间
	now := timeNow()
	s.store.UpdateUser(ctx, user.ID, map[string]interface{}{"last_login_at": &now})

	return user, org, ws, nil
}

func sanitizeSlug(name string) string {
	slug := strings.ToLower(name)
	slug = strings.ReplaceAll(slug, " ", "-")
	// 移除特殊字符
	var result strings.Builder
	for _, r := range slug {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') || r == '-' {
			result.WriteRune(r)
		}
	}
	if result.Len() == 0 {
		return "user"
	}
	return result.String()
}

func timeNow() time.Time { return time.Now() }

// ==================== Google OAuth Provider ====================

// GoogleProvider Google OAuth 2.0 Provider (OpenID Connect)
type GoogleProvider struct {
	clientID     string
	clientSecret string
	redirectURI  string
}

// NewGoogleProvider 创建 Google OAuth Provider
func NewGoogleProvider() *GoogleProvider {
	return &GoogleProvider{
		clientID:     os.Getenv("GOOGLE_CLIENT_ID"),
		clientSecret: os.Getenv("GOOGLE_CLIENT_SECRET"),
		redirectURI:  os.Getenv("GOOGLE_REDIRECT_URI"),
	}
}

func (p *GoogleProvider) IsConfigured() bool {
	return p.clientID != "" && p.clientSecret != "" && p.redirectURI != ""
}

func (p *GoogleProvider) Name() string { return "google" }

func (p *GoogleProvider) GetAuthURL(state string) string {
	u, _ := url.Parse("https://accounts.google.com/o/oauth2/v2/auth")
	q := u.Query()
	q.Set("client_id", p.clientID)
	q.Set("redirect_uri", p.redirectURI)
	q.Set("response_type", "code")
	q.Set("scope", "openid profile email")
	q.Set("state", state)
	q.Set("access_type", "offline")
	q.Set("prompt", "consent")
	u.RawQuery = q.Encode()
	return u.String()
}

func (p *GoogleProvider) ExchangeCode(ctx context.Context, code string) (*OAuthUserInfo, error) {
	// 1. 用 code 换 access_token
	tokenURL := "https://oauth2.googleapis.com/token"
	body := fmt.Sprintf("client_id=%s&client_secret=%s&code=%s&redirect_uri=%s&grant_type=authorization_code",
		p.clientID, p.clientSecret, code, p.redirectURI)

	req, _ := http.NewRequestWithContext(ctx, "POST", tokenURL, strings.NewReader(body))
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("exchange token: %w", err)
	}
	defer resp.Body.Close()

	var tokenResp struct {
		AccessToken string `json:"access_token"`
		IDToken     string `json:"id_token"`
		Error       string `json:"error"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&tokenResp); err != nil {
		return nil, fmt.Errorf("decode token response: %w", err)
	}
	if tokenResp.Error != "" {
		return nil, fmt.Errorf("oauth error: %s", tokenResp.Error)
	}

	// 2. 用 access_token 获取用户信息 (OpenID Connect UserInfo)
	userReq, _ := http.NewRequestWithContext(ctx, "GET", "https://openidconnect.googleapis.com/v1/userinfo", nil)
	userReq.Header.Set("Authorization", "Bearer "+tokenResp.AccessToken)

	userResp, err := http.DefaultClient.Do(userReq)
	if err != nil {
		return nil, fmt.Errorf("get user: %w", err)
	}
	defer userResp.Body.Close()

	var googleUser struct {
		Sub           string `json:"sub"`
		Email         string `json:"email"`
		EmailVerified bool   `json:"email_verified"`
		Name          string `json:"name"`
		Picture       string `json:"picture"`
	}
	if err := json.NewDecoder(userResp.Body).Decode(&googleUser); err != nil {
		return nil, fmt.Errorf("decode user: %w", err)
	}

	if googleUser.Email == "" {
		return nil, fmt.Errorf("no email returned from Google")
	}

	return &OAuthUserInfo{
		Provider:    "google",
		ProviderID:  googleUser.Sub,
		Email:       googleUser.Email,
		DisplayName: googleUser.Name,
		AvatarURL:   googleUser.Picture,
	}, nil
}
