// VERIDACTUS 控制平面 — HTTP 请求处理器（REST API 端点实现）
package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/google/uuid"
)

// ==================== HTTP 工具函数 ====================

// jsonResp 将值序列化为 JSON 写入响应。
func jsonResp(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

// jsonBody 从请求体反序列化 JSON 到 v。
func jsonBody(r *http.Request, v any) error {
	return json.NewDecoder(r.Body).Decode(v)
}

// jsonBodyRaw 从请求体反序列化 JSON 到 v（支持部分更新时的 map 解码）。
func jsonBodyRaw(r *http.Request, v any) error {
	return json.NewDecoder(r.Body).Decode(v)
}

// ==================== Trace 处理器 ====================

// handleTraces 处理 /api/v1/traces 端点（读写执行轨迹，实际存储于数据面）。
func handleTraces(_ *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			jsonResp(w, http.StatusOK, map[string]any{"total": 0, "traces": []any{}})
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed", "hint": "traces are stored in data plane (port 8080)"})
		}
	}
}

// handleTraceByID 处理 /api/v1/traces/:id 端点（按 ID 查询轨迹，实际存储于数据面）。
func handleTraceByID(_ *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			jsonResp(w, http.StatusNotFound, map[string]string{"error": "trace not found", "hint": "traces are stored in data plane (port 8080)"})
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// ==================== Pipeline 处理器 ====================

// handlePipelines 处理 /api/v1/pipelines 端点（CRUD）。
func handlePipelines(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		path := r.URL.Path[len("/api/v1/pipelines"):]
		if path != "" && path != "/" {
			id := path[1:]
			switch r.Method {
			case http.MethodGet:
				p, err := store.GetPipeline(id)
				if err != nil {
					jsonResp(w, http.StatusNotFound, map[string]string{"error": "pipeline not found"})
					return
				}
				jsonResp(w, http.StatusOK, p)
			case http.MethodPut:
				var p Pipeline
				if err := jsonBody(r, &p); err != nil {
					jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
					return
				}
				if err := store.UpdatePipeline(id, p); err != nil {
					jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
					return
				}
				// 返回更新后的 pipeline
				updated, err := store.GetPipeline(id)
				if err != nil {
					jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
					return
				}
				pushPipelinesToDataPlane(store)
				jsonResp(w, http.StatusOK, updated)
			case http.MethodDelete:
				if err := store.DeletePipeline(id); err != nil {
					jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
					return
				}
				pushPipelinesToDataPlane(store)
				jsonResp(w, http.StatusOK, map[string]string{"status": "deleted"})
			default:
				jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
			}
			return
		}

		switch r.Method {
		case http.MethodGet:
			ps, err := store.ListPipelines()
			if err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, map[string]any{"total": len(ps), "pipelines": ps})
		case http.MethodPost:
			var p Pipeline
			if err := jsonBody(r, &p); err != nil {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			pid := uuid.New().String()
			p.ID = pid
			p.PlanID = pid
			p.Created = time.Now().UTC().Format(time.RFC3339)
			if err := store.AddPipeline(p); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			pushPipelinesToDataPlane(store)
			jsonResp(w, http.StatusCreated, p)
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// ==================== Plugin 处理器 ====================

// handlePlugins 处理 /api/v1/plugins 端点（列表与注册）。
func handlePlugins(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			ps, err := store.ListPlugins()
			if err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, map[string]any{"total": len(ps), "plugins": ps})
		case http.MethodPost:
			var p PluginMeta
			if err := jsonBody(r, &p); err != nil {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			p.ID = uuid.New().String()
			if err := store.AddPlugin(p); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusCreated, p)
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// ==================== Policy 处理器 ====================

// handlePolicies 处理 /api/v1/policies 端点（列表与创建）。
func handlePolicies(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			ps, err := store.ListPolicies()
			if err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, map[string]any{"policies": ps})
		case http.MethodPost:
			var p Policy
			if err := jsonBody(r, &p); err != nil {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			p.ID = uuid.New().String()
			p.CreatedAt = time.Now().UTC().Format(time.RFC3339)
			if err := store.AddPolicy(p); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusCreated, p)
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// ==================== API Key 处理器 ====================

// handleApiKeys 处理 /api/v1/apikeys 端点（列表与创建）。
func handleApiKeys(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			keys, err := store.ListApiKeys()
			if err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, map[string]any{"total": len(keys), "keys": keys})
		case http.MethodPost:
			var req struct {
				Name     string `json:"name"`
				TenantID string `json:"tenant_id"`
			}
			if err := jsonBody(r, &req); err != nil {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			if req.Name == "" && req.TenantID == "" {
				// 完全空请求体（无 name 也无 tenant_id）→ 返回错误
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": "name is required"})
				return
			}
			if req.Name == "" {
				req.Name = "unnamed"
			}
			if req.TenantID == "" {
				req.TenantID = "default"
			}
			k := ApiKey{
				ID:        uuid.New().String(),
				Name:      req.Name,
				Key:       "vd-" + uuid.New().String()[:32],
				TenantID:  req.TenantID,
				Status:    "active",
				CreatedAt: time.Now().UTC().Format(time.RFC3339),
			}
			if err := store.AddApiKey(k); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusCreated, k)
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// handleApiKeyByID 处理 /api/v1/apikeys/:id 端点（查看 / 吊销 / 更新）。
func handleApiKeyByID(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := r.URL.Path[len("/api/v1/apikeys/"):]
		switch r.Method {
		case http.MethodGet:
			k, err := store.GetApiKey(id)
			if err != nil {
				jsonResp(w, http.StatusNotFound, map[string]string{"error": "API key not found"})
				return
			}
			jsonResp(w, http.StatusOK, k)
		case http.MethodDelete:
			k, err := store.GetApiKey(id)
			if err != nil {
				jsonResp(w, http.StatusNotFound, map[string]string{"error": "API key not found"})
				return
			}
			k.Status = "revoked"
			if err := store.UpdateApiKey(k); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, map[string]string{"status": "revoked"})
		case http.MethodPut:
			var req struct {
				Name   string `json:"name"`
				Status string `json:"status"`
			}
			if err := jsonBody(r, &req); err != nil {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			k, err := store.GetApiKey(id)
			if err != nil {
				jsonResp(w, http.StatusNotFound, map[string]string{"error": "API key not found"})
				return
			}
			if req.Status != "" {
				k.Status = req.Status
			}
			if req.Name != "" {
				k.Name = req.Name
			}
			if err := store.UpdateApiKey(k); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, k)
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// ==================== Model 处理器 ====================

// handleModels 处理 /api/v1/models 端点（列表与创建）。
func handleModels(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			models, err := store.ListModels()
			if err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, map[string]any{"total": len(models), "models": models})
		case http.MethodPost:
			var m ModelConfig
			if err := jsonBody(r, &m); err != nil {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			if m.Name == "" {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": "name is required"})
				return
			}
			if m.UpstreamModel == "" {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": "upstream_model is required"})
				return
			}
			m.ID = uuid.New().String()
			if m.Status == "" {
				m.Status = "active"
			}
			if err := store.AddModel(m); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			// 创建成功后立即推送到数据面
			pushModelsToDataPlane(store)
			jsonResp(w, http.StatusCreated, m)
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// handleModelByID 处理 /api/v1/models/:id 端点（查看 / 部分更新 / 删除）。
func handleModelByID(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := r.URL.Path[len("/api/v1/models/"):]
		switch r.Method {
		case http.MethodGet:
			m, err := store.GetModel(id)
			if err != nil {
				jsonResp(w, http.StatusNotFound, map[string]string{"error": "model not found"})
				return
			}
			jsonResp(w, http.StatusOK, m)
		case http.MethodPut:
			// 先获取现有模型，然后合并部分更新
			existing, err := store.GetModel(id)
			if err != nil {
				jsonResp(w, http.StatusNotFound, map[string]string{"error": "model not found"})
				return
			}
			var partial map[string]interface{}
			if err := jsonBodyRaw(r, &partial); err != nil {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			// 合并字段
			if v, ok := partial["name"]; ok {
				existing.Name = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["upstream_url"]; ok {
				existing.UpstreamURL = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["upstream_model"]; ok {
				existing.UpstreamModel = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["api_key"]; ok {
				existing.ApiKey = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["api_key_header"]; ok {
				existing.ApiKeyHeader = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["use_proxy"]; ok {
				existing.UseProxy = v.(bool)
			}
			if v, ok := partial["proxy_url"]; ok {
				existing.ProxyURL = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["is_default"]; ok {
				existing.IsDefault = v.(bool)
			}
			if v, ok := partial["status"]; ok {
				existing.Status = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["supported_versions"]; ok {
				if arr, ok2 := v.([]interface{}); ok2 {
					var versions []string
					for _, item := range arr {
						versions = append(versions, fmt.Sprintf("%v", item))
					}
					existing.SupportedVersions = versions
				}
			}
			if err := store.UpdateModel(id, existing); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			pushModelsToDataPlane(store)
			jsonResp(w, http.StatusOK, existing)
		case http.MethodDelete:
			if err := store.DeleteModel(id); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			pushModelsToDataPlane(store)
			jsonResp(w, http.StatusOK, map[string]string{"status": "deleted"})
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// ==================== 数据面存储配置处理器 ====================

// handleDataPlaneConfigs 处理 /api/v1/dataplane-configs 端点（列表与创建）。
func handleDataPlaneConfigs(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			configs, err := store.ListDataPlaneConfigs()
			if err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, map[string]any{"configs": configs})
		case http.MethodPost:
			var c DataPlaneConfig
			if err := jsonBody(r, &c); err != nil {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			c.ID = uuid.New().String()
			c.CreatedAt = time.Now().UTC().Format(time.RFC3339)
			if err := store.AddDataPlaneConfig(c); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusCreated, c)
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// handleDataPlaneConfigByID 处理 /api/v1/dataplane-configs/:id 端点（查看 / 更新 / 删除）。
func handleDataPlaneConfigByID(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := r.URL.Path[len("/api/v1/dataplane-configs/"):]
		switch r.Method {
		case http.MethodGet:
			configs, err := store.ListDataPlaneConfigs()
			if err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			for _, c := range configs {
				if c.ID == id {
					jsonResp(w, http.StatusOK, c)
					return
				}
			}
			jsonResp(w, http.StatusNotFound, map[string]string{"error": "config not found"})
		case http.MethodPut:
			var c DataPlaneConfig
			if err := jsonBody(r, &c); err != nil {
				jsonResp(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			if err := store.UpdateDataPlaneConfig(id, c); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, c)
		case http.MethodDelete:
			if err := store.DeleteDataPlaneConfig(id); err != nil {
				jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
				return
			}
			jsonResp(w, http.StatusOK, map[string]string{"status": "deleted"})
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// ==================== 配置轮询处理器 ====================

// handleConfigPoll 处理 /api/v1/config/poll 端点（数据面增量配置轮询）。
func handleConfigPoll(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		query := r.URL.Query()
		currentVersion := ConfigVersion{
			PipelineVersion: 0,
			PolicyVersion:   0,
			PluginVersion:   0,
			StorageVersion:  0,
			ModelVersion:    0,
		}

		if v := query.Get("pv"); v != "" {
			fmt.Sscanf(v, "%d", &currentVersion.PipelineVersion)
		}
		if v := query.Get("plv"); v != "" {
			fmt.Sscanf(v, "%d", &currentVersion.PluginVersion)
		}
		if v := query.Get("sv"); v != "" {
			fmt.Sscanf(v, "%d", &currentVersion.StorageVersion)
		}
		if v := query.Get("mv"); v != "" {
			fmt.Sscanf(v, "%d", &currentVersion.ModelVersion)
		}

		versions, err := store.GetConfigVersions()
		if err != nil {
			jsonResp(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
			return
		}

		if versions.PipelineVersion == currentVersion.PipelineVersion &&
			versions.PluginVersion == currentVersion.PluginVersion &&
			versions.StorageVersion == currentVersion.StorageVersion &&
			versions.ModelVersion == currentVersion.ModelVersion {
			w.WriteHeader(http.StatusNotModified)
			return
		}

		changeType := "unknown"
		if versions.PipelineVersion > currentVersion.PipelineVersion {
			changeType = "pipeline"
		} else if versions.ModelVersion > currentVersion.ModelVersion {
			changeType = "model"
		} else if versions.PluginVersion > currentVersion.PluginVersion {
			changeType = "plugin"
		} else if versions.StorageVersion > currentVersion.StorageVersion {
			changeType = "storage"
		}

		var data json.RawMessage
		switch changeType {
		case "pipeline":
			pipelines, _ := store.ListPipelines()
			data, _ = json.Marshal(pipelines)
		case "model":
			models, _ := store.ListModels()
			data, _ = json.Marshal(models)
		case "plugin":
			plugins, _ := store.ListPlugins()
			data, _ = json.Marshal(plugins)
		case "storage":
			storage, _ := store.ListDataPlaneConfigs()
			data, _ = json.Marshal(storage)
		default:
			data = []byte("{}")
		}

		payload := ConfigChangePayload{
			ChangeType: changeType,
			Data:       data,
			Version:    versions,
		}

		jsonResp(w, http.StatusOK, payload)
	}
}

// ==================== 健康检查处理器 ====================

// handleHealth 处理 /api/v1/health 端点（服务健康检查）。
func handleHealth(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// 检查数据库连接
		var count int
		err := store.db.QueryRow("SELECT COUNT(*) FROM apikeys").Scan(&count)
		status := "ok"
		if err != nil {
			status = "degraded"
		}
		jsonResp(w, http.StatusOK, map[string]string{"status": status, "version": "0.2.1", "storage": "sqlite"})
	}
}
