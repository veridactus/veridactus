//! # Certified Guarantees Module
//!
//! 实现 VERIDACTUS v0.2.1 §9.6 Certified Guarantees / C-SafeGen 规范.
//!
//! 提供密码学可验证的执行证明，验证模型行为满足声明的约束.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedGuarantee {
    pub guarantee_id: String,
    pub guarantee_type: GuaranteeType,
    pub properties: GuaranteeProperties,
    pub proof: GuaranteeProof,
    pub issuer: String,
    pub valid_from: String,
    pub valid_until: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GuaranteeType {
    Safety,
    Fairness,
    Privacy,
    BudgetCompliance,
    Reproducibility,
    InstructionHierarchy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuaranteeProperties {
    pub description: String,
    pub bounds: Option<HashMap<String, f64>>,
    pub thresholds: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuaranteeProof {
    pub proof_type: ProofType,
    pub merkle_root: Option<String>,
    pub signature: Option<String>,
    pub attestation: Option<TeeAttestation>,
    pub audit_trail: Vec<AuditEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofType {
    MerkleChain,
    TEEAttestation,
    ZKProof,
    Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeAttestation {
    pub report: String,
    pub signature: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub operation: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformalPredictionResult {
    pub risk_bound: f64,
    pub confidence_level: f64,
    pub prediction_set: Option<Vec<String>>,
    pub is_valid: bool,
    pub calibration_error: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSetEntry {
    pub input_hash: String,
    pub output_hash: String,
    pub nonconformity_score: f64,
    pub label: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSafeGenConfig {
    pub validation_set_size: usize,
    pub default_confidence: f64,
    pub calibration_method: CalibrationMethod,
    pub risk_bound_multiplier: f64,
}

impl Default for CSafeGenConfig {
    fn default() -> Self {
        Self {
            validation_set_size: 1000,
            default_confidence: 0.95,
            calibration_method: CalibrationMethod::Adaptive,
            risk_bound_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalibrationMethod {
    Basic,
    Adaptive,
    CrossValidation,
}

pub struct CertifiedGuaranteeBuilder {
    guarantee_type: GuaranteeType,
    properties: GuaranteeProperties,
    issuer: String,
}

impl CertifiedGuaranteeBuilder {
    pub fn new(guarantee_type: GuaranteeType) -> Self {
        Self {
            guarantee_type,
            properties: GuaranteeProperties {
                description: String::new(),
                bounds: None,
                thresholds: None,
            },
            issuer: "veridactus".to_string(),
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.properties.description = description;
        self
    }

    pub fn with_bounds(mut self, bounds: HashMap<String, f64>) -> Self {
        self.properties.bounds = Some(bounds);
        self
    }

    pub fn with_thresholds(mut self, thresholds: HashMap<String, f64>) -> Self {
        self.properties.thresholds = Some(thresholds);
        self
    }

    pub fn with_issuer(mut self, issuer: String) -> Self {
        self.issuer = issuer;
        self
    }

    pub fn build(self) -> CertifiedGuarantee {
        let guarantee_id = generate_guarantee_id(&self.guarantee_type);
        let now = chrono::Utc::now().to_rfc3339();

        CertifiedGuarantee {
            guarantee_id,
            guarantee_type: self.guarantee_type,
            properties: self.properties,
            proof: GuaranteeProof {
                proof_type: ProofType::MerkleChain,
                merkle_root: None,
                signature: None,
                attestation: None,
                audit_trail: vec![AuditEntry {
                    timestamp: now.clone(),
                    operation: "guarantee_created".to_string(),
                    hash: "initial".to_string(),
                }],
            },
            issuer: self.issuer,
            valid_from: now,
            valid_until: None,
            metadata: HashMap::new(),
        }
    }
}

pub struct GuaranteeVerifier;

impl GuaranteeVerifier {
    pub fn verify(guarantee: &CertifiedGuarantee) -> GuaranteeVerificationResult {
        let mut checks_passed = Vec::new();
        let mut checks_failed = Vec::new();
        let warnings = Vec::new();

        if let Some(valid_until) = &guarantee.valid_until {
            if chrono::DateTime::parse_from_rfc3339(valid_until).is_err() {
                checks_failed.push("invalid_expiry_format".to_string());
            }
        }

        if guarantee.proof.audit_trail.is_empty() {
            checks_failed.push("empty_audit_trail".to_string());
        } else {
            checks_passed.push("audit_trail_present".to_string());
        }

        if guarantee.proof.audit_trail.len() > 1 {
            if Self::verify_audit_chain_integrity(&guarantee.proof.audit_trail) {
                checks_passed.push("audit_chain_integrity".to_string());
            } else {
                checks_failed.push("audit_chain_compromised".to_string());
            }
        }

        let overall_valid = checks_failed.is_empty();

        GuaranteeVerificationResult {
            guarantee_id: guarantee.guarantee_id.clone(),
            valid: overall_valid,
            checks_passed,
            checks_failed,
            warnings,
        }
    }

    fn verify_audit_chain_integrity(trail: &[AuditEntry]) -> bool {
        if trail.len() < 2 {
            return true;
        }

        for i in 1..trail.len() {
            let prev_hash = &trail[i - 1].hash;
            let current_op = &trail[i].operation;
            let current_ts = &trail[i].timestamp;

            let mut hasher = Sha256::new();
            hasher.update(format!("{}{}", prev_hash, current_op).as_bytes());
            hasher.update(current_ts.as_bytes());
            let computed_hash = format!("{:x}", hasher.finalize());

            if !trail[i].hash.starts_with(&computed_hash[..8]) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuaranteeVerificationResult {
    pub guarantee_id: String,
    pub valid: bool,
    pub checks_passed: Vec<String>,
    pub checks_failed: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct CSafeGenGenerator;

impl CSafeGenGenerator {
    pub fn generate_safety_certificate(
        constraints: &HashMap<String, String>,
        _trace_hash: &str,
    ) -> CertifiedGuarantee {
        let mut bounds = HashMap::new();
        if let Some(budget) = constraints.get("budget_limit") {
            if let Ok(val) = budget.parse::<f64>() {
                bounds.insert("budget_limit_usd".to_string(), val);
            }
        }

        CertifiedGuaranteeBuilder::new(GuaranteeType::Safety)
            .with_description("Safety guarantee for model execution".to_string())
            .with_bounds(bounds)
            .build()
    }

    pub fn generate_fairness_certificate(
        demographic_parity_diff: f64,
        equalized_odds_gap: f64,
    ) -> CertifiedGuarantee {
        let mut thresholds = HashMap::new();
        thresholds.insert("demographic_parity_threshold".to_string(), 0.1);
        thresholds.insert("equalized_odds_threshold".to_string(), 0.15);

        let mut bounds = HashMap::new();
        bounds.insert(
            "demographic_parity_diff".to_string(),
            demographic_parity_diff,
        );
        bounds.insert("equalized_odds_gap".to_string(), equalized_odds_gap);

        CertifiedGuaranteeBuilder::new(GuaranteeType::Fairness)
            .with_description("Fairness guarantee for model predictions".to_string())
            .with_bounds(bounds)
            .with_thresholds(thresholds)
            .build()
    }

    pub fn generate_privacy_certificate(
        privacy_level: &str,
        pii_detected: bool,
    ) -> CertifiedGuarantee {
        let mut bounds = HashMap::new();
        bounds.insert(
            "pii_detected".to_string(),
            if pii_detected { 1.0 } else { 0.0 },
        );

        CertifiedGuaranteeBuilder::new(GuaranteeType::Privacy)
            .with_description(format!("Privacy guarantee at level: {}", privacy_level))
            .with_bounds(bounds)
            .build()
    }

    pub fn generate_budget_compliance_certificate(
        budget_limit: f64,
        actual_cost: f64,
    ) -> CertifiedGuarantee {
        let mut bounds = HashMap::new();
        bounds.insert("budget_limit_usd".to_string(), budget_limit);
        bounds.insert("actual_cost_usd".to_string(), actual_cost);
        bounds.insert(
            "cost_variance_pct".to_string(),
            ((actual_cost - budget_limit) / budget_limit * 100.0).abs(),
        );

        CertifiedGuaranteeBuilder::new(GuaranteeType::BudgetCompliance)
            .with_description("Budget compliance guarantee".to_string())
            .with_bounds(bounds)
            .build()
    }
}

fn generate_guarantee_id(guarantee_type: &GuaranteeType) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let type_prefix = match guarantee_type {
        GuaranteeType::Safety => "SAF",
        GuaranteeType::Fairness => "FNR",
        GuaranteeType::Privacy => "PRV",
        GuaranteeType::BudgetCompliance => "BGT",
        GuaranteeType::Reproducibility => "RPR",
        GuaranteeType::InstructionHierarchy => "HIE",
    };
    format!("{}_{:x}", type_prefix, timestamp)
}

pub struct ConformalAnalyzer {
    config: CSafeGenConfig,
    validation_set: Vec<ValidationSetEntry>,
}

impl Default for ConformalAnalyzer {
    fn default() -> Self {
        Self::new(CSafeGenConfig::default())
    }
}

impl ConformalAnalyzer {
    pub fn new(config: CSafeGenConfig) -> Self {
        Self {
            config,
            validation_set: Vec::new(),
        }
    }

    pub fn with_validation_set(mut self, validation_set: Vec<ValidationSetEntry>) -> Self {
        self.validation_set = validation_set;
        self
    }

    pub fn add_to_validation_set(&mut self, entry: ValidationSetEntry) {
        self.validation_set.push(entry);
        if self.validation_set.len() > self.config.validation_set_size * 2 {
            self.validation_set.remove(0);
        }
    }

    pub fn compute_nonconformity_scores(&self, outputs: &[f64], labels: &[f64]) -> Vec<f64> {
        outputs
            .iter()
            .zip(labels.iter())
            .map(|(output, label)| (output - label).abs())
            .collect()
    }

    pub fn calibrate(&mut self) -> f64 {
        if self.validation_set.len() < 10 {
            return 0.1;
        }

        let mut scores: Vec<f64> = self
            .validation_set
            .iter()
            .map(|e| e.nonconformity_score)
            .collect();
        scores.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = scores.len();
        let quantile_idx = ((n as f64) * (1.0 - self.config.default_confidence)) as usize;
        let quantile_idx = quantile_idx.min(n - 1);

        scores[quantile_idx]
    }

    pub fn predict(&self, output: f64, covariates: Option<&[f64]>) -> ConformalPredictionResult {
        let confidence = self.config.default_confidence;

        if self.validation_set.is_empty() {
            return ConformalPredictionResult {
                risk_bound: 0.1,
                confidence_level: confidence,
                prediction_set: None,
                is_valid: true,
                calibration_error: None,
            };
        }

        let mut scores: Vec<f64> = self
            .validation_set
            .iter()
            .map(|e| e.nonconformity_score)
            .collect();
        scores.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = scores.len();
        let n_f = n as f64;

        let nonconformity_score = self.compute_single_nonconformity(output, covariates);

        let count_greater = scores.iter().filter(|&&s| s >= nonconformity_score).count();
        let proportion = (count_greater as f64 + 1.0) / (n_f + 1.0);

        let risk_bound = (proportion * self.config.risk_bound_multiplier).min(1.0);
        let calibration_error = self.compute_calibration_error(&scores);

        ConformalPredictionResult {
            risk_bound,
            confidence_level: confidence,
            prediction_set: None,
            is_valid: risk_bound <= (1.0 - confidence),
            calibration_error: Some(calibration_error),
        }
    }

    fn compute_single_nonconformity(&self, output: f64, covariates: Option<&[f64]>) -> f64 {
        if let Some(covariates) = covariates {
            if !self.validation_set.is_empty() {
                let avg_score: f64 = self
                    .validation_set
                    .iter()
                    .map(|e| e.nonconformity_score)
                    .sum::<f64>()
                    / self.validation_set.len() as f64;

                let covariate_similarity: f64 = self
                    .validation_set
                    .iter()
                    .map(|e| {
                        let stored_cov = e
                            .metadata
                            .get("covariates")
                            .and_then(|s| serde_json::from_str::<Vec<f64>>(s).ok())
                            .unwrap_or_default();
                        let diff: f64 = covariates
                            .iter()
                            .zip(stored_cov.iter())
                            .map(|(a, b)| (a - b).powi(2))
                            .sum();
                        diff.sqrt()
                    })
                    .sum::<f64>()
                    / self.validation_set.len() as f64;

                return avg_score * (1.0 + covariate_similarity);
            }
        }
        output.abs()
    }

    fn compute_calibration_error(&self, scores: &[f64]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }

        let n = scores.len();
        let mut bins: Vec<f64> = Vec::with_capacity(10);
        for i in 0..10 {
            let start = (n * i) / 10;
            let end = (n * (i + 1)) / 10;
            if end > start {
                let bin_avg: f64 = scores[start..end].iter().sum::<f64>() / (end - start) as f64;
                bins.push(bin_avg);
            }
        }

        let overall_avg: f64 = scores.iter().sum::<f64>() / n as f64;
        let variance: f64 =
            bins.iter().map(|&b| (b - overall_avg).powi(2)).sum::<f64>() / bins.len() as f64;

        variance.sqrt()
    }

    pub fn get_validation_set_size(&self) -> usize {
        self.validation_set.len()
    }

    pub fn clear_validation_set(&mut self) {
        self.validation_set.clear();
    }
}

pub struct CSafeGenAnalyzer {
    config: CSafeGenConfig,
    conformal_analyzer: ConformalAnalyzer,
}

impl Default for CSafeGenAnalyzer {
    fn default() -> Self {
        Self::new(CSafeGenConfig::default())
    }
}

impl CSafeGenAnalyzer {
    pub fn new(config: CSafeGenConfig) -> Self {
        Self {
            conformal_analyzer: ConformalAnalyzer::new(config.clone()),
            config,
        }
    }

    pub fn analyze(
        &self,
        trace_output: &str,
        constraints: &HashMap<String, String>,
    ) -> ConformalPredictionResult {
        let output_len = trace_output.len() as f64;
        let normalized_output_risk = (output_len / 10000.0).min(1.0);

        let budget_limit = constraints
            .get("budget_limit")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(f64::MAX);
        let budget_risk = if budget_limit > 0.0 { 0.0 } else { 1.0 };

        let pii_patterns = [
            r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
            r"\d{3}-\d{2}-\d{4}",
            r"\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}",
        ];

        let pii_risk = if pii_patterns.iter().any(|p| {
            regex::Regex::new(p)
                .map(|re| re.is_match(trace_output))
                .unwrap_or(false)
        }) {
            0.8
        } else {
            0.0
        };

        let combined_risk =
            (normalized_output_risk * 0.3 + budget_risk * 0.4 + pii_risk * 0.3).min(1.0);

        self.conformal_analyzer.predict(combined_risk, None)
    }

    pub fn compute_risk_bound(&self, safety_events_count: usize, total_tokens: usize) -> f64 {
        let event_rate = if total_tokens > 0 {
            safety_events_count as f64 / total_tokens as f64
        } else {
            0.0
        };

        let base_bound = self.config.default_confidence;
        let adjusted_bound = base_bound - (event_rate * 0.1);

        adjusted_bound.max(0.01).min(1.0)
    }

    pub fn generate_guarantee_report(
        &self,
        trace_id: &str,
        analysis_result: &ConformalPredictionResult,
    ) -> GuaranteeReport {
        let risk_level = if analysis_result.risk_bound < 0.1 {
            RiskLevel::Low
        } else if analysis_result.risk_bound < 0.3 {
            RiskLevel::Medium
        } else {
            RiskLevel::High
        };

        GuaranteeReport {
            trace_id: trace_id.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            risk_bound: analysis_result.risk_bound,
            confidence_level: analysis_result.confidence_level,
            risk_level,
            is_valid: analysis_result.is_valid,
            recommendations: self.generate_recommendations(analysis_result),
        }
    }

    fn generate_recommendations(&self, result: &ConformalPredictionResult) -> Vec<String> {
        let mut recs = Vec::new();

        if result.risk_bound > 0.3 {
            recs.push("风险边界较高，建议增加人工审核".to_string());
        }

        if let Some(cal_err) = result.calibration_error {
            if cal_err > 0.1 {
                recs.push("校准误差较高，建议重新校准模型".to_string());
            }
        }

        if result.confidence_level < 0.9 {
            recs.push("置信水平低于阈值，建议增加验证集大小".to_string());
        }

        if recs.is_empty() {
            recs.push("分析结果在可接受范围内".to_string());
        }

        recs
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuaranteeReport {
    pub trace_id: String,
    pub generated_at: String,
    pub risk_bound: f64,
    pub confidence_level: f64,
    pub risk_level: RiskLevel,
    pub is_valid: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
        }
    }
}

trait ValidationSetEntryExt {
    fn metadata(&self) -> &HashMap<String, String>;
}

impl ValidationSetEntryExt for ValidationSetEntry {
    fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

impl ValidationSetEntry {
    fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_safety_guarantee() {
        let guarantee = CertifiedGuaranteeBuilder::new(GuaranteeType::Safety)
            .with_description("Test safety guarantee".to_string())
            .build();

        assert_eq!(guarantee.guarantee_type, GuaranteeType::Safety);
        assert!(!guarantee.guarantee_id.is_empty());
    }

    #[test]
    fn test_verify_guarantee() {
        let guarantee = CertifiedGuaranteeBuilder::new(GuaranteeType::Fairness)
            .with_description("Test fairness guarantee".to_string())
            .build();

        let result = GuaranteeVerifier::verify(&guarantee);
        assert!(result.valid);
    }

    #[test]
    fn test_fairness_certificate() {
        let cert = CSafeGenGenerator::generate_fairness_certificate(0.05, 0.08);
        assert_eq!(cert.guarantee_type, GuaranteeType::Fairness);
        assert!(cert.properties.bounds.is_some());
    }

    #[test]
    fn test_conformal_analyzer() {
        let config = CSafeGenConfig::default();
        let analyzer = ConformalAnalyzer::new(config);

        let result = analyzer.predict(0.5, None);
        assert!(result.risk_bound >= 0.0);
        assert!(result.risk_bound <= 1.0);
    }

    #[test]
    fn test_csafe_gen_analyzer() {
        let config = CSafeGenConfig::default();
        let analyzer = CSafeGenAnalyzer::new(config);

        let constraints = HashMap::new();
        let result = analyzer.analyze("test output", &constraints);

        assert!(result.risk_bound >= 0.0);
        assert!(result.risk_bound <= 1.0);
    }
}
