//! # Trace 验证模块
//!
//! 提供 Trace 验证 API，用于独立验证 Trace 的 L0 签名完整性。

pub mod verifier;

pub use verifier::*;
