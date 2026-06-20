// VERIDACTUS 控制平面 — 单元测试
package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
)

// ==================== 测试辅助函数 ====================

func setupTestStore(t *testing.T) *Store {
	t.Helper()
	dbPath := t.TempDir() + "/test.db"
	os.Setenv("DB_PATH", dbPath)
	store, err := NewStore(dbPath)
	if err != nil {
		t.Fatalf("创建测试存储失败: %v", err)
	}
	if err := store.InitStorageTables(); err != nil {
		t.Fatalf("初始化存储表失败: %v", err)
	}
	t.Cleanup(func() {
		store.db.Close()
	})
	return store
}

func doRequest(handler http.HandlerFunc, method, path, body string) *httptest.ResponseRecorder {
	var req *http.Request
	if body != "" {
		req = httptest.NewRequest(method, path, strings.NewReader(body))
		req.Header.Set("Content-Type", "application/json")
	} else {
		req = httptest.NewRequest(method, path, nil)
	}
	w := httptest.NewRecorder()
	handler(w, req)
	return w
}

// ==================== Health 检查 ====================

func TestHandleHealth(t *testing.T) {
	store := setupTestStore(t)
	handler := handleHealth(store)
	w := doRequest(handler, "GET", "/api/v1/health", "")

	if w.Code != http.StatusOK {
		t.Errorf("期望状态码 200，得到 %d", w.Code)
	}

	var resp map[string]string
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("解析响应失败: %v", err)
	}

	if resp["status"] != "ok" {
		t.Errorf("期望 status=ok，得到 %s", resp["status"])
	}
	if resp["version"] != "0.2.1" {
		t.Errorf("期望 version=0.2.1，得到 %s", resp["version"])
	}
	if resp["storage"] != "sqlite" {
		t.Errorf("期望 storage=sqlite，得到 %s", resp["storage"])
	}
}

// ==================== Pipeline CRUD ====================

func TestHandlePipelinesCRUD(t *testing.T) {
	store := setupTestStore(t)
	handler := handlePipelines(store)

	// 1. POST — 创建流水线
	createBody := `{
		"name": "test-pipeline",
		"description": "A test pipeline",
		"tenant": "test-org",
		"stages": [
			{
				"placement": "pre_request",
				"parallel": false,
				"plugins": [
					{
						"name": "Auth Validator",
						"type": "native",
						"config": "{}",
						"enabled": true
					}
				]
			}
		]
	}`
	w := doRequest(handler, "POST", "/api/v1/pipelines", createBody)
	if w.Code != http.StatusCreated {
		t.Fatalf("POST 期望 201，得到 %d: %s", w.Code, w.Body.String())
	}

	var created Pipeline
	if err := json.Unmarshal(w.Body.Bytes(), &created); err != nil {
		t.Fatalf("解析创建响应失败: %v", err)
	}
	if created.PlanID == "" {
		t.Fatal("期望 PlanID 不为空")
	}

	// 2. GET (列表) — 验证流水线数量
	w = doRequest(handler, "GET", "/api/v1/pipelines", "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET 列表期望 200，得到 %d", w.Code)
	}
	var listResp struct {
		Total     int        `json:"total"`
		Pipelines []Pipeline `json:"pipelines"`
	}
	if err := json.Unmarshal(w.Body.Bytes(), &listResp); err != nil {
		t.Fatalf("解析列表响应失败: %v", err)
	}
	// 应有 2 个默认 + 1 个测试创建的 = 3
	if listResp.Total < 3 {
		t.Errorf("期望至少 3 pipelines流水线，得到 %d", listResp.Total)
	}

	// 3. GET (按 ID) — 获取刚创建的流水线
	w = doRequest(handler, "GET", "/api/v1/pipelines/"+created.PlanID, "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET 按ID期望 200，得到 %d: %s", w.Code, w.Body.String())
	}
	var fetched Pipeline
	if err := json.Unmarshal(w.Body.Bytes(), &fetched); err != nil {
		t.Fatalf("解析获取响应失败: %v", err)
	}
	if fetched.Name != "test-pipeline" {
		t.Errorf("期望 name=test-pipeline，得到 %s", fetched.Name)
	}

	// 4. PUT — 更新流水线
	updateBody := `{
		"name": "updated-pipeline",
		"description": "Updated description",
		"tenant": "test-org",
		"stages": []
	}`
	w = doRequest(handler, "PUT", "/api/v1/pipelines/"+created.PlanID, updateBody)
	if w.Code != http.StatusOK {
		t.Fatalf("PUT 期望 200，得到 %d: %s", w.Code, w.Body.String())
	}

	// 5. DELETE — 删除流水线
	w = doRequest(handler, "DELETE", "/api/v1/pipelines/"+created.PlanID, "")
	if w.Code != http.StatusOK {
		t.Fatalf("DELETE 期望 200，得到 %d: %s", w.Code, w.Body.String())
	}

	// 6. GET (按 ID) — 确认删除
	w = doRequest(handler, "GET", "/api/v1/pipelines/"+created.PlanID, "")
	if w.Code != http.StatusNotFound {
		t.Errorf("删除后 GET 期望 404，得到 %d", w.Code)
	}
}

// ==================== Plugin 端点 ====================

func TestHandlePlugins(t *testing.T) {
	store := setupTestStore(t)
	handler := handlePlugins(store)

	// GET — 列出插件（应有 10 个默认插件）
	w := doRequest(handler, "GET", "/api/v1/plugins", "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET plugins 期望 200，得到 %d", w.Code)
	}

	var resp struct {
		Total   int          `json:"total"`
		Plugins []PluginMeta `json:"plugins"`
	}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("解析响应失败: %v", err)
	}
	if resp.Total < 10 {
		t.Errorf("期望至少 10 个默认插件，得到 %d", resp.Total)
	}

	// POST — 注册新插件
	createBody := `{"name":"TestPlugin","type":"native","version":"1.0.0","description":"Test"}`
	w = doRequest(handler, "POST", "/api/v1/plugins", createBody)
	if w.Code != http.StatusCreated {
		t.Fatalf("POST plugin 期望 201，得到 %d: %s", w.Code, w.Body.String())
	}
}

// ==================== Policy 端点 ====================

func TestHandlePolicies(t *testing.T) {
	store := setupTestStore(t)
	handler := handlePolicies(store)

	// GET — 列出策略（初始为空）
	w := doRequest(handler, "GET", "/api/v1/policies", "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET policies 期望 200，得到 %d", w.Code)
	}

	// POST — 创建策略
	createBody := `{"name":"test-policy","type":"content-filter","content":"{\"blocked\":[\"violence\"]}"}`
	w = doRequest(handler, "POST", "/api/v1/policies", createBody)
	if w.Code != http.StatusCreated {
		t.Fatalf("POST policy 期望 201，得到 %d: %s", w.Code, w.Body.String())
	}

	var created Policy
	if err := json.Unmarshal(w.Body.Bytes(), &created); err != nil {
		t.Fatalf("解析创建响应失败: %v", err)
	}
	if created.Name != "test-policy" {
		t.Errorf("期望 name=test-policy，得到 %s", created.Name)
	}
}

// ==================== API Key 端点 ====================

func TestHandleApiKeys(t *testing.T) {
	store := setupTestStore(t)
	handler := handleApiKeys(store)

	// GET — 列出 API Key（应有 3 个默认）
	w := doRequest(handler, "GET", "/api/v1/apikeys", "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET apikeys 期望 200，得到 %d", w.Code)
	}

	var listResp struct {
		Total int      `json:"total"`
		Keys  []ApiKey `json:"keys"`
	}
	if err := json.Unmarshal(w.Body.Bytes(), &listResp); err != nil {
		t.Fatalf("解析响应失败: %v", err)
	}
	if listResp.Total < 3 {
		t.Errorf("期望至少 3 个默认 API Key，得到 %d", listResp.Total)
	}

	// POST — 创建新 API Key
	w = doRequest(handler, "POST", "/api/v1/apikeys", `{"name":"test-key","tenant_id":"test-org"}`)
	if w.Code != http.StatusCreated {
		t.Fatalf("POST apikey 期望 201，得到 %d: %s", w.Code, w.Body.String())
	}

	var created ApiKey
	if err := json.Unmarshal(w.Body.Bytes(), &created); err != nil {
		t.Fatalf("解析创建响应失败: %v", err)
	}
	if created.Name != "test-key" {
		t.Errorf("期望 name=test-key，得到 %s", created.Name)
	}
	if !strings.HasPrefix(created.Key, "vd-") {
		t.Errorf("API Key 应以 'vd-' 开头，得到 %s", created.Key)
	}
}

func TestHandleApiKeyByID(t *testing.T) {
	store := setupTestStore(t)
	listHandler := handleApiKeys(store)
	byIDHandler := handleApiKeyByID(store)

	// 创建一个 Key
	w := doRequest(listHandler, "POST", "/api/v1/apikeys", `{"name":"revoke-test","tenant_id":"test"}`)
	var created ApiKey
	json.Unmarshal(w.Body.Bytes(), &created)

	// GET — 获取该 Key
	w = doRequest(byIDHandler, "GET", "/api/v1/apikeys/"+created.ID, "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET apikey 期望 200，得到 %d", w.Code)
	}

	// DELETE — 吊销该 Key
	w = doRequest(byIDHandler, "DELETE", "/api/v1/apikeys/"+created.ID, "")
	if w.Code != http.StatusOK {
		t.Fatalf("DELETE apikey 期望 200，得到 %d: %s", w.Code, w.Body.String())
	}

	var revokeResp map[string]string
	json.Unmarshal(w.Body.Bytes(), &revokeResp)
	if revokeResp["status"] != "revoked" {
		t.Errorf("期望 status=revoked，得到 %s", revokeResp["status"])
	}
}

// ==================== Model 端点 ====================

func TestHandleModels(t *testing.T) {
	store := setupTestStore(t)
	handler := handleModels(store)

	// GET — 列出模型
	w := doRequest(handler, "GET", "/api/v1/models", "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET models 期望 200，得到 %d", w.Code)
	}

	// POST — 创建模型
	createBody := `{"name":"test-model","upstream_url":"https://test.local","upstream_model":"test-v1","status":"active"}`
	w = doRequest(handler, "POST", "/api/v1/models", createBody)
	if w.Code != http.StatusCreated {
		t.Fatalf("POST model 期望 201，得到 %d: %s", w.Code, w.Body.String())
	}
}

func TestHandleModelByID(t *testing.T) {
	store := setupTestStore(t)
	listHandler := handleModels(store)
	byIDHandler := handleModelByID(store)

	// 创建一个模型
	w := doRequest(listHandler, "POST", "/api/v1/models", `{"name":"update-test","upstream_url":"https://test.local","upstream_model":"test-v1"}`)
	var created ModelConfig
	json.Unmarshal(w.Body.Bytes(), &created)

	// GET — 获取该模型
	w = doRequest(byIDHandler, "GET", "/api/v1/models/"+created.ID, "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET model 期望 200，得到 %d", w.Code)
	}

	// PUT — 更新模型
	w = doRequest(byIDHandler, "PUT", "/api/v1/models/"+created.ID, `{"name":"updated-model","status":"inactive"}`)
	if w.Code != http.StatusOK {
		t.Fatalf("PUT model 期望 200，得到 %d: %s", w.Code, w.Body.String())
	}

	var updated ModelConfig
	json.Unmarshal(w.Body.Bytes(), &updated)
	if updated.Name != "updated-model" {
		t.Errorf("期望 name=updated-model，得到 %s", updated.Name)
	}

	// DELETE — 删除模型
	w = doRequest(byIDHandler, "DELETE", "/api/v1/models/"+created.ID, "")
	if w.Code != http.StatusOK {
		t.Fatalf("DELETE model 期望 200，得到 %d", w.Code)
	}
}

// ==================== Traces 端点 ====================

func TestHandleTraces(t *testing.T) {
	store := setupTestStore(t)
	handler := handleTraces(store)

	w := doRequest(handler, "GET", "/api/v1/traces", "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET traces 期望 200，得到 %d", w.Code)
	}
}

func TestHandleTraceByID(t *testing.T) {
	store := setupTestStore(t)
	handler := handleTraceByID(store)

	w := doRequest(handler, "GET", "/api/v1/traces/test-id", "")
	if w.Code != http.StatusNotFound {
		t.Errorf("GET trace 期望 404，得到 %d", w.Code)
	}
}

// ==================== 配置轮询端点 ====================

func TestHandleConfigPoll(t *testing.T) {
	store := setupTestStore(t)
	handler := handleConfigPoll(store)

	// 首次轮询 — 应返回变更（版本不为 0）
	w := doRequest(handler, "GET", "/api/v1/config/poll", "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET config/poll 期望 200，得到 %d", w.Code)
	}

	var payload ConfigChangePayload
	if err := json.Unmarshal(w.Body.Bytes(), &payload); err != nil {
		t.Fatalf("解析 payload 失败: %v", err)
	}

	// 第二次轮询 — 用当前版本号
	pv := payload.Version.PipelineVersion
	mv := payload.Version.ModelVersion
	pollURL := "/api/v1/config/poll?pv=" + int64ToStr(pv) + "&mv=" + int64ToStr(mv)
	w = doRequest(handler, "GET", pollURL, "")
	if w.Code != http.StatusNotModified {
		// 如果没有变更，应返回 304
		t.Logf("config/poll 二次轮询: %d (期望 304 无变更)", w.Code)
	}
}

func int64ToStr(n int64) string {
	if n == 0 {
		return "0"
	}
	result := ""
	for n > 0 {
		result = string(rune('0'+n%10)) + result
		n /= 10
	}
	return result
}

// ==================== 数据面配置端点 ====================

func TestHandleDataPlaneConfigs(t *testing.T) {
	store := setupTestStore(t)
	handler := handleDataPlaneConfigs(store)

	// GET — 列出配置（初始为空）
	w := doRequest(handler, "GET", "/api/v1/dataplane-configs", "")
	if w.Code != http.StatusOK {
		t.Fatalf("GET configs 期望 200，得到 %d", w.Code)
	}

	// POST — 创建配置
	createBody := `{"name":"test-dp","postgres_url":"postgres://localhost","is_active":true}`
	w = doRequest(handler, "POST", "/api/v1/dataplane-configs", createBody)
	if w.Code != http.StatusCreated {
		t.Fatalf("POST config 期望 201，得到 %d: %s", w.Code, w.Body.String())
	}
}

// ==================== Store CRUD ====================

func TestStorePipelineCRUD(t *testing.T) {
	store := setupTestStore(t)

	// 创建
	p := Pipeline{
		PlanID:      "test-plan-1",
		Name:        "test",
		Description: "test desc",
		Tenant:      "test-org",
		Stages:      []StageConfig{},
		Created:     "2026-01-01T00:00:00Z",
	}
	if err := store.AddPipeline(p); err != nil {
		t.Fatalf("AddPipeline 失败: %v", err)
	}

	// 查询
	fetched, err := store.GetPipeline("test-plan-1")
	if err != nil {
		t.Fatalf("GetPipeline 失败: %v", err)
	}
	if fetched.Name != "test" {
		t.Errorf("期望 name=test，得到 %s", fetched.Name)
	}

	// 更新
	fetched.Name = "updated"
	if err := store.UpdatePipeline("test-plan-1", fetched); err != nil {
		t.Fatalf("UpdatePipeline 失败: %v", err)
	}

	// 删除
	if err := store.DeletePipeline("test-plan-1"); err != nil {
		t.Fatalf("DeletePipeline 失败: %v", err)
	}
}

func TestStoreApiKeyCRUD(t *testing.T) {
	store := setupTestStore(t)

	k := ApiKey{
		ID:        "key-1",
		Name:      "test-key",
		Key:       "vd-test123",
		TenantID:  "test-org",
		Status:    "active",
		CreatedAt: "2026-01-01T00:00:00Z",
	}
	if err := store.AddApiKey(k); err != nil {
		t.Fatalf("AddApiKey 失败: %v", err)
	}

	fetched, err := store.GetApiKey("key-1")
	if err != nil {
		t.Fatalf("GetApiKey 失败: %v", err)
	}
	if fetched.Name != "test-key" {
		t.Errorf("期望 name=test-key，得到 %s", fetched.Name)
	}
}

func TestStoreModelCRUD(t *testing.T) {
	store := setupTestStore(t)

	m := ModelConfig{
		ID:            "model-1",
		Name:          "test-model",
		UpstreamURL:   "https://test.local",
		UpstreamModel: "test-v1",
		Status:        "active",
	}
	if err := store.AddModel(m); err != nil {
		t.Fatalf("AddModel 失败: %v", err)
	}

	fetched, err := store.GetModel("model-1")
	if err != nil {
		t.Fatalf("GetModel 失败: %v", err)
	}
	if fetched.Name != "test-model" {
		t.Errorf("期望 name=test-model，得到 %s", fetched.Name)
	}

	// 删除
	if err := store.DeleteModel("model-1"); err != nil {
		t.Fatalf("DeleteModel 失败: %v", err)
	}
}

func TestStoreConfigVersions(t *testing.T) {
	store := setupTestStore(t)

	cv, err := store.GetConfigVersions()
	if err != nil {
		t.Fatalf("GetConfigVersions 失败: %v", err)
	}
	if cv.ModelVersion < 1 {
		t.Errorf("期望 ModelVersion >= 1，得到 %d", cv.ModelVersion)
	}

	// 递增版本号
	if err := store.IncrementConfigVersion("pipeline"); err != nil {
		t.Fatalf("IncrementConfigVersion 失败: %v", err)
	}

	newCV, _ := store.GetConfigVersions()
	if newCV.PipelineVersion <= cv.PipelineVersion {
		t.Errorf("期望递增后的版本大于原版本: %d <= %d", newCV.PipelineVersion, cv.PipelineVersion)
	}
}
