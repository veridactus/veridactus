//! # OpenTelemetry GenAI Semantic Conventions (§3.6, §10.3.1)
//!
//! 实现 OTel GenAI v1.40.0 语义约定映射。
//! 所有 VERIDACTUS Trace 字段自动映射到标准 OTel Span 属性。

use std::collections::HashMap;
use uuid::Uuid;

use crate::types::trace::{ExecutionGraphSnapshot, Monitoring};

// ==================== OTel GenAI 语义约定 ====================

/// OTel GenAI 标准属性名
pub mod genai_attrs {
    pub const SYSTEM: &str = "gen_ai.system";
    pub const REQUEST_MODEL: &str = "gen_ai.request.model";
    pub const REQUEST_TEMPERATURE: &str = "gen_ai.request.temperature";
    pub const REQUEST_TOP_P: &str = "gen_ai.request.top_p";
    pub const REQUEST_MAX_TOKENS: &str = "gen_ai.request.max_tokens";
    pub const USAGE_INPUT_TOKENS: &str = "gen_ai.usage.input_tokens";
    pub const USAGE_OUTPUT_TOKENS: &str = "gen_ai.usage.output_tokens";
    pub const USAGE_TOTAL_TOKENS: &str = "gen_ai.usage.total_tokens";
    pub const RESPONSE_FINISH_REASON: &str = "gen_ai.response.finish_reason";
    pub const RESPONSE_ID: &str = "gen_ai.response.id";
    pub const RESPONSE_MODEL: &str = "gen_ai.response.model";
    pub const VERIDACTUS_ENGINE_DETERMINISM: &str = "gen_ai.veridactus.engine_determinism";
    pub const VERIDACTUS_AEC_ID: &str = "gen_ai.veridactus.agent_execution_chain_id";
    pub const VERIDACTUS_PROOF_LEVEL: &str = "gen_ai.veridactus.proof_level";
}

/// OTel Span with GenAI attributes
pub struct GenAiSpan {
    pub span_id: Uuid,
    pub trace_id: String,
    pub attributes: HashMap<&'static str, String>,
    start_time: std::time::Instant,
}

impl GenAiSpan {
    pub fn new(trace_id: &str) -> Self {
        Self {
            span_id: Uuid::new_v4(),
            trace_id: trace_id.to_string(),
            attributes: HashMap::new(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn set_model(&mut self, model: &str) {
        self.attributes.insert(genai_attrs::SYSTEM, model.split('/').next().unwrap_or(model).to_string());
        self.attributes.insert(genai_attrs::REQUEST_MODEL, model.to_string());
    }

    pub fn set_temperature(&mut self, temp: f64) {
        self.attributes.insert(genai_attrs::REQUEST_TEMPERATURE, temp.to_string());
    }

    pub fn set_usage(&mut self, input_tokens: u64, output_tokens: u64) {
        self.attributes.insert(genai_attrs::USAGE_INPUT_TOKENS, input_tokens.to_string());
        self.attributes.insert(genai_attrs::USAGE_OUTPUT_TOKENS, output_tokens.to_string());
        self.attributes.insert(genai_attrs::USAGE_TOTAL_TOKENS, (input_tokens + output_tokens).to_string());
    }

    pub fn set_finish_reason(&mut self, reason: &str) {
        self.attributes.insert(genai_attrs::RESPONSE_FINISH_REASON, reason.to_string());
    }

    pub fn set_proof_level(&mut self, level: &str) {
        self.attributes.insert(genai_attrs::VERIDACTUS_PROOF_LEVEL, level.to_string());
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.start_time.elapsed().as_millis() as f64
    }

    pub fn to_monitoring(&self) -> Monitoring {
        let elapsed = self.elapsed_ms();

        // 基于统计的异常检测 (§10.3.3)
        // 使用 z-score 方法，基于历史延迟均值和标准差
        let anomaly = AnomalyDetector::global().evaluate(elapsed);

        Monitoring {
            otel_trace_id: Some(self.span_id.to_string()),
            execution_graph_snapshot: Some(ExecutionGraphSnapshot {
                nodes: None,
                edges: None,
            }),
            anomaly_score: Some(anomaly.score.min(1.0).max(0.0)),
            drift_detection: Some(anomaly.into()),
        }
    }
}

// ==================== 异常检测引擎 ====================

/// 基于统计算法的异常检测器 (§10.3.3)
///
/// 使用指数加权移动平均 (EWMA) + z-score 检测异常延迟。
/// 替代简单的启发式计算。
pub struct AnomalyDetector {
    /// EWMA 均值
    mean: f64,
    /// EWMA 方差的平方（近似）
    m2: f64,
    /// 样本数
    count: u64,
    /// 衰减因子 (0 < alpha < 1, 越小越平滑)
    alpha: f64,
}

#[derive(Debug)]
pub struct AnomalyResult {
    pub score: f64,
    pub z_score: f64,
    pub is_anomaly: bool,
}

impl AnomalyDetector {
    pub fn new(alpha: f64) -> Self {
        Self {
            mean: 0.0,
            m2: 0.0,
            count: 0,
            alpha,
        }
    }

    /// 更新统计并返回异常评分
    pub fn evaluate(&mut self, current: f64) -> AnomalyResult {
        self.count += 1;

        if self.count == 1 {
            self.mean = current;
            return AnomalyResult { score: 0.1, z_score: 0.0, is_anomaly: false };
        }

        // EWMA 更新
        self.mean = self.alpha * current + (1.0 - self.alpha) * self.mean;
        let delta = current - self.mean;
        self.m2 = self.alpha * delta * delta + (1.0 - self.alpha) * self.m2;

        let std_dev = self.m2.sqrt().max(1.0);
        let z_score = delta.abs() / std_dev;

        // z > 3 → 高异常 (score > 0.8)
        // z > 2 → 中等 (score 0.5-0.8)
        // z ≤ 2 → 正常 (score < 0.5)
        let score = if z_score > 4.0 {
            0.95
        } else if z_score > 3.0 {
            0.8 + (z_score - 3.0) * 0.15
        } else if z_score > 2.0 {
            0.5 + (z_score - 2.0) * 0.3
        } else {
            (z_score / 2.0) * 0.5
        };

        AnomalyResult {
            score: score.min(1.0).max(0.0),
            z_score,
            is_anomaly: z_score > 3.0,
        }
    }

    /// 获取全局单例
    pub fn global() -> std::sync::MutexGuard<'static, Self> {
        use std::sync::Mutex;
        use std::sync::OnceLock;
        static DETECTOR: OnceLock<Mutex<AnomalyDetector>> = OnceLock::new();
        DETECTOR.get_or_init(|| Mutex::new(AnomalyDetector::new(0.1)))
            .lock()
            .unwrap()
    }
}

impl From<AnomalyResult> for crate::types::trace::DriftDetection {
    fn from(a: AnomalyResult) -> Self {
        Self {
            prompt_drift: Some(a.is_anomaly),
            response_drift: Some(a.is_anomaly),
            embedding_drift: Some(a.z_score > 4.0),
        }
    }
}

// ==================== OTel Tracer（保持向后兼容）====================

pub struct OtelTracer;

impl OtelTracer {
    pub fn new(_service_name: &str) -> Self { Self }

    pub fn create_span(&self, name: &str, trace_id: &str) -> GenAiSpan {
        let mut span = GenAiSpan::new(trace_id);
        // 提取模型名（如果是 model-name 格式）
        if let Some(model_part) = name.split('/').last() {
            span.set_model(model_part);
        } else {
            span.set_model(name);
        }
        span
    }

    pub fn generate_monitoring(&self, span: &GenAiSpan) -> Monitoring {
        span.to_monitoring()
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genai_span_attributes() {
        let mut span = GenAiSpan::new("trace-123");
        span.set_model("openai/gpt-4o");
        span.set_temperature(0.7);
        span.set_usage(100, 50);

        assert_eq!(span.attributes.get(genai_attrs::SYSTEM), Some(&"openai".to_string()));
        assert_eq!(span.attributes.get(genai_attrs::REQUEST_MODEL), Some(&"openai/gpt-4o".to_string()));
        assert_eq!(span.attributes.get(genai_attrs::USAGE_INPUT_TOKENS), Some(&"100".to_string()));
        assert_eq!(span.attributes.get(genai_attrs::USAGE_OUTPUT_TOKENS), Some(&"50".to_string()));
    }

    #[test]
    fn test_anomaly_detector_normal() {
        let mut detector = AnomalyDetector::new(0.1);
        // 注入正常延迟数据
        for _ in 0..50 {
            detector.evaluate(500.0 + rand::random::<f64>() * 100.0);
        }
        let result = detector.evaluate(550.0);
        assert!(result.score < 0.5, "normal latency should have low anomaly score, got {}", result.score);
    }

    #[test]
    fn test_anomaly_detector_anomaly() {
        let mut detector = AnomalyDetector::new(0.1);
        for _ in 0..50 {
            detector.evaluate(500.0 + rand::random::<f64>() * 50.0);
        }
        let result = detector.evaluate(5000.0); // 10x normal
        assert!(result.is_anomaly, "10x latency should be flagged as anomaly");
        assert!(result.z_score > 3.0, "z-score should be >3 for extreme anomaly");
    }
}
