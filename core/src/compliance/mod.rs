//! # Compliance Mapping
//!
//! 严格遵循 VERIDACTUS v0.2.1 §7.5 Compliance Mapping。
//! 实现自动将 trace 字段映射到监管条例条款。

pub mod hipaa_pci;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceMapping {
    pub regulation: String,
    pub article: String,
    pub article_title: Option<String>,
    pub trace_fields: Vec<String>,
    pub verification_method: VerificationMethod,
    pub required: bool,
    pub evidence_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    Automated,
    ManualReview,
    Sampled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_id: String,
    pub timestamp: String,
    pub trace_id: String,
    pub mappings: Vec<ComplianceMappingResult>,
    pub overall_compliant: bool,
    pub violations: Vec<ComplianceViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceMappingResult {
    pub regulation: String,
    pub article: String,
    pub compliant: bool,
    pub evidence_hash: Option<String>,
    pub verified_at: String,
    pub verification_method: VerificationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub regulation: String,
    pub article: String,
    pub violation_type: ViolationType,
    pub severity: ViolationSeverity,
    pub description: String,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    MissingField,
    InvalidValue,
    InsufficientEvidence,
    TimingViolation,
    AccessControlViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ViolationSeverity {
    Critical,
    High,
    Medium,
    Low,
}

pub struct ComplianceMapper {
    mappings: Vec<ComplianceMapping>,
}

impl ComplianceMapper {
    pub fn new() -> Self {
        Self {
            mappings: Self::default_mappings(),
        }
    }

    fn default_mappings() -> Vec<ComplianceMapping> {
        vec![
            ComplianceMapping {
                regulation: "EU_AI_ACT_2025".to_string(),
                article: "Article 53(1)(a)".to_string(),
                article_title: Some("Transparency and information to users".to_string()),
                trace_fields: vec![
                    "output.response".to_string(),
                    "constraints_applied.privacy_level".to_string(),
                    "proof_chain".to_string(),
                ],
                verification_method: VerificationMethod::Automated,
                required: true,
                evidence_location: None,
            },
            ComplianceMapping {
                regulation: "EU_AI_ACT_2025".to_string(),
                article: "Article 53(1)(b)".to_string(),
                article_title: Some("Human oversight".to_string()),
                trace_fields: vec![
                    "constraints_applied.guardrails_active".to_string(),
                    "observations.human_in_the_loop".to_string(),
                ],
                verification_method: VerificationMethod::Sampled,
                required: true,
                evidence_location: None,
            },
            ComplianceMapping {
                regulation: "EU_AI_ACT_2025".to_string(),
                article: "Article 54".to_string(),
                article_title: Some("Risk management system".to_string()),
                trace_fields: vec![
                    "observations.risk_score".to_string(),
                    "constraints_applied.policy_evaluation".to_string(),
                ],
                verification_method: VerificationMethod::Automated,
                required: true,
                evidence_location: None,
            },
            ComplianceMapping {
                regulation: "GDPR".to_string(),
                article: "Article 5(1)(a)".to_string(),
                article_title: Some("Lawfulness, fairness and transparency".to_string()),
                trace_fields: vec![
                    "constraints_applied.privacy_level".to_string(),
                    "observations.fairness_check".to_string(),
                ],
                verification_method: VerificationMethod::Automated,
                required: true,
                evidence_location: None,
            },
            ComplianceMapping {
                regulation: "GDPR".to_string(),
                article: "Article 17".to_string(),
                article_title: Some("Right to erasure ('right to be forgotten')".to_string()),
                trace_fields: vec!["metadata.ttl_expire_at".to_string()],
                verification_method: VerificationMethod::ManualReview,
                required: false,
                evidence_location: None,
            },
            ComplianceMapping {
                regulation: "NIST_AI_RM".to_string(),
                article: "4.1".to_string(),
                article_title: Some("Risk Assessment".to_string()),
                trace_fields: vec![
                    "observations.risk_score".to_string(),
                    "constraints_applied.policy_evaluation".to_string(),
                ],
                verification_method: VerificationMethod::Automated,
                required: false,
                evidence_location: None,
            },
        ]
    }

    pub fn map_trace(&self, trace_data: &HashMap<String, serde_json::Value>) -> ComplianceReport {
        let mut results = Vec::new();
        let mut violations = Vec::new();

        for mapping in &self.mappings {
            let mut compliant = true;
            let mut missing_fields = Vec::new();

            for field in &mapping.trace_fields {
                if !trace_data.contains_key(field) {
                    compliant = false;
                    missing_fields.push(field.clone());
                }
            }

            if !compliant {
                violations.push(ComplianceViolation {
                    regulation: mapping.regulation.clone(),
                    article: mapping.article.clone(),
                    violation_type: ViolationType::MissingField,
                    severity: if mapping.required {
                        ViolationSeverity::High
                    } else {
                        ViolationSeverity::Low
                    },
                    description: format!("Missing required fields: {}", missing_fields.join(", ")),
                    suggested_action: "Ensure all required fields are present in the trace"
                        .to_string(),
                });
            }

            results.push(ComplianceMappingResult {
                regulation: mapping.regulation.clone(),
                article: mapping.article.clone(),
                compliant,
                evidence_hash: None,
                verified_at: chrono::Utc::now().to_rfc3339(),
                verification_method: mapping.verification_method.clone(),
            });
        }

        ComplianceReport {
            report_id: format!("compliance_{}", uuid::Uuid::new_v4()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_data
                .get("trace_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            mappings: results,
            overall_compliant: violations.is_empty(),
            violations,
        }
    }

    pub fn get_mappings_for_regulation(&self, regulation: &str) -> Vec<&ComplianceMapping> {
        self.mappings
            .iter()
            .filter(|m| m.regulation == regulation)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_compliance_mapping() {
        let mapper = ComplianceMapper::new();

        let mut trace_data = HashMap::new();
        trace_data.insert("trace_id".to_string(), json!("test-trace-123"));
        trace_data.insert("output.response".to_string(), json!("Hello"));
        trace_data.insert(
            "constraints_applied.privacy_level".to_string(),
            json!("masked"),
        );
        trace_data.insert("proof_chain".to_string(), json!([]));

        let report = mapper.map_trace(&trace_data);

        assert!(!report.overall_compliant);
        assert!(!report.violations.is_empty());
    }

    #[test]
    fn test_get_mappings_for_regulation() {
        let mapper = ComplianceMapper::new();
        let mappings = mapper.get_mappings_for_regulation("GDPR");

        assert_eq!(mappings.len(), 2);
        assert!(mappings.iter().any(|m| m.article == "Article 5(1)(a)"));
    }
}
