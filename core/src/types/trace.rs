//! # Trace 数据类型
//!
//! 严格遵循 VERIDACTUS v0.2.1 §3.0 Data Model & Trace Schema。
//! Trace 是整个系统的核心数据对象，记录单次 LLM 执行的完整生命周期。

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::constraints::ConstraintsApplied;
use super::error::ErrorObject;
use super::proof::Proofs;
use super::{RedTeamEvent, SafetyEvent};

/// 执行状态枚举，对应协议 §6.1 状态机
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionState {
    /// 请求接收、上下文初始化
    #[serde(rename = "INIT")]
    Init,
    /// （可选）跨代理委托验证
    #[serde(rename = "DELEGATION_VALIDATE")]
    DelegationValidate,
    /// 静态约束检查与预算预占
    #[serde(rename = "CONSTRAINT_EVAL")]
    ConstraintEval,
    /// 转发到上游 LLM，流式生成中
    #[serde(rename = "EXECUTING")]
    Executing,
    /// 输出验证与风险评估
    #[serde(rename = "VALIDATION")]
    Validation,
    /// 证明生成、持久化完成（终态）
    #[serde(rename = "FINALIZED")]
    Finalized,
    /// 约束阻断或上游异常（终态）
    #[serde(rename = "FAILED")]
    Failed,
}

/// 状态转换记录（§3.0 $defs.state_transition）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// 起始状态
    pub from: ExecutionState,
    /// 目标状态
    pub to: ExecutionState,
    /// 转换时间戳（RFC 3339）
    pub timestamp: String,
    /// 逻辑序列号（用于分布式部署的主排序）
    pub transition_index: u32,
}

/// 输入快照（§3.0 input）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Input {
    /// 用户提示（字符串或消息数组）
    pub prompt: Option<serde_json::Value>,
    /// 请求参数（model, temperature, max_tokens 等）
    pub params: Option<serde_json::Value>,
    /// 业务元数据
    pub metadata: Option<serde_json::Value>,
}

/// 输出快照（§3.0 output）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Output {
    /// 最终生成内容
    pub response: Option<serde_json::Value>,
    /// 是否被截断
    #[serde(default)]
    pub truncated: bool,
    /// 完成原因
    pub finish_reason: Option<String>,
}

/// 预算感知事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAwarenessEvent {
    /// 事件时间戳
    pub timestamp: String,
    /// 剩余预算
    pub budget_remaining: f64,
    /// 预算百分比
    pub budget_pct: f64,
}

/// 预算感知信息（§3.0 observations.budget_awareness）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetAwareness {
    /// SSE 预算事件
    pub sse_events: Option<Vec<BudgetAwarenessEvent>>,
    /// 注入的提示后缀
    pub injected_prompt_suffix: Option<String>,
}

/// 指令层次结构日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionHierarchyLog {
    /// 有效指令树
    pub effective_tree: serde_json::Value,
    /// 被抑制的指令
    pub suppressed_instructions: Vec<String>,
}

/// 经认证的保证（§3.0 observations.certified_guarantee）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedGuarantee {
    /// 方法论标识符
    pub methodology: String,
    /// 风险上界
    pub risk_bound: f64,
    /// 置信水平
    pub confidence_level: f64,
    /// 已验证的安全声明
    pub claim_verified: String,
    /// 计算完成时间
    pub generated_at: String,
}

/// 公平性检查结果（§3.0 observations.fairness_check）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FairnessCheck {
    /// 检查是否通过
    pub passed: Option<bool>,
    /// 公平性得分（0-1，越高越公平）
    pub fairness_score: Option<f64>,
    /// 受保护属性列表
    pub protected_attributes: Option<Vec<String>>,
    /// 各属性的公平性指标
    pub metrics: Option<Vec<FairnessMetric>>,
    /// 偏差检测结果
    pub bias_detection: Option<BiasDetection>,
    /// 检查时间
    pub checked_at: Option<String>,
}

/// 公平性指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FairnessMetric {
    /// 属性名称
    pub attribute: String,
    /// 指标类型（如 demographic_parity, equalized_odds 等）
    pub metric_type: String,
    /// 测量值
    pub value: f64,
    /// 是否通过阈值
    pub passed: bool,
    /// 阈值
    pub threshold: f64,
}

/// 偏差检测结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiasDetection {
    /// 是否检测到偏差
    pub detected: bool,
    /// 偏差类型
    pub bias_type: Option<String>,
    /// 受影响的群体
    pub affected_groups: Option<Vec<String>>,
    /// 建议的缓解措施
    pub mitigation_suggestion: Option<String>,
}

/// 重放快照（§9.4.2），以 _ 开头字段将被排除在签名之外
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReplaySnapshot {
    /// 重放模式
    #[serde(rename = "_mode")]
    pub mode: Option<String>,
    /// 交互记录
    #[serde(rename = "_interactions")]
    pub interactions: Option<Vec<ReplayInteraction>>,
    /// 环境快照
    #[serde(rename = "_environment_snapshot")]
    pub environment_snapshot: Option<EnvironmentSnapshot>,
}

/// 重放交互记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayInteraction {
    /// 序号
    pub sequence: u64,
    /// 模型标识符
    pub model: String,
    /// 提示哈希
    pub prompt_hash: String,
    /// 响应哈希
    pub response_hash: String,
    /// 使用的 token 数
    pub tokens_used: u64,
    /// 延迟（毫秒）
    pub latency_ms: u64,
}

/// 环境快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    /// 模型版本
    pub model_version: String,
    /// SDK 版本
    pub sdk_version: String,
    /// 引擎确定性策略
    pub engine_determinism_strategy: Option<String>,
    /// 记录时间
    pub recorded_at: String,
}

/// 执行图快照
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionGraphSnapshot {
    /// 节点
    pub nodes: Option<Vec<serde_json::Value>>,
    /// 边
    pub edges: Option<Vec<serde_json::Value>>,
}

/// 异常检测信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DriftDetection {
    /// 提示漂移
    pub prompt_drift: Option<bool>,
    /// 响应漂移
    pub response_drift: Option<bool>,
    /// 嵌入漂移
    pub embedding_drift: Option<bool>,
}

/// 监控信息（§3.0 observations.monitoring）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Monitoring {
    /// OTel trace ID
    pub otel_trace_id: Option<String>,
    /// 执行图快照
    pub execution_graph_snapshot: Option<ExecutionGraphSnapshot>,
    /// 异常得分
    pub anomaly_score: Option<f64>,
    /// 漂移检测
    pub drift_detection: Option<DriftDetection>,
}

/// 批准记录（§3.0 observations.approval）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    /// 审批人
    pub approved_by: String,
    /// 审批时间
    pub approved_at: String,
    /// 理由
    pub rationale: String,
    /// 被门控的操作
    pub action_gated: String,
}

/// 观测数据（§3.0 observations）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Observations {
    /// token 计数
    pub tokens_count: Option<u64>,
    /// 预估成本（美元，6 位小数微美元精度）
    pub cost_estimated_usd: Option<f64>,
    /// 延迟（毫秒）
    pub latency_ms: Option<u64>,
    /// 状态转换记录
    pub state_transitions: Option<Vec<StateTransition>>,
    /// 错误信息
    pub error: Option<ErrorObject>,
    /// 运行时监控数据
    pub monitoring: Option<Monitoring>,
    /// 预算感知事件
    pub budget_awareness: Option<BudgetAwareness>,
    /// 安全事件
    pub safety_events: Option<Vec<SafetyEvent>>,
    /// 红队事件
    pub red_team_events: Option<Vec<RedTeamEvent>>,
    /// 经认证的保证
    pub certified_guarantee: Option<CertifiedGuarantee>,
    /// 公平性检查结果
    pub fairness_check: Option<FairnessCheck>,
    /// 重放快照（以 _ 开头，排除在签名外）
    pub replay_snapshot: Option<ReplaySnapshot>,
    /// 批准记录
    pub approval: Option<Approval>,
    /// 内部指标（以 _internal 开头，排除在签名外）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _internal_metrics: Option<serde_json::Value>,
}

/// 供应链接口（§3.0 supply_chain）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SupplyChain {
    /// 模型信息
    pub model: Option<serde_json::Value>,
    /// 推理引擎
    pub inference_engine: Option<serde_json::Value>,
    /// 部署信息
    pub deployment: Option<serde_json::Value>,
    /// 软件物料清单
    pub sbom: Option<Vec<serde_json::Value>>,
}

/// 代理执行链（§3.0 agent_execution_chain）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionChain {
    /// 链 ID
    pub chain_id: Uuid,
    /// 根 Trace ID
    pub root_trace_id: Uuid,
    /// Merkle 根
    pub merkle_root: Option<String>,
    /// 参与代理
    pub agents: Vec<serde_json::Value>,
    /// 边关系
    pub edges: Vec<serde_json::Value>,
}

/// 委托链（§3.0 delegation_chain）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationChain {
    /// 根主体
    pub root_principal: String,
    /// 委托路径
    pub delegation_path: Vec<serde_json::Value>,
    /// 链 Merkle 根
    pub chain_merkle_root: Option<String>,
}

/// 合规映射（§3.0 compliance_mappings）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceMapping {
    /// 法规名称
    pub regulation: String,
    /// 法律条款
    pub article: Option<String>,
    /// 要求描述
    pub requirement: String,
    /// 关联的 Trace 字段
    pub trace_field: String,
    /// 满足状态
    pub satisfaction: String,
}

/// 引擎确定性声明（§3.0 engine_determinism）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineDeterminism {
    /// 确定性策略
    pub strategy: String,
    /// 框架版本
    pub framework_version: Option<String>,
    /// 是否保证比特级一致性
    pub bitwise_guarantee: Option<bool>,
}

/// 主 Trace 结构体（§3.0 Core Trace Structure）
///
/// 这是 VERIDACTUS 系统的核心数据对象，记录单次 LLM 执行的完整生命周期。
/// 严格遵循 trace-schema.json v0.2.1。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    /// 全局唯一执行标识符（RFC 4122 UUID）
    pub trace_id: Uuid,
    /// 父 Trace ID（用于重放谱系）
    pub parent_id: Option<Uuid>,
    /// 会话 ID（多轮或批量请求的逻辑分组）
    pub session_id: Option<Uuid>,
    /// 多租户标识符
    pub tenant_id: Option<String>,
    /// 当前执行状态
    pub execution_state: Option<ExecutionState>,
    /// 提供商/模型标识符（格式: vendor/model）
    pub model: String,
    /// 引擎确定性声明
    pub engine_determinism: Option<EngineDeterminism>,
    /// 输入快照
    pub input: Option<Input>,
    /// 输出快照
    pub output: Option<Output>,
    /// 观测数据
    pub observations: Option<Observations>,
    /// 密码学证明链
    pub proofs: Proofs,
    /// 应用的约束快照
    pub constraints_applied: Option<ConstraintsApplied>,
    /// 供应链接口证明
    pub supply_chain: Option<SupplyChain>,
    /// 多代理执行链
    pub agent_execution_chain: Option<AgentExecutionChain>,
    /// 委托链
    pub delegation_chain: Option<DelegationChain>,
    /// 合规映射
    pub compliance_mappings: Option<Vec<ComplianceMapping>>,
    /// 创建时间（RFC 3339）
    pub created_at: String,
    /// TTL 过期时间
    pub ttl_expire_at: Option<String>,
    /// 命名空间隔离的扩展字段
    pub extensions: Option<serde_json::Value>,
}

impl Trace {
    /// 创建一个新的 Trace（仅必需字段）
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            parent_id: None,
            session_id: None,
            tenant_id: None,
            execution_state: Some(ExecutionState::Init),
            model: model.into(),
            engine_determinism: None,
            input: None,
            output: None,
            observations: None,
            proofs: Proofs::default(),
            constraints_applied: None,
            supply_chain: None,
            agent_execution_chain: None,
            delegation_chain: None,
            compliance_mappings: None,
            created_at: Utc::now().to_rfc3339(),
            ttl_expire_at: None,
            extensions: None,
        }
    }
}
