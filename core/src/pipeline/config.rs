//! # 流水线配置类型
//!
//! 严格遵循 AI.md §6.3 流水线编译与执行计划。

use serde::{Deserialize, Serialize};

use crate::plugin::PluginType;

/// 插件放置阶段（AI.md §6.3）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Placement {
    /// 请求预处理阶段（CONSTRAINT_EVAL 期间执行）
    #[serde(rename = "pre_request")]
    PreRequest,
    /// 流式处理阶段（EXECUTING 期间执行）
    #[serde(rename = "streaming")]
    Streaming,
    /// 响应后处理阶段
    #[serde(rename = "post_response")]
    PostResponse,
    /// 异步最终化阶段（后台执行）
    #[serde(rename = "async_finalize")]
    AsyncFinalize,
}

/// 版本不匹配策略（AI.md §6.5）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum VersionMismatchPolicy {
    /// 跳过不兼容插件
    #[default]
    #[serde(rename = "skip")]
    Skip,
    /// 失败并报错
    #[serde(rename = "fail")]
    Fail,
}

/// 插件配置（DAG 中的节点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// 插件名称
    pub name: String,
    /// 插件类型
    pub r#type: PluginType,
    /// 插件配置
    pub config: serde_json::Value,
    /// 依赖的插件名称列表
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 端点（仅 gRPC 插件）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// 所需运行时能力
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

/// 阶段配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageConfig {
    /// 放置阶段
    pub placement: Placement,
    /// 是否允许并行执行
    #[serde(default)]
    pub parallel: bool,
    /// 插件列表
    pub plugins: Vec<PluginConfig>,
    /// 版本不匹配策略
    #[serde(default)]
    pub on_version_mismatch: VersionMismatchPolicy,
}

/// 执行计划（AI.md §6.3）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// 计划 ID
    pub plan_id: String,
    /// 租户
    pub tenant: Option<String>,
    /// 阶段列表
    pub stages: Vec<StageConfig>,
}

impl ExecutionPlan {
    /// 创建默认执行计划（含预算和认证插件）
    pub fn default_plan() -> Self {
        Self {
            plan_id: "default-plan".to_string(),
            tenant: Some("default".to_string()),
            stages: vec![
                StageConfig {
                    placement: Placement::PreRequest,
                    parallel: false,
                    plugins: vec![PluginConfig {
                        name: "budget-guard".to_string(),
                        r#type: PluginType::Native,
                        config: serde_json::json!({"limit_usd": 100.0, "strategy": "hard_stop"}),
                        depends_on: vec![],
                        endpoint: None,
                        required_capabilities: vec![],
                    }],
                    on_version_mismatch: VersionMismatchPolicy::Skip,
                },
                StageConfig {
                    placement: Placement::Streaming,
                    parallel: true,
                    plugins: vec![],
                    on_version_mismatch: VersionMismatchPolicy::Skip,
                },
            ],
        }
    }
}

/// Wasm 沙箱资源限制（AI.md §6.1.1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSandboxLimits {
    /// 最大内存页数（每页 64KB）
    pub memory_max_pages: u32,
    /// 每次请求的燃料
    pub fuel_per_request: u64,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
    /// 允许的系统调用
    pub allowed_syscalls: Vec<String>,
}

impl Default for WasmSandboxLimits {
    fn default() -> Self {
        Self {
            memory_max_pages: 256,        // 16MB
            fuel_per_request: 10_000_000, // ~100ms CPU
            timeout_ms: 500,
            allowed_syscalls: vec![],
        }
    }
}

/// VERIDACTUS 标准能力定义（§4.4）
pub mod capabilities {
    /// 核心能力（所有实现必须支持）
    pub const CORE_STREAMING: &str = "core:streaming";
    pub const CORE_TRACE: &str = "core:trace";
    pub const CORE_BUDGET: &str = "core:budget";

    /// 隐私能力
    pub const PRIVACY_MASKED: &str = "privacy:masked";
    pub const PRIVACY_HASH_ONLY: &str = "privacy:hash_only";
    pub const PRIVACY_TEE_PRIVATE: &str = "privacy:tee_private";

    /// Guardrail 能力
    pub const GUARDRAIL_G1: &str = "guardrail:G1";
    pub const GUARDRAIL_G2: &str = "guardrail:G2";
    pub const GUARDRAIL_G3: &str = "guardrail:G3";
    pub const GUARDRAIL_G4: &str = "guardrail:G4";

    /// 可重现性能力
    pub const REPRO_BOUNDED: &str = "repro:bounded";
    pub const REPRO_STRICT: &str = "repro:strict";

    /// 证明能力
    pub const PROOF_L0: &str = "proof:L0";
    pub const PROOF_L1: &str = "proof:L1";
    pub const PROOF_L2A: &str = "proof:L2A";
    pub const PROOF_L2B: &str = "proof:L2B";

    /// 高级能力
    pub const CAPABILITY_NEGOTIATION: &str = "capability:negotiation";
    pub const CERTIFIED_GUARANTEE: &str = "advanced:certified_guarantee";
    pub const DIFF_REPORT: &str = "advanced:diff_report";
    pub const DRIFT_DETECTION: &str = "advanced:drift_detection";
    pub const REPLAY_ENGINE: &str = "advanced:replay_engine";
    pub const CONSTRAINED_DECODING: &str = "advanced:constrained_decoding";
    pub const COMPLIANCE_EU_AI_ACT: &str = "compliance:EU_AI_ACT_GPAI";
    pub const COMPLIANCE_NIST_AI: &str = "compliance:NIST_AI_600_1";

    /// 获取服务器默认支持的能力列表
    pub fn default_server_capabilities() -> Vec<String> {
        vec![
            CORE_STREAMING.to_string(),
            CORE_TRACE.to_string(),
            CORE_BUDGET.to_string(),
            PRIVACY_MASKED.to_string(),
            PRIVACY_HASH_ONLY.to_string(),
            GUARDRAIL_G1.to_string(),
            GUARDRAIL_G2.to_string(),
            GUARDRAIL_G3.to_string(),
            GUARDRAIL_G4.to_string(),
            REPRO_BOUNDED.to_string(),
            PROOF_L0.to_string(),
            CAPABILITY_NEGOTIATION.to_string(),
            DIFF_REPORT.to_string(),
            DRIFT_DETECTION.to_string(),
        ]
    }

    /// 获取客户端必须支持的能力列表（核心能力）
    pub fn required_client_capabilities() -> Vec<String> {
        vec![CORE_STREAMING.to_string(), CORE_TRACE.to_string()]
    }

    /// 解析能力字符串为类别
    pub fn parse_capability_category(cap: &str) -> Option<&'static str> {
        if cap.starts_with("core:") {
            Some("core")
        } else if cap.starts_with("privacy:") {
            Some("privacy")
        } else if cap.starts_with("guardrail:") {
            Some("guardrail")
        } else if cap.starts_with("repro:") {
            Some("repro")
        } else if cap.starts_with("proof:") {
            Some("proof")
        } else if cap.starts_with("advanced:") {
            Some("advanced")
        } else if cap.starts_with("compliance:") {
            Some("compliance")
        } else {
            None
        }
    }

    /// 检查能力是否兼容
    pub fn capabilities_compatible(client_caps: &[String], server_caps: &[String]) -> Vec<String> {
        client_caps
            .iter()
            .filter(|cap| server_caps.contains(cap))
            .cloned()
            .collect()
    }
}

/// 能力协商结果
#[derive(Debug, Clone)]
pub struct CapabilityNegotiationResult {
    /// 是否协商成功
    pub success: bool,
    /// 服务器接受的能力列表
    pub accepted: Vec<String>,
    /// 协商失败的原因（如果有）
    pub error_message: Option<String>,
    /// 缺失的核心能力（如果有）
    pub missing_core: Vec<String>,
}

impl Default for CapabilityNegotiationResult {
    fn default() -> Self {
        Self {
            success: true,
            accepted: Vec::new(),
            error_message: None,
            missing_core: Vec::new(),
        }
    }
}

/// 运行时能力
#[derive(Debug, Clone, Default)]
pub struct RuntimeCapabilities {
    /// 协议版本
    pub protocol_version: String,
    /// 可用能力列表
    pub capabilities: Vec<String>,
}

impl RuntimeCapabilities {
    /// 检查是否包含某能力
    pub fn contains(&self, cap: &str) -> bool {
        self.capabilities.contains(&cap.to_string())
    }

    /// 执行能力协商（§4.4）
    ///
    /// # 参数
    /// * `client_capabilities` - 客户端请求的能力列表
    ///
    /// # 返回
    /// 协商结果
    pub fn negotiate(&self, client_capabilities: &[String]) -> CapabilityNegotiationResult {
        use capabilities::*;

        // 检查客户端是否包含必需的核心能力
        let required = required_client_capabilities();
        let missing_core: Vec<String> = required
            .iter()
            .filter(|r| !client_capabilities.contains(r))
            .cloned()
            .collect();

        if !missing_core.is_empty() {
            return CapabilityNegotiationResult {
                success: false,
                accepted: vec![],
                error_message: Some(format!(
                    "Missing required capabilities: {}",
                    missing_core.join(", ")
                )),
                missing_core,
            };
        }

        // 计算双方都支持的能力
        let accepted = capabilities_compatible(client_capabilities, &self.capabilities);

        // 如果客户端请求的任何能力都不被服务器支持，返回错误
        if !accepted.is_empty() || client_capabilities.is_empty() {
            CapabilityNegotiationResult {
                success: true,
                accepted,
                error_message: None,
                missing_core: vec![],
            }
        } else {
            CapabilityNegotiationResult {
                success: false,
                accepted: vec![],
                error_message: Some("No compatible capabilities found".to_string()),
                missing_core: vec![],
            }
        }
    }
}
