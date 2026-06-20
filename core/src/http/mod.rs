//! # HTTP 模块
//!
//! 提供 VERIDACTUS HTTP/SSE 服务器和头部解析功能。
//! 兼容 OpenAI Chat Completions API 接口。

pub mod error_handler;
pub mod handlers;
pub mod headers;
pub mod helpers;
pub mod regex_registry;
pub mod server;
pub mod streaming;
