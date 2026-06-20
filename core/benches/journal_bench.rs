//! # Journal 性能基准测试
//!
//! 测量 Execution Journal 操作的性能。
//! 遵循 AI.md §9.2 性能基线要求。

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use veridactus_core::types::journal::{ExecutionJournal, JournalEventType};
use veridactus_core::types::Action;
use std::collections::BTreeMap;

/// 基准测试：Journal 创建
fn bench_journal_creation(c: &mut Criterion) {
    c.bench_function("journal_creation", |b| {
        b.iter(|| {
            let trace_id = uuid::Uuid::new_v4();
            black_box(ExecutionJournal::new(trace_id, "bench-tenant"));
        });
    });
}

/// 基准测试：事件追加
fn bench_event_append(c: &mut Criterion) {
    let trace_id = uuid::Uuid::new_v4();
    let mut journal = ExecutionJournal::new(trace_id, "bench-tenant");

    c.bench_function("journal_event_append", |b| {
        b.iter(|| {
            let event = JournalEventType::PluginDecision {
                plugin_name: "budget".to_string(),
                action: Action::Continue,
                latency_us: 10,
            };
            black_box(journal.append_event(event));
        });
    });
}

/// 基准测试：哈希链验证（10 个事件）
fn bench_chain_verification(c: &mut Criterion) {
    let trace_id = uuid::Uuid::new_v4();
    let mut journal = ExecutionJournal::new(trace_id, "bench-tenant");

    // 追加 10 个事件
    let mut headers = BTreeMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    
    for i in 0..10 {
        let event = if i % 2 == 0 {
            JournalEventType::RequestReceived {
                method: "POST".to_string(),
                path: "/v1/chat/completions".to_string(),
                headers: headers.clone(),
                body_hash: format!("body_hash_{}", i),
            }
        } else {
            JournalEventType::PluginDecision {
                plugin_name: "budget".to_string(),
                action: Action::Continue,
                latency_us: 15,
            }
        };
        journal.append_event(event);
    }

    c.bench_function("journal_chain_verification_10_events", |b| {
        b.iter(|| {
            black_box(journal.verify_chain().unwrap());
        });
    });
}

/// 基准测试：L0 签名生成
fn bench_l0_signature_generation(c: &mut Criterion) {
    use veridactus_core::crypto::signature::generate_l0_proof;
    use veridactus_core::types::trace::{Input, Output, Trace};
    use veridactus_core::types::proof::Proofs;
    use uuid::Uuid;

    let mut trace = Trace {
        trace_id: Uuid::new_v4(),
        parent_id: None,
        session_id: None,
        tenant_id: Some("bench".to_string()),
        execution_state: None,
        model: "openai/gpt-4o".to_string(),
        engine_determinism: None,
        input: Some(Input {
            prompt: Some(serde_json::json!([{"role": "user", "content": "Hello, how are you?"}])),
            params: Some(serde_json::json!({"temperature": 0.7, "max_tokens": 100})),
            metadata: None,
        }),
        output: Some(Output {
            response: Some(serde_json::json!("I'm doing well, thank you for asking!")),
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

    c.bench_function("l0_signature_generation_trace_under_50KB", |b| {
        b.iter(|| {
            let proof = generate_l0_proof(&mut trace);
            black_box(proof);
        });
    });
}

/// 基准测试：JCS 规范化
fn bench_jcs_canonicalization(c: &mut Criterion) {
    use veridactus_core::crypto::jcs::jcs_canonicalize;

    let value = serde_json::json!({
        "trace_id": "550e8400-e29b-41d4-a716-446655440000",
        "model": "openai/gpt-4o",
        "created_at": "2026-05-12T10:00:00Z",
        "proofs": {
            "proof_chain": [
                {
                    "level": "L0",
                    "type": "hash_chain",
                    "signature": "",
                    "canonicalization_method": "rfc8785"
                }
            ]
        },
        "input": {
            "prompt": [{"role": "user", "content": "Hello"}],
            "params": {"temperature": 0.7}
        },
        "output": {
            "response": "Hi there!",
            "truncated": false,
            "finish_reason": "stop"
        }
    });

    c.bench_function("jcs_canonicalization", |b| {
        b.iter(|| {
            black_box(jcs_canonicalize(&value));
        });
    });
}

criterion_group!(
    benches,
    bench_journal_creation,
    bench_event_append,
    bench_chain_verification,
    bench_l0_signature_generation,
    bench_jcs_canonicalization,
);
criterion_main!(benches);
