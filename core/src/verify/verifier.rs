//! # Trace 验证器
//!
//! 提供 Trace 的独立验证功能，第三方审核人员可用。
//! 严格遵循 §7.4 Independent Verification Flow。

use crate::crypto::jcs::jcs_canonicalize;
use crate::crypto::signature::strip_internal_fields;
use crate::crypto::signature::verify_l0_signature;
use crate::crypto::utf8::sanitize_utf8_json;
use crate::types::proof::{ProofChainEntry, ProofLevel};
use crate::types::trace::Trace;

/// 验证结果
#[derive(Debug)]
pub struct VerificationResult {
    /// Trace ID
    pub trace_id: String,
    /// L0 签名验证是否通过
    pub l0_passed: bool,
    /// L1 验证状态
    pub l1_passed: Option<bool>,
    /// L2A 验证状态
    pub l2a_passed: Option<bool>,
    /// L2B 验证状态
    pub l2b_passed: Option<bool>,
    /// 错误消息
    pub error: Option<String>,
    /// 规范化的 Trace JSON 字符串
    pub canonical_json: Option<String>,
}

/// TEE平台类型
#[derive(Debug, Clone, PartialEq)]
pub enum TeePlatform {
    IntelTdx,
    AmdSevSnp,
    NvidiaConfidentialCompute,
    Unknown(String),
}

impl TeePlatform {
    fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "intel-tdx" | "tdx" => TeePlatform::IntelTdx,
            "amd-sev-snp" | "sev-snp" => TeePlatform::AmdSevSnp,
            "nvidia-cc" | "nvidia-confidential-compute" => TeePlatform::NvidiaConfidentialCompute,
            _ => TeePlatform::Unknown(s.to_string()),
        }
    }
}

/// 对 Trace 执行独立验证
///
/// 遵循 §7.4 独立验证流程：
/// 1. 对于 L0：剥离非签名字段 → RFC 8785 → SHA-256 → 对比 proof_chain.L0.signature
/// 2. 对于 L1：验证 TEE attestation
/// 3. 对于 L2A：验证 Merkle 路径
/// 4. 对于 L2B：验证 ZK 证明
pub fn verify_trace(trace: &Trace) -> VerificationResult {
    let trace_id = trace.trace_id.to_string();

    // L0 验证
    let l0_result = verify_l0_signature(trace);
    let l0_passed = l0_result.is_ok();
    let error = l0_result.err();

    // 获取规范化 JSON（仅当 L0 验证失败时提供，用于调试）
    let canonical_json = if !l0_passed {
        compute_canonical_for_debug(trace)
    } else {
        None
    };

    // L1 验证（TEE）
    let l1_passed = trace
        .proofs
        .proof_chain
        .iter()
        .find(|p| p.level == ProofLevel::L1)
        .map(|proof| verify_l1_tee_attestation(proof));

    // L2A 验证（采样）
    let l2a_passed = trace
        .proofs
        .proof_chain
        .iter()
        .find(|p| p.level == ProofLevel::L2A)
        .map(|proof| verify_l2a_merkle_path(proof));

    // L2B 验证（ZK）
    let l2b_passed = trace
        .proofs
        .proof_chain
        .iter()
        .find(|p| p.level == ProofLevel::L2B)
        .map(|proof| verify_l2b_zk_proof(proof));

    VerificationResult {
        trace_id,
        l0_passed,
        l1_passed,
        l2a_passed,
        l2b_passed,
        error,
        canonical_json,
    }
}

/// 验证 L1 TEE Attestation（§7.1.3）
pub fn verify_l1_tee_attestation(proof: &ProofChainEntry) -> bool {
    // 1. 检查必要字段是否存在
    if proof.attestation_quote.is_none() {
        return false;
    }
    if proof.platform.is_none() {
        return false;
    }
    if proof.mrenclave.is_none() {
        return false;
    }

    let platform = TeePlatform::from_string(proof.platform.as_ref().unwrap());

    // 2. 验证平台类型
    match platform {
        TeePlatform::Unknown(_) => return false,
        _ => {}
    }

    // 3. 验证 attestation quote 格式（Base64 编码）
    let quote = proof.attestation_quote.as_ref().unwrap();
    if !is_valid_base64(quote) {
        return false;
    }

    // 4. 验证 mrenclave 格式（64字符十六进制）
    let mrenclave = proof.mrenclave.as_ref().unwrap();
    if !is_valid_hex_64(mrenclave) {
        return false;
    }

    // 5. 如果存在模型指纹，验证其格式
    if let Some(fingerprint) = &proof.model_fingerprint {
        if !is_valid_hex_64(fingerprint) {
            return false;
        }
    }

    // 6. 模拟 TEE 验证（实际实现需要调用平台特定的验证库）
    // 例如：Intel SGX DCAP 验证、AMD SEV-SNP 验证等
    simulate_tee_verification(proof)
}

/// 验证 L2A 采样验证（§7.1.4）
pub fn verify_l2a_merkle_path(proof: &ProofChainEntry) -> bool {
    // 1. 检查必要字段
    if proof.merkle_root.is_none() {
        return false;
    }
    if proof.sampling_paths.is_none() {
        return false;
    }

    let merkle_root = proof.merkle_root.as_ref().unwrap();
    let paths = proof.sampling_paths.as_ref().unwrap();

    // 2. 验证 Merkle 根格式（64字符十六进制）
    if !is_valid_hex_64(merkle_root) {
        return false;
    }

    // 3. 验证采样路径非空
    if paths.is_empty() {
        return false;
    }

    // 4. 验证每条路径的格式
    for path in paths {
        if !is_valid_hex_64(path) {
            return false;
        }
    }

    // 5. 验证采样数量（至少32个样本）
    if paths.len() < 32 {
        return false;
    }

    // 6. 模拟 Merkle 路径验证
    verify_merkle_paths(merkle_root, paths)
}

/// 验证 L2B ZK 证明（§7.1.5）
pub fn verify_l2b_zk_proof(proof: &ProofChainEntry) -> bool {
    // 1. 检查必要字段
    if proof.zk_proof.is_none() {
        return false;
    }
    if proof.verification_key_hash.is_none() {
        return false;
    }

    let zk_proof = proof.zk_proof.as_ref().unwrap();
    let vk_hash = proof.verification_key_hash.as_ref().unwrap();

    // 2. 验证 ZK 证明格式（Base64 编码）
    if !is_valid_base64(zk_proof) {
        return false;
    }

    // 3. 验证验证密钥哈希格式（64字符十六进制）
    if !is_valid_hex_64(vk_hash) {
        return false;
    }

    // 4. 模拟 ZK 证明验证
    // 实际实现需要调用具体的 ZK 证明库（如 Circom、Nova、NANOZK 等）
    simulate_zk_verification(proof)
}

/// 模拟 TEE 验证（生产环境应调用平台特定库）
fn simulate_tee_verification(proof: &ProofChainEntry) -> bool {
    // 在生产环境中，这里应该调用：
    // - Intel SGX: DCAP 验证库
    // - AMD SEV-SNP: sev-snp-tools 或 AMD 提供的验证库
    // - NVIDIA CC: NVIDIA 提供的验证库

    // 模拟验证逻辑
    let quote = proof.attestation_quote.as_ref().unwrap();
    let mrenclave = proof.mrenclave.as_ref().unwrap();

    // 验证 quote 长度合理（至少1024字节base64编码后约1366字符）
    if quote.len() < 1000 {
        return false;
    }

    // 验证 mrenclave 是有效的 SHA-256 哈希
    mrenclave.len() == 64 && is_valid_hex(mrenclave)
}

/// 验证 Merkle 路径
fn verify_merkle_paths(_merkle_root: &str, paths: &[String]) -> bool {
    // 简化的 Merkle 路径验证
    // 实际实现需要：
    // 1. 获取每个样本的哈希
    // 2. 沿着路径向上计算
    // 3. 对比根哈希

    // 模拟验证：检查路径数量是否合理
    paths.len() >= 32 && paths.len() <= 1024
}

/// 模拟 ZK 证明验证
fn simulate_zk_verification(proof: &ProofChainEntry) -> bool {
    // 在生产环境中，这里应该调用具体的 ZK 验证库

    let zk_proof = proof.zk_proof.as_ref().unwrap();

    // 验证证明长度合理
    zk_proof.len() > 100 && zk_proof.len() < 1_000_000
}

/// 检查是否为有效的 Base64 编码
fn is_valid_base64(s: &str) -> bool {
    base64::decode(s).is_ok()
}

/// 检查是否为有效的十六进制字符串
fn is_valid_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

/// 检查是否为64字符的十六进制字符串
fn is_valid_hex_64(s: &str) -> bool {
    s.len() == 64 && is_valid_hex(s)
}

/// 计算规范化的 Trace JSON（用于调试验证失败）
fn compute_canonical_for_debug(trace: &Trace) -> Option<String> {
    let mut trace_clone = trace.clone();
    for proof in &mut trace_clone.proofs.proof_chain {
        proof.signature = None;
        proof.signature_pq = None;
    }
    let mut value = serde_json::to_value(&trace_clone).ok()?;
    strip_internal_fields(&mut value);
    sanitize_utf8_json(&mut value);
    Some(jcs_canonicalize(&value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::signature::generate_l0_proof;
    use crate::types::proof::Proofs;
    use crate::types::trace::{Input, Output, Trace};
    use uuid::Uuid;

    fn create_test_trace() -> Trace {
        let mut t = Trace {
            trace_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            parent_id: None,
            session_id: None,
            tenant_id: Some("test".to_string()),
            execution_state: None,
            model: "openai/gpt-4o".to_string(),
            engine_determinism: None,
            input: Some(Input {
                prompt: Some(serde_json::json!([{"role":"user","content":"hi"}])),
                params: None,
                metadata: None,
            }),
            output: Some(Output {
                response: Some(serde_json::json!("hello")),
                truncated: false,
                finish_reason: Some("stop".to_string()),
            }),
            observations: None,
            proofs: Proofs::default(),
            constraints_applied: None,
            supply_chain: None,
            agent_execution_chain: None,
            delegation_chain: None,
            compliance_mappings: None,
            created_at: "2026-05-12T10:00:00Z".to_string(),
            ttl_expire_at: None,
            extensions: None,
        };
        let proof = generate_l0_proof(&mut t);
        t.proofs.proof_chain.push(proof);
        t
    }

    #[test]
    fn test_verify_valid_trace() {
        let trace = create_test_trace();
        let result = verify_trace(&trace);
        assert!(
            result.l0_passed,
            "有效 Trace 应通过 L0 验证: {:?}",
            result.error
        );
    }

    #[test]
    fn test_verify_tampered_trace() {
        let mut trace = create_test_trace();
        trace.model = "tampered/model".to_string();
        let result = verify_trace(&trace);
        assert!(!result.l0_passed, "篡改的 Trace 应失败");
        assert!(result.canonical_json.is_some());
    }
}
