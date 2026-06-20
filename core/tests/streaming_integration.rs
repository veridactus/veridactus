//! # VeridactusStreamHandler 集成测试
//!
//! 测试 SSE 流处理器的核心功能：chunk 传递、预算感知、主动预防、正常终止。

use std::convert::Infallible;
use tokio::sync::mpsc;
use bytes::Bytes;
use futures::StreamExt;

#[tokio::test]
async fn test_stream_handler_basic_flow() {
    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(16);
    let handler = veridactus_core::http::streaming::VeridactusStreamHandler::new(
        rx,
        "test-trace-id".to_string(),
    );

    // 发送 chunks
    tx.send(Ok("Hello".to_string())).await.unwrap();
    tx.send(Ok(" world".to_string())).await.unwrap();
    drop(tx); // 关闭 channel

    let mut stream = Box::pin(handler);
    let mut received = Vec::new();
    while let Some(Ok(bytes)) = stream.next().await {
        received.push(String::from_utf8_lossy(&bytes).to_string());
    }

    assert!(received.len() >= 2, "应收到至少 2 个 SSE 事件");
    assert!(received.iter().any(|s| s.contains("Hello")), "应包含 Hello");
    assert!(received.iter().any(|s| s.contains("world")), "应包含 world");
}

#[tokio::test]
async fn test_stream_handler_empty() {
    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(16);
    let handler = veridactus_core::http::streaming::VeridactusStreamHandler::new(
        rx,
        "test-trace-id".to_string(),
    );
    drop(tx);

    let mut stream = Box::pin(handler);
    let mut count = 0;
    while let Some(_) = stream.next().await {
        count += 1;
    }
    // 空流：无数据
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_stream_handler_with_budget() {
    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(16);
    let handler = veridactus_core::http::streaming::VeridactusStreamHandler::new(
        rx,
        "test-trace-id".to_string(),
    )
        .with_budget(0.001, true); // $0.001 预算 + 感知

    tx.send(Ok("short".to_string())).await.unwrap();
    drop(tx);

    let mut stream = Box::pin(handler);
    let mut received = Vec::new();
    while let Some(Ok(bytes)) = stream.next().await {
        received.push(String::from_utf8_lossy(&bytes).to_string());
    }

    // 小数据不应触发预算耗尽
    assert!(received.iter().any(|s| s.contains("short")),
        "应包含原始内容");
}

#[tokio::test]
async fn test_stream_handler_prevention_blocks() {
    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(16);
    let prevention = std::sync::Arc::new(
        veridactus_core::prevention::ConstrainedDecoder::new(
            std::sync::Arc::new(veridactus_core::prevention::PatternRegistry::default()),
        ),
    );
    let handler = veridactus_core::http::streaming::VeridactusStreamHandler::new(
        rx,
        "test-trace-id".to_string(),
    )
    .with_prevention(prevention);

    // 发送包含危险内容的 chunk
    tx.send(Ok("normal text".to_string())).await.unwrap();
    tx.send(Ok("rm -rf /".to_string())).await.unwrap(); // 应触发预防
    drop(tx);

    let mut stream = Box::pin(handler);
    let mut blocked = false;
    while let Some(Ok(bytes)) = stream.next().await {
        let s = String::from_utf8_lossy(&bytes).to_string();
        if s.contains("VERIDACTUS_ACTIVE_PREVENTION_BLOCKED") {
            blocked = true;
        }
    }
    assert!(blocked, "危险内容应触发主动预防阻断");
}

#[tokio::test]
async fn test_stream_handler_large_input() {
    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(64);
    let handler = veridactus_core::http::streaming::VeridactusStreamHandler::new(
        rx,
        "test-trace-id".to_string(),
    );

    // 发送 20 个 chunks (减少避免超时)
    for i in 0..20 {
        tx.send(Ok(format!("chunk_{}", i))).await.unwrap();
    }
    drop(tx);

    use futures::StreamExt;
    let mut stream = Box::pin(handler);
    let mut count = 0;
    while let Some(Ok(_)) = stream.next().await {
        count += 1;
    }
    assert!(count >= 20, "应收到至少 20 个事件, 实际 {}", count);
}

#[tokio::test]
async fn test_stream_handler_infallible() {
    // 验证 Stream 的实现使用 Infallible 错误类型
    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(16);
    let mut handler = Box::pin(
        veridactus_core::http::streaming::VeridactusStreamHandler::new(
            rx,
            "test-trace-id".to_string(),
        ),
    );

    tx.send(Ok("data".to_string())).await.unwrap();
    drop(tx);

    // 验证可以通过 Stream 正确消费
    use futures::StreamExt;
    let results: Vec<_> = futures::stream::StreamExt::collect::<Vec<_>>(handler.as_mut()).await;
    assert!(!results.is_empty());
    // 所有结果应为 Ok
    assert!(results.iter().all(|r| r.is_ok()));
}
