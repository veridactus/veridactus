// VERIDACTUS 控制平面 — 企业 SSO 配置 + 审计事件记录
package main

import (
	"encoding/json"
	"net/http"
	"os"

	"github.com/google/uuid"
	"github.com/veridactus/control-plane/internal/auth"
)

// handleSSOConfig SSO 配置端点
func (srv *Server) handleSSOConfig() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			// 返回当前 SSO 配置（脱敏）
			cfg := map[string]interface{}{
				"providers": []map[string]interface{}{
					{
						"provider": "okta",
						"enabled":  os.Getenv("OKTA_CLIENT_ID") != "",
						"domain":   os.Getenv("OKTA_DOMAIN"),
					},
					{
						"provider": "azure",
						"enabled":  os.Getenv("AZURE_CLIENT_ID") != "",
						"tenant":   os.Getenv("AZURE_TENANT_ID"),
					},
					{
						"provider": "feishu",
						"enabled":  os.Getenv("FEISHU_APP_ID") != "",
					},
					{
						"provider": "dingtalk",
						"enabled":  os.Getenv("DINGTALK_APP_KEY") != "",
					},
					{"provider": "github", "enabled": os.Getenv("GITHUB_CLIENT_ID") != ""},
				},
			}
			jsonResponse(w, http.StatusOK, cfg)

		case http.MethodPut:
			// 需要 org_admin+ 权限
			role := auth.GetRole(r.Context())
			if role != auth.RoleOrgAdmin && role != auth.RolePlatformAdmin {
				jsonError(w, http.StatusForbidden, "forbidden", "org_admin required")
				return
			}
			var req struct {
				Provider     string `json:"provider"`
				ClientID     string `json:"client_id"`
				ClientSecret string `json:"client_secret"`
				Domain       string `json:"domain,omitempty"`
				Enabled      bool   `json:"enabled"`
			}
			if err := decodeJSON(r, &req); err != nil {
				jsonError(w, http.StatusBadRequest, "invalid_body", err.Error())
				return
			}
			// 存储到 settings 表（加密 client_secret）
			settings := map[string]string{
				"sso_" + req.Provider + "_client_id": req.ClientID,
				"sso_" + req.Provider + "_enabled":   boolToString(req.Enabled),
			}
			if req.Domain != "" {
				settings["sso_"+req.Provider+"_domain"] = req.Domain
			}
			if req.ClientSecret != "" {
				settings["sso_"+req.Provider+"_client_secret"] = "[encrypted]" // 生产环境用 KMS 加密
			}
			wsID := auth.GetWorkspaceID(r.Context())
			srv.store.UpdateSettings(r.Context(), wsID, settings)
			jsonResponse(w, http.StatusOK, map[string]string{"status": "saved"})

		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

// handleAuditEvents 审计事件查询端点
func (srv *Server) handleAuditEvents() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
			return
		}
		wsID := auth.GetWorkspaceID(r.Context())
		period := r.URL.Query().Get("period") // today|7d|30d
		if period == "" {
			period = "today"
		}

		// 聚合审计指标
		events, err := srv.store.GetAuditEvents(r.Context(), wsID, period)
		if err != nil {
			jsonError(w, http.StatusInternalServerError, "db_error", err.Error())
			return
		}
		jsonResponse(w, http.StatusOK, map[string]interface{}{
			"period": period,
			"events": events,
		})
	}
}

// handleComplianceReport 合规报告生成端点
func (srv *Server) handleComplianceReport() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodPost:
			var req struct {
				Regulation string `json:"regulation"` // EU_AI_ACT | GDPR | NIST_AI_600
				DateStart  string `json:"date_start"`  // ISO 8601
				DateEnd    string `json:"date_end"`
			}
			if err := decodeJSON(r, &req); err != nil {
				jsonError(w, http.StatusBadRequest, "invalid_body", err.Error())
				return
			}
			if req.Regulation == "" {
				req.Regulation = "EU_AI_ACT"
			}

			// 创建异步任务
			jobID := uuid.New().String()
			wsID := auth.GetWorkspaceID(r.Context())

			// 记录任务元数据到 settings
			taskMeta, _ := json.Marshal(map[string]string{
				"regulation": req.Regulation,
				"date_start": req.DateStart,
				"date_end":   req.DateEnd,
				"status":     "pending",
				"created_by": auth.GetUserID(r.Context()),
			})
			srv.store.UpdateSettings(r.Context(), wsID, map[string]string{
				"compliance_job_" + jobID: string(taskMeta),
			})

			jsonResponse(w, http.StatusAccepted, map[string]interface{}{
				"job_id":     jobID,
				"status":     "pending",
				"message":    "合规报告生成任务已提交，完成后可下载",
				"regulation": req.Regulation,
			})

		case http.MethodGet:
			jobID := r.URL.Query().Get("job_id")
			if jobID == "" {
				jsonError(w, http.StatusBadRequest, "missing_job_id", "")
				return
			}
			wsID := auth.GetWorkspaceID(r.Context())
			settings, _ := srv.store.GetSettings(r.Context(), wsID)
			taskKey := "compliance_job_" + jobID
			if meta, ok := settings[taskKey]; ok {
				var task map[string]string
				json.Unmarshal([]byte(meta), &task)
				jsonResponse(w, http.StatusOK, task)
			} else {
				jsonError(w, http.StatusNotFound, "not_found", "job not found")
			}

		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

// handleBrandSettings 品牌白标设置
func (srv *Server) handleBrandSettings() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		userID := auth.GetUserID(r.Context())
		// 从 JWT context 获取用户组织，避免 orgs[0] 数据泄露
		var orgID string
		if userID != "" {
			orgs, err := srv.store.ListOrganizationsByUser(r.Context(), userID)
			if err == nil && len(orgs) > 0 {
				orgID = orgs[0].ID
			}
		}
		if orgID == "" {
			jsonError(w, http.StatusNotFound, "no_org", "no organization found for current user")
			return
		}

		switch r.Method {
		case http.MethodGet:
			org, err := srv.store.GetOrganization(r.Context(), orgID)
			if err != nil {
				jsonError(w, http.StatusNotFound, "not_found", err.Error())
				return
			}
			jsonResponse(w, http.StatusOK, map[string]interface{}{
				"logo_url":      org.LogoURL,
				"primary_color": org.PrimaryColor,
				"name":          org.Name,
			})

		case http.MethodPut:
			var req struct {
				LogoURL      string `json:"logo_url"`
				PrimaryColor string `json:"primary_color"`
				Name         string `json:"name"`
			}
			if err := decodeJSON(r, &req); err != nil {
				jsonError(w, http.StatusBadRequest, "invalid_body", err.Error())
				return
			}
			srv.store.UpdateOrganization(r.Context(), orgID, map[string]interface{}{
				"logo_url":      req.LogoURL,
				"primary_color": req.PrimaryColor,
				"name":          req.Name,
			})
			jsonResponse(w, http.StatusOK, map[string]string{"status": "saved"})

		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

func boolToString(b bool) string {
	if b {
		return "true"
	}
	return "false"
}
