//! # 硬件 TEE/ZK/Sigstore 认证接口 (§7.1.3-§7.2)
//!
//! 本模块定义真实硬件认证的标准接口。
//! 当硬件 SDK 可用时，实现对应 trait 并注册到注册表即可获得完整 L1/L2 证明能力。
//!
//! ## 支持平台
//! - Intel TDX / AMD SEV-SNP / NVIDIA CC / Arm CCA
//! - NANOZK / zkLLM 2.0 / Circom
//! - Sigstore / Cosign (Rekor transparency log)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

// ==================== TEE ====================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeePlatform { IntelTdx, AmdSevSnp, NvidiaCc, ArmCca }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeAttestationReport {
    pub platform: TeePlatform,
    pub attestation_quote: String,
    pub mrenclave: String,
    pub model_fingerprint: String,
    pub runtime_config_hash: String,
    pub timestamp: String,
    pub freshness_nonce: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeVerificationResult {
    pub verified: bool,
    pub platform: TeePlatform,
    pub mrenclave: String,
    pub issuer: String,
    pub tcb_status: String,
    pub verified_at: String,
}

#[derive(Debug)]
pub enum TeeError {
    PlatformNotAvailable(String),
    AttestationFailed(String),
    VerificationFailed(String),
    SdkError(String),
}

impl fmt::Display for TeeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlatformNotAvailable(s) => write!(f, "TEE not available: {}", s),
            Self::AttestationFailed(s) => write!(f, "TEE attestation failed: {}", s),
            Self::VerificationFailed(s) => write!(f, "TEE verification failed: {}", s),
            Self::SdkError(s) => write!(f, "TEE SDK error: {}", s),
        }
    }
}
impl std::error::Error for TeeError {}

#[async_trait]
pub trait TeeAttestor: Send + Sync {
    fn platform(&self) -> TeePlatform;
    async fn generate_quote(&self, report_data: &[u8], nonce: &[u8]) -> Result<String, TeeError>;
    async fn verify_quote(&self, quote: &str) -> Result<TeeVerificationResult, TeeError>;
    fn mrenclave(&self) -> String;
    fn is_available(&self) -> bool;
}

pub struct TeeRegistry { attestors: Vec<Box<dyn TeeAttestor>> }

impl TeeRegistry {
    pub fn new() -> Self { Self { attestors: Vec::new() } }
    pub fn register(&mut self, a: Box<dyn TeeAttestor>) {
        tracing::info!("TEE attestor registered: {:?}", a.platform());
        self.attestors.push(a);
    }
    pub fn get_preferred(&self, pref: Option<TeePlatform>) -> Option<&dyn TeeAttestor> {
        if let Some(p) = pref {
            self.attestors.iter().find(|a| a.platform() == p && a.is_available()).map(|a| a.as_ref())
        } else {
            self.attestors.iter().find(|a| a.is_available()).map(|a| a.as_ref())
        }
    }
    pub fn available_platforms(&self) -> Vec<TeePlatform> {
        self.attestors.iter().filter(|a| a.is_available()).map(|a| a.platform()).collect()
    }
}
impl Default for TeeRegistry { fn default() -> Self { Self::new() } }

// ==================== ZK ====================

#[async_trait]
pub trait ZkProver: Send + Sync {
    fn proof_system(&self) -> &str;
    fn verification_key_hash(&self) -> String;
    async fn prove(&self, witness: &[u8], timeout_ms: u64) -> Result<String, ZkError>;
    async fn verify(&self, proof: &str, public_inputs: &[u8]) -> Result<bool, ZkError>;
    fn supports_aggregation(&self) -> bool;
    fn estimated_proving_time_ms(&self, layers: usize) -> u64;
}

#[derive(Debug)]
pub enum ZkError {
    Timeout(u64),
    ProofTooLarge(usize),
    LibraryError(String),
    VerificationFailed,
}
impl fmt::Display for ZkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout(ms) => write!(f, "ZK timeout: {}ms", ms),
            Self::ProofTooLarge(sz) => write!(f, "ZK proof too large: {} bytes", sz),
            Self::LibraryError(s) => write!(f, "ZK lib error: {}", s),
            Self::VerificationFailed => write!(f, "ZK verification failed"),
        }
    }
}
impl std::error::Error for ZkError {}

pub struct ZkProofManager { provers: Vec<Box<dyn ZkProver>> }
impl ZkProofManager {
    pub fn new() -> Self { Self { provers: Vec::new() } }
    pub fn register(&mut self, p: Box<dyn ZkProver>) { self.provers.push(p); }
    pub fn get(&self, system: &str) -> Option<&dyn ZkProver> {
        self.provers.iter().find(|p| p.proof_system() == system).map(|p| p.as_ref())
    }
}
impl Default for ZkProofManager { fn default() -> Self { Self::new() } }

// ==================== Sigstore ====================

#[async_trait]
pub trait SigstoreVerifier: Send + Sync {
    async fn verify(&self, model_hash: &str, signature: &str, log_entry: &str) -> Result<bool, SigstoreError>;
}

#[derive(Debug)]
pub enum SigstoreError { VerificationFailed(String), LogEntryNotFound(String), SdkError(String) }
impl fmt::Display for SigstoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VerificationFailed(s) => write!(f, "Sigstore verification failed: {}", s),
            Self::LogEntryNotFound(s) => write!(f, "Log entry not found: {}", s),
            Self::SdkError(s) => write!(f, "Sigstore SDK error: {}", s),
        }
    }
}
impl std::error::Error for SigstoreError {}
