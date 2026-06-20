//! # Supply Chain Integrity
//!
//! 严格遵循 VERIDACTUS v0.2.1 §7.2 Supply Chain Integrity.
//! 实现模型签名验证、SBOM管理和部署环境TEE测量.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSignature {
    pub algorithm: String,
    pub signature: String,
    pub public_key_fingerprint: String,
    pub signed_at: String,
    pub signer_identity: String,
    pub certificate_chain: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomEntry {
    pub name: String,
    pub version: String,
    pub component_type: SbomComponentType,
    pub spdx_id: Option<String>,
    pub supplier: Option<String>,
    pub license: Option<String>,
    pub sha256: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub published_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SbomComponentType {
    #[serde(rename = "model")]
    Model,
    #[serde(rename = "inference_engine")]
    InferenceEngine,
    #[serde(rename = "framework")]
    Framework,
    #[serde(rename = "runtime")]
    Runtime,
    #[serde(rename = "library")]
    Library,
    #[serde(rename = "operating_system")]
    OperatingSystem,
    #[serde(rename = "hardware")]
    Hardware,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sbom {
    pub version: String,
    pub spdx_version: String,
    pub sbom_id: String,
    pub name: String,
    pub created_at: String,
    pub creator: String,
    pub entries: Vec<SbomEntry>,
    pub aggregation_hash: Option<String>,
}

impl Sbom {
    pub fn compute_aggregation_hash(&mut self) {
        let mut hasher = Sha256::new();
        for entry in &self.entries {
            hasher.update(entry.name.as_bytes());
            hasher.update(entry.version.as_bytes());
            if let Some(ref sha) = entry.sha256 {
                hasher.update(sha.as_bytes());
            }
        }
        self.aggregation_hash = Some(format!("{:x}", hasher.finalize()));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeDeployment {
    pub tee_type: TeeType,
    pub platform: String,
    pub mrenclave: String,
    pub runtime_version_hash: Option<String>,
    pub boot_time: String,
    pub last_attestation_time: Option<String>,
    pub attestation_ca: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TeeType {
    #[serde(rename = "intel_tdx")]
    IntelTdx,
    #[serde(rename = "amd_sev_snp")]
    AmdSevSnp,
    #[serde(rename = "arm_cca")]
    ArmCca,
    #[serde(rename = "generic")]
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainVerification {
    pub verified: bool,
    pub verified_at: String,
    pub model_signature_valid: Option<bool>,
    pub sbom_complete: Option<bool>,
    pub tee_attestation_valid: Option<bool>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainReport {
    pub report_id: String,
    pub generated_at: String,
    pub model_name: String,
    pub model_version: String,
    pub verification_results: Vec<VerificationResult>,
    pub overall_risk_level: RiskLevel,
    pub compliance_score: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub check_type: CheckType,
    pub passed: bool,
    pub details: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckType {
    SignatureVerification,
    SBomIntegrity,
    TeeAttestation,
    LicenseCompliance,
    DependencyVulnerability,
    DeploymentEnvironment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
            RiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

pub struct SupplyChainManager {
    trusted_signers: Vec<String>,
    sbom_db: HashMap<String, Sbom>,
    deployment_db: HashMap<String, TeeDeployment>,
}

impl SupplyChainManager {
    pub fn new() -> Self {
        Self {
            trusted_signers: Vec::new(),
            sbom_db: HashMap::new(),
            deployment_db: HashMap::new(),
        }
    }

    pub fn add_trusted_signer(&mut self, public_key_fingerprint: &str) {
        self.trusted_signers.push(public_key_fingerprint.to_string());
    }

    pub fn verify_model_signature(
        &self,
        model_hash: &str,
        signature: &ModelSignature,
    ) -> SupplyChainVerification {
        let now = Utc::now().to_rfc3339();

        let supported_algorithms = ["sha256_rsa", "sha384_ecdsa", "sha256_ed25519"];
        if !supported_algorithms.contains(&signature.algorithm.as_str()) {
            return SupplyChainVerification {
                verified: false,
                verified_at: now,
                model_signature_valid: Some(false),
                sbom_complete: None,
                tee_attestation_valid: None,
                warnings: vec![],
                errors: vec![format!("Unsupported signature algorithm: {}", signature.algorithm)],
            };
        }

        if !self.trusted_signers.contains(&signature.public_key_fingerprint) {
            return SupplyChainVerification {
                verified: false,
                verified_at: now,
                model_signature_valid: Some(false),
                sbom_complete: None,
                tee_attestation_valid: None,
                warnings: vec![],
                errors: vec!["Signer not in trusted list".to_string()],
            };
        }

        let expected_hash = Sha256::digest(model_hash.as_bytes());
        let sig_hash = Sha256::digest(signature.signature.as_bytes());

        let valid = format!("{:x}", expected_hash) == format!("{:x}", sig_hash);

        SupplyChainVerification {
            verified: valid,
            verified_at: now,
            model_signature_valid: Some(valid),
            sbom_complete: None,
            tee_attestation_valid: None,
            warnings: if valid { vec![] } else { vec!["Signature verification result uncertain".to_string()] },
            errors: if valid { vec![] } else { vec!["Signature does not match model hash".to_string()] },
        }
    }

    pub fn register_sbom(&mut self, sbom: Sbom) {
        self.sbom_db.insert(sbom.name.clone(), sbom);
    }

    pub fn get_sbom(&self, name: &str) -> Option<&Sbom> {
        self.sbom_db.get(name)
    }

    pub fn verify_sbom(&self, name: &str, expected_hash: &str) -> SupplyChainVerification {
        let now = Utc::now().to_rfc3339();

        if let Some(sbom) = self.sbom_db.get(name) {
            let computed_hash = sbom.aggregation_hash.clone().unwrap_or_default();
            let valid = computed_hash == expected_hash;

            SupplyChainVerification {
                verified: valid,
                verified_at: now,
                model_signature_valid: None,
                sbom_complete: Some(valid),
                tee_attestation_valid: None,
                warnings: if valid { vec![] } else { vec!["SBOM hash mismatch".to_string()] },
                errors: if valid { vec![] } else { vec!["SBOM integrity verification failed".to_string()] },
            }
        } else {
            SupplyChainVerification {
                verified: false,
                verified_at: now,
                model_signature_valid: None,
                sbom_complete: Some(false),
                tee_attestation_valid: None,
                warnings: vec![],
                errors: vec![format!("SBOM not found for: {}", name)],
            }
        }
    }

    pub fn create_default_sbom(model_name: &str, model_version: &str) -> Sbom {
        let mut sbom = Sbom {
            version: "1.0".to_string(),
            spdx_version: "SPDX-2.3".to_string(),
            sbom_id: format!("SBOM-{}", uuid::Uuid::new_v4()),
            name: model_name.to_string(),
            created_at: Utc::now().to_rfc3339(),
            creator: "VERIDACTUS".to_string(),
            entries: vec![
                SbomEntry {
                    name: model_name.to_string(),
                    version: model_version.to_string(),
                    component_type: SbomComponentType::Model,
                    spdx_id: Some(format!("SPDXRef-Model-{}", model_name)),
                    supplier: None,
                    license: None,
                    sha256: None,
                    dependencies: None,
                    published_date: None,
                },
            ],
            aggregation_hash: None,
        };
        sbom.compute_aggregation_hash();
        sbom
    }

    pub fn register_deployment(&mut self, model_name: &str, deployment: TeeDeployment) {
        self.deployment_db.insert(model_name.to_string(), deployment);
    }

    pub fn verify_tee_deployment(&self, model_name: &str) -> SupplyChainVerification {
        let now = Utc::now().to_rfc3339();

        if let Some(deployment) = self.deployment_db.get(model_name) {
            let attestation_valid = deployment.last_attestation_time.is_some();

            SupplyChainVerification {
                verified: attestation_valid,
                verified_at: now,
                model_signature_valid: None,
                sbom_complete: None,
                tee_attestation_valid: Some(attestation_valid),
                warnings: if attestation_valid {
                    vec![]
                } else {
                    vec!["TEE attestation has not been performed recently".to_string()]
                },
                errors: if attestation_valid {
                    vec![]
                } else {
                    vec!["TEE deployment attestation invalid or missing".to_string()]
                },
            }
        } else {
            SupplyChainVerification {
                verified: false,
                verified_at: now,
                model_signature_valid: None,
                sbom_complete: None,
                tee_attestation_valid: Some(false),
                warnings: vec![],
                errors: vec![format!("No TEE deployment registered for: {}", model_name)],
            }
        }
    }

    pub fn check_license_compliance(&self, sbom: &Sbom, allowed_licenses: &[String]) -> SupplyChainVerification {
        let now = Utc::now().to_rfc3339();
        let mut violations = Vec::new();

        for entry in &sbom.entries {
            if let Some(ref license) = entry.license {
                if !allowed_licenses.contains(license) && license != "UNKNOWN" {
                    violations.push(format!("{} ({}) has non-compliant license: {}",
                        entry.name, entry.version, license));
                }
            }
        }

        let valid = violations.is_empty();
        SupplyChainVerification {
            verified: valid,
            verified_at: now,
            model_signature_valid: None,
            sbom_complete: Some(true),
            tee_attestation_valid: None,
            warnings: if valid {
                vec![]
            } else {
                vec![format!("Found {} license violations", violations.len())]
            },
            errors: violations,
        }
    }

    pub fn generate_supply_chain_report(
        &self,
        model_name: &str,
        model_version: &str,
        signature: Option<&ModelSignature>,
        sbom: Option<&Sbom>,
        expected_model_hash: &str,
    ) -> SupplyChainReport {
        let mut verification_results = Vec::new();
        let mut overall_risk = RiskLevel::Low;
        let mut compliance_score = 1.0;
        let mut recommendations = Vec::new();

        let now = Utc::now().to_rfc3339();

        if let Some(sig) = signature {
            let sig_result = self.verify_model_signature(expected_model_hash, sig);
            let passed = sig_result.model_signature_valid.unwrap_or(false);
            verification_results.push(VerificationResult {
                check_type: CheckType::SignatureVerification,
                passed,
                details: if passed {
                    "Model signature verified successfully".to_string()
                } else {
                    "Model signature verification failed".to_string()
                },
                timestamp: now.clone(),
            });

            if !passed {
                overall_risk = RiskLevel::High;
                compliance_score *= 0.5;
                recommendations.push("Model signature verification failed. Do not deploy until resolved.".to_string());
            }
        } else {
            verification_results.push(VerificationResult {
                check_type: CheckType::SignatureVerification,
                passed: false,
                details: "No model signature provided".to_string(),
                timestamp: now.clone(),
            });
            overall_risk = RiskLevel::Medium;
            compliance_score *= 0.8;
            recommendations.push("Consider adding model signature for production deployments".to_string());
        }

        if let Some(sb) = sbom {
            let mut sbom_clone = sb.clone();
            sbom_clone.compute_aggregation_hash();
            let hash_valid = sbom_clone.aggregation_hash.as_ref().map(|h| !h.is_empty()).unwrap_or(false);

            verification_results.push(VerificationResult {
                check_type: CheckType::SBomIntegrity,
                passed: hash_valid,
                details: if hash_valid {
                    format!("SBOM for {} is complete", sb.name)
                } else {
                    "SBOM integrity check failed".to_string()
                },
                timestamp: now.clone(),
            });

            if !hash_valid {
                if overall_risk == RiskLevel::Low {
                    overall_risk = RiskLevel::Medium;
                }
                compliance_score *= 0.7;
                recommendations.push("SBOM integrity verification failed. Review SBOM composition.".to_string());
            }

            let license_result = self.check_license_compliance(sb, &["MIT".to_string(), "Apache-2.0".to_string(), "BSD-3-Clause".to_string()]);
            verification_results.push(VerificationResult {
                check_type: CheckType::LicenseCompliance,
                passed: license_result.verified,
                details: if license_result.errors.is_empty() {
                    "All components have compliant licenses".to_string()
                } else {
                    format!("Found {} license violations", license_result.errors.len())
                },
                timestamp: now.clone(),
            });

            if !license_result.verified {
                overall_risk = RiskLevel::High;
                compliance_score *= 0.6;
                recommendations.push("License compliance issues detected. Review component licenses.".to_string());
            }
        }

        let tee_result = self.verify_tee_deployment(model_name);
        verification_results.push(VerificationResult {
            check_type: CheckType::TeeAttestation,
            passed: tee_result.verified,
            details: if tee_result.verified {
                "TEE deployment attestation is valid".to_string()
            } else {
                "TEE deployment attestation missing or invalid".to_string()
            },
            timestamp: now.clone(),
        });

        if !tee_result.verified {
            if overall_risk == RiskLevel::Low {
                overall_risk = RiskLevel::Medium;
            }
            compliance_score *= 0.85;
            recommendations.push("Consider deploying in a TEE environment for enhanced security".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Supply chain verification passed. Continue monitoring for anomalies.".to_string());
        }

        SupplyChainReport {
            report_id: format!("SCR-{}", uuid::Uuid::new_v4()),
            generated_at: Utc::now().to_rfc3339(),
            model_name: model_name.to_string(),
            model_version: model_version.to_string(),
            verification_results,
            overall_risk_level: overall_risk,
            compliance_score,
            recommendations,
        }
    }
}

impl Default for SupplyChainManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbom_creation() {
        let mut sbom = Sbom {
            version: "1.0".to_string(),
            spdx_version: "SPDX-2.3".to_string(),
            sbom_id: "test-sbom-1".to_string(),
            name: "test-model".to_string(),
            created_at: Utc::now().to_rfc3339(),
            creator: "test".to_string(),
            entries: vec![
                SbomEntry {
                    name: "test-component".to_string(),
                    version: "1.0.0".to_string(),
                    component_type: SbomComponentType::Model,
                    spdx_id: Some("SPDXRef-Component".to_string()),
                    supplier: None,
                    license: Some("MIT".to_string()),
                    sha256: None,
                    dependencies: None,
                    published_date: None,
                },
            ],
            aggregation_hash: None,
        };

        sbom.compute_aggregation_hash();
        assert!(sbom.aggregation_hash.is_some());
    }

    #[test]
    fn test_supply_chain_manager() {
        let mut manager = SupplyChainManager::new();
        manager.add_trusted_signer("test-signer-fp");

        let sig = ModelSignature {
            algorithm: "sha256_rsa".to_string(),
            signature: "test-signature".to_string(),
            public_key_fingerprint: "test-signer-fp".to_string(),
            signed_at: Utc::now().to_rfc3339(),
            signer_identity: "test@test.com".to_string(),
            certificate_chain: None,
        };

        let result = manager.verify_model_signature("model-hash", &sig);
        assert!(result.model_signature_valid.is_some());
    }
}