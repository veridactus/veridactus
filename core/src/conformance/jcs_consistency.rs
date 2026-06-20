//! # JCS 跨语言一致性测试
//!
//! 遵循 §13.1 PROOF_JCS_CONSISTENCY 测试要求。
//! 确保 Rust 实现的 JCS 规范化与 Go/Python/JS 参考库输出一致。
//!
//! 参考库（§3.2.2）：
//! - Go: `github.com/cyberphone/json-canonicalization`
//! - Python: `jsoncanon`
//! - JS: `canonicalize`
//! - Rust: `json-canon`

use crate::crypto::jcs::jcs_canonicalize;
use serde_json::Value;

/// JCS 一致性测试用例
pub struct JcsConsistencyTestCase {
    /// 测试名称
    pub name: &'static str,
    /// 输入 JSON
    pub input: &'static str,
    /// 期望的规范化输出
    pub expected: &'static str,
}

/// JCS 一致性测试向量集
///
/// 这些测试向量与 Go/Python/JS 参考库的测试向量保持一致。
pub fn get_jcs_test_vectors() -> Vec<JcsConsistencyTestCase> {
    vec![
        JcsConsistencyTestCase {
            name: "simple_object",
            input: r#"{"z":1,"a":2}"#,
            expected: r#"{"a":2,"z":1}"#,
        },
        JcsConsistencyTestCase {
            name: "nested_object",
            input: r#"{"outer":{"b":2,"a":1},"inner":3}"#,
            expected: r#"{"inner":3,"outer":{"a":1,"b":2}}"#,
        },
        JcsConsistencyTestCase {
            name: "array_preserves_order",
            input: r#"{"items":[3,1,2]}"#,
            expected: r#"{"items":[3,1,2]}"#,
        },
        JcsConsistencyTestCase {
            name: "special_characters",
            input: r#"{"msg":"hello\nworld"}"#,
            expected: r#"{"msg":"hello\nworld"}"#,
        },
        JcsConsistencyTestCase {
            name: "unicode_characters",
            input: r#"{"greeting":"héllo","你好":"world"}"#,
            expected: r#"{"greeting":"héllo","你好":"world"}"#, // non-ASCII preserved, keys sorted by Unicode
        },
        JcsConsistencyTestCase {
            name: "number_normalization",
            input: r#"{"int_val":1.0,"float_val":1.5,"zero":0.0}"#,
            expected: r#"{"float_val":1.5,"int_val":1,"zero":0}"#,
        },
        JcsConsistencyTestCase {
            name: "booleans_and_null",
            input: r#"{"a":null,"b":true,"c":false}"#,
            expected: r#"{"a":null,"b":true,"c":false}"#,
        },
        JcsConsistencyTestCase {
            name: "trace_minimal",
            input: r#"{"trace_id":"550e8400-e29b-41d4-a716-446655440000","model":"openai/gpt-4o","created_at":"2026-05-12T10:00:00Z","proofs":{"proof_chain":[{"level":"L0","type":"hash_chain","signature":"","canonicalization_method":"rfc8785"}]}}"#,
            expected: r#"{"created_at":"2026-05-12T10:00:00Z","model":"openai/gpt-4o","proofs":{"proof_chain":[{"canonicalization_method":"rfc8785","level":"L0","signature":"","type":"hash_chain"}]},"trace_id":"550e8400-e29b-41d4-a716-446655440000"}"#,
        },
    ]
}

/// 运行所有 JCS 一致性测试
///
/// # 返回
/// 测试结果列表，每个结果包含测试名称和是否通过
pub fn run_jcs_consistency_tests() -> Vec<(&'static str, bool, String)> {
    let mut results = Vec::new();

    for test_case in get_jcs_test_vectors() {
        let value: Value = match serde_json::from_str(test_case.input) {
            Ok(v) => v,
            Err(e) => {
                results.push((test_case.name, false, format!("JSON parse failed: {}", e)));
                continue;
            }
        };

        let actual = jcs_canonicalize(&value);
        let passed = actual == test_case.expected;

        if passed {
            results.push((test_case.name, true, String::new()));
        } else {
            results.push((
                test_case.name,
                false,
                format!("\n期望: {}\n实际: {}", test_case.expected, actual),
            ));
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 运行所有 JCS 一致性测试
    #[test]
    fn test_jcs_consistency_suite() {
        let results = run_jcs_consistency_tests();
        let failures: Vec<_> = results.iter().filter(|(_, passed, _)| !passed).collect();

        if !failures.is_empty() {
            let mut msg = String::from("JCS 一致性测试失败:\n");
            for (name, _, detail) in &failures {
                msg.push_str(&format!("  - {}: {}\n", name, detail));
            }
            panic!("{}", msg);
        }

        // 全部通过
        assert_eq!(failures.len(), 0);
    }

    /// 测试跨语言兼容性（模拟 Go/Python 输出）
    #[test]
    fn test_cross_language_compatibility() {
        let trace_json = r#"{"a":1,"b":"test","c":null}"#;
        let value: Value = serde_json::from_str(trace_json).unwrap();
        let result = jcs_canonicalize(&value);

        // 此输出应与 Go 的 json-canonicalization 和 Python 的 jsoncanon 一致
        assert_eq!(result, r#"{"a":1,"b":"test","c":null}"#);
    }
}
