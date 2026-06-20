//! # 签名验证测试
//!
//! 测试 L0 签名生成和验证的正确性。
//! 包括 UTF-8 截断场景、内部字段剥离等边缘情况。

use uuid::Uuid;

use crate::crypto::signature::{generate_l0_proof, verify_l0_signature};
use crate::types::proof::Proofs;
use crate::types::trace::{Input, Observations, Output, Trace};

/// 创建一个测试用 Trace
fn create_test_trace() -> Trace {
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

/// 测试结果
pub struct SignatureTestResult {
    /// 测试名称
    pub name: &'static str,
    /// 是否通过
    pub passed: bool,
    /// 错误信息
    pub error: String,
}

/// 运行所有签名验证测试
pub fn run_signature_verification_tests() -> Vec<SignatureTestResult> {
    let mut results = Vec::new();

    // Test 1: 基本签名生成
    {
        let mut trace = create_test_trace();
        let proof = generate_l0_proof(&mut trace);
        trace.proofs.proof_chain.push(proof);
        match verify_l0_signature(&trace) {
            Ok(()) => results.push(SignatureTestResult {
                name: "basic_signature_verification",
                passed: true,
                error: String::new(),
            }),
            Err(e) => results.push(SignatureTestResult {
                name: "basic_signature_verification",
                passed: false,
                error: e,
            }),
        }
    }

    // Test 2: 检测篡改
    {
        let mut trace = create_test_trace();
        let proof = generate_l0_proof(&mut trace);
        trace.proofs.proof_chain.push(proof);
        trace.model = "tampered/model".to_string();
        match verify_l0_signature(&trace) {
            Err(_) => results.push(SignatureTestResult {
                name: "tamper_detection",
                passed: true,
                error: String::new(),
            }),
            Ok(()) => results.push(SignatureTestResult {
                name: "tamper_detection",
                passed: false,
                error: "Expected tamper detection but signature verified".to_string(),
            }),
        }
    }

    // Test 3: UTF-8 截断
    {
        let mut trace = create_test_trace();
        trace.output = Some(Output {
            response: Some(serde_json::json!("Text with \u{FFFD} truncated UTF-8")),
            truncated: true,
            finish_reason: Some("length".to_string()),
        });
        let proof = generate_l0_proof(&mut trace);
        trace.proofs.proof_chain.push(proof);
        match verify_l0_signature(&trace) {
            Ok(()) => results.push(SignatureTestResult {
                name: "utf8_truncation_handling",
                passed: true,
                error: String::new(),
            }),
            Err(e) => results.push(SignatureTestResult {
                name: "utf8_truncation_handling",
                passed: false,
                error: e,
            }),
        }
    }

    // Test 4: 内部字段排除
    {
        let mut trace = create_test_trace();
        trace.observations = Some(Observations {
            _internal_metrics: Some(serde_json::json!({"secret": "value"})),
            ..Default::default()
        });
        let proof = generate_l0_proof(&mut trace);
        trace.proofs.proof_chain.push(proof);
        match verify_l0_signature(&trace) {
            Ok(()) => results.push(SignatureTestResult {
                name: "internal_fields_excluded",
                passed: true,
                error: String::new(),
            }),
            Err(e) => results.push(SignatureTestResult {
                name: "internal_fields_excluded",
                passed: false,
                error: e,
            }),
        }
    }

    // Test 5: 签名确定性
    {
        let mut trace1 = create_test_trace();
        let mut trace2 = create_test_trace();
        let proof1 = generate_l0_proof(&mut trace1);
        let proof2 = generate_l0_proof(&mut trace2);
        results.push(SignatureTestResult {
            name: "signature_determinism",
            passed: proof1.signature == proof2.signature,
            error: if proof1.signature != proof2.signature {
                format!("Signatures do not match: {} != {}", 
                    proof1.signature.unwrap_or_default(),
                    proof2.signature.unwrap_or_default())
            } else {
                String::new()
            },
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 运行完整的签名验证测试套件
    #[test]
    fn test_signature_verification_suite() {
        let results = run_signature_verification_tests();
        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();

        if !failures.is_empty() {
            let mut msg = String::from("签名验证测试失败:\n");
            for result in &failures {
                msg.push_str(&format!("  - {}: {}\n", result.name, result.error));
            }
            panic!("{}", msg);
        }

        assert_eq!(failures.len(), 0);
    }
}
