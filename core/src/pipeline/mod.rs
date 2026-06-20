//! # 治理流水线引擎 (Pipeline Engine)
//!
//! 严格遵循 AI.md §6.3-§6.5。
//! 编译 DAG 执行计划，执行插件流水线（并行/串行混合）。

pub mod compiler;
pub mod config;
pub mod executor;

pub use compiler::*;
pub use config::*;
pub use executor::*;
