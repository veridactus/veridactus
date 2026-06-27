//! VERIDACTUS Prometheus 指标导出 (Phase 5)
//! 生产级可观测性 — 端点: GET /metrics
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

fn metrics_registry() -> &'static Mutex<MetricsRegistry> {
    static METRICS: OnceLock<Mutex<MetricsRegistry>> = OnceLock::new();
    METRICS.get_or_init(|| Mutex::new(MetricsRegistry::new()))
}

pub struct Counter {
    pub name: String,
    pub help: String,
    pub labels: HashMap<String, String>,
    pub value: AtomicU64,
}
impl Counter {
    pub fn inc(&self) { self.value.fetch_add(1, Ordering::Relaxed); }
    pub fn add(&self, n: u64) { self.value.fetch_add(n, Ordering::Relaxed); }
    pub fn get(&self) -> u64 { self.value.load(Ordering::Relaxed) }
}

struct MetricsRegistry {
    counters: HashMap<String, Counter>,
    started_at: Instant,
}
impl MetricsRegistry {
    fn new() -> Self { Self { counters: HashMap::new(), started_at: Instant::now() } }
    fn register(&mut self, name: &str, help: &str, labels: HashMap<String, String>) {
        self.counters.insert(name.to_string(), Counter { name: name.to_string(), help: help.to_string(), labels, value: AtomicU64::new(0) });
    }
    fn render(&self) -> String {
        let mut out = format!("# HELP veridactus_uptime_seconds Data plane uptime\n# TYPE veridactus_uptime_seconds gauge\nveridactus_uptime_seconds {}\n# HELP veridactus_info Version\n# TYPE veridactus_info gauge\nveridactus_info{{version=\"0.3.0\"}} 1\n", self.started_at.elapsed().as_secs());
        for c in self.counters.values() {
            let labels: Vec<String> = c.labels.iter().map(|(k,v)| format!("{}=\"{}\"",k,v)).collect();
            out.push_str(&format!("# HELP {} {}\n# TYPE {} counter\n{}{{{}}} {}\n", c.name, c.help, c.name, c.name, labels.join(","), c.get()));
        }
        out
    }
}

pub fn init_metrics() {
    let mut r = metrics_registry().lock().unwrap();
    r.register("veridactus_requests_total", "Total requests", HashMap::new());
    r.register("veridactus_sse_connections_total", "Total SSE connections", HashMap::new());
    r.register("veridactus_tokens_consumed_total", "Total tokens", HashMap::new());
    r.register("veridactus_budget_exceeded_total", "Budget exceeded events", HashMap::new());
    r.register("veridactus_safety_events_total", "Safety events", HashMap::from([("type".into(),"all".into())]));
    r.register("veridactus_pii_detections_total", "PII detections", HashMap::new());
    r.register("veridactus_injection_blocks_total", "Injection blocks", HashMap::new());
    r.register("veridactus_l0_signatures_total", "L0 signatures", HashMap::new());
    r.register("veridactus_l0_verify_failures_total", "L0 verify failures", HashMap::new());
    r.register("veridactus_request_errors_total", "Request errors", HashMap::new());
}

pub fn render_metrics() -> String { metrics_registry().lock().unwrap().render() }
pub fn inc_requests() { if let Ok(r) = metrics_registry().lock() { if let Some(c) = r.counters.get("veridactus_requests_total") { c.inc(); } } }
pub fn add_tokens(n: u64) { if let Ok(r) = metrics_registry().lock() { if let Some(c) = r.counters.get("veridactus_tokens_consumed_total") { c.add(n); } } }
pub fn inc_budget_exceeded() { if let Ok(r) = metrics_registry().lock() { if let Some(c) = r.counters.get("veridactus_budget_exceeded_total") { c.inc(); } } }
pub fn inc_safety_event() { if let Ok(r) = metrics_registry().lock() { if let Some(c) = r.counters.get("veridactus_safety_events_total") { c.inc(); } } }
pub fn inc_l0_signature() { if let Ok(r) = metrics_registry().lock() { if let Some(c) = r.counters.get("veridactus_l0_signatures_total") { c.inc(); } } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_init_and_render() { init_metrics(); inc_requests(); add_tokens(100); let o = render_metrics(); assert!(o.contains("veridactus_requests_total")); assert!(o.contains("veridactus_uptime_seconds")); }
}
