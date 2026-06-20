//! # 可观测性模块（§3.0, §10.3）
//!
//! 提供监控、追踪和日志功能的集成。
//! - otel: OpenTelemetry 分布式追踪
//! - metrics: 标签化 Prometheus 指标

pub mod metrics;
pub mod otel;

pub use metrics::LabeledMetrics;
pub use otel::OtelTracer;
