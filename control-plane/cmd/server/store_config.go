// VERIDACTUS 控制平面 — 数据面存储配置管理与配置变更推送
package main

import (
	"bytes"
	"database/sql"
	"encoding/json"
	"net/http"
	"time"
)

// DataPlaneConfig 表示一个数据面实例的存储后端配置。
type DataPlaneConfig struct {
	ID            string `json:"id"`
	Name          string `json:"name"`
	PostgresURL          string `json:"postgres_url"`
	RedisURL             string `json:"redis_url"`
	S3Endpoint           string `json:"s3_endpoint"`
	S3Bucket             string `json:"s3_bucket"`
	S3AccessKey          string `json:"s3_access_key,omitempty"`
	S3SecretKey          string `json:"s3_secret_key,omitempty"`
	IsActive             bool   `json:"is_active"`
	LastHeartbeat        string `json:"last_heartbeat,omitempty"`
	CreatedAt            string `json:"created_at"`
	UpstreamBaseURL      string `json:"upstream_base_url,omitempty"`
	ProtocolVersion      string `json:"protocol_version,omitempty"`
	ControlPlaneURL      string `json:"control_plane_url,omitempty"`
	ConfigPullInterval   int    `json:"config_pull_interval_secs,omitempty"`
	SupportedProofLevels string `json:"supported_proof_levels,omitempty"`
	ConformanceLevel     string `json:"conformance_level,omitempty"`
	UpdatedAt            string `json:"updated_at,omitempty"`
}

// ConfigVersion 记录各配置实体的当前版本号，用于增量同步。
type ConfigVersion struct {
	PipelineVersion int64 `json:"pipeline_version"`
	PolicyVersion   int64 `json:"policy_version"`
	PluginVersion   int64 `json:"plugin_version"`
	StorageVersion  int64 `json:"storage_version"`
	ModelVersion    int64 `json:"model_version"`
}

// ConfigChangePayload 是控制面到数据面的配置变更推送载荷。
type ConfigChangePayload struct {
	ChangeType string          `json:"change_type"`
	Data       json.RawMessage `json:"data"`
	Version    ConfigVersion   `json:"version"`
}

// dataPlaneURL 是数据面的管理端点地址。
var dataPlaneURL = getEnv("VERIDACTUS_DATA_PLANE_URL", "http://localhost:8080")

// InitStorageTables 创建 data_plane_configs 表并初始化 config_versions 种子行。
func (s *Store) InitStorageTables() error {
	queries := []string{
		`CREATE TABLE IF NOT EXISTS data_plane_configs (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			postgres_url TEXT,
			redis_url TEXT,
			s3_endpoint TEXT,
			s3_bucket TEXT,
			s3_access_key TEXT,
			s3_secret_key TEXT,
			is_active INTEGER NOT NULL DEFAULT 0,
			last_heartbeat TEXT,
			created_at TEXT NOT NULL
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
		_, err := s.db.Exec(q)
		if err != nil {
			return err
		}
	}
	return nil
}

// GetConfigVersions 获取所有配置实体的当前版本号。
func (s *Store) GetConfigVersions() (ConfigVersion, error) {
	var cv ConfigVersion
	rows, err := s.db.Query(`SELECT key, value FROM config_versions`)
	if err != nil {
		return cv, err
	}
	defer rows.Close()

	for rows.Next() {
		var key string
		var value int64
		if err := rows.Scan(&key, &value); err != nil {
			return cv, err
		}
		switch key {
		case "pipeline_version":
			cv.PipelineVersion = value
		case "policy_version":
			cv.PolicyVersion = value
		case "plugin_version":
			cv.PluginVersion = value
		case "storage_version":
			cv.StorageVersion = value
		case "model_version":
			cv.ModelVersion = value
		}
	}
	return cv, nil
}

// IncrementConfigVersion 将指定变更类型的配置版本号递增 1。
func (s *Store) IncrementConfigVersion(changeType string) error {
	key := changeType + "_version"
	_, err := s.db.Exec(`UPDATE config_versions SET value = value + 1 WHERE key = ?`, key)
	return err
}

// ListDataPlaneConfigs 列出所有数据面存储配置。
func (s *Store) ListDataPlaneConfigs() ([]DataPlaneConfig, error) {
	rows, err := s.db.Query(`SELECT id, name, postgres_url, redis_url, s3_endpoint, s3_bucket, s3_access_key, s3_secret_key, is_active, last_heartbeat, created_at FROM data_plane_configs`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var configs []DataPlaneConfig
	for rows.Next() {
		var c DataPlaneConfig
		var lastHeartbeat, createdAt sql.NullString
		var s3AccessKey, s3SecretKey sql.NullString
		if err := rows.Scan(&c.ID, &c.Name, &c.PostgresURL, &c.RedisURL, &c.S3Endpoint, &c.S3Bucket, &s3AccessKey, &s3SecretKey, &c.IsActive, &lastHeartbeat, &createdAt); err != nil {
			return nil, err
		}
		if s3AccessKey.Valid {
			c.S3AccessKey = s3AccessKey.String
		}
		if s3SecretKey.Valid {
			c.S3SecretKey = s3SecretKey.String
		}
		if lastHeartbeat.Valid {
			c.LastHeartbeat = lastHeartbeat.String
		}
		c.CreatedAt = createdAt.String
		configs = append(configs, c)
	}
	return configs, nil
}

// AddDataPlaneConfig 新增一条数据面存储配置，并递增配置版本号。
func (s *Store) AddDataPlaneConfig(c DataPlaneConfig) error {
	_, err := s.db.Exec(`INSERT OR REPLACE INTO data_plane_configs (id, name, postgres_url, redis_url, s3_endpoint, s3_bucket, s3_access_key, s3_secret_key, is_active, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		c.ID, c.Name, c.PostgresURL, c.RedisURL, c.S3Endpoint, c.S3Bucket, c.S3AccessKey, c.S3SecretKey, c.IsActive, c.CreatedAt)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion("storage")
}

// UpdateDataPlaneConfig 更新指定数据面存储配置，并递增配置版本号。
func (s *Store) UpdateDataPlaneConfig(id string, c DataPlaneConfig) error {
	_, err := s.db.Exec(`UPDATE data_plane_configs SET name = ?, postgres_url = ?, redis_url = ?, s3_endpoint = ?, s3_bucket = ?, s3_access_key = ?, s3_secret_key = ?, is_active = ? WHERE id = ?`,
		c.Name, c.PostgresURL, c.RedisURL, c.S3Endpoint, c.S3Bucket, c.S3AccessKey, c.S3SecretKey, c.IsActive, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion("storage")
}

// DeleteDataPlaneConfig 删除指定数据面存储配置，并递增配置版本号。
func (s *Store) DeleteDataPlaneConfig(id string) error {
	_, err := s.db.Exec(`DELETE FROM data_plane_configs WHERE id = ?`, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion("storage")
}

// UpdateDataPlaneHeartbeat 更新指定数据面实例的心跳时间戳。
func (s *Store) UpdateDataPlaneHeartbeat(id string) error {
	_, err := s.db.Exec(`UPDATE data_plane_configs SET last_heartbeat = ? WHERE id = ?`,
		time.Now().UTC().Format(time.RFC3339), id)
	return err
}

// GetActiveDataPlaneConfig 获取当前活跃的数据面配置（is_active = 1）。
func (s *Store) GetActiveDataPlaneConfig() (*DataPlaneConfig, error) {
	var c DataPlaneConfig
	var lastHeartbeat, createdAt sql.NullString
	var s3AccessKey, s3SecretKey sql.NullString
	err := s.db.QueryRow(`SELECT id, name, postgres_url, redis_url, s3_endpoint, s3_bucket, s3_access_key, s3_secret_key, is_active, last_heartbeat, created_at FROM data_plane_configs WHERE is_active = 1 LIMIT 1`).
		Scan(&c.ID, &c.Name, &c.PostgresURL, &c.RedisURL, &c.S3Endpoint, &c.S3Bucket, &s3AccessKey, &s3SecretKey, &c.IsActive, &lastHeartbeat, &createdAt)
	if err != nil {
		return nil, err
	}
	if s3AccessKey.Valid {
		c.S3AccessKey = s3AccessKey.String
	}
	if s3SecretKey.Valid {
		c.S3SecretKey = s3SecretKey.String
	}
	if lastHeartbeat.Valid {
		c.LastHeartbeat = lastHeartbeat.String
	}
	c.CreatedAt = createdAt.String
	return &c, nil
}

// mustMarshal 将值序列化为 JSON，忽略错误（用于推送载荷）。
func mustMarshal(v any) json.RawMessage {
	b, _ := json.Marshal(v)
	return b
}

// pushModelsToDataPlane 将全部模型配置异步推送到数据面。
func pushModelsToDataPlane(store *Store) {
	go func() {
		models, err := store.ListModels()
		if err != nil {
			logWarn("Model config push failed (fetch): %v", err)
			return
		}
		versions, _ := store.GetConfigVersions()
		payload := ConfigChangePayload{
			ChangeType: "model",
			Data:       mustMarshal(models),
			Version:    versions,
		}
		body, _ := json.Marshal(payload)
		client := &http.Client{Timeout: 10 * time.Second}
		resp, err := client.Post(dataPlaneURL+"/v1/admin/config/sync", "application/json", bytes.NewReader(body))
		if err != nil {
			logWarn("Model config push failed (connect): %v", err)
			return
		}
		resp.Body.Close()
		logInfo("Model config pushed to DP: %d models", len(models))
	}()
}

// pushPipelinesToDataPlane 将全部流水线配置异步推送到数据面。
func pushPipelinesToDataPlane(store *Store) {
	go func() {
		pipelines, err := store.ListPipelines()
		if err != nil {
			logWarn("Pipeline config push failed (fetch): %v", err)
			return
		}
		versions, _ := store.GetConfigVersions()
		payload := ConfigChangePayload{
			ChangeType: "pipeline",
			Data:       mustMarshal(pipelines),
			Version:    versions,
		}
		body, _ := json.Marshal(payload)
		client := &http.Client{Timeout: 10 * time.Second}
		resp, err := client.Post(dataPlaneURL+"/v1/admin/config/sync", "application/json", bytes.NewReader(body))
		if err != nil {
			logWarn("Pipeline config push failed (connect)", "error", err.Error())
			return
		}
		resp.Body.Close()
		logInfo("Pipeline config pushed to DP", "count", len(pipelines))
	}()
}
