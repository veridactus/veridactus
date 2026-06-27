// VERIDACTUS 控制平面 — 路由注册
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/veridactus/control-plane/internal/auth"
	"github.com/veridactus/control-plane/internal/crypto"
	"github.com/veridactus/control-plane/internal/model"
	"github.com/veridactus/control-plane/internal/store"
)

// Server 控制面 HTTP 服务器
type Server struct {
	store      store.StoreFacade
	oauthSvc   *auth.OAuthService
	adminKey   string
	jwtSecret  string
	dpHost     string // 数据面地址，用于模型更新通知
	httpClient *http.Client
}

// NewServer 创建服务器实例
func NewServer(s store.StoreFacade, jwtSecret, adminKey, dpHost string) *Server {
	return &Server{
		store:      s,
		oauthSvc:   auth.NewOAuthService(s, jwtSecret),
		adminKey:   adminKey,
		jwtSecret:  jwtSecret,
		dpHost:     dpHost,
		httpClient: &http.Client{Timeout: 10 * time.Second},
	}
}

// getUserOrgID 从 JWT context 获取用户组织 ID（安全方式，不含 orgs[0] 泄露）
func (srv *Server) getUserOrgID(r *http.Request) string {
	userID := auth.GetUserID(r.Context())
	if userID == "" {
		return ""
	}
	orgs, err := srv.store.ListOrganizationsByUser(r.Context(), userID)
	if err != nil || len(orgs) == 0 {
		return ""
	}
	return orgs[0].ID
}

// canAccessResource 显式校验当前用户是否有权访问指定资源的工作空间
// 返回 true 表示用户有权访问（或为平台管理员）
func (srv *Server) canAccessResource(r *http.Request, resourceWSID string) bool {
	if resourceWSID == "" {
		return false
	}
	userRole := auth.GetRole(r.Context())
	if userRole == auth.RolePlatformAdmin {
		return true
	}
	userWSID := srv.getWorkspaceIDSafe(r)
	return userWSID == resourceWSID
}

// canAccessOrg 显式校验当前用户是否有权访问指定组织
func (srv *Server) canAccessOrg(r *http.Request, orgID string) bool {
	if orgID == "" {
		return false
	}
	userRole := auth.GetRole(r.Context())
	if userRole == auth.RolePlatformAdmin {
		return true
	}
	userOrgID := srv.getUserOrgID(r)
	return userOrgID == orgID
}

// requirePermission 检查当前用户是否有指定权限（Casbin RBAC 驱动）
// 若鉴权失败，向 ResponseWriter 写入 403 并返回 false
func (srv *Server) requirePermission(w http.ResponseWriter, r *http.Request, permission string) bool {
	role := auth.GetRole(r.Context())
	if auth.CheckPermission(role, permission) {
		return true
	}
	jsonError(w, http.StatusForbidden, "forbidden",
		fmt.Sprintf("permission '%s' required (role: %s)", permission, role))
	return false
}

// getWorkspaceIDSafe 从 JWT context 获取 workspace ID，若为空则通过用户组织查找
func (srv *Server) getWorkspaceIDSafe(r *http.Request) string {
	wsID := auth.GetWorkspaceID(r.Context())
	if wsID != "" {
		return wsID
	}
	orgID := srv.getUserOrgID(r)
	if orgID == "" {
		return ""
	}
	wss, err := srv.store.ListWorkspaces(r.Context(), orgID)
	if err != nil || len(wss) == 0 {
		return ""
	}
	return wss[0].ID
}

// RegisterRoutes 注册所有路由
func (srv *Server) RegisterRoutes(mux *http.ServeMux) {
	// ==================== 公开端点 ====================
	mux.HandleFunc("/api/v1/health", srv.handleHealth())
	mux.HandleFunc("/api/v1/config/poll", srv.handleConfigPoll())

	// ==================== OAuth 认证 ====================
	mux.HandleFunc("/api/v1/auth/login/github", srv.handleOAuthLogin("github"))
	mux.HandleFunc("/api/v1/auth/callback/github", srv.handleOAuthCallback("github"))
	mux.HandleFunc("/api/v1/auth/login/google", srv.handleOAuthLogin("google"))
	mux.HandleFunc("/api/v1/auth/callback/google", srv.handleOAuthCallback("google"))
	mux.HandleFunc("/api/v1/auth/login/wechat", srv.handleOAuthLogin("wechat"))
	mux.HandleFunc("/api/v1/auth/callback/wechat", srv.handleOAuthCallback("wechat"))
	mux.HandleFunc("/api/v1/auth/wechat/callback-page", srv.handleWeChatCallbackPage())
	mux.HandleFunc("/api/v1/auth/wechat/status", srv.handleWeChatStatus())
	mux.HandleFunc("/api/v1/auth/bind-phone", srv.handleBindPhone())
	mux.Handle("/api/v1/auth/register", auth.RateLimitRegister(srv.handleEmailRegister()))
	mux.Handle("/api/v1/auth/login", auth.RateLimitLogin(srv.handleEmailLogin()))
	mux.Handle("/api/v1/auth/phone/send", auth.RateLimitPhone(srv.handlePhoneSendCode()))
	mux.HandleFunc("/api/v1/auth/phone/verify", srv.handlePhoneVerify())
	mux.HandleFunc("/api/v1/auth/refresh", srv.handleRefreshToken())
	mux.HandleFunc("/api/v1/auth/logout", srv.handleLogout())

	// ==================== 需要认证的端点 ====================
	// 这些路由需要包装在 JWT 中间件中
	// 在 main.go 中通过 middleware chain 实现
	mux.HandleFunc("/api/v1/orgs", srv.handleOrgs())
	mux.HandleFunc("/api/v1/orgs/", srv.handleOrgByID())
	mux.HandleFunc("/api/v1/workspaces", srv.handleWorkspaces())
	mux.HandleFunc("/api/v1/workspaces/", srv.handleWorkspaceByID())
	mux.HandleFunc("/api/v1/workspaces/members", srv.handleMembers())
	mux.HandleFunc("/api/v1/pipelines", srv.handlePipelines())
	mux.HandleFunc("/api/v1/pipelines/", srv.handlePipelineByID())
	mux.HandleFunc("/api/v1/plugins", srv.handlePlugins())
	mux.HandleFunc("/api/v1/plugins/", srv.handlePluginByID())
	mux.HandleFunc("/api/v1/policies", srv.handlePolicies())
	mux.HandleFunc("/api/v1/policies/", srv.handlePolicyByID())
	mux.HandleFunc("/api/v1/apikeys", srv.handleApiKeys())
	mux.HandleFunc("/api/v1/apikeys/", srv.handleApiKeyByID())
	mux.HandleFunc("/api/v1/models", srv.handleModels())
	mux.HandleFunc("/api/v1/models/", srv.handleModelByID())
	mux.HandleFunc("/api/v1/virtual-keys", srv.handleVirtualKeys())
	mux.HandleFunc("/api/v1/virtual-keys/", srv.handleVirtualKeyByID())
	mux.HandleFunc("/api/v1/wallets", srv.handleWallet())
	mux.HandleFunc("/api/v1/wallets/", srv.handleWallet())
	mux.HandleFunc("/api/v1/settings", srv.handleSettings())
	mux.HandleFunc("/api/v1/settings/", srv.handleSettings())
	mux.HandleFunc("/api/v1/traces", srv.handleTraces())
	mux.HandleFunc("/api/v1/traces/", srv.handleTraces())

	// ==================== Stripe 计费 ====================
	mux.HandleFunc("/api/v1/billing/checkout", srv.handleStripeCheckout())
	mux.HandleFunc("/api/v1/billing/webhook", srv.handleStripeWebhook())

	// ==================== Platform LLM Pool ====================
	mux.HandleFunc("/api/v1/platform/pool", srv.handlePlatformPool())
	mux.HandleFunc("/api/v1/platform/models", srv.handlePlatformModels())

	// ==================== Phase 4: 企业级端点 ====================
	mux.HandleFunc("/api/v1/enterprise/sso", srv.handleSSOConfig())
	mux.HandleFunc("/api/v1/audit/events", srv.handleAuditEvents())
	mux.HandleFunc("/api/v1/compliance/reports", srv.handleComplianceReport())
	mux.HandleFunc("/api/v1/brand", srv.handleBrandSettings())

	// ==================== 内部端点 (数据面调用) ====================
	mux.HandleFunc("/internal/resolve-key", srv.handleResolveKey())
}

// ==================== 通用工具函数 ====================

func jsonResponse(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

func jsonError(w http.ResponseWriter, status int, code, msg string) {
	jsonResponse(w, status, map[string]string{"error": code, "message": msg})
}

func decodeJSON(r *http.Request, v any) error {
	return json.NewDecoder(r.Body).Decode(v)
}

// extractPathID 从 URL 路径中提取 ID (/api/v1/resource/{id})
func extractPathID(path, prefix string) string {
	trimmed := strings.TrimPrefix(path, prefix)
	trimmed = strings.TrimPrefix(trimmed, "/")
	return trimmed
}

// ==================== 健康检查 ====================

func (srv *Server) handleHealth() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only GET allowed")
			return
		}
		status := "ok"
		if err := srv.store.HealthCheck(r.Context()); err != nil {
			status = "degraded"
			log.Printf("health check: %v", err)
		}
		jsonResponse(w, http.StatusOK, map[string]string{
			"status": status, "version": "0.3.0-dev", "phase": "multi-tenant",
		})
	}
}

// ==================== OAuth ====================

func (srv *Server) handleOAuthLogin(provider string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch provider {
		case "github":
			gh := auth.NewGitHubProvider()
			if !gh.IsConfigured() {
				jsonResponse(w, http.StatusOK, map[string]interface{}{
					"auth_url": "", "error": "github_oauth_not_configured",
					"message": "GitHub OAuth 未配置",
					"fallback": "email", "email_enabled": true,
				})
				return
			}
			state := generateState()
			jsonResponse(w, http.StatusOK, map[string]string{"auth_url": gh.GetAuthURL(state), "state": state})
		case "google":
			gp := auth.NewGoogleProvider()
			if !gp.IsConfigured() {
				jsonResponse(w, http.StatusOK, map[string]interface{}{
					"auth_url": "", "error": "google_oauth_not_configured",
					"message": "Google OAuth 未配置（需设置 GOOGLE_CLIENT_ID/GOOGLE_CLIENT_SECRET/GOOGLE_REDIRECT_URI）",
					"fallback": "email", "email_enabled": true,
				})
				return
			}
			state := generateState()
			jsonResponse(w, http.StatusOK, map[string]string{"auth_url": gp.GetAuthURL(state), "state": state})
		case "wechat":
			wx := auth.NewWeChatProvider()
			state := generateState()
			auth.RegisterWeChatState(state) // 🔑 注册状态用于轮询
			devCode := fmt.Sprintf("dev_%s", state[:16])
			result := map[string]interface{}{
				"state":      state,
				"login_url":  wx.GetAuthURL(state),
				"dev_mode":   !wx.IsConfigured(),
				"dev_code":   devCode,
				"qr_url":     "", // 由前端生成
				"status_url": fmt.Sprintf("/api/v1/auth/wechat/status?state=%s", state),
			}
			if !wx.IsConfigured() {
				result["dev_url"] = fmt.Sprintf("/api/v1/auth/wechat/callback-page?code=%s&state=%s", devCode, state)
			}
			jsonResponse(w, http.StatusOK, result)
		default:
			jsonError(w, http.StatusBadRequest, "unsupported_provider", "支持: github, google, wechat")
		}
	}
}

func (srv *Server) handleOAuthCallback(provider string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		code := r.URL.Query().Get("code")
		if code == "" {
			jsonError(w, http.StatusBadRequest, "missing_code", "code is required")
			return
		}

		// WeChat 使用独立的 LoginOrCreate 服务
		if provider == "wechat" {
			wx := auth.NewWeChatProvider()
			info, err := wx.ExchangeCode(r.Context(), code)
			if err != nil {
				jsonError(w, http.StatusUnauthorized, "wechat_failed", err.Error())
				return
			}
			svc := auth.NewWeChatLoginService(srv.store, srv.jwtSecret)
			result, err := svc.LoginOrCreateByWeChat(r.Context(), info)
			if err != nil {
				jsonError(w, http.StatusInternalServerError, "login_failed", err.Error())
				return
			}
			// 🔑 标记 state 完成，前端轮询会检测到
			stateParam := r.URL.Query().Get("state")
			if stateParam != "" {
				auth.CompleteWeChatState(stateParam, result.Token, result.NeedBindPhone)
			}
			jsonResponse(w, http.StatusOK, result)
			return
		}

		var prov auth.OAuthProvider
		switch provider {
		case "github":
			prov = auth.NewGitHubProvider()
		case "google":
			prov = auth.NewGoogleProvider()
		default:
			jsonError(w, http.StatusBadRequest, "unsupported", "provider not supported")
			return
		}

		info, err := prov.ExchangeCode(r.Context(), code)
		if err != nil {
			log.Printf("oauth exchange: %v", err)
			jsonError(w, http.StatusUnauthorized, "oauth_failed", "failed to exchange code")
			return
		}

		user, org, ws, err := srv.oauthSvc.LoginOrCreate(r.Context(), info)
		if err != nil {
			log.Printf("login/create: %v", err)
			jsonError(w, http.StatusInternalServerError, "login_failed", err.Error())
			return
		}

		// 签发 JWT
		accessToken, err := auth.GenerateAccessToken(user.ID, user.Email, org.ID, ws.ID, "workspace_admin")
		if err != nil {
			jsonError(w, http.StatusInternalServerError, "token_failed", "failed to generate token")
			return
		}

		// 生成刷新令牌
		refreshToken, _ := crypto.GenerateRefreshToken()
		refreshHash := hashToken(refreshToken)
		srv.store.CreateRefreshToken(r.Context(), user.ID, refreshHash, timeNow().Add(30*24*time.Hour).UTC().Format(time.RFC3339))

		jsonResponse(w, http.StatusOK, map[string]interface{}{
			"access_token":  accessToken,
			"refresh_token": refreshToken,
			"token_type":    "Bearer",
			"expires_in":    900,
			"user":          user,
			"org":           org,
			"workspace":     ws,
		})
	}
}

func (srv *Server) handleRefreshToken() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST")
			return
		}
		var req struct{ RefreshToken string `json:"refresh_token"` }
		if err := decodeJSON(r, &req); err != nil {
			jsonError(w, http.StatusBadRequest, "invalid_body", err.Error())
			return
		}

		rt, err := srv.store.GetRefreshToken(r.Context(), hashToken(req.RefreshToken))
		if err != nil || rt == nil {
			jsonError(w, http.StatusUnauthorized, "invalid_token", "refresh token invalid or expired")
			return
		}
		expTime, _ := time.Parse(time.RFC3339, rt.ExpiresAt)
		if time.Now().After(expTime) {
			srv.store.RevokeRefreshToken(r.Context(), rt.TokenHash)
			jsonError(w, http.StatusUnauthorized, "expired_token", "refresh token expired")
			return
		}

		// 获取用户信息
		user, _ := srv.store.GetUser(r.Context(), rt.UserID)
		if user == nil {
			jsonError(w, http.StatusUnauthorized, "user_not_found", "")
			return
		}

		// 签发新 token（使用用户所属组织）
		orgs, _ := srv.store.ListOrganizationsByUser(r.Context(), user.ID)
		if len(orgs) == 0 {
			jsonError(w, http.StatusInternalServerError, "no_org", "user has no organization")
			return
		}
		wss, _ := srv.store.ListWorkspaces(r.Context(), orgs[0].ID)
		if len(wss) == 0 {
			jsonError(w, http.StatusInternalServerError, "no_ws", "organization has no workspace")
			return
		}
		wsMember, _ := srv.store.GetMember(r.Context(), wss[0].ID, user.ID)
		role := "workspace_admin"
		if wsMember != nil {
			role = wsMember.Role
		}
		accessToken, _ := auth.GenerateAccessToken(user.ID, user.Email, orgs[0].ID, wss[0].ID, role)

		// 吊销旧 token，签发新的
		srv.store.RevokeRefreshToken(r.Context(), rt.TokenHash)
		newRefresh, _ := crypto.GenerateRefreshToken()
		srv.store.CreateRefreshToken(r.Context(), user.ID, hashToken(newRefresh), timeNow().Add(30*24*time.Hour).Format(time.RFC3339))

		jsonResponse(w, http.StatusOK, map[string]interface{}{
			"access_token": accessToken, "refresh_token": newRefresh, "token_type": "Bearer", "expires_in": 900,
		})
	}
}

func (srv *Server) handleLogout() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST")
			return
		}
		userID := auth.GetUserID(r.Context())
		if userID != "" {
			srv.store.RevokeUserRefreshTokens(r.Context(), userID)
		}
		jsonResponse(w, http.StatusOK, map[string]string{"status": "logged_out"})
	}
}

// ==================== 组织 ====================

func (srv *Server) handleOrgs() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			userID := auth.GetUserID(r.Context())
			role := auth.GetRole(r.Context())
			var orgs []model.Organization
			var err error
			// 平台管理员可查看所有组织，普通用户仅查看自己所属组织
			if role == auth.RolePlatformAdmin {
				orgs, err = srv.store.ListOrganizations(r.Context())
			} else if userID != "" {
				orgs, err = srv.store.ListOrganizationsByUser(r.Context(), userID)
			} else {
				orgs = []model.Organization{}
			}
			if err != nil {
				jsonError(w, http.StatusInternalServerError, "db_error", err.Error())
				return
			}
			jsonResponse(w, http.StatusOK, map[string]interface{}{"organizations": orgs})
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

func (srv *Server) handleOrgByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := extractPathID(r.URL.Path, "/api/v1/orgs/")
		if id == "" {
			jsonError(w, http.StatusBadRequest, "missing_id", "")
			return
		}
		switch r.Method {
		case http.MethodGet:
			// 显式所有权校验：确保用户有权访问此组织
			if !srv.canAccessOrg(r, id) {
				jsonError(w, http.StatusForbidden, "forbidden", "access denied to this organization")
				return
			}
			org, err := srv.store.GetOrganization(r.Context(), id)
			if err != nil {
				jsonError(w, http.StatusNotFound, "not_found", err.Error())
				return
			}
			jsonResponse(w, http.StatusOK, org)
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

// ==================== 工作空间 ====================

func (srv *Server) handleWorkspaces() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		orgID := r.URL.Query().Get("org_id")

		switch r.Method {
		case http.MethodGet:
			if orgID == "" {
				// 从 JWT context 安全推断 org
				orgID = srv.getUserOrgID(r)
			}
			wss, err := srv.store.ListWorkspaces(r.Context(), orgID)
			if err != nil {
				jsonError(w, http.StatusInternalServerError, "db_error", err.Error())
				return
			}
			jsonResponse(w, http.StatusOK, map[string]interface{}{"workspaces": wss})
		case http.MethodPost:
			if !srv.requirePermission(w, r, "workspace:write") { return }
			// 需要 workspace_admin+ 角色
			var wsReq struct {
				Name        string `json:"name"`
				Description string `json:"description,omitempty"`
			}
			if err := decodeJSON(r, &wsReq); err != nil {
				jsonError(w, http.StatusBadRequest, "invalid_body", err.Error())
				return
			}
			if wsReq.Name == "" {
				jsonError(w, http.StatusBadRequest, "missing_name", "name is required")
				return
			}
			orgID := srv.getUserOrgID(r)
			if orgID == "" {
				jsonError(w, http.StatusBadRequest, "no_org", "cannot determine user organization")
				return
			}
			ws := &model.Workspace{
				ID:          uuid.New().String(),
				OrgID:       orgID,
				Name:        wsReq.Name,
				Slug:        sanitizeSlug(wsReq.Name),
				Description: wsReq.Description,
			}
			if err := srv.store.CreateWorkspace(r.Context(), ws); err != nil {
				jsonError(w, http.StatusInternalServerError, "create_failed", err.Error())
				return
			}
			jsonResponse(w, http.StatusCreated, ws)
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

func (srv *Server) handleWorkspaceByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := extractPathID(r.URL.Path, "/api/v1/workspaces/")
		if id == "" { jsonError(w, http.StatusBadRequest, "missing_id", ""); return }
		switch r.Method {
		case http.MethodGet:
			ws, err := srv.store.GetWorkspace(r.Context(), id)
			if err != nil { jsonError(w, http.StatusNotFound, "not_found", err.Error()); return }
			jsonResponse(w, http.StatusOK, ws)
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

func (srv *Server) handleMembers() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		wsID := r.URL.Query().Get("workspace_id")
		if wsID == "" { wsID = auth.GetWorkspaceID(r.Context()) }
		if wsID == "" { jsonError(w, http.StatusBadRequest, "missing_workspace_id", ""); return }
		switch r.Method {
		case http.MethodGet:
			members, err := srv.store.ListMembers(r.Context(), wsID)
			if err != nil { jsonError(w, http.StatusInternalServerError, "db_error", err.Error()); return }
			jsonResponse(w, http.StatusOK, map[string]interface{}{"members": members})
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

// ==================== Pipeline (委托) ====================

func (srv *Server) handlePipelines() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		wsID := srv.getWorkspaceIDSafe(r)
		orgID := srv.getUserOrgID(r)
		switch r.Method {
		case http.MethodGet:
			ps, err := srv.store.ListPipelines(r.Context(), wsID)
			if err != nil { jsonError(w, http.StatusInternalServerError, "db_error", err.Error()); return }
			jsonResponse(w, http.StatusOK, map[string]interface{}{"total": len(ps), "pipelines": ps})
		case http.MethodPost:
			if !srv.requirePermission(w, r, "pipeline:write") { return }
			var p model.Pipeline
			if err := decodeJSON(r, &p); err != nil {
				jsonError(w, http.StatusBadRequest, "invalid_body", err.Error()); return
			}
			pid := uuid.New().String()
			p.ID = pid; p.PlanID = pid; p.WorkspaceID = wsID
			p.OrgID = orgID
			p.Created = timeNow().UTC().Format(time.RFC3339)
			if p.Name == "" { p.Name = "Untitled" }
			if p.Tenant == "" { p.Tenant = "default" }
			if err := srv.store.CreatePipeline(r.Context(), &p); err != nil {
				jsonError(w, http.StatusInternalServerError, "create_failed", err.Error()); return
			}
			jsonResponse(w, http.StatusCreated, p)
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

func (srv *Server) handlePipelineByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := extractPathID(r.URL.Path, "/api/v1/pipelines/")
		if id == "" { jsonError(w, http.StatusBadRequest, "missing_id", ""); return }
		switch r.Method {
		case http.MethodGet:
			p, err := srv.store.GetPipeline(r.Context(), id)
			if err != nil { jsonError(w, http.StatusNotFound, "not_found", err.Error()); return }
			// 显式所有权校验：确保 pipeline 属于用户的工作空间
			if !srv.canAccessResource(r, p.WorkspaceID) {
				jsonError(w, http.StatusForbidden, "forbidden", "access denied to this pipeline")
				return
			}
			jsonResponse(w, http.StatusOK, p)
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

// ==================== 其他 Handler 占位 ====================
// 完整实现由后续 Phase 1 迭代补充

func (srv *Server) handlePlugins() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet { jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only GET"); return }
		wsID := auth.GetWorkspaceID(r.Context())
		ps, err := srv.store.ListPlugins(r.Context(), wsID)
		if err != nil { jsonError(w, http.StatusInternalServerError, "db_error", err.Error()); return }
		jsonResponse(w, http.StatusOK, map[string]interface{}{"plugins": ps})
	}
}
func (srv *Server) handlePluginByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		jsonError(w, http.StatusNotImplemented, "not_implemented", "coming in Phase 1.1")
	}
}
func (srv *Server) handlePolicies() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet { jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only GET"); return }
		wsID := auth.GetWorkspaceID(r.Context())
		ps, err := srv.store.ListPolicies(r.Context(), wsID)
		if err != nil { jsonError(w, http.StatusInternalServerError, "db_error", err.Error()); return }
		jsonResponse(w, http.StatusOK, map[string]interface{}{"policies": ps})
	}
}
func (srv *Server) handlePolicyByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		jsonError(w, http.StatusNotImplemented, "not_implemented", "coming in Phase 1.1")
	}
}
func (srv *Server) handleApiKeys() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet { jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only GET"); return }
		wsID := auth.GetWorkspaceID(r.Context())
		keys, err := srv.store.ListApiKeys(r.Context(), wsID)
		if err != nil { jsonError(w, http.StatusInternalServerError, "db_error", err.Error()); return }
		jsonResponse(w, http.StatusOK, map[string]interface{}{"keys": keys})
	}
}
func (srv *Server) handleApiKeyByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet { jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only GET"); return }
		id := extractPathID(r.URL.Path, "/api/v1/apikeys/")
		k, err := srv.store.GetApiKey(r.Context(), id)
		if err != nil { jsonError(w, http.StatusNotFound, "not_found", ""); return }
		jsonResponse(w, http.StatusOK, k)
	}
}
func (srv *Server) handleModels() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		wsID := auth.GetWorkspaceID(r.Context())
		switch r.Method {
		case http.MethodGet:
			models, err := srv.store.ListModels(r.Context(), wsID)
			if err != nil { jsonError(w, http.StatusInternalServerError, "db_error", err.Error()); return }
			jsonResponse(w, http.StatusOK, map[string]interface{}{"models": models})
		case http.MethodPost:
			// ⚠️ 生产环境应启用 RBAC: requirePermission(w, r, "model:create")
			var m model.ModelConfig
			if err := decodeJSON(r, &m); err != nil { jsonError(w, http.StatusBadRequest, "invalid_json", err.Error()); return }
			if m.ID == "" { m.ID = uuid.New().String() }
			m.WorkspaceID = wsID
			m.OrgID = srv.getUserOrgID(r)
			if m.Status == "" { m.Status = "active" }
			if err := srv.store.CreateModel(r.Context(), &m); err != nil {
				jsonError(w, http.StatusInternalServerError, "create_failed", err.Error()); return
			}
			_ = srv.store.IncrementConfigVersion(r.Context(), "model")
			go srv.notifyDataPlaneModelRefresh(m.Name) // 异步通知数据面刷新
			jsonResponse(w, http.StatusCreated, m)
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only GET/POST")
		}
	}
}
func (srv *Server) handleModelByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := extractPathID(r.URL.Path, "/api/v1/models/")
		switch r.Method {
		case http.MethodGet:
			m, err := srv.store.GetModel(r.Context(), id)
			if err != nil { jsonError(w, http.StatusNotFound, "not_found", ""); return }
			jsonResponse(w, http.StatusOK, m)
		case http.MethodPut:
			var m model.ModelConfig
			if err := decodeJSON(r, &m); err != nil { jsonError(w, http.StatusBadRequest, "invalid_json", err.Error()); return }
			modelName := m.Name
			if err := srv.store.UpdateModel(r.Context(), id, &m); err != nil {
				jsonError(w, http.StatusInternalServerError, "update_failed", err.Error()); return
			}
			_ = srv.store.IncrementConfigVersion(r.Context(), "model")
			go srv.notifyDataPlaneModelRefresh(modelName) // 异步通知数据面刷新
			jsonResponse(w, http.StatusOK, m)
		case http.MethodDelete:
			// 删除前获取模型名，用于通知数据面
			if existing, _ := srv.store.GetModel(r.Context(), id); existing != nil {
				modelName := existing.Name
				if err := srv.store.DeleteModel(r.Context(), id); err != nil {
					jsonError(w, http.StatusInternalServerError, "delete_failed", err.Error()); return
				}
				_ = srv.store.IncrementConfigVersion(r.Context(), "model")
				go srv.notifyDataPlaneModelRefresh(modelName) // 异步通知
				jsonResponse(w, http.StatusOK, map[string]string{"status": "deleted"})
				return
			}
			if err := srv.store.DeleteModel(r.Context(), id); err != nil {
				jsonError(w, http.StatusInternalServerError, "delete_failed", err.Error()); return
			}
			_ = srv.store.IncrementConfigVersion(r.Context(), "model")
			jsonResponse(w, http.StatusOK, map[string]string{"status": "deleted"})
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only GET/PUT/DELETE")
		}
	}
}

// notifyDataPlaneModelRefresh 通过 HTTP POST 通知数据面刷新指定模型的 API key
func (srv *Server) notifyDataPlaneModelRefresh(modelName string) {
	if srv.dpHost == "" {
		return // 数据面未配置，跳过通知（开发环境）
	}
	url := fmt.Sprintf("http://%s/internal/refresh-model", srv.dpHost)
	body, _ := json.Marshal(map[string]string{"model_name": modelName})
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		logWarn("notifyDataPlane: failed to create request", "error", err)
		return
	}
	req.Header.Set("Content-Type", "application/json")
	// 内部端点使用 Admin Key 鉴权
	if srv.adminKey != "" {
		req.Header.Set("X-Admin-Key", srv.adminKey)
	}
	resp, err := srv.httpClient.Do(req)
	if err != nil {
		logWarn("notifyDataPlane: request failed", "model", modelName, "error", err)
		return
	}
	defer resp.Body.Close()
	if resp.StatusCode == http.StatusOK {
		logInfo("notifyDataPlane: model refresh notified", "model", modelName)
	} else {
		logWarn("notifyDataPlane: unexpected response", "model", modelName, "status", resp.StatusCode)
	}
}

func (srv *Server) handleVirtualKeys() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		wsID := srv.getWorkspaceIDSafe(r)
		userID := auth.GetUserID(r.Context())
		switch r.Method {
		case http.MethodGet:
			keys, err := srv.store.ListVirtualKeys(r.Context(), wsID)
			if err != nil { jsonError(w, http.StatusInternalServerError, "db_error", err.Error()); return }
			jsonResponse(w, http.StatusOK, map[string]interface{}{"virtual_keys": keys})
		case http.MethodPost:
			if !srv.requirePermission(w, r, "virtual_key:create") { return }
			var req struct {
				Name          string   `json:"name"`
				Type          string   `json:"type"`
				ProviderKey   string   `json:"provider_key,omitempty"`
				AllowedModels []string `json:"allowed_models,omitempty"`
			}
			if err := decodeJSON(r, &req); err != nil {
				jsonError(w, http.StatusBadRequest, "invalid_body", err.Error()); return
			}
			if req.Name == "" || req.Type == "" {
				jsonError(w, http.StatusBadRequest, "missing_fields", "name and type required"); return
			}
			if req.Type != "byok" && req.Type != "platform" {
				jsonError(w, http.StatusBadRequest, "invalid_type", "type must be byok or platform"); return
			}
			if req.Type == "byok" && req.ProviderKey == "" {
				jsonError(w, http.StatusBadRequest, "missing_key", "provider_key required for BYOK"); return
			}

			key, err := crypto.GenerateAPIKey()
			if err != nil { jsonError(w, http.StatusInternalServerError, "gen_failed", ""); return }
			keyHash := crypto.HashKey(key)

			var encryptedKey string
			if req.Type == "byok" {
				env, err := crypto.EncryptProviderKey(req.ProviderKey, "")
				if err != nil { jsonError(w, http.StatusInternalServerError, "encrypt_failed", err.Error()); return }
				encBytes, _ := json.Marshal(env)
				encryptedKey = string(encBytes)
			}

			allowedJSON, _ := json.Marshal(req.AllowedModels)
			vk := &model.VirtualKey{
				ID: uuid.New().String(), WorkspaceID: wsID, Name: req.Name,
				KeyPrefix: "vd-" + key[:8], KeyHash: keyHash, Type: req.Type,
				ProviderKeyEncrypted: encryptedKey, AllowedModels: string(allowedJSON),
				RateLimitRPM: 60, RateLimitTPM: 100000, Status: "active", CreatedBy: userID,
			}
			created, err := srv.store.CreateVirtualKey(r.Context(), vk)
			if err != nil { jsonError(w, http.StatusInternalServerError, "create_failed", err.Error()); return }
			jsonResponse(w, http.StatusCreated, map[string]interface{}{
				"virtual_key": created, "full_key": key,
			})
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}
func (srv *Server) handleVirtualKeyByID() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := extractPathID(r.URL.Path, "/api/v1/virtual-keys/")
		if id == "" { jsonError(w, http.StatusBadRequest, "missing_id", ""); return }
		switch r.Method {
		case http.MethodGet:
			vk, err := srv.store.GetVirtualKey(r.Context(), id)
			if err != nil { jsonError(w, http.StatusNotFound, "not_found", err.Error()); return }
			jsonResponse(w, http.StatusOK, vk)
		case http.MethodDelete:
			if err := srv.store.RevokeVirtualKey(r.Context(), id); err != nil {
				jsonError(w, http.StatusInternalServerError, "revoke_failed", err.Error()); return
			}
			jsonResponse(w, http.StatusOK, map[string]string{"status": "revoked"})
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}
func (srv *Server) handleWallet() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		wsID := srv.getWorkspaceIDSafe(r)
		switch r.Method {
		case http.MethodGet:
			wallet, err := srv.store.GetWallet(r.Context(), wsID)
			if err != nil { jsonError(w, http.StatusNotFound, "not_found", "no wallet"); return }
			jsonResponse(w, http.StatusOK, wallet)
		case http.MethodPost:
			if !srv.requirePermission(w, r, "billing:write") { return }
			var req struct{ AmountMicro int64 `json:"amount_usd_micro"` }
			if err := decodeJSON(r, &req); err != nil { jsonError(w, http.StatusBadRequest, "invalid_body", ""); return }
			if req.AmountMicro <= 0 { jsonError(w, http.StatusBadRequest, "invalid_amount", "amount must be positive"); return }
			wallet, err := srv.store.GetWallet(r.Context(), wsID)
			if err != nil { jsonError(w, http.StatusNotFound, "no_wallet", "create workspace first"); return }
			if err := srv.store.UpdateWalletBalance(r.Context(), wsID, req.AmountMicro); err != nil {
				jsonError(w, http.StatusInternalServerError, "topup_failed", err.Error()); return
			}
			tx := &model.Transaction{
				ID: uuid.New().String(), WorkspaceID: wsID, WalletID: wallet.ID,
				Type: "credit", AmountUSDMicro: req.AmountMicro,
				BalanceAfterMicro: wallet.BalanceUSDMicro + req.AmountMicro,
				Description: "Manual topup",
			}
			srv.store.CreateTransaction(r.Context(), tx)
			updated, _ := srv.store.GetWallet(r.Context(), wsID)
			jsonResponse(w, http.StatusOK, updated)
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}
func (srv *Server) handleSettings() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		wsID := auth.GetWorkspaceID(r.Context())
		switch r.Method {
		case http.MethodGet:
			settings, _ := srv.store.GetSettings(r.Context(), wsID)
			jsonResponse(w, http.StatusOK, map[string]interface{}{"settings": settings})
		case http.MethodPost:
			if !srv.requirePermission(w, r, "settings:write") { return }
			var req struct{ Settings map[string]string `json:"settings"` }
			if err := decodeJSON(r, &req); err != nil { jsonError(w, http.StatusBadRequest, "invalid_body", ""); return }
			srv.store.UpdateSettings(r.Context(), wsID, req.Settings)
			jsonResponse(w, http.StatusOK, map[string]string{"status": "saved"})
		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}
func (srv *Server) handleTraces() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet { jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only GET"); return }
		jsonResponse(w, http.StatusOK, map[string]interface{}{
			"total": 0, "traces": []interface{}{},
			"hint": "traces are stored in data plane (port 8080)",
		})
	}
}
func (srv *Server) handleConfigPoll() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet { jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only GET"); return }

		// 解析客户端当前版本号
		q := r.URL.Query()
		_clientPV := q.Get("pv"); _clientPlV := q.Get("plv"); _clientSV := q.Get("sv"); _clientMV := q.Get("mv")
		_ = _clientPV; _ = _clientPlV; _ = _clientSV; _ = _clientMV // TODO: 按变更类型增量返回

		versions, err := srv.store.GetConfigVersions(r.Context())
		if err != nil { jsonError(w, http.StatusInternalServerError, "db_error", err.Error()); return }

		// 检测 model_version 变化 → 返回最新模型列表
		// 简化实现：每次 poll 都返回最新 model 数据（生产环境应增量推送）
		models, mErr := srv.store.ListModels(r.Context(), "")
		if mErr != nil { jsonError(w, http.StatusInternalServerError, "db_error", mErr.Error()); return }

		jsonResponse(w, http.StatusOK, map[string]interface{}{
			"change_type": "model",
			"data":        models,
			"version":     versions,
		})
	}
}
func (srv *Server) handleResolveKey() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST"); return
		}
		var req struct{ VirtualKeyHash string `json:"virtual_key_hash"`; Model string `json:"model"` }
		if err := decodeJSON(r, &req); err != nil { jsonError(w, http.StatusBadRequest, "invalid_body", ""); return }
		vk, err := srv.store.GetVirtualKeyByHash(r.Context(), req.VirtualKeyHash)
		if err != nil || vk == nil || vk.Status != "active" {
			jsonResponse(w, http.StatusOK, map[string]interface{}{"resolved": false, "error": "key not found or inactive"}); return
		}
		providerKey := ""
		if vk.Type == "byok" && vk.ProviderKeyEncrypted != "" {
			var envelope crypto.EnvelopeEncrypted
			if err := json.Unmarshal([]byte(vk.ProviderKeyEncrypted), &envelope); err == nil {
				if dec, e := crypto.DecryptProviderKey(&envelope, ""); e == nil { providerKey = dec }
			}
		}
		wallet, _ := srv.store.GetWallet(r.Context(), vk.WorkspaceID)
		remaining := int64(0)
		if wallet != nil { remaining = wallet.BalanceUSDMicro }
		jsonResponse(w, http.StatusOK, map[string]interface{}{
			"resolved": true, "provider_key": providerKey,
			"workspace_id": vk.WorkspaceID, "budget_remaining_micro": remaining,
		})
	}
}

// ==================== Email/Password 注册 ====================

func (srv *Server) handleEmailRegister() http.HandlerFunc {
	svc := auth.NewEmailAuthService(srv.store)
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST"); return
		}
		var req auth.RegisterRequest
		if err := decodeJSON(r, &req); err != nil { jsonError(w, http.StatusBadRequest, "invalid_body", ""); return }
		result, err := svc.Register(r.Context(), req)
		if err != nil { jsonError(w, http.StatusBadRequest, "register_failed", err.Error()); return }
		jsonResponse(w, http.StatusCreated, result)
	}
}

func (srv *Server) handleEmailLogin() http.HandlerFunc {
	svc := auth.NewEmailAuthService(srv.store)
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST"); return
		}
		var req struct{ Email string `json:"email"`; Password string `json:"password"` }
		if err := decodeJSON(r, &req); err != nil { jsonError(w, http.StatusBadRequest, "invalid_body", ""); return }
		result, err := svc.Login(r.Context(), req.Email, req.Password)
		if err != nil { jsonError(w, http.StatusUnauthorized, "login_failed", err.Error()); return }
		jsonResponse(w, http.StatusOK, result)
	}
}

func (srv *Server) handlePhoneSendCode() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST"); return
		}
		var req struct{ Phone string `json:"phone"` }
		if err := decodeJSON(r, &req); err != nil { jsonError(w, http.StatusBadRequest, "invalid_body", ""); return }
		if req.Phone == "" { jsonError(w, http.StatusBadRequest, "missing_phone", ""); return }
		_, provider, err := auth.SendPhoneCode(r.Context(), srv.store, req.Phone)
		if err != nil { jsonError(w, http.StatusInternalServerError, "send_failed", err.Error()); return }
		jsonResponse(w, http.StatusOK, map[string]string{
			"phone": req.Phone, "status": "sent", "provider": provider,
		})
	}
}

// handleBindPhone 微信登录后绑定手机号
func (srv *Server) handleBindPhone() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST"); return
		}
		var req struct {
			Phone string `json:"phone"`
			Code  string `json:"code"`
		}
		if err := decodeJSON(r, &req); err != nil { jsonError(w, http.StatusBadRequest, "invalid_body", ""); return }
		userID := auth.GetUserID(r.Context())
		if userID == "" { jsonError(w, http.StatusUnauthorized, "auth_required", "请先登录"); return }
		if err := auth.BindPhoneAfterWeChatLogin(r.Context(), srv.store, userID, req.Phone, req.Code); err != nil {
			jsonError(w, http.StatusBadRequest, "bind_failed", err.Error()); return
		}
		jsonResponse(w, http.StatusOK, map[string]string{"status": "bound", "phone": req.Phone})
	}
}

func (srv *Server) handlePhoneVerify() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost { jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST"); return }
		var req struct{ Phone string `json:"phone"`; Code string `json:"code"` }
		if err := decodeJSON(r, &req); err != nil { jsonError(w, http.StatusBadRequest, "invalid_body", ""); return }
		if auth.VerifyPhoneCode(r.Context(), srv.store, req.Phone, req.Code) {
			jsonResponse(w, http.StatusOK, map[string]string{"status": "verified"})
		} else {
			jsonError(w, http.StatusBadRequest, "invalid_code", "verification failed")
		}
	}
}

// handleWeChatStatus 微信扫码状态轮询
func (srv *Server) handleWeChatStatus() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		state := r.URL.Query().Get("state")
		if state == "" { jsonError(w, http.StatusBadRequest, "missing_state", ""); return }
		ws := auth.GetWeChatState(state)
		if ws == nil { jsonResponse(w, http.StatusOK, map[string]string{"status": "pending"}); return }
		resp := map[string]interface{}{"status": ws.Status}
		if ws.Status == "completed" { resp["token"] = ws.Token; resp["need_bind_phone"] = ws.NeedBind }
		jsonResponse(w, http.StatusOK, resp)
	}
}

// ==================== 计费（国内支付待接入）====================
// NOTE: 当前为开发占位实现。国内生产环境需接入支付宝/微信支付。
// Stripe SDK 适用于国际版，可通过 VERIDACTUS_PAYMENT_PROVIDER 环境变量切换。
// 支付接口契约保持稳定：POST /api/v1/billing/checkout → {session_id, checkout_url}

func (srv *Server) handleStripeCheckout() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST"); return
		}
		var req struct{ AmountUSD float64 `json:"amount_usd"` }
		if err := decodeJSON(r, &req); err != nil { jsonError(w, http.StatusBadRequest, "invalid_body", ""); return }
		wsID := srv.getWorkspaceIDSafe(r)
		// Stripe Checkout Session 创建
		// 生产环境: stripe.CheckoutSessionParams → sessions.New()
		// 开发环境: 返回模拟 checkout URL
		sessionID := uuid.New().String()
		checkoutURL := "https://checkout.stripe.com/pay/" + sessionID
		jsonResponse(w, http.StatusOK, map[string]interface{}{
			"session_id": sessionID, "checkout_url": checkoutURL,
			"amount_usd": req.AmountUSD, "amount_micro": int64(req.AmountUSD * 1_000_000),
			"workspace_id": wsID, "status": "pending",
		})
	}
}

func (srv *Server) handleStripeWebhook() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "only POST"); return
		}
		var event struct {
			Type string `json:"type"`
			Data struct {
				Object struct {
					ID        string `json:"id"`
					AmountTotal int64 `json:"amount_total"`
					Metadata  struct {
						WorkspaceID string `json:"workspace_id"`
					} `json:"metadata"`
				} `json:"object"`
			} `json:"data"`
		}
		if err := decodeJSON(r, &event); err != nil { jsonError(w, http.StatusBadRequest, "invalid_webhook", ""); return }

		if event.Type == "checkout.session.completed" {
			wsID := event.Data.Object.Metadata.WorkspaceID
			amountMicro := event.Data.Object.AmountTotal // Stripe 金额是分, 转微美元 ×10000
			wallet, _ := srv.store.GetWallet(r.Context(), wsID)
			if wallet != nil {
				srv.store.UpdateWalletBalance(r.Context(), wsID, amountMicro*10000)
				srv.store.CreateTransaction(r.Context(), &model.Transaction{
					ID: uuid.New().String(), WorkspaceID: wsID, WalletID: wallet.ID,
					Type: "credit", AmountUSDMicro: amountMicro * 10000,
					BalanceAfterMicro: wallet.BalanceUSDMicro + amountMicro*10000,
					Description: "Stripe payment: " + event.Data.Object.ID,
				})
			}
			log.Printf("Stripe payment completed: ws=%s amount=%d", wsID, amountMicro)
		}
		jsonResponse(w, http.StatusOK, map[string]string{"received": "true"})
	}
}


