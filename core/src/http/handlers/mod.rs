//! # HTTP 请求处理器模块
//!
//! 治理请求处理器的子模块，按功能拆分。
//! 当前主要处理逻辑仍在 server.rs 中，后续版本将逐步迁移。

pub mod admin;
pub mod traces;
