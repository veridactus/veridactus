//! # 密码学原语模块
//!
//! 提供 VERIDACTUS 所需的密码学操作：
//! - JCS 规范化（RFC 8785 JSON Canonicalization Scheme）
//! - UTF-8 安全处理
//! - L0 签名生成与验证

pub mod jcs;
pub mod utf8;
pub mod signature;
pub mod merkle;
pub mod zk;
