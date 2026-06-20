// Package model defines all data types for the VERIDACTUS control plane.
package model

import "encoding/json"

// Trace represents an execution trace from the data plane.
type Trace struct {
	TraceID        string `json:"trace_id"`
	Model          string `json:"model"`
	TenantID       string `json:"tenant_id"`
	ExecutionState string `json:"execution_state"`
	CreatedAt      string `json:"created_at"`
	Signature      string `json:"signature,omitempty"`
}

// Pipeline represents a governance pipeline configuration.
type Pipeline struct {
	ID          string        `json:"id"`
	PlanID      string        `json:"plan_id"`
	Name        string        `json:"name"`
	Description string        `json:"description"`
	Tenant      string        `json:"tenant"`
	Stages      []StageConfig `json:"stages"`
	Created     string        `json:"created_at"`
}

// StageConfig defines a pipeline stage with its plugins.
type StageConfig struct {
	Placement string       `json:"placement"`
	Parallel  bool         `json:"parallel"`
	Plugins   []PluginConf `json:"plugins"`
}

// PluginConf defines a plugin configuration within a stage.
type PluginConf struct {
	Name    string `json:"name"`
	Type    string `json:"type"`
	Config  string `json:"config"`
	Enabled bool   `json:"enabled"`
}

// PluginMeta represents plugin metadata.
type PluginMeta struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Type        string `json:"type"`
	Version     string `json:"version"`
	Description string `json:"description"`
	Config      string `json:"config,omitempty"`
}

// Policy represents a governance policy.
type Policy struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Type      string `json:"type"`
	Content   string `json:"content"`
	CreatedAt string `json:"created_at"`
}

// ApiKey represents an API key with tenant association.
type ApiKey struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Key       string `json:"key"`
	TenantID  string `json:"tenant_id"`
	Status    string `json:"status"`
	CreatedAt string `json:"created_at"`
	LastUsed  string `json:"last_used,omitempty"`
}

// ModelConfig represents a model routing configuration.
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

// ConfigVersion tracks configuration version numbers for sync.
type ConfigVersion struct {
	Key   string `json:"key"`
	Value int64  `json:"value"`
}

// ConfigChangePayload is the payload format for pushing config changes.
type ConfigChangePayload struct {
	ChangeType string          `json:"change_type"`
	Data       json.RawMessage `json:"data"`
	Version    []ConfigVersion `json:"version,omitempty"`
}

// DataPlaneConfig represents a data plane configuration.
type DataPlaneConfig struct {
	ID               string `json:"id"`
	Name             string `json:"name"`
	UpstreamBaseURL  string `json:"upstream_base_url"`
	ProtocolVersion  string `json:"protocol_version"`
	ControlPlaneURL  string `json:"control_plane_url"`
	ConfigPullSecs   int    `json:"config_pull_interval_secs"`
	SupportedProofs  string `json:"supported_proof_levels"`
	ConformanceLevel string `json:"conformance_level"`
	CreatedAt        string `json:"created_at"`
	UpdatedAt        string `json:"updated_at"`
}
