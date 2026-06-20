//! # VERIDACTUS Core Library
//!
//! 可信 AI 执行治理基础设施的核心库实现。
//! 遵循 VERIDACTUS Protocol Specification v0.2.1。
//!
//! ## 模块结构
//!
//! - `types` - 核心数据类型（Trace、Journal、Proof 等）
//! - `crypto` - 密码学原语（JCS 规范化、签名、UTF-8 安全处理）
//! - `http` - HTTP/SSE 服务器、头部解析
//! - `plugin` - 治理插件框架
//! - `audit` - 审计令牌验证
//! - `conformance` - 一致性测试工具

pub mod types;
pub mod crypto;
pub mod http;
pub mod plugin;
pub mod pipeline;
pub mod audit;
pub mod auth;
pub mod store;
pub mod privacy;
pub mod replay;
pub mod verify;
pub mod conformance;
pub mod delegation;
pub mod keymanager;
pub mod longrunning;
pub mod dispatcher;
pub mod attestation;
pub mod configsync;
pub mod hooks;
pub mod constraints;
pub mod observability;
pub mod diff;
pub mod middleware;
pub mod prevention;
pub mod supply_chain;
pub mod governance_dsl;
pub mod fairness;
pub mod redteam;
pub mod guarantees;
pub mod gdpr;
pub mod compliance;
pub mod agent_chain;
pub mod budget;

/// VERIDACTUS 协议版本常量
pub const PROTOCOL_VERSION: &str = "0.2.1";

/// 支持的协议版本范围
pub const PROTOCOL_VERSION_MIN: &str = "0.1";
pub const PROTOCOL_VERSION_MAX: &str = "0.2";

/// 默认超时设置
pub const DEFAULT_ZK_PROOF_TIMEOUT_MS: u64 = 5000;
pub const DEFAULT_TEE_CACHE_TTL_SECONDS: u64 = 300;
pub const DEFAULT_BUDGET_BUFFER_RATIO: f64 = 0.001;
