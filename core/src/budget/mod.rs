//! # 预算管理模块
//!
//! 协议 §5.0 预算控制：预估 + 硬截断 + 实际成本追踪
//!
//! ## 架构
//! - `cost_estimation`: pre-flight 成本预估（token count × pricing）
//! - 集成到 `server.rs` 的 `handle_chat_completion` 预检阶段

pub mod cost_estimation;
pub mod stream_guard;

