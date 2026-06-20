// VERIDACTUS 控制平面 — 业务实体 CRUD 操作（Pipeline / Plugin / Policy / ApiKey / Model）
package main

import (
	"database/sql"
	"encoding/json"
)

// ==================== Pipeline 操作 ====================

// GetPipeline 根据 ID 获取单条流水线。
func (s *Store) GetPipeline(id string) (Pipeline, error) {
	var p Pipeline
	var stagesJSON string
	err := s.db.QueryRow(`SELECT plan_id, COALESCE(name,''), COALESCE(description,''), tenant, stages, created_at FROM pipelines WHERE plan_id = ?`, id).
		Scan(&p.PlanID, &p.Name, &p.Description, &p.Tenant, &stagesJSON, &p.Created)
	if err != nil {
		return p, err
	}
	p.ID = p.PlanID
	err = json.Unmarshal([]byte(stagesJSON), &p.Stages)
	return p, err
}

// ListPipelines 列出所有流水线。
func (s *Store) ListPipelines() ([]Pipeline, error) {
	rows, err := s.db.Query(`SELECT plan_id, COALESCE(name,''), COALESCE(description,''), tenant, stages, created_at FROM pipelines`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var pipelines []Pipeline
	for rows.Next() {
		var p Pipeline
		var stagesJSON string
		if err := rows.Scan(&p.PlanID, &p.Name, &p.Description, &p.Tenant, &stagesJSON, &p.Created); err != nil {
			return nil, err
		}
		p.ID = p.PlanID
		if err := json.Unmarshal([]byte(stagesJSON), &p.Stages); err != nil {
			return nil, err
		}
		pipelines = append(pipelines, p)
	}
	return pipelines, nil
}

// AddPipeline 新增一条流水线。
func (s *Store) AddPipeline(p Pipeline) error {
	stagesJSON, err := json.Marshal(p.Stages)
	if err != nil {
		return err
	}
	_, err = s.db.Exec(`INSERT INTO pipelines (plan_id, name, description, tenant, stages, created_at) VALUES (?, ?, ?, ?, ?, ?)`,
		p.PlanID, p.Name, p.Description, p.Tenant, string(stagesJSON), p.Created)
	return err
}

// UpdatePipeline 更新指定流水线，并递增配置版本号。
func (s *Store) UpdatePipeline(id string, p Pipeline) error {
	stagesJSON, err := json.Marshal(p.Stages)
	if err != nil {
		return err
	}
	_, err = s.db.Exec(`UPDATE pipelines SET name = ?, description = ?, tenant = ?, stages = ? WHERE plan_id = ?`,
		p.Name, p.Description, p.Tenant, string(stagesJSON), id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion("pipeline")
}

// DeletePipeline 删除指定流水线，并递增配置版本号。
func (s *Store) DeletePipeline(id string) error {
	_, err := s.db.Exec(`DELETE FROM pipelines WHERE plan_id = ?`, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion("pipeline")
}

// ==================== Plugin 操作 ====================

// ListPlugins 列出所有已注册插件。
func (s *Store) ListPlugins() ([]PluginMeta, error) {
	rows, err := s.db.Query(`SELECT id, name, type, version, description, COALESCE(config, '{}') FROM plugins`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var plugins []PluginMeta
	for rows.Next() {
		var p PluginMeta
		if err := rows.Scan(&p.ID, &p.Name, &p.Type, &p.Version, &p.Description, &p.Config); err != nil {
			return nil, err
		}
		plugins = append(plugins, p)
	}
	return plugins, nil
}

// AddPlugin 注册一个新插件，并递增配置版本号。
func (s *Store) AddPlugin(p PluginMeta) error {
	if p.Config == "" {
		p.Config = "{}"
	}
	_, err := s.db.Exec(`INSERT INTO plugins (id, name, type, version, description, config) VALUES (?, ?, ?, ?, ?, ?)`,
		p.ID, p.Name, p.Type, p.Version, p.Description, p.Config)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion("plugin")
}

// ==================== Policy 操作 ====================

// ListPolicies 列出所有策略。
func (s *Store) ListPolicies() ([]Policy, error) {
	rows, err := s.db.Query(`SELECT id, name, type, content, created_at FROM policies`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var policies []Policy
	for rows.Next() {
		var p Policy
		if err := rows.Scan(&p.ID, &p.Name, &p.Type, &p.Content, &p.CreatedAt); err != nil {
			return nil, err
		}
		policies = append(policies, p)
	}
	return policies, nil
}

// AddPolicy 添加一条新策略。
func (s *Store) AddPolicy(p Policy) error {
	_, err := s.db.Exec(`INSERT INTO policies (id, name, type, content, created_at) VALUES (?, ?, ?, ?, ?)`,
		p.ID, p.Name, p.Type, p.Content, p.CreatedAt)
	return err
}

// ==================== API Key 操作 ====================

// ListApiKeys 列出所有 API 密钥。
func (s *Store) ListApiKeys() ([]ApiKey, error) {
	rows, err := s.db.Query(`SELECT id, name, key, tenant_id, status, created_at, last_used FROM apikeys`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var keys []ApiKey
	for rows.Next() {
		var k ApiKey
		var lastUsed sql.NullString
		if err := rows.Scan(&k.ID, &k.Name, &k.Key, &k.TenantID, &k.Status, &k.CreatedAt, &lastUsed); err != nil {
			return nil, err
		}
		if lastUsed.Valid {
			k.LastUsed = lastUsed.String
		}
		keys = append(keys, k)
	}
	return keys, nil
}

// AddApiKey 创建一个新的 API 密钥。
func (s *Store) AddApiKey(k ApiKey) error {
	_, err := s.db.Exec(`INSERT INTO apikeys (id, name, key, tenant_id, status, created_at, last_used) VALUES (?, ?, ?, ?, ?, ?, ?)`,
		k.ID, k.Name, k.Key, k.TenantID, k.Status, k.CreatedAt, k.LastUsed)
	return err
}

// GetApiKey 根据 ID 获取单个 API 密钥。
func (s *Store) GetApiKey(id string) (ApiKey, error) {
	var k ApiKey
	var lastUsed sql.NullString
	err := s.db.QueryRow(`SELECT id, name, key, tenant_id, status, created_at, last_used FROM apikeys WHERE id = ?`, id).
		Scan(&k.ID, &k.Name, &k.Key, &k.TenantID, &k.Status, &k.CreatedAt, &lastUsed)
	if err != nil {
		return k, err
	}
	if lastUsed.Valid {
		k.LastUsed = lastUsed.String
	}
	return k, nil
}

// UpdateApiKey 更新指定 API 密钥的全部字段。
func (s *Store) UpdateApiKey(k ApiKey) error {
	_, err := s.db.Exec(`UPDATE apikeys SET name = ?, key = ?, tenant_id = ?, status = ?, created_at = ?, last_used = ? WHERE id = ?`,
		k.Name, k.Key, k.TenantID, k.Status, k.CreatedAt, k.LastUsed, k.ID)
	return err
}

// ==================== Model 操作 ====================

// ListModels 列出所有模型配置。
func (s *Store) ListModels() ([]ModelConfig, error) {
	rows, err := s.db.Query(`SELECT id, name, upstream_url, upstream_model, api_key, api_key_header, use_proxy, proxy_url, is_default, supported_versions, status FROM models`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var models []ModelConfig
	for rows.Next() {
		var m ModelConfig
		var isDefault, useProxy int
		var versionsJSON string
		var apiKey, apiKeyHeader, proxyURL sql.NullString
		if err := rows.Scan(&m.ID, &m.Name, &m.UpstreamURL, &m.UpstreamModel, &apiKey, &apiKeyHeader, &useProxy, &proxyURL, &isDefault, &versionsJSON, &m.Status); err != nil {
			return nil, err
		}
		m.IsDefault = intToBool(isDefault)
		m.UseProxy = intToBool(useProxy)
		if apiKey.Valid {
			m.ApiKey = apiKey.String
		}
		if apiKeyHeader.Valid {
			m.ApiKeyHeader = apiKeyHeader.String
		}
		if proxyURL.Valid {
			m.ProxyURL = proxyURL.String
		}
		if versionsJSON != "" {
			json.Unmarshal([]byte(versionsJSON), &m.SupportedVersions)
		}
		models = append(models, m)
	}
	return models, nil
}

// AddModel 添加一个新模型配置，并递增配置版本号。
func (s *Store) AddModel(m ModelConfig) error {
	versionsJSON, _ := json.Marshal(m.SupportedVersions)
	_, err := s.db.Exec(`INSERT INTO models (id, name, upstream_url, upstream_model, api_key, api_key_header, use_proxy, proxy_url, is_default, supported_versions, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		m.ID, m.Name, m.UpstreamURL, m.UpstreamModel, m.ApiKey, m.ApiKeyHeader, boolToInt(m.UseProxy), m.ProxyURL, boolToInt(m.IsDefault), string(versionsJSON), m.Status)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion("model")
}

// GetModel 根据 ID 获取单个模型配置。
func (s *Store) GetModel(id string) (ModelConfig, error) {
	var m ModelConfig
	var isDefault, useProxy int
	var versionsJSON string
	var apiKey, apiKeyHeader, proxyURL sql.NullString
	err := s.db.QueryRow(`SELECT id, name, upstream_url, upstream_model, api_key, api_key_header, use_proxy, proxy_url, is_default, supported_versions, status FROM models WHERE id = ?`, id).
		Scan(&m.ID, &m.Name, &m.UpstreamURL, &m.UpstreamModel, &apiKey, &apiKeyHeader, &useProxy, &proxyURL, &isDefault, &versionsJSON, &m.Status)
	if err != nil {
		return m, err
	}
	m.IsDefault = intToBool(isDefault)
	m.UseProxy = intToBool(useProxy)
	if apiKey.Valid {
		m.ApiKey = apiKey.String
	}
	if apiKeyHeader.Valid {
		m.ApiKeyHeader = apiKeyHeader.String
	}
	if proxyURL.Valid {
		m.ProxyURL = proxyURL.String
	}
	if versionsJSON != "" {
		json.Unmarshal([]byte(versionsJSON), &m.SupportedVersions)
	}
	return m, nil
}

// UpdateModel 更新指定模型配置，并递增配置版本号。
func (s *Store) UpdateModel(id string, m ModelConfig) error {
	versionsJSON, _ := json.Marshal(m.SupportedVersions)
	_, err := s.db.Exec(`UPDATE models SET name = ?, upstream_url = ?, upstream_model = ?, api_key = ?, api_key_header = ?, use_proxy = ?, proxy_url = ?, is_default = ?, supported_versions = ?, status = ? WHERE id = ?`,
		m.Name, m.UpstreamURL, m.UpstreamModel, m.ApiKey, m.ApiKeyHeader, boolToInt(m.UseProxy), m.ProxyURL, boolToInt(m.IsDefault), string(versionsJSON), m.Status, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion("model")
}

// DeleteModel 删除指定模型配置，并递增配置版本号。
func (s *Store) DeleteModel(id string) error {
	_, err := s.db.Exec(`DELETE FROM models WHERE id = ?`, id)
	if err != nil {
		return err
	}
	return s.IncrementConfigVersion("model")
}
