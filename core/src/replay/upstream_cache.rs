//! # 上游响应缓存（M14）
//!
//! 严格遵循 AI.md §7.3。
//! 重放时缓存上游 LLM 响应，避免重复调用。

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 缓存键（AI.md §7.3）
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct CacheKey {
    /// 模型名称
    pub model: String,
    /// 请求规范化输入的 SHA-256
    pub request_hash: String,
    /// 参数（temperature/seed 等）的 SHA-256
    pub params_hash: String,
    /// 上游 API 版本
    pub upstream_version: String,
}

impl CacheKey {
    /// 从模型、消息和参数创建缓存键
    pub fn new(
        model: impl Into<String>,
        messages: &serde_json::Value,
        params: &serde_json::Value,
        upstream_version: impl Into<String>,
    ) -> Self {
        let request_hash = sha256_json(messages);
        let params_hash = sha256_json(params);
        Self {
            model: model.into(),
            request_hash,
            params_hash,
            upstream_version: upstream_version.into(),
        }
    }
}

/// 缓存的响应
#[derive(Clone, Debug)]
pub struct CachedResponse {
    /// 响应体（JSON Value）
    pub response: serde_json::Value,
    /// 插入时间
    pub inserted_at: Instant,
    /// TTL
    pub ttl: Duration,
}

impl CachedResponse {
    /// 检查缓存是否有效
    pub fn is_fresh(&self) -> bool {
        self.inserted_at.elapsed() < self.ttl
    }
}

/// 上游响应缓存（AI.md §7.3 UpstreamResponseCache）
pub struct UpstreamResponseCache {
    /// 缓存条目
    entries: HashMap<CacheKey, CachedResponse>,
    /// 默认 TTL
    default_ttl: Duration,
    /// 最大条目数
    max_entries: usize,
}

impl UpstreamResponseCache {
    /// 创建新的缓存
    pub fn new(default_ttl_secs: u64, max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl: Duration::from_secs(default_ttl_secs),
            max_entries,
        }
    }

    /// 获取缓存的响应（AI.md §7.3）
    ///
    /// 精确匹配：所有字段必须完全一致。
    pub fn get(&self, key: &CacheKey) -> Option<&CachedResponse> {
        self.entries.get(key).filter(|entry| entry.is_fresh())
    }

    /// 插入缓存（AI.md §7.3）
    ///
    /// 在达到最大条目数时淘汰最早的条目。
    pub fn insert(&mut self, key: CacheKey, response: serde_json::Value) {
        // 淘汰过期条目
        self.evict_expired();

        // 如果缓存满了，淘汰最旧的
        if self.entries.len() >= self.max_entries {
            if let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, v)| v.inserted_at)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&oldest_key);
            }
        }

        self.entries.insert(
            key,
            CachedResponse {
                response,
                inserted_at: Instant::now(),
                ttl: self.default_ttl,
            },
        );
    }

    /// 清除所有缓存
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 获取当前条目数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 清除过期条目
    fn evict_expired(&mut self) {
        self.entries.retain(|_, v| v.is_fresh());
    }
}

/// 计算 JSON 值的 SHA-256
fn sha256_json(value: &serde_json::Value) -> String {
    let json_str = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json_str.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_hit() {
        let mut cache = UpstreamResponseCache::new(60, 100);
        let key = CacheKey::new(
            "model-x",
            &serde_json::json!([{"role":"user","content":"hi"}]),
            &serde_json::json!({"temp":0.0}),
            "v1",
        );
        let response = serde_json::json!({"response": "hello"});
        cache.insert(key.clone(), response.clone());

        let cached = cache.get(&key).unwrap();
        assert_eq!(cached.response, response);
    }

    #[test]
    fn test_cache_miss() {
        let cache = UpstreamResponseCache::new(60, 100);
        let key = CacheKey::new(
            "model-x",
            &serde_json::json!([{"role":"user","content":"hi"}]),
            &serde_json::json!({"temp":0.0}),
            "v1",
        );
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = UpstreamResponseCache::new(60, 2); // 最多2条
        let k1 = CacheKey::new(
            "m1",
            &serde_json::json!([{"role":"user","content":"a"}]),
            &serde_json::json!({}),
            "v1",
        );
        let k2 = CacheKey::new(
            "m2",
            &serde_json::json!([{"role":"user","content":"b"}]),
            &serde_json::json!({}),
            "v1",
        );
        let k3 = CacheKey::new(
            "m3",
            &serde_json::json!([{"role":"user","content":"c"}]),
            &serde_json::json!({}),
            "v1",
        );

        cache.insert(k1.clone(), serde_json::json!("r1"));
        cache.insert(k2.clone(), serde_json::json!("r2"));
        assert_eq!(cache.len(), 2);

        cache.insert(k3.clone(), serde_json::json!("r3"));
        // k1 应被淘汰
        assert!(cache.get(&k1).is_none());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_expiry() {
        let mut cache = UpstreamResponseCache::new(0, 100); // 0秒TTL
        let key = CacheKey::new(
            "m",
            &serde_json::json!([{"role":"user","content":"hi"}]),
            &serde_json::json!({}),
            "v1",
        );
        cache.insert(key.clone(), serde_json::json!("r"));
        thread::sleep(Duration::from_millis(10));
        assert!(cache.get(&key).is_none());
    }
}
