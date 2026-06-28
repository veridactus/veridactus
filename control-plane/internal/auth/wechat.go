// VERIDACTUS 控制平面 — 微信 OAuth 认证
// 微信开放平台 OAuth 2.0 扫码登录 + 手机号绑定
package auth

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"time"

	"github.com/google/uuid"
	"github.com/veridactus/control-plane/internal/model"
	"github.com/veridactus/control-plane/internal/store"
)

// WeChatProvider 微信开放平台 OAuth Provider
// 文档: https://developers.weixin.qq.com/doc/oplatform/Website_App/WeChat_Login/Wechat_Login.html
type WeChatProvider struct {
	appID       string
	appSecret   string
	redirectURI string
}

func NewWeChatProvider() *WeChatProvider {
	return &WeChatProvider{
		appID:       os.Getenv("WECHAT_APP_ID"),
		appSecret:   os.Getenv("WECHAT_APP_SECRET"),
		redirectURI: os.Getenv("WECHAT_REDIRECT_URI"),
	}
}

func (p *WeChatProvider) Name() string { return "wechat" }

func (p *WeChatProvider) IsConfigured() bool {
	return p.appID != "" && p.appSecret != "" && p.redirectURI != ""
}

// GetAuthURL 生成微信扫码登录 URL
//
// 生产环境: https://open.weixin.qq.com/connect/qrconnect
// 参数: appid, redirect_uri, response_type=code, scope=snsapi_login, state
// 返回带 #wechat_redirect 的 URL
func (p *WeChatProvider) GetAuthURL(state string) string {
	u, _ := url.Parse("https://open.weixin.qq.com/connect/qrconnect")
	q := u.Query()
	q.Set("appid", p.appID)
	q.Set("redirect_uri", p.redirectURI)
	q.Set("response_type", "code")
	q.Set("scope", "snsapi_login")
	q.Set("state", state)
	u.RawQuery = q.Encode()
	return u.String() + "#wechat_redirect"
}

// WeChatTokenResponse 微信 access_token 响应
type WeChatTokenResponse struct {
	AccessToken  string `json:"access_token"`
	ExpiresIn    int    `json:"expires_in"`
	RefreshToken string `json:"refresh_token"`
	OpenID       string `json:"openid"`
	Scope        string `json:"scope"`
	UnionID      string `json:"unionid"`
	ErrCode      int    `json:"errcode"`
	ErrMsg       string `json:"errmsg"`
}

// WeChatUserInfo 微信用户信息
type WeChatUserInfo struct {
	OpenID     string `json:"openid"`
	Nickname   string `json:"nickname"`
	Sex        int    `json:"sex"`
	Province   string `json:"province"`
	City       string `json:"city"`
	Country    string `json:"country"`
	HeadImgURL string `json:"headimgurl"`
	UnionID    string `json:"unionid"`
	ErrCode    int    `json:"errcode"`
	ErrMsg     string `json:"errmsg"`
}

// ExchangeCode 用授权码换取 access_token + 用户信息
func (p *WeChatProvider) ExchangeCode(ctx context.Context, code string) (*OAuthUserInfo, error) {
	// 模式: 未配置时自动降级到开发模式
	if !p.IsConfigured() {
		return p.devModeFlow(code)
	}
	return p.productionFlow(ctx, code)
}

// productionFlow 生产环境: 真实调用微信 API
func (p *WeChatProvider) productionFlow(ctx context.Context, code string) (*OAuthUserInfo, error) {
	// 1. 用 code 换 access_token
	tokenURL := fmt.Sprintf(
		"https://api.weixin.qq.com/sns/oauth2/access_token?appid=%s&secret=%s&code=%s&grant_type=authorization_code",
		p.appID, p.appSecret, code,
	)
	resp, err := http.Get(tokenURL)
	if err != nil {
		return nil, fmt.Errorf("微信 token 请求失败: %w", err)
	}
	defer resp.Body.Close()

	var tokenResp WeChatTokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&tokenResp); err != nil {
		return nil, fmt.Errorf("解析微信 token 响应失败: %w", err)
	}
	if tokenResp.ErrCode != 0 {
		return nil, fmt.Errorf("微信返回错误: %s (code=%d)", tokenResp.ErrMsg, tokenResp.ErrCode)
	}

	// 2. 获取用户信息
	userURL := fmt.Sprintf(
		"https://api.weixin.qq.com/sns/userinfo?access_token=%s&openid=%s",
		tokenResp.AccessToken, tokenResp.OpenID,
	)
	userResp, err := http.Get(userURL)
	if err != nil {
		return nil, fmt.Errorf("微信用户信息请求失败: %w", err)
	}
	defer userResp.Body.Close()

	var userInfo WeChatUserInfo
	if err := json.NewDecoder(userResp.Body).Decode(&userInfo); err != nil {
		return nil, fmt.Errorf("解析微信用户信息失败: %w", err)
	}
	if userInfo.ErrCode != 0 {
		return nil, fmt.Errorf("微信获取用户信息错误: %s", userInfo.ErrMsg)
	}

	// 优先用 unionid 作为唯一标识
	providerID := userInfo.UnionID
	if providerID == "" {
		providerID = userInfo.OpenID
	}

	return &OAuthUserInfo{
		Provider:    "wechat",
		ProviderID:  providerID,
		Email:       fmt.Sprintf("wx_%s@wechat.user", providerID[:12]),
		DisplayName: userInfo.Nickname,
		AvatarURL:   userInfo.HeadImgURL,
	}, nil
}

// devModeFlow 开发模式: 无需真实微信 AppID，使用模拟数据
// 用户访问 http://localhost:8081/api/v1/auth/callback/wechat?code=dev_demo
// 即可使用演示账户登录
func (p *WeChatProvider) devModeFlow(code string) (*OAuthUserInfo, error) {
	// 生成稳定的模拟 openid (基于传入的 code hash)
	demoID := hashString(code)
	if len(demoID) > 16 {
		demoID = demoID[:16]
	}

	return &OAuthUserInfo{
		Provider:    "wechat",
		ProviderID:  "wx_demo_" + demoID,
		Email:       fmt.Sprintf("wx_%s@wechat.demo", demoID),
		DisplayName: fmt.Sprintf("微信用户_%s", demoID[:6]),
		AvatarURL:   "https://api.dicebear.com/7.x/bottts/svg?seed=" + demoID,
	}, nil
}

// ==================== Phone Binding ====================

// BindPhoneRequest 绑定手机号请求
type BindPhoneRequest struct {
	Phone string `json:"phone"`
	Code  string `json:"code"`
}

// BindPhoneService 微信登录后绑定手机号服务
func BindPhoneAfterWeChatLogin(ctx context.Context, s store.StoreFacade, userID, phone, code string) error {
	// 1. 验证手机验证码
	if !VerifyPhoneCode(ctx, s, phone, code) {
		return fmt.Errorf("验证码错误或已过期")
	}

	// 2. 检查手机号是否已被其他用户绑定
	existing, _ := s.GetUserByPhone(ctx, phone)
	if existing != nil && existing.ID != userID {
		return fmt.Errorf("该手机号已被其他账户绑定")
	}

	// 3. 更新用户手机号
	now := time.Now().UTC().Format(time.RFC3339)
	return s.UpdateUser(ctx, userID, map[string]interface{}{
		"phone":          phone,
		"auth_provider":  "wechat_phone",
		"updated_at":     now,
	})
}

// ==================== WeChat + Phone Auto-Login ====================

type WeChatLoginService struct {
	store     store.StoreFacade
	jwtSecret string
}

func NewWeChatLoginService(s store.StoreFacade, jwtSecret string) *WeChatLoginService {
	return &WeChatLoginService{store: s, jwtSecret: jwtSecret}
}

// LoginOrCreateByWeChat 微信登录或自动注册
// 1. 检查 unionid/openid 是否存在
// 2. 存在 → 直接登录
// 3. 不存在 → 自动创建账户 + 组织 + 工作空间
func (s *WeChatLoginService) LoginOrCreateByWeChat(ctx context.Context, info *OAuthUserInfo) (*RegisterResult, error) {
	// 1. 查找已有用户
	user, err := s.store.GetUserByProvider(ctx, info.Provider, info.ProviderID)
	
	if err != nil || user == nil {
		// 2. 新用户 → 自动创建
		user = &model.User{
			ID:             uuid.New().String(),
			Email:          info.Email,
			DisplayName:    info.DisplayName,
			AvatarURL:      info.AvatarURL,
			AuthProvider:   info.Provider,
			AuthProviderID: info.ProviderID,
		}
		if err := s.store.CreateUser(ctx, user); err != nil {
			return nil, fmt.Errorf("创建用户失败: %w", err)
		}

		// 自动创建组织 + 工作空间
		orgID := uuid.New().String()
		org := &model.Organization{
			ID:   orgID,
			Name: user.DisplayName + "'s Workspace",
			Slug: sanitizeForSlug(user.DisplayName),
			Plan: "personal",
		}
		s.store.CreateOrganization(ctx, org)

		wsID := uuid.New().String()
		ws := &model.Workspace{
			ID:    wsID,
			OrgID: orgID,
			Name:  "Default",
			Slug:  "default",
		}
		s.store.CreateWorkspace(ctx, ws)

		s.store.AddMember(ctx, &model.WorkspaceMember{
			ID:          uuid.New().String(),
			WorkspaceID: wsID,
			UserID:      user.ID,
			Role:        "workspace_admin",
		})

		// 签发 JWT
		token, _ := GenerateAccessTokenWithPlan(user.ID, user.Email, orgID, wsID, "workspace_admin", "personal")

		return &RegisterResult{
			User:      user,
			Org:       org,
			Workspace: ws,
			Token:     token,
			NeedBindPhone: true, // 微信新用户提示绑定手机
		}, nil
	}

	// 3. 已有用户 → 登录
	members, _ := s.store.ListUserMemberships(ctx, user.ID)
	orgID, wsID := "", ""
	if len(members) > 0 {
		wsID = members[0].WorkspaceID
		ws, _ := s.store.GetWorkspace(ctx, wsID)
		if ws != nil { orgID = ws.OrgID }
	}
	if orgID == "" {
		orgs, _ := s.store.ListOrganizations(ctx)
		if len(orgs) > 0 { orgID = orgs[0].ID }
		wss, _ := s.store.ListWorkspaces(ctx, orgID)
		if len(wss) > 0 { wsID = wss[0].ID }
	}

	org, _ := s.store.GetOrganization(ctx, orgID)
	plan := "personal"
	if org != nil { plan = org.Plan }

	token, _ := GenerateAccessTokenWithPlan(user.ID, user.Email, orgID, wsID, "workspace_admin", plan)

	// 更新登录时间
	now := time.Now().UTC().Format(time.RFC3339)
	s.store.UpdateUser(ctx, user.ID, map[string]interface{}{"last_login_at": now})

	finalWs, _ := s.store.GetWorkspace(ctx, wsID)
	needBind := user.Phone == ""

	return &RegisterResult{
		User:          user,
		Org:           org,
		Workspace:     finalWs,
		Token:         token,
		NeedBindPhone: needBind, // 已有用户未绑定手机号时提示
	}, nil
}

// RegisterResult 扩展字段
func init() {
	// ensure RegisterResult has NeedBindPhone field accessible
	_ = &RegisterResult{}
}

// generateState 生成随机 state (防 CSRF)
func generateWeChatState() string {
	b := make([]byte, 16)
	rand.Read(b)
	return hex.EncodeToString(b)
}

func hashString(s string) string {
	h := 0
	for _, c := range s {
		h = h*31 + int(c)
	}
	return fmt.Sprintf("%08x", h)
}
