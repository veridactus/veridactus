//! # 证明生成模块 — L0/L1/L2A/L2B
//!
//! L0: 哈希链 (已有的 crypto/signature)
//! L1: 软件认证 (Ed25519 签名替代 TEE)
//! L2A: 采样验证 (Merkle 承诺)
//! L2B: ZK 证明框架
//!
//! TEE 说明: Intel TDX/AMD SEV 需要专用硬件。
//! 为在无 TEE 硬件时提供 L1 证明能力，使用 Ed25519 签名作为"软件级认证"。
//! 这提供与 TEE 相同的 API 接口，但基于可信签名密钥而非硬件飞地。

pub mod software_tee;
/// 硬件 TEE/ZK/Sigstore 接口（等待硬件 SDK 集成）
pub mod hardware;

pub use software_tee::*;
