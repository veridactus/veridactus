// @title VERIDACTUS Control Plane API
// @version 0.2.1
// @description VERIDACTUS Control Plane REST API for managing pipelines, plugins, models, and API keys.
// @termsOfService http://swagger.io/terms/

// @contact.name VERIDACTUS Team
// @contact.url https://github.com/veridactus/veridactus
// @contact.email tsc@veridactus.ai

// @license.name Apache 2.0
// @license.url https://www.apache.org/licenses/LICENSE-2.0.html

// @host localhost:8081
// @BasePath /api/v1

// @securityDefinitions.apikey AdminKey
// @in header
// @name X-Admin-Key

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
// @Summary Serialize JSON response
// @Description 将任意值序列化为JSON并写入HTTP响应
// @Tags Utilities
// @Accept json
// @Produce json
// @Param data body interface{} true "Response data"
// @Param status code int true "HTTP status code"
// @Success 200 {object} map[string]interface{}
// @Router / [get]
func jsonResp(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

// ==================== Health Check ====================

// handleHealth 健康检查端点
// @Summary 健康检查
// @Description 检查控制平面服务状态
// @Tags System
// @Produce json
// @Success 200 {object} map[string]string "服务健康"
// @Failure 503 {object} map[string]string "服务降级"
// @Router /api/v1/health [get]
func handleHealth(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			var count int
			err := store.db.QueryRow("SELECT COUNT(*) FROM apikeys").Scan(&count)
			status := "ok"
			if err != nil {
				status = "degraded"
			}
			jsonResp(w, http.StatusOK, map[string]string{"status": status, "version": "0.2.1", "storage": "sqlite"})
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// ==================== Pipeline Handlers ====================

// handlePipelines 处理流水线CRUD操作
// @Summary 流水线管理
// @Description 创建、列出、更新和删除治理流水线
// @Tags Pipelines
// @Accept json
// @Produce json
// @Param id path string false "Pipeline ID"
// @Param pipeline body Pipeline false "Pipeline data"
// @Success 200 {object} map[string]interface{} "成功"
// @Success 201 {object} Pipeline "创建成功"
// @Failure 400 {object} map[string]string "请求错误"
// @Failure 404 {object} map[string]string "未找到"
// @Failure 500 {object} map[string]string "服务器错误"
// @Security AdminKey
// @Router /api/v1/pipelines [get]
// @Router /api/v1/pipelines [post]
// @Router /api/v1/pipelines/{id} [get]
// @Router /api/v1/pipelines/{id} [put]
// @Router /api/v1/pipelines/{id} [delete]
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

// ==================== Plugin Handlers ====================

// handlePlugins 处理插件注册和管理
// @Summary 插件管理
// @Description 列出和注册治理插件
// @Tags Plugins
// @Accept json
// @Produce json
// @Param plugin body PluginMeta false "Plugin data"
// @Success 200 {object} map[string]interface{} "成功"
// @Success 201 {object} PluginMeta "创建成功"
// @Failure 400 {object} map[string]string "请求错误"
// @Security AdminKey
// @Router /api/v1/plugins [get]
// @Router /api/v1/plugins [post]
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

// ==================== Model Handlers ====================

// handleModels 处理模型配置CRUD
// @Summary 模型配置管理
// @Description 创建、列出、更新和删除AI模型配置
// @Tags Models
// @Accept json
// @Produce json
// @Param id path string false "Model ID"
// @Param model body ModelConfig false "Model configuration"
// @Success 200 {object} map[string]interface{} "成功"
// @Success 201 {object} ModelConfig "创建成功"
// @Failure 400 {object} map[string]string "请求错误"
// @Failure 404 {object} map[string]string "未找到"
// @Security AdminKey
// @Router /api/v1/models [get]
// @Router /api/v1/models [post]
// @Router /api/v1/models/{id} [get]
// @Router /api/v1/models/{id} [put]
// @Router /api/v1/models/{id} [delete]
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
			pushModelsToDataPlane(store)
			jsonResp(w, http.StatusCreated, m)
		default:
			jsonResp(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
		}
	}
}

// handleModelByID 处理单个模型配置操作
// @Summary 单个模型配置
// @Description 获取、更新或删除指定的模型配置
// @Tags Models
// @Accept json
// @Produce json
// @Param id path string true "Model ID"
// @Param model body ModelConfig false "Model configuration"
// @Success 200 {object} ModelConfig "成功"
// @Success 201 {object} ModelConfig "更新成功"
// @Failure 404 {object} map[string]string "未找到"
// @Security AdminKey
// @Router /api/v1/models/{id} [get]
// @Router /api/v1/models/{id} [put]
// @Router /api/v1/models/{id} [delete]
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
			if v, ok := partial["name"]; ok {
				existing.Name = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["upstream_model"]; ok {
				existing.UpstreamModel = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["api_key"]; ok {
				existing.ApiKey = fmt.Sprintf("%v", v)
			}
			if v, ok := partial["is_default"]; ok {
				existing.IsDefault = v.(bool)
			}
			if v, ok := partial["status"]; ok {
				existing.Status = fmt.Sprintf("%v", v)
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

// ==================== API Key Handlers ====================

// handleApiKeys 处理API密钥管理
// @Summary API密钥管理
// @Description 创建和列出API密钥
// @Tags API Keys
// @Accept json
// @Produce json
// @Param apikey body map[string]string false "API key data"
// @Success 200 {object} map[string]interface{} "成功"
// @Success 201 {object} ApiKey "创建成功"
// @Failure 400 {object} map[string]string "请求错误"
// @Security AdminKey
// @Router /api/v1/apikeys [get]
// @Router /api/v1/apikeys [post]
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

// handleApiKeyByID 处理单个API密钥操作
// @Summary 单个API密钥操作
// @Description 获取、更新或吊销指定的API密钥
// @Tags API Keys
// @Accept json
// @Produce json
// @Param id path string true "API Key ID"
// @Param apikey body map[string]string false "API key data"
// @Success 200 {object} ApiKey "成功"
// @Success 200 {object} map[string]string "吊销成功"
// @Failure 404 {object} map[string]string "未找到"
// @Security AdminKey
// @Router /api/v1/apikeys/{id} [get]
// @Router /api/v1/apikeys/{id} [put]
// @Router /api/v1/apikeys/{id} [delete]
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

// ==================== Config Poll Handler ====================

// handleConfigPoll 处理数据面配置轮询
// @Summary 配置轮询
// @Description 数据面轮询获取配置变更
// @Tags Configuration
// @Produce json
// @Param pv query int false "Pipeline version"
// @Param plv query int false "Plugin version"
// @Param sv query int false "Storage version"
// @Param mv query int false "Model version"
// @Success 200 {object} ConfigChangePayload "配置变更"
// @Success 304 {string} string "无变更"
// @Router /api/v1/config/poll [get]
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
