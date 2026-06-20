//! # GDPR Right to Erasure
//!
//! 严格遵循 VERIDACTUS v0.2.1 §8.7 Data Retention and Lifecycle.
//! 实现 GDPR/CCPA "被遗忘权"功能，支持级联删除和审计日志。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionRequest {
    pub request_id: String,
    pub deletion_type: DeletionType,
    pub target_id: String,
    pub requester_identity: Option<String>,
    pub timestamp: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeletionType {
    TraceId,
    SessionId,
    UserId,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionResult {
    pub request_id: String,
    pub success: bool,
    pub deleted_count: usize,
    pub retained_signatures: Vec<String>,
    pub audit_log_entry: DeletionAuditEntry,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionAuditEntry {
    pub audit_id: String,
    pub request_id: String,
    pub deletion_type: DeletionType,
    pub target_id: String,
    pub deleted_count: usize,
    pub retained_signature_hashes: Vec<String>,
    pub deleted_at: String,
    pub deleted_by: Option<String>,
    pub compliance_evidence: ComplianceEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceEvidence {
    pub regulation: String,
    pub article: String,
    pub basis: String,
    pub data_subject_right: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetainedSignature {
    pub trace_id: String,
    pub audit_signature: String,
    pub deletion_timestamp: String,
    pub privacy_level: String,
}

pub struct GdprErasureManager {
    storage_backend: Box<dyn DeletionStorage + Send + Sync>,
    retention_policy: RetentionPolicy,
}

pub trait DeletionStorage {
    fn delete_by_trace_id(&self, trace_id: &str) -> Result<DeletionResult, DeletionError>;
    fn delete_by_session_id(&self, session_id: &str) -> Result<DeletionResult, DeletionError>;
    fn delete_by_user_id(&self, user_id: &str) -> Result<DeletionResult, DeletionError>;
    fn retain_signature(&self, trace_id: &str, audit_signature: &str) -> Result<(), DeletionError>;
    fn get_deletion_log(&self, request_id: &str) -> Option<DeletionAuditEntry>;
    fn list_deletion_logs(&self, limit: usize) -> Vec<DeletionAuditEntry>;
}

#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub minimum_retention_days: u32,
    pub retain_signatures: bool,
    pub audit_log_retention_days: u32,
}

#[derive(Debug)]
pub enum DeletionError {
    StorageError(String),
    NotFound,
    Forbidden,
    ValidationError(String),
    ComplianceError(String),
}

impl std::fmt::Display for DeletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeletionError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            DeletionError::NotFound => write!(f, "Not found"),
            DeletionError::Forbidden => write!(f, "Forbidden"),
            DeletionError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            DeletionError::ComplianceError(msg) => write!(f, "Compliance error: {}", msg),
        }
    }
}

impl std::error::Error for DeletionError {}

impl GdprErasureManager {
    pub fn new(storage_backend: Box<dyn DeletionStorage + Send + Sync>) -> Self {
        Self {
            storage_backend,
            retention_policy: RetentionPolicy {
                minimum_retention_days: 30,
                retain_signatures: true,
                audit_log_retention_days: 365,
            },
        }
    }

    pub fn with_retention_policy(mut self, policy: RetentionPolicy) -> Self {
        self.retention_policy = policy;
        self
    }

    pub fn process_deletion(&self, request: DeletionRequest) -> Result<DeletionResult, DeletionError> {
        let result = match request.deletion_type {
            DeletionType::TraceId => self.storage_backend.delete_by_trace_id(&request.target_id),
            DeletionType::SessionId => self.storage_backend.delete_by_session_id(&request.target_id),
            DeletionType::UserId => self.storage_backend.delete_by_user_id(&request.target_id),
            DeletionType::All => return Err(DeletionError::Forbidden),
        };

        result
    }

    pub fn get_deletion_proof(&self, request_id: &str) -> Option<DeletionAuditEntry> {
        self.storage_backend.get_deletion_log(request_id)
    }

    pub fn list_deletion_history(&self, limit: usize) -> Vec<DeletionAuditEntry> {
        self.storage_backend.list_deletion_logs(limit)
    }
}

fn generate_request_id() -> String {
    format!("del_{}", Uuid::new_v4())
}

fn generate_audit_id() -> String {
    format!("audit_{}", Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockStorage;

    impl DeletionStorage for MockStorage {
        fn delete_by_trace_id(&self, _trace_id: &str) -> Result<DeletionResult, DeletionError> {
            Ok(DeletionResult {
                request_id: "test_req".to_string(),
                success: true,
                deleted_count: 1,
                retained_signatures: vec!["sig1".to_string()],
                audit_log_entry: DeletionAuditEntry {
                    audit_id: "audit1".to_string(),
                    request_id: "test_req".to_string(),
                    deletion_type: DeletionType::TraceId,
                    target_id: "trace1".to_string(),
                    deleted_count: 1,
                    retained_signature_hashes: vec!["sig1".to_string()],
                    deleted_at: "2026-05-17T10:00:00Z".to_string(),
                    deleted_by: None,
                    compliance_evidence: ComplianceEvidence {
                        regulation: "GDPR".to_string(),
                        article: "Article 17".to_string(),
                        basis: "Right to erasure".to_string(),
                        data_subject_right: "Right to be forgotten".to_string(),
                    },
                },
                error_message: None,
            })
        }

        fn delete_by_session_id(&self, _session_id: &str) -> Result<DeletionResult, DeletionError> {
            Ok(DeletionResult {
                request_id: "test_req".to_string(),
                success: true,
                deleted_count: 5,
                retained_signatures: vec!["sig1".to_string(), "sig2".to_string()],
                audit_log_entry: DeletionAuditEntry {
                    audit_id: "audit2".to_string(),
                    request_id: "test_req".to_string(),
                    deletion_type: DeletionType::SessionId,
                    target_id: "session1".to_string(),
                    deleted_count: 5,
                    retained_signature_hashes: vec!["sig1".to_string(), "sig2".to_string()],
                    deleted_at: "2026-05-17T10:00:00Z".to_string(),
                    deleted_by: None,
                    compliance_evidence: ComplianceEvidence {
                        regulation: "GDPR".to_string(),
                        article: "Article 17".to_string(),
                        basis: "Right to erasure".to_string(),
                        data_subject_right: "Right to be forgotten".to_string(),
                    },
                },
                error_message: None,
            })
        }

        fn delete_by_user_id(&self, _user_id: &str) -> Result<DeletionResult, DeletionError> {
            Ok(DeletionResult {
                request_id: "test_req".to_string(),
                success: true,
                deleted_count: 10,
                retained_signatures: vec![],
                audit_log_entry: DeletionAuditEntry {
                    audit_id: "audit3".to_string(),
                    request_id: "test_req".to_string(),
                    deletion_type: DeletionType::UserId,
                    target_id: "user1".to_string(),
                    deleted_count: 10,
                    retained_signature_hashes: vec![],
                    deleted_at: "2026-05-17T10:00:00Z".to_string(),
                    deleted_by: None,
                    compliance_evidence: ComplianceEvidence {
                        regulation: "GDPR".to_string(),
                        article: "Article 17".to_string(),
                        basis: "Right to erasure".to_string(),
                        data_subject_right: "Right to be forgotten".to_string(),
                    },
                },
                error_message: None,
            })
        }

        fn retain_signature(&self, _trace_id: &str, _audit_signature: &str) -> Result<(), DeletionError> {
            Ok(())
        }

        fn get_deletion_log(&self, _request_id: &str) -> Option<DeletionAuditEntry> {
            None
        }

        fn list_deletion_logs(&self, _limit: usize) -> Vec<DeletionAuditEntry> {
            Vec::new()
        }
    }

    #[test]
    fn test_delete_by_trace_id() {
        let manager = GdprErasureManager::new(Box::new(MockStorage));
        let request = DeletionRequest {
            request_id: generate_request_id(),
            deletion_type: DeletionType::TraceId,
            target_id: "trace123".to_string(),
            requester_identity: None,
            timestamp: "2026-05-17T10:00:00Z".to_string(),
            reason: None,
        };

        let result = manager.process_deletion(request).unwrap();
        assert!(result.success);
        assert_eq!(result.deleted_count, 1);
        assert_eq!(result.audit_log_entry.compliance_evidence.regulation, "GDPR");
    }

    #[test]
    fn test_delete_all_forbidden() {
        let manager = GdprErasureManager::new(Box::new(MockStorage));
        let request = DeletionRequest {
            request_id: generate_request_id(),
            deletion_type: DeletionType::All,
            target_id: "all".to_string(),
            requester_identity: None,
            timestamp: "2026-05-17T10:00:00Z".to_string(),
            reason: None,
        };

        let result = manager.process_deletion(request);
        assert!(matches!(result, Err(DeletionError::Forbidden)));
    }
}