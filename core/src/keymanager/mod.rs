//! # 密钥管理器 (M15)
//!
//! 严格遵循 AI.md §8.1 密钥管理生命周期设计。
//! 管理签名密钥的生成、轮换、存储和审计。

pub mod manager;

pub use manager::*;
