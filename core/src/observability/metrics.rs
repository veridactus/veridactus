//! # 标签化 Prometheus 指标（§10.3.4）
//!
//! 使用标准 Prometheus client 输出带标签的指标。
//! 支持按 status/model/level 等维度聚合。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 带标签的计数器
#[derive(Clone)]
pub struct LabeledCounter {
    inner: Arc<Mutex<HashMap<String, u64>>>,
}

impl LabeledCounter {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn inc(&self, labels: &[(&str, &str)]) {
        let key = Self::build_key(labels);
        let mut map = self.inner.lock().unwrap();
        *map.entry(key).or_insert(0) += 1;
    }

    pub fn get(&self, labels: &[(&str, &str)]) -> u64 {
        let key = Self::build_key(labels);
        self.inner.lock().unwrap().get(&key).copied().unwrap_or(0)
    }

    fn build_key(labels: &[(&str, &str)]) -> String {
        let mut parts: Vec<String> = labels.iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect();
        parts.sort();
        parts.join(",")
    }

    pub fn export(&self, name: &str, help: &str) -> String {
        let map = self.inner.lock().unwrap();
        let mut out = format!("# HELP {} {}\n", name, help);
        out.push_str(&format!("# TYPE {} counter\n", name));
        for (labels, val) in map.iter() {
            out.push_str(&format!("{}{{{}}} {}\n", name, labels, val));
        }
        out
    }
}

/// 带标签的 Gauge
#[derive(Clone)]
pub struct LabeledGauge {
    inner: Arc<Mutex<HashMap<String, f64>>>,
}

impl LabeledGauge {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn set(&self, labels: &[(&str, &str)], value: f64) {
        let key = LabeledCounter::build_key(labels);
        self.inner.lock().unwrap().insert(key, value);
    }

    pub fn export(&self, name: &str, help: &str) -> String {
        let map = self.inner.lock().unwrap();
        let mut out = format!("# HELP {} {}\n", name, help);
        out.push_str(&format!("# TYPE {} gauge\n", name));
        for (labels, val) in map.iter() {
            out.push_str(&format!("{}{{{}}} {}\n", name, labels, val));
        }
        out
    }
}

// ==================== Histogram 支持 ====================

/// 标准延迟桶（毫秒）: 1, 5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000, +Inf
const LATENCY_BUCKETS_MS: &[f64] = &[1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0];

/// 带标签的 Histogram（Prometheus 标准格式）
#[derive(Clone)]
pub struct LabeledHistogram {
    inner: Arc<Mutex<HashMap<String, Vec<f64>>>>,
    sum: Arc<Mutex<HashMap<String, f64>>>,
    count: Arc<Mutex<HashMap<String, u64>>>,
    buckets: &'static [f64],
}

impl LabeledHistogram {
    pub fn new(buckets: &'static [f64]) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            sum: Arc::new(Mutex::new(HashMap::new())),
            count: Arc::new(Mutex::new(HashMap::new())),
            buckets,
        }
    }

    /// 默认延迟桶
    pub fn latency() -> Self { Self::new(LATENCY_BUCKETS_MS) }

    /// 记录一个观测值
    pub fn observe(&self, labels: &[(&str, &str)], value: f64) {
        let key = LabeledCounter::build_key(labels);
        
        // 更新桶计数
        let mut buckets_map = self.inner.lock().unwrap();
        let bucket_counts = buckets_map.entry(key.clone()).or_insert_with(|| vec![0.0; self.buckets.len() + 1]);
        let bucket = self.buckets.iter().position(|&b| value <= b).unwrap_or(self.buckets.len());
        bucket_counts[bucket] += 1.0;

        // 更新 sum
        let mut sum_map = self.sum.lock().unwrap();
        *sum_map.entry(key.clone()).or_insert(0.0) += value;

        // 更新 count
        let mut count_map = self.count.lock().unwrap();
        *count_map.entry(key).or_insert(0) += 1;
    }

    pub fn export(&self, name: &str, help: &str) -> String {
        let bucket_map = self.inner.lock().unwrap();
        let sum_map = self.sum.lock().unwrap();
        let count_map = self.count.lock().unwrap();

        let mut out = format!("# HELP {} {}\n", name, help);
        out.push_str(&format!("# TYPE {} histogram\n", name));

        for (key, bucket_counts) in bucket_map.iter() {
            let labels_str = if key.is_empty() { String::new() } else { format!("{{{}}}", key) };
            let count = count_map.get(key).copied().unwrap_or(0);
            let sum = sum_map.get(key).copied().unwrap_or(0.0);

            // 每个桶: name_bucket{le="..."} N
            for (i, &bound) in self.buckets.iter().enumerate() {
                out.push_str(&format!("{}_bucket{}le=\"{}\"}} {}\n",
                    name, labels_str, bound, bucket_counts[i] as u64));
            }
            out.push_str(&format!("{}_bucket{}le=\"+Inf\"}} {}\n",
                name, labels_str, bucket_counts.last().copied().unwrap_or(0.0) as u64));

            // sum + count
            out.push_str(&format!("{}_sum{} {:.3}\n", name, labels_str, sum));
            out.push_str(&format!("{}_count{} {}\n", name, labels_str, count));
        }
        out
    }
}

// ==================== 审计日志器 ====================

/// 审计日志器 — 独立于业务日志的合规审计通道（§10.1）
#[derive(Clone)]
pub struct AuditLogger {
    inner: Arc<Mutex<Vec<AuditEntry>>>,
    max_entries: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub event_type: String,
    pub trace_id: Option<String>,
    pub tenant_id: Option<String>,
    pub detail: String,
    pub severity: String,
}

impl AuditLogger {
    pub fn new(max_entries: usize) -> Self {
        Self { inner: Arc::new(Mutex::new(Vec::with_capacity(max_entries))), max_entries }
    }

    pub fn log(&self, event_type: &str, trace_id: Option<&str>, tenant_id: Option<&str>, detail: &str, severity: &str) {
        let mut entries = self.inner.lock().unwrap();
        if entries.len() >= self.max_entries {
            entries.remove(0);
        }
        entries.push(AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: event_type.to_string(),
            trace_id: trace_id.map(|s| s.to_string()),
            tenant_id: tenant_id.map(|s| s.to_string()),
            detail: detail.to_string(),
            severity: severity.to_string(),
        });
    }

    pub fn export(&self) -> Vec<AuditEntry> {
        self.inner.lock().unwrap().clone()
    }

    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(&*self.inner.lock().unwrap()).unwrap_or_default()
    }
}

// ==================== 增强的指标注册表 ====================
#[derive(Clone)]
pub struct LabeledMetrics {
    pub requests_total: LabeledCounter,
    pub constraint_violations_total: LabeledCounter,
    pub budget_remaining: LabeledGauge,
    pub latency_seconds: LabeledGauge,
    pub latency_distribution_ms: LabeledHistogram,
    pub guardrail_activations: LabeledCounter,
    pub active_prevention_blocks: LabeledCounter,
    pub asi_risks_flagged: LabeledCounter,
}

impl LabeledMetrics {
    pub fn new() -> Self {
        Self {
            requests_total: LabeledCounter::new(),
            constraint_violations_total: LabeledCounter::new(),
            budget_remaining: LabeledGauge::new(),
            latency_seconds: LabeledGauge::new(),
            latency_distribution_ms: LabeledHistogram::latency(),
            guardrail_activations: LabeledCounter::new(),
            active_prevention_blocks: LabeledCounter::new(),
            asi_risks_flagged: LabeledCounter::new(),
        }
    }

    /// 导出所有指标为 Prometheus 文本格式
    pub fn export_all(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.requests_total.export("veridactus_requests_total", "Total requests processed"));
        out.push_str("\n");
        out.push_str(&self.constraint_violations_total.export("veridactus_constraint_violations_total", "Constraint violations by type"));
        out.push_str("\n");
        out.push_str(&self.budget_remaining.export("veridactus_budget_remaining", "Current budget remaining by tenant"));
        out.push_str("\n");
        out.push_str(&self.latency_seconds.export("veridactus_latency_seconds", "Request latency by phase"));
        out.push_str("\n");
        out.push_str(&self.latency_distribution_ms.export("veridactus_latency_distribution_ms", "Request latency distribution (histogram)"));
        out.push_str("\n");
        out.push_str(&self.guardrail_activations.export("veridactus_guardrail_activations_total", "Guardrail triggers by level and severity"));
        out.push_str("\n");
        out.push_str(&self.active_prevention_blocks.export("veridactus_active_prevention_blocks_total", "Tokens blocked by constrained decoding"));
        out.push_str("\n");
        out.push_str(&self.asi_risks_flagged.export("veridactus_asi_risks_flagged_total", "OWASP ASI risks flagged"));
        out
    }
}

impl Default for LabeledMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_labeled_counter() {
        let c = LabeledCounter::new();
        c.inc(&[("status", "200"), ("model", "gpt-4o")]);
        c.inc(&[("status", "200"), ("model", "gpt-4o")]);
        c.inc(&[("status", "429"), ("model", "gpt-4o")]);

        assert_eq!(c.get(&[("status", "200"), ("model", "gpt-4o")]), 2);
        assert_eq!(c.get(&[("status", "429"), ("model", "gpt-4o")]), 1);

        let out = c.export("test_total", "Test counter");
        assert!(out.contains("test_total{"));
        assert!(out.contains("200"));
        assert!(out.contains("gpt-4o"));
    }

    #[test]
    fn test_labeled_gauge() {
        let g = LabeledGauge::new();
        g.set(&[("tenant", "acme")], 0.05);
        g.set(&[("tenant", "corp")], 0.10);
        let out = g.export("test_gauge", "Test gauge");
        assert!(out.contains("acme"));
        assert!(out.contains("corp"));
    }

    #[test]
    fn test_full_export_format() {
        let m = LabeledMetrics::new();
        m.requests_total.inc(&[("status", "200")]);
        m.budget_remaining.set(&[("tenant", "test")], 0.05);
        let out = m.export_all();
        assert!(out.contains("veridactus_requests_total"));
        assert!(out.contains("veridactus_budget_remaining"));
        assert!(out.contains("# HELP"));
        assert!(out.contains("# TYPE"));
    }
}
