//! # 治理插件框架（M03）
//!
//! 严格遵循 AI.md §6.0 插件化流水线设计。
//! 支持 Native (Rust)、Wasm 和 External gRPC 三种插件类型。

pub mod governance;
pub mod guardrails;
pub mod pii_detector;
pub mod output_filter;
pub mod semantic_guard;
pub mod production_plugins;

pub use governance::*;
pub use guardrails::*;
pub use pii_detector::*;
pub use output_filter::*;
pub use semantic_guard::*;
pub use production_plugins::*;
