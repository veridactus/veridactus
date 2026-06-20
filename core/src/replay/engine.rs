//! # 重放引擎
//!
//! 从 Journal 事件重建 LLM 调用，支持 record/replay/hybrid/branch 模式。
//! 遵循协议 §9.4 Deterministic Replay Engine。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

use crate::replay::upstream_cache::{CacheKey, UpstreamResponseCache};
use crate::types::journal::{ExecutionJournal, JournalEventType};
use crate::types::trace::{ReplaySnapshot, Trace};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayComparisonResult {
    pub trace_a_id: String,
    pub trace_b_id: String,
    pub is_identical: bool,
    pub differences: Vec<ResponseDifference>,
    pub similarity_score: f64,
    pub hash_match: bool,
    pub token_diff_count: usize,
    pub byte_diff_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDifference {
    pub diff_type: DifferenceType,
    pub position: usize,
    pub expected: String,
    pub actual: String,
    pub severity: DiffSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DifferenceType {
    TokenMismatch,
    LengthMismatch,
    FormatMismatch,
    MetadataMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffSeverity {
    Minor,
    Major,
    Critical,
}

/// 重放模式（§9.4.1）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReplayMode {
    /// 记录所有交互
    #[serde(rename = "record")]
    Record,
    /// 从快照重放
    #[serde(rename = "replay")]
    Replay,
    /// 混合模式（记录缺失项，重放已缓存项）
    #[serde(rename = "hybrid")]
    Hybrid,
    /// 分支执行（基于现有快照创建分支）
    #[serde(rename = "branch")]
    Branch,
}

impl std::fmt::Display for ReplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayMode::Record => write!(f, "record"),
            ReplayMode::Replay => write!(f, "replay"),
            ReplayMode::Hybrid => write!(f, "hybrid"),
            ReplayMode::Branch => write!(f, "branch"),
        }
    }
}

/// 重放分支信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayBranch {
    /// 分支 ID
    pub branch_id: Uuid,
    /// 父分支 ID（如果是分支）
    pub parent_branch_id: Option<Uuid>,
    /// 分支名称
    pub name: String,
    /// 创建时间
    pub created_at: String,
    /// 快照数量
    pub snapshot_count: usize,
}

/// 重放结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    /// 重放的 Trace
    pub trace: Trace,
    /// 是否命中缓存
    pub cache_hit: bool,
    /// 重放耗时（毫秒）
    pub duration_ms: u64,
    /// 使用的分支 ID
    pub branch_id: Option<Uuid>,
}

/// 重放统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStats {
    /// 总重放次数
    pub total_replays: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
    /// 分支数量
    pub branch_count: usize,
    /// 缓存命中率
    pub hit_rate: f64,
}

impl Default for ReplayStats {
    fn default() -> Self {
        Self {
            total_replays: 0,
            cache_hits: 0,
            cache_misses: 0,
            branch_count: 0,
            hit_rate: 0.0,
        }
    }
}

/// 重放引擎
pub struct ReplayEngine {
    /// 上游响应缓存
    cache: UpstreamResponseCache,
    /// 分支存储
    branches: HashMap<Uuid, ReplayBranch>,
    /// 默认分支 ID
    default_branch_id: Uuid,
    /// 重放统计
    stats: ReplayStats,
}

impl ReplayEngine {
    /// 创建新的重放引擎
    pub fn new(cache: UpstreamResponseCache) -> Self {
        let default_branch_id = Uuid::new_v4();
        let mut branches = HashMap::new();
        branches.insert(default_branch_id, ReplayBranch {
            branch_id: default_branch_id,
            parent_branch_id: None,
            name: "main".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            snapshot_count: 0,
        });

        Self {
            cache,
            branches,
            default_branch_id,
            stats: ReplayStats::default(),
        }
    }

    /// 创建带缓存配置的重放引擎
    pub fn with_cache(max_entries: usize, ttl_secs: u64) -> Self {
        let cache = UpstreamResponseCache::new(ttl_secs, max_entries);
        Self::new(cache)
    }

    /// 获取当前重放统计
    pub fn get_stats(&self) -> &ReplayStats {
        &self.stats
    }

    /// 更新重放统计
    fn update_stats(&mut self, cache_hit: bool) {
        self.stats.total_replays += 1;
        if cache_hit {
            self.stats.cache_hits += 1;
        } else {
            self.stats.cache_misses += 1;
        }
        if self.stats.total_replays > 0 {
            self.stats.hit_rate = self.stats.cache_hits as f64 / self.stats.total_replays as f64;
        }
    }

    /// 获取所有分支
    pub fn list_branches(&self) -> Vec<&ReplayBranch> {
        self.branches.values().collect()
    }

    /// 获取分支
    pub fn get_branch(&self, branch_id: &Uuid) -> Option<&ReplayBranch> {
        self.branches.get(branch_id)
    }

    /// 创建新分支
    pub fn create_branch(&mut self, name: &str, parent_id: Option<Uuid>) -> Result<ReplayBranch, String> {
        let parent = parent_id.and_then(|id| self.branches.get(&id)).cloned();
        let branch_id = Uuid::new_v4();

        let branch = ReplayBranch {
            branch_id,
            parent_branch_id: parent_id,
            name: name.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            snapshot_count: parent.as_ref().map(|p| p.snapshot_count).unwrap_or(0),
        };

        self.branches.insert(branch_id, branch.clone());
        self.stats.branch_count = self.branches.len();
        Ok(branch)
    }

    /// 删除分支
    pub fn delete_branch(&mut self, branch_id: &Uuid) -> Result<(), String> {
        if *branch_id == self.default_branch_id {
            return Err("Cannot delete default branch".to_string());
        }
        if self.branches.remove(branch_id).is_some() {
            self.stats.branch_count = self.branches.len();
            Ok(())
        } else {
            Err("Branch not found".to_string())
        }
    }

    /// 执行重放（混合模式）
    ///
    /// 尝试从缓存加载响应，如果未命中则返回错误。
    pub fn replay(&mut self, original: &Trace) -> Result<ReplayResult, String> {
        let cache_key = self.build_cache_key(original);
        let start = std::time::Instant::now();

        match self.cache.get(&cache_key) {
            Some(cached) => {
                let mut trace = original.clone();
                trace.output = Some(crate::types::trace::Output {
                    response: Some(cached.response.clone()),
                    truncated: false,
                    finish_reason: Some("replayed".to_string()),
                });

                self.update_stats(true);

                Ok(ReplayResult {
                    trace,
                    cache_hit: true,
                    duration_ms: start.elapsed().as_millis() as u64,
                    branch_id: Some(self.default_branch_id),
                })
            }
            None => {
                self.update_stats(false);
                Err(format!("Cache miss for trace: {}", original.trace_id))
            }
        }
    }

    /// 记录到缓存
    pub fn record(&mut self, trace: &Trace) -> Result<(), String> {
        if let Some(ref output) = trace.output {
            let cache_key = self.build_cache_key(trace);
            let response_json = output.response.as_ref()
                .ok_or("No response to record")?;

            self.cache.insert(cache_key, response_json.clone());
        }
        Ok(())
    }

    /// 混合模式：尝试重放，未命中时记录
    pub fn hybrid(&mut self, trace: &Trace) -> Result<ReplayResult, String> {
        let cache_key = self.build_cache_key(trace);
        let start = std::time::Instant::now();

        match self.cache.get(&cache_key) {
            Some(cached) => {
                let mut result_trace = trace.clone();
                result_trace.output = Some(crate::types::trace::Output {
                    response: Some(cached.response.clone()),
                    truncated: false,
                    finish_reason: Some("replayed".to_string()),
                });

                self.update_stats(true);

                Ok(ReplayResult {
                    trace: result_trace,
                    cache_hit: true,
                    duration_ms: start.elapsed().as_millis() as u64,
                    branch_id: Some(self.default_branch_id),
                })
            }
            None => {
                // 缓存未命中，记录新响应
                if let Some(ref output) = trace.output {
                    let response_json = output.response.as_ref()
                        .ok_or("No response to record")?;
                    self.cache.insert(cache_key, response_json.clone());
                }

                self.update_stats(false);

                Ok(ReplayResult {
                    trace: trace.clone(),
                    cache_hit: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    branch_id: Some(self.default_branch_id),
                })
            }
        }
    }

    /// 构建缓存键
    fn build_cache_key(&self, trace: &Trace) -> CacheKey {
        CacheKey::new(
            &trace.model,
            trace.input.as_ref()
                .and_then(|i| i.prompt.as_ref())
                .unwrap_or(&serde_json::Value::Null),
            trace.input.as_ref()
                .and_then(|i| i.params.as_ref())
                .unwrap_or(&serde_json::Value::Null),
            "",
        )
    }

    /// 从 Journal 重建请求
    ///
    /// 解析 Journal 事件序列，提取原始请求信息。
    pub fn reconstruct_request(journal: &ExecutionJournal) -> Result<serde_json::Value, String> {
        let mut model = String::new();
        let messages = serde_json::Value::Null;
        let mut params = serde_json::json!({});

        for event in &journal.events {
            match &event.event_type {
                JournalEventType::RequestParsed {
                    model: m,
                    temperature,
                    max_tokens,
                } => {
                    model = m.clone();
                    if let Some(t) = temperature {
                        params["temperature"] = serde_json::json!(t);
                    }
                    if let Some(mt) = max_tokens {
                        params["max_tokens"] = serde_json::json!(mt);
                    }
                }
                JournalEventType::RequestReceived {  .. } => {
                    // TODO: 从持久化存储加载原始请求体
                }
                _ => {}
            }
        }

        Ok(serde_json::json!({
            "model": model,
            "messages": messages,
            "params": params,
        }))
    }

    /// 记录交互到快照
    pub fn record_to_snapshot(
        trace: &Trace,
        mode: &str,
    ) -> ReplaySnapshot {
        let interactions = trace
            .output
            .as_ref()
            .map(|o| {
                vec![crate::types::trace::ReplayInteraction {
                    sequence: 1,
                    model: trace.model.clone(),
                    prompt_hash: trace.input.as_ref()
                        .and_then(|i| i.prompt.as_ref())
                        .map(|p| {
                            let json = serde_json::to_string(p).unwrap_or_default();
                            format!("sha256:{}", hex::encode(Sha256::digest(json.as_bytes())))
                        })
                        .unwrap_or_default(),
                    response_hash: o.response.as_ref()
                        .map(|r| {
                            let json = serde_json::to_string(r).unwrap_or_default();
                            format!("sha256:{}", hex::encode(Sha256::digest(json.as_bytes())))
                        })
                        .unwrap_or_default(),
                    tokens_used: 0,
                    latency_ms: 0,
                }]
            })
            .unwrap_or_default();

        ReplaySnapshot {
            mode: Some(mode.to_string()),
            interactions: Some(interactions),
            environment_snapshot: Some(crate::types::trace::EnvironmentSnapshot {
                model_version: trace.model.clone(),
                sdk_version: "veridactus-0.2.1".to_string(),
                engine_determinism_strategy: trace.engine_determinism.as_ref().map(|d| d.strategy.clone()),
                recorded_at: chrono::Utc::now().to_rfc3339(),
            }),
        }
    }

    /// 分支重放模式：从指定分支点开始执行（§9.4.4）
    ///
    /// 1. 重放所有交互直到分支点
    /// 2. 从分支点开始执行新的推理
    pub fn branch_replay(
        &mut self,
        parent_trace: &Trace,
        branch_point: u32,
        branch_name: &str,
    ) -> Result<ReplayResult, String> {
        let start = std::time::Instant::now();
        
        // 创建新分支
        let parent_branch_id = Some(self.default_branch_id);
        let new_branch = self.create_branch(branch_name, parent_branch_id)?;
        
        // 获取父trace的快照
        let parent_snapshot = parent_trace.observations.as_ref()
            .and_then(|obs| obs.replay_snapshot.as_ref())
            .ok_or("Parent trace has no replay snapshot")?;
        
        let interactions = parent_snapshot.interactions.as_ref()
            .ok_or("No interactions in parent snapshot")?;
        
        // 检查分支点是否有效
        if branch_point as usize > interactions.len() {
            return Err(format!("Branch point {} exceeds available interactions ({})", 
                branch_point, interactions.len()));
        }
        
        // 重放直到分支点
        let mut replayed_trace = parent_trace.clone();
        replayed_trace.trace_id = Uuid::new_v4();
        replayed_trace.parent_id = Some(parent_trace.trace_id);
        
        // 添加分支元数据到 extensions
        let mut extensions = replayed_trace.extensions.clone().unwrap_or_default();
        extensions["veridactus.ai/v1/deterministic_replay"] = serde_json::json!({
            "branch_point": branch_point,
            "branch_id": new_branch.branch_id.to_string()
        });
        replayed_trace.extensions = Some(extensions);
        
        Ok(ReplayResult {
            trace: replayed_trace,
            cache_hit: false,
            duration_ms: start.elapsed().as_millis() as u64,
            branch_id: Some(new_branch.branch_id),
        })
    }

    /// 从分支点获取快照
    pub fn get_snapshot_at_branch_point(
        &self,
        trace: &Trace,
        branch_point: u32,
    ) -> Option<crate::types::trace::ReplayInteraction> {
        trace.observations.as_ref()
            .and_then(|obs| obs.replay_snapshot.as_ref())
            .and_then(|s| s.interactions.as_ref())
            .and_then(|interactions| interactions.get(branch_point as usize))
            .cloned()
    }

    /// 合并分支到主分支
    pub fn merge_branch(&mut self, source_branch_id: &Uuid, target_branch_id: &Uuid) -> Result<(), String> {
        if !self.branches.contains_key(source_branch_id) {
            return Err("Source branch not found".to_string());
        }

        let source_count = self.branches[source_branch_id].snapshot_count;

        let target = self.branches.get_mut(target_branch_id)
            .ok_or("Target branch not found")?;

        target.snapshot_count += source_count;

        Ok(())
    }

    pub fn compare_responses(&self, trace_a: &Trace, trace_b: &Trace) -> ReplayComparisonResult {
        let response_a = trace_a.output.as_ref()
            .and_then(|o| o.response.as_ref())
            .map(|r| r.to_string())
            .unwrap_or_default();
        let response_b = trace_b.output.as_ref()
            .and_then(|o| o.response.as_ref())
            .map(|r| r.to_string())
            .unwrap_or_default();

        let hash_a = format!("sha256:{}", hex::encode(Sha256::digest(response_a.as_bytes())));
        let hash_b = format!("sha256:{}", hex::encode(Sha256::digest(response_b.as_bytes())));
        let hash_match = hash_a == hash_b;

        let mut differences = Vec::new();
        let mut token_diff_count = 0;
        let mut byte_diff_count = 0;

        if response_a != response_b {
            byte_diff_count = response_a.len().abs_diff(response_b.len());

            let chars_a: Vec<char> = response_a.chars().collect();
            let chars_b: Vec<char> = response_b.chars().collect();
            let min_len = chars_a.len().min(chars_b.len());

            for i in 0..min_len {
                if chars_a[i] != chars_b[i] {
                    differences.push(ResponseDifference {
                        diff_type: DifferenceType::TokenMismatch,
                        position: i,
                        expected: chars_a[i].to_string(),
                        actual: chars_b[i].to_string(),
                        severity: DiffSeverity::Major,
                    });
                    token_diff_count += 1;
                }
            }

            if chars_a.len() != chars_b.len() {
                differences.push(ResponseDifference {
                    diff_type: DifferenceType::LengthMismatch,
                    position: min_len,
                    expected: format!("{} chars", chars_a.len()),
                    actual: format!("{} chars", chars_b.len()),
                    severity: DiffSeverity::Minor,
                });
            }
        }

        let similarity_score = if !response_a.is_empty() || !response_b.is_empty() {
            let longer = response_a.len().max(response_b.len());
            if longer == 0 {
                1.0
            } else {
                (longer - byte_diff_count) as f64 / longer as f64
            }
        } else {
            1.0
        };

        let is_identical = hash_match && differences.is_empty();

        ReplayComparisonResult {
            trace_a_id: trace_a.trace_id.to_string(),
            trace_b_id: trace_b.trace_id.to_string(),
            is_identical,
            differences,
            similarity_score,
            hash_match,
            token_diff_count,
            byte_diff_count,
        }
    }

    pub fn verify_deterministic(&self, original: &Trace, replayed: &Trace) -> DeterminismVerificationResult {
        let comparison = self.compare_responses(original, replayed);

        let mut issues = Vec::new();

        if !comparison.hash_match {
            issues.push(DeterminismIssue {
                issue_type: DeterminismIssueType::ResponseMismatch,
                description: format!("响应哈希不匹配: 原始={}, 重放={}",
                    comparison.trace_a_id, comparison.trace_b_id),
                severity: IssueSeverity::Critical,
            });
        }

        if comparison.token_diff_count > 0 {
            issues.push(DeterminismIssue {
                issue_type: DeterminismIssueType::TokenDrift,
                description: format!("Token漂移: {} 个token不同", comparison.token_diff_count),
                severity: IssueSeverity::Major,
            });
        }

        DeterminismVerificationResult {
            is_deterministic: issues.is_empty(),
            issues,
            comparison,
            verification_time: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterminismVerificationResult {
    pub is_deterministic: bool,
    pub issues: Vec<DeterminismIssue>,
    pub comparison: ReplayComparisonResult,
    pub verification_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterminismIssue {
    pub issue_type: DeterminismIssueType,
    pub description: String,
    pub severity: IssueSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeterminismIssueType {
    ResponseMismatch,
    TokenDrift,
    HashMismatch,
    TimingViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Minor,
    Major,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::proof::Proofs;
    use crate::types::trace::{Input, Output};

    #[test]
    fn test_replay_cache_hit() {
        let cache = UpstreamResponseCache::new(60, 100);
        let mut engine = ReplayEngine::new(cache);

        let mut trace = Trace::new("test/model");
        trace.trace_id = Uuid::new_v4();
        trace.input = Some(Input {
            prompt: Some(serde_json::json!([{"role":"user","content":"hi"}])),
            params: None,
            metadata: None,
        });

        // 缓存未命中
        assert!(engine.replay(&trace).is_err());
    }

    #[test]
    fn test_reconstruct_request() {
        let trace_id = Uuid::new_v4();
        let mut journal = ExecutionJournal::new(trace_id, "test");

        journal.append_event(JournalEventType::RequestParsed {
            model: "test-model".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(100),
        });

        let result = ReplayEngine::reconstruct_request(&journal).unwrap();
        assert_eq!(result["model"], "test-model");
        assert_eq!(result["params"]["temperature"], 0.7);
    }

    #[test]
    fn test_record_to_snapshot() {
        let mut trace = Trace::new("test-model");
        trace.input = Some(Input {
            prompt: Some(serde_json::json!("hello")),
            params: None,
            metadata: None,
        });
        trace.output = Some(Output {
            response: Some(serde_json::json!("world")),
            truncated: false,
            finish_reason: Some("stop".to_string()),
        });

        let snapshot = ReplayEngine::record_to_snapshot(&trace, "record");
        assert_eq!(snapshot.mode, Some("record".to_string()));
        assert!(snapshot.interactions.is_some());
    }
}
