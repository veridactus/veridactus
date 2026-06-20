//! # 委托验证模块 (M12)
//!
//! 严格遵循 AI.md §5.5 复合委托令牌验证器。
//! 支持 Ed25519 签名、TEE Quote 和 ZK Proof 三种认证类型。

pub mod validator;

pub use validator::*;
