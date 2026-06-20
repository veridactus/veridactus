//! # Governance DSL Parser
//!
//! 解析 YAML 格式的治理策略 DSL.
//!
//! 支持的 DSL 结构（§5.8.1）：
//! ```yaml
//! version: "0.2.0"
//! intents:
//!   budget: "cost_effective"
//!   privacy: "pii_not_stored"
//! policies:
//!   - id: "production-budget"
//!     type: "budget"
//!     description: "Standard budget controls"
//!     rules:
//!       - condition: "risk_score > 0.7"
//!         action: "require_approval"
//!       - condition: "risk_score > 0.3"
//!         action: "degrade"
//!     defaults:
//!       strategy: "hard_stop"
//!       limit_usd: 0.05
//! ```

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[cfg(test)]
use serde_yaml;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceDsl {
    pub version: String,
    #[serde(default)]
    pub intents: Option<IntentDeclarations>,
    #[serde(default)]
    pub policies: Vec<PolicyDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDeclarations {
    #[serde(default)]
    pub budget: Option<String>,
    #[serde(default)]
    pub privacy: Option<String>,
    #[serde(default)]
    pub safety: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDefinition {
    pub id: String,
    #[serde(rename = "type")]
    pub policy_type: PolicyType,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
    #[serde(default)]
    pub defaults: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyType {
    Budget,
    ActivePrevention,
    ToolConstraint,
    Guardrails,
    Compliance,
    Reproducibility,
    InstructionHierarchy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PolicyRule {
    Allow,
    Block { message: Option<String> },
    Degrade { degrade_action: Option<String>, degrade_target: Option<String> },
    RequireApproval { message: Option<String> },
    Flag,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: String,
    pub value: serde_json::Value,
}

impl GovernanceDsl {
    pub fn parse(yaml_content: &str) -> Result<Self, DslParseError> {
        serde_yaml::from_str(yaml_content).map_err(|e| DslParseError {
            message: format!("YAML parsing error: {}", e),
            line: None,
            column: None,
        })
    }

    pub fn compute_source_hash(&self) -> String {
        let yaml_str = serde_yaml::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(yaml_str.as_bytes());
        format!("sha256:{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DslParseError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl std::fmt::Display for DslParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DSL Parse Error: {}", self.message)?;
        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, " at line {}, column {}", line, col)?;
        }
        Ok(())
    }
}

impl std::error::Error for DslParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DSL: &str = r#"
version: "0.2.0"
intents:
  budget: "cost_effective"
  privacy: "pii_not_stored"
policies:
  - id: "production-budget"
    type: budget
    description: "Standard budget controls for production workloads"
    rules:
      - action: require_approval
        message: "High-risk request requires human approval"
    defaults:
      strategy: "hard_stop"
      buffer_ratio: 0.001
      limit_usd: 0.05

  - id: "pii-prevention"
    type: active-prevention
    description: "Actively prevent PII generation via constrained decoding"
    rules:
      - action: block
        message: "PII generation blocked"
"#;

    #[test]
    fn test_parse_simple_dsl() {
        let dsl = GovernanceDsl::parse(SAMPLE_DSL).expect("Failed to parse DSL");
        assert_eq!(dsl.version, "0.2.0");
        assert!(dsl.intents.is_some());

        let intents = dsl.intents.unwrap();
        assert_eq!(intents.budget, Some("cost_effective".to_string()));
        assert_eq!(intents.privacy, Some("pii_not_stored".to_string()));

        assert_eq!(dsl.policies.len(), 2);
        assert_eq!(dsl.policies[0].id, "production-budget");
        assert_eq!(dsl.policies[1].id, "pii-prevention");
    }

    #[test]
    fn test_source_hash() {
        let dsl = GovernanceDsl::parse(SAMPLE_DSL).expect("Failed to parse DSL");
        let hash = dsl.compute_source_hash();
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71); // sha256: + 64 hex chars
    }
}