//! # HTTP 模块
//!
//! 提供 VERIDACTUS HTTP/SSE 服务器和头部解析功能。
//! 兼容 OpenAI Chat Completions API 接口。

pub mod headers;
pub mod server;
pub mod error_handler;
pub mod streaming;
pub mod helpers;
pub mod handlers;
pub mod regex_registry;
