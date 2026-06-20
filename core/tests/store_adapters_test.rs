//! # 存储适配器集成测试
//!
//! 测试 InMemoryTraceStore 和 HybridTraceStore 的 CRUD 操作。

use uuid::Uuid;
use veridactus_core::store::adapters::memory::InMemoryTraceStore;
use veridactus_core::store::TraceStore;
use veridactus_core::types::trace::Trace;
use veridactus_core::types::proof::{Proofs, ProofChainEntry, ProofLevel, ProofType};

fn create_test_trace(id: &str, tenant: &str) -> Trace {
    let mut trace = Trace::new("glm-5.1".to_string());
    trace.trace_id = Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4());
    trace.tenant_id = Some(tenant.to_string());
    trace.proofs = Proofs {
        proof_chain: vec![ProofChainEntry {
            level: ProofLevel::L0,
            r#type: ProofType::HashChain,
            signature: Some("test_sig".to_string()),
            signature_pq: None, attestation_quote: None,
            model_fingerprint: None, platform: None, mrenclave: None,
            merkle_root: None, sampling_paths: None,
            zk_proof: None, verification_key_hash: None,
            proof_aggregation_root: None,
            canonicalization_method: "rfc8785".to_string(),
        }],
        aggregated_root: None,
    };
    trace
}

#[tokio::test]
async fn test_memory_store_save_and_get() {
    let store = InMemoryTraceStore::new();
    let trace = create_test_trace("00000000-0000-0000-0000-000000000001", "tenant-a");

    store.save(trace.clone()).await.expect("save failed");
    let retrieved = store.get(&trace.trace_id).await;
    assert!(retrieved.is_some(), "应能获取已存储的 Trace");
    assert_eq!(retrieved.unwrap().trace_id, trace.trace_id);
}

#[tokio::test]
async fn test_memory_store_list_by_tenant() {
    let store = InMemoryTraceStore::new();
    let t1 = create_test_trace("00000000-0000-0000-0000-000000000002", "tenant-a");
    let t2 = create_test_trace("00000000-0000-0000-0000-000000000003", "tenant-b");

    store.save(t1).await.unwrap();
    store.save(t2).await.unwrap();

    let a = store.list(Some("tenant-a"), 100, 0).await;
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].tenant_id, Some("tenant-a".to_string()));

    let all = store.list(None, 100, 0).await;
    assert!(all.len() >= 2);
}

#[tokio::test]
async fn test_memory_store_delete() {
    let store = InMemoryTraceStore::new();
    let trace = create_test_trace("00000000-0000-0000-0000-000000000004", "tenant-c");
    store.save(trace.clone()).await.unwrap();

    let deleted = store.delete(&trace.trace_id).await.expect("delete failed");
    assert!(deleted.is_some());

    let after = store.get(&trace.trace_id).await;
    assert!(after.is_none(), "删除后应无法获取");
}

#[tokio::test]
async fn test_memory_store_count() {
    let store = InMemoryTraceStore::new();
    assert_eq!(store.count(None).await, 0);

    for i in 0..5 {
        let trace = create_test_trace(
            &format!("00000000-0000-0000-0000-00000000000{}", i + 10),
            "tenant-x",
        );
        store.save(trace).await.unwrap();
    }

    assert_eq!(store.count(None).await, 5);
    assert_eq!(store.count(Some("tenant-x")).await, 5);
    assert_eq!(store.count(Some("nonexistent")).await, 0);
}

#[tokio::test]
async fn test_memory_store_lru_eviction() {
    // 使用小容量测试 LRU 淘汰
    let store = InMemoryTraceStore::new().with_capacity(100);

    // 存储 150 个 trace，应淘汰最旧的
    for i in 0..150 {
        let trace = create_test_trace(&format!("00000000-0000-0000-0000-{:012}", i), "lru-tenant");
        store.save(trace).await.unwrap();
    }

    let count = store.count(None).await;
    assert!(count <= 125, "LRU 淘汰后应≤125: 实际 {}", count);
    assert!(count >= 75, "不应淘汰太多: 实际 {}", count);
}

#[tokio::test]
async fn test_memory_store_delete_by_session() {
    let store = InMemoryTraceStore::new();
    let session_id = Uuid::new_v4();

    for i in 0..3 {
        let mut trace = create_test_trace(
            &format!("00000000-0000-0000-0000-{:012}", i + 100),
            "session-tenant",
        );
        trace.session_id = Some(session_id);
        store.save(trace).await.unwrap();
    }

    let deleted = store
        .delete_by_session(&session_id)
        .await
        .expect("delete by session failed");
    assert_eq!(deleted.len(), 3);
}

#[tokio::test]
async fn test_memory_store_clone_is_empty() {
    let store = InMemoryTraceStore::new();
    let trace = create_test_trace("00000000-0000-0000-0000-000000000020", "clone-tenant");
    store.save(trace).await.unwrap();

    // Clone 创建一个新的空 store（这是适配器的设计）
    let cloned = store.clone();
    assert_eq!(cloned.count(None).await, 0, "Clone 应为独立空 store");
    assert_eq!(store.count(None).await, 1, "原始 store 不受影响");
}
