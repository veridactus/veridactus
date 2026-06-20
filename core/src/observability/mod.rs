//! # 可观测性模块（§3.0, §10.3）
//!
//! 提供监控、追踪和日志功能的集成。
//! - otel: OpenTelemetry 分布式追踪
//! - metrics: 标签化 Prometheus 指标

pub mod otel;
pub mod metrics;

pub use otel::OtelTracer;
pub use metrics::LabeledMetrics;