//! # HIPAA + PCI-DSS 合规映射
//!
//! 协议 §7.5 扩展合规框架映射

use serde::{Deserialize, Serialize};

/// 合规映射条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceEntry {
    pub framework: String,
    pub control_id: String,
    pub description: String,
    pub evidence: String,
    pub status: ComplianceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceStatus {
    Compliant,
    PartialCompliant,
    NonCompliant,
    NotApplicable,
}

/// HIPAA 合规映射
pub fn hipaa_mappings() -> Vec<ComplianceEntry> {
    vec![
        ComplianceEntry {
            framework: "HIPAA".into(),
            control_id: "164.312(a)(1)".into(),
            description: "Access Control — Unique User Identification".into(),
            evidence: "API Key authentication per tenant".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "HIPAA".into(),
            control_id: "164.312(a)(2)(iv)".into(),
            description: "Encryption and Decryption — PHI at rest".into(),
            evidence: "AES-256 storage encryption via L0 hash chain".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "HIPAA".into(),
            control_id: "164.312(b)".into(),
            description: "Audit Controls — Record and examine activity".into(),
            evidence: "Trace records with full audit chain (L0-L2B)".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "HIPAA".into(),
            control_id: "164.312(c)(1)".into(),
            description: "Integrity — Protect ePHI from improper alteration".into(),
            evidence: "SHA-256 L0 audit signatures prevent tampering".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "HIPAA".into(),
            control_id: "164.312(d)".into(),
            description: "Person or Entity Authentication".into(),
            evidence: "Ed25519 delegation token + API key verification".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "HIPAA".into(),
            control_id: "164.312(e)(1)".into(),
            description: "Transmission Security — Integrity controls".into(),
            evidence: "Transport over HTTPS with JCS canonicalization".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "HIPAA".into(),
            control_id: "164.310(d)(2)(iii)".into(),
            description: "Accountability — Record of PHI disclosures".into(),
            evidence: "GdprErasureManager tracks all data access".into(),
            status: ComplianceStatus::PartialCompliant,
        },
    ]
}

/// PCI-DSS 合规映射
pub fn pci_dss_mappings() -> Vec<ComplianceEntry> {
    vec![
        ComplianceEntry {
            framework: "PCI-DSS".into(),
            control_id: "Req 3.4".into(),
            description: "Render PAN unreadable anywhere it is stored".into(),
            evidence: "PII Detector masks credit card numbers".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "PCI-DSS".into(),
            control_id: "Req 4.1".into(),
            description: "Use strong cryptography for transmission of cardholder data".into(),
            evidence: "TLS 1.2+ enforced for all API communication".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "PCI-DSS".into(),
            control_id: "Req 6.5".into(),
            description: "Address common coding vulnerabilities in development".into(),
            evidence: "Rust memory safety + cargo audit in CI".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "PCI-DSS".into(),
            control_id: "Req 7.1".into(),
            description: "Limit access to system components by business need-to-know".into(),
            evidence: "Role-based API key scoping per tenant".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "PCI-DSS".into(),
            control_id: "Req 10.1".into(),
            description: "Implement audit trails to reconstruct events".into(),
            evidence: "Full Trace recording with proof chain and timestamps".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "PCI-DSS".into(),
            control_id: "Req 10.2.1".into(),
            description: "All individual user accesses to cardholder data".into(),
            evidence: "tenant_id scoped traces track per-user access".into(),
            status: ComplianceStatus::Compliant,
        },
        ComplianceEntry {
            framework: "PCI-DSS".into(),
            control_id: "Req 11.4".into(),
            description: "Use intrusion-detection to monitor all traffic".into(),
            evidence: "G1 guardrail detects prompt injection and jailbreak".into(),
            status: ComplianceStatus::PartialCompliant,
        },
    ]
}
