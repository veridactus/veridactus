// VERIDACTUS 控制平面 — 数据模型定义
package main

// Trace 表示来自数据面的执行轨迹。
type Trace struct {
	TraceID        string `json:"trace_id"`
	Model          string `json:"model"`
	TenantID       string `json:"tenant_id"`
	ExecutionState string `json:"execution_state"`
	CreatedAt      string `json:"created_at"`
	Signature      string `json:"signature,omitempty"`
}

// Pipeline 表示一条治理流水线配置。
type Pipeline struct {
	ID          string        `json:"id"`
	PlanID      string        `json:"plan_id"`
	Name        string        `json:"name"`
	Description string        `json:"description"`
	Tenant      string        `json:"tenant"`
	Stages      []StageConfig `json:"stages"`
	Created     string        `json:"created_at"`
}

// StageConfig 定义流水线阶段及其插件。
type StageConfig struct {
	Placement string       `json:"placement"`
	Parallel  bool         `json:"parallel"`
	Plugins   []PluginConf `json:"plugins"`
}

// PluginConf 定义流水线阶段内的插件配置。
type PluginConf struct {
	Name    string `json:"name"`
	Type    string `json:"type"`
	Config  string `json:"config"`
	Enabled bool   `json:"enabled"`
}

// PluginMeta 表示插件元数据。
type PluginMeta struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Type        string `json:"type"`
	Version     string `json:"version"`
	Description string `json:"description"`
	Config      string `json:"config,omitempty"`
}

// Policy 表示一条治理策略。
type Policy struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Type      string `json:"type"`
	Content   string `json:"content"`
	CreatedAt string `json:"created_at"`
}

// ApiKey 表示一个与租户关联的 API 密钥。
type ApiKey struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Key       string `json:"key"`
	TenantID  string `json:"tenant_id"`
	Status    string `json:"status"`
	CreatedAt string `json:"created_at"`
	LastUsed  string `json:"last_used,omitempty"`
}

// ModelConfig 表示一个模型路由配置。
type ModelConfig struct {
	ID                string   `json:"id"`
	Name              string   `json:"name"`
	UpstreamURL       string   `json:"upstream_url"`
	UpstreamModel     string   `json:"upstream_model"`
	ApiKey            string   `json:"api_key,omitempty"`
	ApiKeyHeader      string   `json:"api_key_header,omitempty"`
	UseProxy          bool     `json:"use_proxy"`
	ProxyURL          string   `json:"proxy_url,omitempty"`
	IsDefault         bool     `json:"is_default"`
	SupportedVersions []string `json:"supported_versions,omitempty"`
	Status            string   `json:"status"`
}
