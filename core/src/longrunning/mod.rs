//! # 长周期 Trace 管理器 (M16)
//!
//! 严格遵循 AI.md §8.2。
//! 管理长周期执行的 Trace，支持分段存储和异步结果聚合。

pub mod manager;

pub use manager::*;
