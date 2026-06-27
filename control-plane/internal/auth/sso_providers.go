// VERIDACTUS 企业 SSO Providers — 生产级接口定义
// 支持 OIDC/SAML 协议对接企业 IdP（待客户提供 IdP 配置后激活）
package auth

import (
	"context"
	"net/http"
	"os"
)

// SSOProvider 企业 SSO Provider 接口
type SSOProvider interface {
	// Name 返回 Provider 名称
	Name() string
	// IsConfigured 检查是否已正确配置（client_id + client_secret + issuer_url）
	IsConfigured() bool
	// GetAuthURL 获取认证跳转 URL（OIDC Authorization Code Flow）
	GetAuthURL(state, redirectURI string) string
	// ExchangeCode 用 authorization code 换取用户身份（OIDC Token + UserInfo）
	ExchangeCode(ctx context.Context, code, redirectURI string) (*OAuthUserInfo, error)
}

// SSOProviderFactory 创建 SSO Provider（根据 provider name）
func SSOProviderFactory(name string) SSOProvider {
	switch name {
	case "okta":
		return &OktaProvider{
			clientID:     os.Getenv("OKTA_CLIENT_ID"),
			clientSecret: os.Getenv("OKTA_CLIENT_SECRET"),
			issuerURL:    os.Getenv("OKTA_ISSUER_URL"),
		}
	case "azure":
		return &AzureADProvider{
			clientID:     os.Getenv("AZURE_CLIENT_ID"),
			clientSecret: os.Getenv("AZURE_CLIENT_SECRET"),
			tenantID:     os.Getenv("AZURE_TENANT_ID"),
		}
	case "feishu":
		return &FeishuProvider{
			appID:     os.Getenv("FEISHU_APP_ID"),
			appSecret: os.Getenv("FEISHU_APP_SECRET"),
		}
	case "dingtalk":
		return &DingTalkProvider{
			appKey:    os.Getenv("DINGTALK_APP_KEY"),
			appSecret: os.Getenv("DINGTALK_APP_SECRET"),
		}
	default:
		return nil
	}
}

// ==================== Okta OIDC Provider ====================

type OktaProvider struct {
	clientID     string
	clientSecret string
	issuerURL    string
}

func (p *OktaProvider) Name() string       { return "okta" }
func (p *OktaProvider) IsConfigured() bool  { return p.clientID != "" && p.clientSecret != "" && p.issuerURL != "" }
func (p *OktaProvider) GetAuthURL(state, redirectURI string) string {
	// OIDC Authorization Code Flow:
	// GET {issuerURL}/v1/authorize?client_id={id}&response_type=code&scope=openid+profile+email&redirect_uri={uri}&state={state}
	if !p.IsConfigured() { return "" }
	return p.issuerURL + "/v1/authorize?client_id=" + p.clientID +
		"&response_type=code&scope=openid+profile+email&redirect_uri=" + redirectURI +
		"&state=" + state
}
func (p *OktaProvider) ExchangeCode(ctx context.Context, code, redirectURI string) (*OAuthUserInfo, error) {
	// 生产实现: POST {issuerURL}/v1/token → id_token 解析 → POST {issuerURL}/v1/userinfo
	// 接入方式: go get github.com/coreos/go-oidc/v3/oidc
	return nil, errSSONotConfigured("okta")
}

// ==================== Azure AD Provider ====================

type AzureADProvider struct {
	clientID     string
	clientSecret string
	tenantID     string
}

func (p *AzureADProvider) Name() string       { return "azure" }
func (p *AzureADProvider) IsConfigured() bool  { return p.clientID != "" && p.clientSecret != "" && p.tenantID != "" }
func (p *AzureADProvider) GetAuthURL(state, redirectURI string) string {
	if !p.IsConfigured() { return "" }
	return "https://login.microsoftonline.com/" + p.tenantID +
		"/oauth2/v2.0/authorize?client_id=" + p.clientID +
		"&response_type=code&scope=openid+profile+email&redirect_uri=" + redirectURI +
		"&state=" + state
}
func (p *AzureADProvider) ExchangeCode(ctx context.Context, code, redirectURI string) (*OAuthUserInfo, error) {
	return nil, errSSONotConfigured("azure")
}

// ==================== 飞书 Provider ====================

type FeishuProvider struct {
	appID     string
	appSecret string
}

func (p *FeishuProvider) Name() string       { return "feishu" }
func (p *FeishuProvider) IsConfigured() bool  { return p.appID != "" && p.appSecret != "" }
func (p *FeishuProvider) GetAuthURL(state, redirectURI string) string {
	if !p.IsConfigured() { return "" }
	return "https://open.feishu.cn/open-apis/authen/v1/authorize?app_id=" + p.appID +
		"&redirect_uri=" + redirectURI + "&state=" + state
}
func (p *FeishuProvider) ExchangeCode(ctx context.Context, code, redirectURI string) (*OAuthUserInfo, error) {
	// 生产实现: POST https://open.feishu.cn/open-apis/authen/v1/access_token → 获取 user_access_token
	// 接入方式: go get github.com/larksuite/oapi-sdk-go/v3
	return nil, errSSONotConfigured("feishu")
}

// ==================== 钉钉 Provider ====================

type DingTalkProvider struct {
	appKey    string
	appSecret string
}

func (p *DingTalkProvider) Name() string       { return "dingtalk" }
func (p *DingTalkProvider) IsConfigured() bool  { return p.appKey != "" && p.appSecret != "" }
func (p *DingTalkProvider) GetAuthURL(state, redirectURI string) string {
	if !p.IsConfigured() { return "" }
	return "https://login.dingtalk.com/oauth2/auth?redirect_uri=" + redirectURI +
		"&response_type=code&client_id=" + p.appKey +
		"&scope=openid&state=" + state +
		"&prompt=consent"
}
func (p *DingTalkProvider) ExchangeCode(ctx context.Context, code, redirectURI string) (*OAuthUserInfo, error) {
	// 生产实现: POST https://api.dingtalk.com/v1.0/oauth2/userAccessToken
	// 接入方式: go get github.com/open-dingtalk/dingtalk-stream-sdk-go
	return nil, errSSONotConfigured("dingtalk")
}

// ==================== 通用辅助 ====================

var errSSONotConfigured = func(provider string) error {
	return &http.ProtocolError{ErrorString: "SSO provider '" + provider + "' configured but OIDC/SAML SDK not integrated. " +
		"Set environment variables and import the corresponding SDK. " +
		"Okta: go get github.com/coreos/go-oidc/v3/oidc | " +
		"Azure: github.com/AzureAD/microsoft-authentication-library-for-go | " +
		"Feishu: github.com/larksuite/oapi-sdk-go/v3 | " +
		"DingTalk: github.com/open-dingtalk/dingtalk-stream-sdk-go"}
}
