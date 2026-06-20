// VERIDACTUS 控制平面 — SQLite 存储层初始化和数据引导
package main

import (
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"os"
	"time"

	"github.com/google/uuid"
	_ "github.com/mattn/go-sqlite3"
)

// Store 封装了 SQLite 数据库连接。
type Store struct {
	db *sql.DB
}

// NewStore 打开（或创建）SQLite 数据库并初始化表结构及默认数据。
func NewStore(dbPath string) (*Store, error) {
	db, err := sql.Open("sqlite3", dbPath)
	if err != nil {
		return nil, err
	}

	// 启用 WAL 模式提高性能
	_, err = db.Exec("PRAGMA journal_mode=WAL;")
	if err != nil {
		return nil, err
	}

	// 创建表
	err = createTables(db)
	if err != nil {
		return nil, err
	}

	// 初始化默认数据（仅在表为空时）
	err = initDefaultData(db)
	if err != nil {
		return nil, err
	}

	// 数据库迁移：为已有表添加新列（忽略错误）
	migratePipelineTable(db)

	return &Store{db: db}, nil
}

// migratePipelineTable 尝试为 pipelines 表添加新列（忽略已存在的错误）。
// SQLite 不支持 IF NOT EXISTS 用于 ADD COLUMN，因此尝试执行并忽略列已存在的错误。
func migratePipelineTable(db *sql.DB) {
	_, err1 := db.Exec(`ALTER TABLE pipelines ADD COLUMN name TEXT NOT NULL DEFAULT ''`)
	if err1 != nil {
		logInfo("migration: name column (skip if exists)", "error", err1.Error())
	}
	_, err2 := db.Exec(`ALTER TABLE pipelines ADD COLUMN description TEXT NOT NULL DEFAULT ''`)
	if err2 != nil {
		logInfo("migration: description column (skip if exists)", "error", err2.Error())
	}
}

func createTables(db *sql.DB) error {
	queries := []string{
		`CREATE TABLE IF NOT EXISTS pipelines (
			plan_id TEXT PRIMARY KEY,
			name TEXT NOT NULL DEFAULT '',
			description TEXT NOT NULL DEFAULT '',
			tenant TEXT NOT NULL,
			stages TEXT NOT NULL,
			created_at TEXT NOT NULL
		);`,
		`CREATE TABLE IF NOT EXISTS plugins (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL,
			type TEXT NOT NULL,
			version TEXT,
			description TEXT,
			config TEXT DEFAULT '{}'
		);`,
		`CREATE TABLE IF NOT EXISTS policies (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL,
			type TEXT NOT NULL,
			content TEXT NOT NULL,
			created_at TEXT NOT NULL
		);`,
		`CREATE TABLE IF NOT EXISTS apikeys (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL,
			key TEXT NOT NULL UNIQUE,
			tenant_id TEXT NOT NULL,
			status TEXT NOT NULL,
			created_at TEXT NOT NULL,
			last_used TEXT
		);`,
		`CREATE TABLE IF NOT EXISTS models (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			upstream_url TEXT NOT NULL,
			upstream_model TEXT NOT NULL,
			api_key TEXT,
			api_key_header TEXT DEFAULT 'Authorization',
			use_proxy INTEGER NOT NULL DEFAULT 0,
			proxy_url TEXT,
			is_default INTEGER NOT NULL DEFAULT 0,
			supported_versions TEXT,
			status TEXT NOT NULL DEFAULT 'active'
		);`,
		`CREATE TABLE IF NOT EXISTS traces (
			trace_id TEXT PRIMARY KEY,
			model TEXT NOT NULL,
			tenant_id TEXT NOT NULL,
			execution_state TEXT NOT NULL,
			created_at TEXT NOT NULL,
			signature TEXT
		);`,
		`CREATE TABLE IF NOT EXISTS config_versions (
			key TEXT PRIMARY KEY,
			value INTEGER NOT NULL DEFAULT 0
		);`,
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('pipeline_version', 0);`,
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('policy_version', 0);`,
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('plugin_version', 0);`,
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('storage_version', 0);`,
		`INSERT OR IGNORE INTO config_versions (key, value) VALUES ('model_version', 0);`,
	}

	for _, q := range queries {
		_, err := db.Exec(q)
		if err != nil {
			return err
		}
	}
	return nil
}

func initDefaultData(db *sql.DB) error {
	// 检查是否已有数据
	var count int
	err := db.QueryRow("SELECT COUNT(*) FROM apikeys").Scan(&count)
	if err != nil {
		return err
	}
	if count > 0 {
		return nil // 已有数据，无需初始化
	}

	// 添加默认的 API Keys（使用随机生成的安全密钥）
	randomKey := func() string {
		b := make([]byte, 32)
		rand.Read(b)
		return "vd-" + hex.EncodeToString(b)[:32]
	}
	defaultKeys := []ApiKey{
		{ID: uuid.New().String(), Name: "Production API Key", Key: randomKey(), TenantID: "acme-corp", Status: "active", CreatedAt: "2026-05-01T00:00:00Z"},
		{ID: uuid.New().String(), Name: "Staging API Key", Key: randomKey(), TenantID: "acme-corp", Status: "active", CreatedAt: "2026-04-15T00:00:00Z"},
		{ID: uuid.New().String(), Name: "CI/CD Pipeline Key", Key: randomKey(), TenantID: "acme-corp", Status: "rotated", CreatedAt: "2026-03-20T00:00:00Z"},
	}

	for _, k := range defaultKeys {
		logInfo("Default API Key generated: %s -> %s...%s", k.Name, k.Key[:10], k.Key[len(k.Key)-8:])
		_, err := db.Exec(`INSERT INTO apikeys (id, name, key, tenant_id, status, created_at) VALUES (?, ?, ?, ?, ?, ?)`,
			k.ID, k.Name, k.Key, k.TenantID, k.Status, k.CreatedAt)
		if err != nil {
			return err
		}
	}

	// 添加默认的模型配置（敏感凭证从环境变量读取）
	geminiKey := os.Getenv("VERIDACTUS_GEMINI_API_KEY")
	azureKey := os.Getenv("VERIDACTUS_AZURE_AI_API_KEY")
	proxyURL := os.Getenv("VERIDACTUS_PROXY_URL")

	defaultUpstreamURL := os.Getenv("VERIDACTUS_DEFAULT_UPSTREAM_URL")
	if defaultUpstreamURL == "" {
		defaultUpstreamURL = "http://localhost:11434" // default ollama
	}
	zhipuKey := os.Getenv("VERIDACTUS_ZHIPU_API_KEY")
	if zhipuKey == "" {
		zhipuKey = "89f155e74b424fe7b82ccbc11d12e791.mLDuSRdpV4YV5Bfz"
	}
	ernieKey := os.Getenv("VERIDACTUS_ERNIE_API_KEY")
	if ernieKey == "" {
		ernieKey = "bce-v3/ALTAK-vLfpctcJIirnt2VPY3Jwi/bcfedf33ae5ebcf877f824294db18fe97b7cacc5"
	}
	defaultModels := []ModelConfig{
		{ID: uuid.New().String(), Name: "glm-5.1", UpstreamURL: "https://open.bigmodel.cn", UpstreamModel: "glm-5.1", ApiKey: zhipuKey, ApiKeyHeader: "Authorization", IsDefault: true, Status: "active", SupportedVersions: []string{"0.1", "0.2"}, UseProxy: false},
		{ID: uuid.New().String(), Name: "ernie-5.0", UpstreamURL: "https://qianfan.baidubce.com", UpstreamModel: "ernie-5.0", ApiKey: ernieKey, ApiKeyHeader: "Authorization", IsDefault: false, Status: "active", SupportedVersions: []string{"0.2"}, UseProxy: false},
	}

	// 仅当环境变量存在时才添加需要 API Key 的模型
	if geminiKey != "" {
		defaultModels = append(defaultModels, ModelConfig{
			ID: uuid.New().String(), Name: "gemini-flash",
			UpstreamURL: "https://generativelanguage.googleapis.com/v1beta/models",
			UpstreamModel: "gemini-flash-latest",
			ApiKey: geminiKey, ApiKeyHeader: "X-goog-api-key",
			IsDefault: false, Status: "active",
			SupportedVersions: []string{"0.2"},
			UseProxy: proxyURL != "", ProxyURL: proxyURL,
		})
	}
	if azureKey != "" {
		defaultModels = append(defaultModels, ModelConfig{
			ID: uuid.New().String(), Name: "gpt-4o",
			UpstreamURL: "https://models.inference.ai.azure.com",
			UpstreamModel: "gpt-4o",
			ApiKey: azureKey, ApiKeyHeader: "Authorization",
			IsDefault: false, Status: "active",
			SupportedVersions: []string{"0.2"},
			UseProxy: proxyURL != "", ProxyURL: proxyURL,
		})
	}

	for _, m := range defaultModels {
		versionsJSON, _ := json.Marshal(m.SupportedVersions)
		_, errInsert := db.Exec(`INSERT INTO models (id, name, upstream_url, upstream_model, api_key, api_key_header, use_proxy, proxy_url, is_default, supported_versions, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			m.ID, m.Name, m.UpstreamURL, m.UpstreamModel, m.ApiKey, m.ApiKeyHeader, boolToInt(m.UseProxy), m.ProxyURL, boolToInt(m.IsDefault), string(versionsJSON), m.Status)
		if errInsert != nil {
			return errInsert
		}
	}

	// 增加模型版本号，触发数据平面同步
	_, err = db.Exec(`UPDATE config_versions SET value = value + 1 WHERE key = 'model_version'`)
	if err != nil {
		return err
	}

	// 添加默认的插件配置
	defaultPlugins := []PluginMeta{
		{
			ID:          uuid.New().String(),
			Name:        "PII Detector",
			Type:        "native",
			Version:     "0.2.1",
			Description: "生产级PII检测插件 - 检测并遮蔽身份证、信用卡、电话、邮箱等敏感信息",
			Config:      `{"enabled":true,"action_on_detect":"mask","log_only":false,"mask_character":"*","detect_types":["china_id_card","credit_card","phone_number","email"]}`,
		},
		{
			ID:          uuid.New().String(),
			Name:        "Budget Guard",
			Type:        "native",
			Version:     "1.0.0",
			Description: "API预算守卫 - 限制每分钟/每小时/每天的API调用成本",
			Config:      `{"limit_usd":10.0,"window":"daily"}`,
		},
		{
			ID:          uuid.New().String(),
			Name:        "Auth Validator",
			Type:        "native",
			Version:     "1.0.0",
			Description: "认证验证器 - 验证API密钥和委托令牌",
			Config:      `{}`,
		},
		{
			ID:          uuid.New().String(),
			Name:        "Keyword Guardrail",
			Type:        "wasm",
			Version:     "0.2.0",
			Description: "WASM内容过滤器 - 实时过滤敏感关键词",
			Config:      `{"patterns":["violence","hate","illegal"]}`,
		},
		{
			ID:          uuid.New().String(),
			Name:        "Drift Detector",
			Type:        "grpc",
			Version:     "0.2.0",
			Description: "漂移检测器 - 分析嵌入向量的语义一致性",
			Config:      `{"threshold":0.7}`,
		},
		{
			ID:          uuid.New().String(),
			Name:        "TEE Attestation",
			Type:        "grpc",
			Version:     "0.2.0",
			Description: "TEE证明生成器 - 生成L1 TEE证明并验证",
			Config:      `{"platform":"tdx"}`,
		},
		{
			ID:          uuid.New().String(),
			Name:        "Trace Finalizer",
			Type:        "native",
			Version:     "1.0.0",
			Description: "Trace终结器 - 计算L0签名并完成Trace",
			Config:      `{}`,
		},
		{
			ID:          uuid.New().String(),
			Name:        "C-SafeGen",
			Type:        "grpc",
			Version:     "1.0.0",
			Description: "共形安全生成器 - 基于共形分析的可证明保证",
			Config:      `{"methodology":"C-SafeGen_v1.0"}`,
		},
		{
			ID:          uuid.New().String(),
			Name:        "Route Selector",
			Type:        "native",
			Version:     "1.0.0",
			Description: "路由选择器 - 智能模型路由和故障转移",
			Config:      `{"default":"deepseek-r1:14b"}`,
		},
		{
			ID:          uuid.New().String(),
			Name:        "Semantic Analyzer",
			Type:        "grpc",
			Version:     "0.1.0",
			Description: "语义分析器 - 高级语义分析用于输出验证",
			Config:      `{}`,
		},
	}

	for _, p := range defaultPlugins {
		configJSON := p.Config
		if configJSON == "" {
			configJSON = "{}"
		}
		_, err := db.Exec(`INSERT INTO plugins (id, name, type, version, description, config) VALUES (?, ?, ?, ?, ?, ?)`,
			p.ID, p.Name, p.Type, p.Version, p.Description, configJSON)
		if err != nil {
			return err
		}
	}

	// 添加默认的流水线
	defaultPipelines := []Pipeline{
		{
			PlanID: uuid.New().String(),
			Tenant: "acme-corp",
			Stages: []StageConfig{
				{Placement: "pre_request", Parallel: false, Plugins: []PluginConf{
					{Name: "Budget Guard", Type: "native", Config: `{"limit_usd":0.10}`, Enabled: true},
					{Name: "Auth Validator", Type: "native", Config: "{}", Enabled: true},
				}},
				{Placement: "streaming", Parallel: true, Plugins: []PluginConf{
					{Name: "Keyword Filter", Type: "wasm", Config: `{"patterns":["violence","hate"]}`, Enabled: true},
					{Name: "PII Detector", Type: "native", Config: `{"action_on_detect":"mask","detect_types":["china_id_card","credit_card","phone_number","email"]}`, Enabled: true},
				}},
				{Placement: "post_response", Parallel: false, Plugins: []PluginConf{
					{Name: "Trace Finalizer", Type: "native", Config: "{}", Enabled: true},
				}},
				{Placement: "async", Parallel: true, Plugins: []PluginConf{
					{Name: "Drift Detector", Type: "grpc", Config: `{"threshold":0.7}`, Enabled: true},
					{Name: "TEE Attestation", Type: "grpc", Config: `{"platform":"tdx"}`, Enabled: true},
				}},
			},
			Created: time.Now().UTC().Format(time.RFC3339),
		},
		{
			PlanID: uuid.New().String(),
			Tenant: "acme-corp",
			Stages: []StageConfig{
				{Placement: "pre_request", Parallel: false, Plugins: []PluginConf{
					{Name: "Route Selector", Type: "native", Config: `{"default":"deepseek-r1:14b"}`, Enabled: true},
				}},
				{Placement: "streaming", Parallel: false, Plugins: []PluginConf{
					{Name: "PII Detector", Type: "native", Config: `{"action_on_detect":"mask","detect_types":["china_id_card","credit_card","phone_number","email"]}`, Enabled: true},
				}},
				{Placement: "async", Parallel: false, Plugins: []PluginConf{
					{Name: "C-SafeGen", Type: "grpc", Config: `{"methodology":"C-SafeGen_v1.0"}`, Enabled: true},
				}},
			},
			Created: time.Now().UTC().Format(time.RFC3339),
		},
	}

	for _, p := range defaultPipelines {
		stagesJSON, _ := json.Marshal(p.Stages)
		_, err := db.Exec(`INSERT INTO pipelines (plan_id, tenant, stages, created_at) VALUES (?, ?, ?, ?)`,
			p.PlanID, p.Tenant, string(stagesJSON), p.Created)
		if err != nil {
			return err
		}
	}

	logInfo("Default data init complete")
	return nil
}

func boolToInt(b bool) int {
	if b {
		return 1
	}
	return 0
}

func intToBool(i int) bool {
	return i != 0
}
