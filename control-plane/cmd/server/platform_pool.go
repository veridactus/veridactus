// Unified 平台 LLM Master Key 池 — AI-1.md 指令 3.2
package main

import (
	"context"
	"encoding/json"
	"net/http"
	"os"

	"github.com/google/uuid"
	"github.com/veridactus/control-plane/internal/auth"
	"github.com/veridactus/control-plane/internal/crypto"
)

// PlatformPoolEntry 平台聚合模型池条目
type PlatformPoolEntry struct {
	ID           string `json:"id"`
	Model        string `json:"model"`         // glm-5.1 | deepseek-v3 | gpt-4o
	Provider     string `json:"provider"`       // zhipu | deepseek | openai
	UpstreamURL  string `json:"upstream_url"`   // https://open.bigmodel.cn
	MasterKeyEnc string `json:"-"`              // 加密的 Master Key (不导出)
	Enabled      bool   `json:"enabled"`
	Priority     int    `json:"priority"`       // 负载均衡优先级
}

// handlePlatformPool 平台 LLM 池管理
func (srv *Server) handlePlatformPool() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := auth.GetRole(r.Context())
		if role != auth.RolePlatformAdmin && role != auth.RoleOrgAdmin {
			jsonError(w, http.StatusForbidden, "forbidden", "platform_admin required"); return
		}

		switch r.Method {
		case http.MethodGet:
			pool := srv.loadPlatformPool(r.Context())
			// 脱敏后返回
			safe := make([]map[string]interface{}, 0)
			for _, e := range pool {
				safe = append(safe, map[string]interface{}{
					"id": e.ID, "model": e.Model, "provider": e.Provider,
					"upstream_url": e.UpstreamURL, "enabled": e.Enabled, "priority": e.Priority,
					"has_master_key": e.MasterKeyEnc != "",
				})
			}
			jsonResponse(w, http.StatusOK, map[string]interface{}{"pool": safe})

		case http.MethodPost:
			var req struct {
				Model       string `json:"model"`
				Provider    string `json:"provider"`
				UpstreamURL string `json:"upstream_url"`
				MasterKey   string `json:"master_key"`
				Priority    int    `json:"priority"`
			}
			if err := decodeJSON(r, &req); err != nil {
				jsonError(w, http.StatusBadRequest, "invalid_body", ""); return
			}

			// 加密 Master Key
			envelope, err := crypto.EncryptProviderKey(req.MasterKey, "")
			if err != nil {
				jsonError(w, http.StatusInternalServerError, "encrypt_failed", err.Error()); return
			}
			encJSON, _ := json.Marshal(envelope)

			entry := PlatformPoolEntry{
				ID: uuid.New().String(), Model: req.Model, Provider: req.Provider,
				UpstreamURL: req.UpstreamURL, MasterKeyEnc: string(encJSON),
				Enabled: true, Priority: req.Priority,
			}

			pool := srv.loadPlatformPool(r.Context())
			pool = append(pool, entry)
			srv.savePlatformPool(r.Context(), pool)
			jsonResponse(w, http.StatusCreated, map[string]interface{}{"added": entry.ID, "model": req.Model})

		case http.MethodDelete:
			id := r.URL.Query().Get("id")
			if id == "" { jsonError(w, http.StatusBadRequest, "missing_id", ""); return }
			pool := srv.loadPlatformPool(r.Context())
			filtered := make([]PlatformPoolEntry, 0)
			for _, e := range pool {
				if e.ID != id { filtered = append(filtered, e) }
			}
			srv.savePlatformPool(r.Context(), filtered)
			jsonResponse(w, http.StatusOK, map[string]string{"status": "removed"})

		default:
			jsonError(w, http.StatusMethodNotAllowed, "method_not_allowed", "")
		}
	}
}

func (srv *Server) loadPlatformPool(ctx context.Context) []PlatformPoolEntry {
	settings, _ := srv.store.GetSettings(ctx, "platform")
	raw, ok := settings["llm_pool"]
	if !ok { return []PlatformPoolEntry{} }
	var pool []PlatformPoolEntry
	json.Unmarshal([]byte(raw), &pool)
	return pool
}

func (srv *Server) savePlatformPool(ctx context.Context, pool []PlatformPoolEntry) {
	data, _ := json.Marshal(pool)
	srv.store.UpdateSettings(ctx, "platform", map[string]string{"llm_pool": string(data)})
}

// GetPlatformMasterKey 获取平台 Master Key（供 /internal/resolve-key 使用）
func (srv *Server) GetPlatformMasterKey(ctx context.Context, model string) (string, string) {
	pool := srv.loadPlatformPool(ctx)
	for _, e := range pool {
		if e.Model == model && e.Enabled && e.MasterKeyEnc != "" {
			var envelope crypto.EnvelopeEncrypted
			if err := json.Unmarshal([]byte(e.MasterKeyEnc), &envelope); err == nil {
				dec, err := crypto.DecryptProviderKey(&envelope, "")
				if err == nil { return dec, e.UpstreamURL }
			}
		}
	}
	// 环境变量兜底
	if key := os.Getenv("VERIDACTUS_PLATFORM_MASTER_KEY"); key != "" {
		url := os.Getenv("VERIDACTUS_PLATFORM_UPSTREAM_URL")
		if url == "" { url = "https://open.bigmodel.cn" }
		return key, url
	}
	return "", ""
}

// handlePlatformModels 列出可用平台模型
func (srv *Server) handlePlatformModels() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		pool := srv.loadPlatformPool(r.Context())
		// 如果没有配置池，从环境变量读取默认模型
		if len(pool) == 0 {
			defaultModels := getDefaultPlatformModels()
			jsonResponse(w, http.StatusOK, map[string]interface{}{
				"models": defaultModels, "source": "defaults",
			})
			return
		}
		models := make([]map[string]interface{}, 0)
		for _, e := range pool {
			if e.Enabled {
				models = append(models, map[string]interface{}{
					"model": e.Model, "provider": e.Provider, "priority": e.Priority,
				})
			}
		}
		jsonResponse(w, http.StatusOK, map[string]interface{}{"models": models, "source": "pool"})
	}
}

func getDefaultPlatformModels() []map[string]interface{} {
	return []map[string]interface{}{
		{"model": "glm-5.1", "provider": "zhipu", "description": "智谱 GLM-5.1"},
		{"model": "deepseek-v3", "provider": "deepseek", "description": "DeepSeek V3"},
		{"model": "gpt-4o", "provider": "openai", "description": "OpenAI GPT-4o"},
		{"model": "claude-3.5-sonnet", "provider": "anthropic", "description": "Anthropic Claude 3.5"},
	}
}
