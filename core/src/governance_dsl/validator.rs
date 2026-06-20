//! # Governance DSL Validator
//!
//! 验证 DSL 策略的有效性和完整性.

use super::parser::{GovernanceDsl, PolicyType};

#[derive(Debug, Clone)]
pub struct DslValidator {
    supported_versions: Vec<String>,
}

impl Default for DslValidator {
    fn default() -> Self {
        Self {
            supported_versions: vec!["0.2.0".to_string(), "0.2.1".to_string()],
        }
    }
}

impl DslValidator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn validate(&self, dsl: &GovernanceDsl) -> Result<(), DslValidationError> {
        let mut errors = Vec::new();

        if !self.supported_versions.contains(&dsl.version) {
            errors.push(format!(
                "Unsupported DSL version: {}. Supported: {:?}",
                dsl.version, self.supported_versions
            ));
        }

        for policy in &dsl.policies {
            if let Some(err) = self.validate_policy(policy) {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(DslValidationError { errors })
        }
    }

    fn validate_policy(&self, policy: &super::parser::PolicyDefinition) -> Option<String> {
        if policy.id.is_empty() {
            return Some("Policy id cannot be empty".to_string());
        }

        if policy.id.contains(' ') {
            return Some(format!("Policy id '{}' cannot contain spaces", policy.id));
        }

        match policy.policy_type {
            PolicyType::Budget => self.validate_budget_policy(policy),
            PolicyType::ActivePrevention => self.validate_active_prevention_policy(policy),
            _ => None,
        }
    }

    fn validate_budget_policy(&self, policy: &super::parser::PolicyDefinition) -> Option<String> {
        if let Some(limit) = policy.defaults.get("limit_usd") {
            if let Some(limit_f64) = limit.as_f64() {
                if limit_f64 < 0.0 {
                    return Some(format!(
                        "Budget limit must be non-negative in policy '{}'",
                        policy.id
                    ));
                }
                if limit_f64 > 1000.0 {
                    return Some(format!(
                        "Budget limit exceeds maximum (1000 USD) in policy '{}'",
                        policy.id
                    ));
                }
            } else {
                return Some(format!(
                    "Invalid budget limit type in policy '{}'",
                    policy.id
                ));
            }
        }
        None
    }

    fn validate_active_prevention_policy(&self, policy: &super::parser::PolicyDefinition) -> Option<String> {
        if policy.rules.is_empty() {
            return Some(format!(
                "Active prevention policy '{}' must have at least one rule",
                policy.id
            ));
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct DslValidationError {
    pub errors: Vec<String>,
}

impl std::fmt::Display for DslValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DSL Validation Errors:\n")?;
        for (i, err) in self.errors.iter().enumerate() {
            write!(f, "  {}. {}", i + 1, err)?;
        }
        Ok(())
    }
}

impl std::error::Error for DslValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_dsl() {
        let yaml = r#"
version: "0.2.0"
policies:
  - id: "test-policy"
    type: budget
    defaults:
      limit_usd: 0.05
"#;
        let dsl = GovernanceDsl::parse(yaml).expect("Failed to parse");
        let validator = DslValidator::new();
        assert!(validator.validate(&dsl).is_ok());
    }

    #[test]
    fn test_validate_invalid_version() {
        let yaml = r#"
version: "99.99"
policies: []
"#;
        let dsl = GovernanceDsl::parse(yaml).expect("Failed to parse");
        let validator = DslValidator::new();
        assert!(validator.validate(&dsl).is_err());
    }
}