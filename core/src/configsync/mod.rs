//! # 配置同步 (M09)
//!
//! 长轮询机制: 数据平面定期向控制平面拉取配置变更。
//! 遵循 AI.md §2.1 Config Sync 架构描述。

pub mod client;
pub mod longpoll;

pub use client::*;
pub use longpoll::*;
