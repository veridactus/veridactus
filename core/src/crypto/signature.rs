//! # L0 签名生成与验证
//!
//! 严格遵循 VERIDACTUS Protocol §7.1.2 和 AI.md §5.2。
//!
//! L0 签名流程（§3.3）：
//! 1. 将 proof_chain 中所有 signature 字段置空
//! 2. 递归剥离所有以 _ 开头的字段和 observations.internal_metrics
//! 3. 应用 UTF-8 安全处理
//! 4. 执行 RFC 8785 JCS 规范化
//! 5. 计算 SHA-256
//! 6. 输出 64 字符十六进制字符串

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::crypto::jcs::jcs_canonicalize;
use crate::crypto::utf8::sanitize_utf8_json;
use crate::types::proof::{ProofChainEntry, ProofLevel, ProofType};
use crate::types::trace::Trace;

/// 递归剥离所有以 `_` 开头的字段（规范 §7.1.2 要求）
///
/// 规则（AI.md §5.2）：
/// - 递归处理 JSON 对象的所有层级
/// - 删除键以 '_' 开头的所有字段
/// - 递归处理数组元素
/// - 原始类型无需处理
pub fn strip_internal_fields(value: &mut Value) {
    match value {
        Value::Object(obj) => {
            // 收集需要删除的键（避免迭代时修改）
            let keys_to_remove: Vec<String> = obj
                .keys()
                .filter(|k| k.starts_with('_'))
                .cloned()
                .collect();

            // 删除 _ 开头的字段
            for key in keys_to_remove {
                obj.remove(&key);
            }

            // 递归处理剩余字段
            for (_, v) in obj.iter_mut() {
                strip_internal_fields(v);
            }
        }
        Value::Array(arr) => {
            // 递归处理数组元素
            for item in arr.iter_mut() {
                strip_internal_fields(item);
            }
        }
        _ => {} // 原始类型无需处理
    }
}

/// 生成 L0 签名证明
///
/// 严格遵循规范 §7.1.2 和 AI.md §5.2 的 6 步流程。
///
/// # 参数
/// * `trace` - 完整的 Trace 对象（会被修改用于签名计算，但最终恢复）
///
/// # 返回
/// L0 ProofChainEntry
pub fn generate_l0_proof(trace: &mut Trace) -> ProofChainEntry {
    // ① 备份原始 proof_chain
    let original_chain = trace.proofs.proof_chain.clone();

    // ② 确保存在一个 L0 占位条目（如果没有则创建）
    let has_l0 = trace.proofs.proof_chain.iter().any(|p| p.level == ProofLevel::L0);
    if !has_l0 {
        trace.proofs.proof_chain.push(ProofChainEntry {
            level: ProofLevel::L0,
            r#type: ProofType::HashChain,
            signature: None,
            signature_pq: None,
            attestation_quote: None,
            model_fingerprint: None,
            platform: None,
            mrenclave: None,
            merkle_root: None,
            sampling_paths: None,
            zk_proof: None,
            verification_key_hash: None,
            proof_aggregation_root: None,
            canonicalization_method: "rfc8785".to_string(),
        });
    }

    // ③ 将所有 proof 的 signature 字段置空（规范要求）
    for proof in &mut trace.proofs.proof_chain {
        proof.signature = None;
        proof.signature_pq = None;
    }

    // ④ 剥离内部字段（_ 开头）
    let trace_value = serde_json::to_value(&*trace).unwrap();
    let mut sanitized = trace_value.clone();
    strip_internal_fields(&mut sanitized);

    // ⑤ UTF-8 安全处理
    sanitize_utf8_json(&mut sanitized);

    // ⑥ JCS 规范化 + SHA-256
    let canonical = jcs_canonicalize(&sanitized);
    let signature = compute_sha256_hex(canonical.as_bytes());

    // ⑦ 恢复原始 proof_chain（避免副作用）
    trace.proofs.proof_chain = original_chain;

    ProofChainEntry {
        level: ProofLevel::L0,
        r#type: ProofType::HashChain,
        signature: Some(signature),
        signature_pq: None,
        attestation_quote: None,
        model_fingerprint: None,
        platform: None,
        mrenclave: None,
        merkle_root: None,
        sampling_paths: None,
        zk_proof: None,
        verification_key_hash: None,
        proof_aggregation_root: None,
        canonicalization_method: "rfc8785".to_string(),
    }
}

/// 验证 Trace 的 L0 签名
///
/// # 参数
/// * `trace` - 待验证的 Trace 对象
///
/// # 返回
/// * `Ok(())` - 签名验证通过
/// * `Err(String)` - 验证失败，返回错误原因
pub fn verify_l0_signature(trace: &Trace) -> Result<(), String> {
    // 获取 L0 证明条目
    let l0_proof = trace
        .proofs
        .proof_chain
        .iter()
        .find(|p| p.level == ProofLevel::L0)
        .ok_or_else(|| "L0 proof entry not found".to_string())?;

    let expected_signature = l0_proof
        .signature
        .as_ref()
        .ok_or_else(|| "L0 signature field empty".to_string())?;

    // 克隆 Trace 用于验证
    let mut trace_clone = trace.clone();

    // 备份并清空 proof_chain 的 signature
    let original_chain = trace_clone.proofs.proof_chain.clone();
    for proof in &mut trace_clone.proofs.proof_chain {
        proof.signature = None;
        proof.signature_pq = None;
    }

    // 剥离内部字段
    let mut sanitized = serde_json::to_value(&trace_clone).unwrap();
    strip_internal_fields(&mut sanitized);

    // UTF-8 安全处理
    sanitize_utf8_json(&mut sanitized);

    // JCS 规范化 + SHA-256
    let canonical = jcs_canonicalize(&sanitized);
    let computed_signature = compute_sha256_hex(canonical.as_bytes());

    // 恢复
    trace_clone.proofs.proof_chain = original_chain;

    // 比对签名
    if computed_signature == *expected_signature {
        Ok(())
    } else {
        Err(format!(
            "L0 签名不匹配\n期望: {}\n计算: {}",
            expected_signature, computed_signature
        ))
    }
}

/// 计算 SHA-256 十六进制摘要
pub fn compute_sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// 获取 JCS 规范化后的字符串（用于调试/审计）
pub fn get_canonicalized_trace(trace: &Trace) -> Result<String, String> {
    let mut trace_clone = trace.clone();

    // 清空 signature
    for proof in &mut trace_clone.proofs.proof_chain {
        proof.signature = None;
        proof.signature_pq = None;
    }

    // 剥离内部字段
    let mut sanitized = serde_json::to_value(&trace_clone)
        .map_err(|e| format!("Serialization failed: {}", e))?;
    strip_internal_fields(&mut sanitized);
    sanitize_utf8_json(&mut sanitized);

    Ok(jcs_canonicalize(&sanitized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::proof::Proofs;
    use crate::types::trace::{Input, Observations, Output};
    use uuid::Uuid;

    /// 创建用于测试的最小 Trace
    fn create_minimal_trace() -> Trace {
        Trace {
            trace_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            parent_id: None,
            session_id: None,
            tenant_id: Some("test-tenant".to_string()),
            execution_state: None,
            model: "openai/gpt-4o".to_string(),
            engine_determinism: None,
            input: Some(Input {
                prompt: Some(serde_json::json!([{"role": "user", "content": "Hello"}])),
                params: Some(serde_json::json!({"temperature": 0.7})),
                metadata: None,
            }),
            output: Some(Output {
                response: Some(serde_json::json!("Hi there!")),
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
        }
    }

    /// 测试 L0 签名生成
    #[test]
    fn test_l0_proof_generation() {
        let mut trace = create_minimal_trace();
        let l0_proof = generate_l0_proof(&mut trace);

        assert_eq!(l0_proof.level, ProofLevel::L0);
        assert_eq!(l0_proof.r#type, ProofType::HashChain);
        assert!(l0_proof.signature.is_some());
        assert_eq!(l0_proof.signature.as_ref().unwrap().len(), 64);
    }

    /// 测试 L0 签名验证通过
    #[test]
    fn test_l0_signature_verification_pass() {
        let mut trace = create_minimal_trace();
        let l0_proof = generate_l0_proof(&mut trace);
        trace.proofs.proof_chain.push(l0_proof);

        assert!(verify_l0_signature(&trace).is_ok());
    }

    /// 测试 L0 签名验证失败（数据被篡改）
    #[test]
    fn test_l0_signature_verification_fail_tampered() {
        let mut trace = create_minimal_trace();
        let l0_proof = generate_l0_proof(&mut trace);
        trace.proofs.proof_chain.push(l0_proof);

        // 篡改输出
        if let Some(ref mut output) = trace.output {
            output.response = Some(serde_json::json!("Tampered response"));
        }

        assert!(verify_l0_signature(&trace).is_err());
    }

    /// 测试 _ 开头的字段被排除在签名外
    #[test]
    fn test_internal_fields_excluded_from_signature() {
        let mut trace = create_minimal_trace();
        trace.observations = Some(Observations {
            replay_snapshot: None, // 不设 replay_snapshot
            _internal_metrics: Some(serde_json::json!({"secret_key": "should_not_be_signed"})),
            ..Default::default()
        });

        let l0_proof = generate_l0_proof(&mut trace);
        trace.proofs.proof_chain.push(l0_proof);

        assert!(verify_l0_signature(&trace).is_ok());
    }

    /// 测试 UTF-8 截断场景的签名一致性
    #[test]
    fn test_utf8_truncated_signing() {
        let mut trace = create_minimal_trace();
        // 模拟截断的 UTF-8 输出
        trace.output = Some(Output {
            response: Some(serde_json::json!("Hello \u{FFFD} World")),
            truncated: true,
            finish_reason: Some("length".to_string()),
        });

        let l0_proof = generate_l0_proof(&mut trace);
        trace.proofs.proof_chain.push(l0_proof);

        assert!(verify_l0_signature(&trace).is_ok());
    }

    /// 测试多次签名一致性
    #[test]
    fn test_signature_deterministic() {
        let mut trace1 = create_minimal_trace();
        let mut trace2 = create_minimal_trace();

        let proof1 = generate_l0_proof(&mut trace1);
        let proof2 = generate_l0_proof(&mut trace2);

        // 相同 Trace 应该生成相同签名
        assert_eq!(proof1.signature, proof2.signature);
    }

    /// 测试 strip_internal_fields 递归深度
    #[test]
    fn test_strip_internal_fields_nested() {
        let mut value = serde_json::json!({
            "a": 1,
            "_secret": "hidden",
            "nested": {
                "_depth1": "should_remove",
                "keep": {
                    "_depth2": "should_remove_too",
                    "also_keep": true
                }
            },
            "arr": [
                {"_item_secret": "remove_this"},
                {"visible": true}
            ]
        });

        strip_internal_fields(&mut value);

        assert_eq!(value.get("_secret"), None);
        assert_eq!(value.pointer("/nested/_depth1"), None);
        assert_eq!(value.pointer("/nested/keep/_depth2"), None);
        assert_eq!(value.pointer("/arr/0/_item_secret"), None);
        assert!(value.pointer("/arr/1/visible").is_some());
    }
}
