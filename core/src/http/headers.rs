//! # VERIDACTUS HTTP Headers 定义
//!
//! 严格遵循 VERIDACTUS v0.2.1 §4.0 Transport & Headers。
//! 定义所有 VERIDACTUS 请求/响应头部。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// VERIDACTUS 请求头部解析后的结构化数据
#[derive(Debug, Clone, Default)]
pub struct VeridactusRequestHeaders {
    /// 客户端支持的最高协议版本
    pub version: Option<String>,
    /// 客户端支持的扩展能力列表
    pub capabilities: Option<Vec<String>>,
    /// 触发的治理动作
    pub action: Option<VeridactusAction>,
    /// 基线 Trace ID（用于重放）
    pub baseline_ref: Option<String>,
    /// 预算上限（美元）
    pub budget_limit: Option<f64>,
    /// 预算策略
    pub budget_strategy: Option<String>,
    /// 隐私级别
    pub privacy_level: Option<String>,
    /// 是否启用差异输出比较
    pub diff_output: Option<bool>,
    /// 焦点字段（JSONPath 表达式）
    pub focus_fields: Option<Vec<String>>,
    /// 覆盖模型标识符
    pub override_model: Option<String>,
    /// 激活的守卫级别
    pub guardrails: Option<Vec<String>>,
    /// 守卫严格度
    pub guardrails_strictness: Option<String>,
    /// 指令层次模式
    pub instruction_hierarchy: Option<String>,
    /// 认证保证请求（格式: methodology:risk_bound@confidence）
    pub certified_guarantee: Option<String>,
    /// 合规配置文件
    pub compliance_profile: Option<String>,
    /// 漂移测试套件 ID
    pub drift_suite_id: Option<String>,
    /// 委托令牌（Base64 编码）
    pub trust_delegation_token: Option<String>,
    /// 审计令牌
    pub audit_token: Option<String>,
    /// ZK 证明超时（毫秒）
    pub proof_timeout: Option<u64>,
    /// 证明模式
    pub proof_mode: Option<String>,
    /// 传入的原始头部（用于 Journal 记录）
    pub raw_headers: BTreeMap<String, String>,
}

/// VERIDACTUS 动作枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VeridactusAction {
    /// 保存基线
    #[serde(rename = "save-baseline")]
    SaveBaseline,
    /// 重放
    #[serde(rename = "replay")]
    Replay,
    /// 审计导出
    #[serde(rename = "audit-export")]
    AuditExport,
    /// 漂移测试
    #[serde(rename = "drift-test")]
    DriftTest,
}

/// VERIDACTUS 响应头部构建器
#[derive(Debug, Clone, Default)]
pub struct VeridactusResponseHeaders {
    /// 协商后的协议版本
    pub version: String,
    /// 代理接受的扩展能力
    pub accepted: Option<Vec<String>>,
    /// Trace ID
    pub trace_id: String,
    /// 差异报告（Base64 编码）
    pub diff_report: Option<String>,
    /// 认证保证报告（Base64 编码）
    pub certified_guarantee_report: Option<String>,
    /// 实际消耗成本（美元）
    pub cost_consumed: Option<f64>,
    /// 剩余预算
    pub budget_remaining: Option<f64>,
    /// 是否截断
    pub truncated: Option<bool>,
    /// 告警信息
    pub warning: Option<String>,
    /// 包含的证明级别
    pub proof_levels: Option<Vec<String>>,
    /// ASI 风险标记列表
    pub asi_risks_flagged: Option<Vec<String>>,
}

impl VeridactusResponseHeaders {
    /// 将响应头部构建为 HTTP 头部映射
    pub fn to_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        headers.insert("VERIDACTUS-Version".to_string(), self.version.clone());

        if let Some(accepted) = &self.accepted {
            headers.insert("VERIDACTUS-Accepted".to_string(), accepted.join(","));
        }

        headers.insert("VERIDACTUS-Trace-Id".to_string(), self.trace_id.clone());

        if let Some(diff) = &self.diff_report {
            headers.insert("VERIDACTUS-Diff-Report".to_string(), diff.clone());
        }

        if let Some(cg) = &self.certified_guarantee_report {
            headers.insert(
                "VERIDACTUS-Certified-Guarantee-Report".to_string(),
                cg.clone(),
            );
        }

        if let Some(cost) = self.cost_consumed {
            headers.insert(
                "VERIDACTUS-Cost-Consumed".to_string(),
                format!("{:.6}", cost),
            );
        }

        if let Some(budget) = self.budget_remaining {
            headers.insert(
                "VERIDACTUS-Budget-Remaining".to_string(),
                format!("{:.6}", budget),
            );
        }

        if let Some(truncated) = self.truncated {
            headers.insert("VERIDACTUS-Truncated".to_string(), truncated.to_string());
        }

        if let Some(warning) = &self.warning {
            headers.insert("VERIDACTUS-Warning".to_string(), warning.clone());
        }

        if let Some(levels) = &self.proof_levels {
            headers.insert("VERIDACTUS-Proof-Levels".to_string(), levels.join(","));
        }

        if let Some(risks) = &self.asi_risks_flagged {
            headers.insert("VERIDACTUS-ASI-Risks-Flagged".to_string(), risks.join(","));
        }

        headers
    }
}

/// 解析 VERIDACTUS 请求头部
///
/// # 参数
/// * `headers` - HTTP 头部映射（BTreeMap 以保持键顺序一致性）
///
/// # 返回
/// 结构化的 VERIDACTUS 头部数据
pub fn parse_veridactus_headers(headers: &BTreeMap<String, String>) -> VeridactusRequestHeaders {
    let mut result = VeridactusRequestHeaders::default();

    for (key, value) in headers {
        let lower_key = key.to_lowercase();
        match lower_key.as_str() {
            "veridactus-version" => result.version = Some(value.clone()),
            "veridactus-capabilities" => {
                result.capabilities =
                    Some(value.split(',').map(|s| s.trim().to_string()).collect());
            }
            "veridactus-action" => {
                result.action = match value.as_str() {
                    "save-baseline" => Some(VeridactusAction::SaveBaseline),
                    "replay" => Some(VeridactusAction::Replay),
                    "audit-export" => Some(VeridactusAction::AuditExport),
                    "drift-test" => Some(VeridactusAction::DriftTest),
                    _ => None,
                };
            }
            "veridactus-baseline-ref" => result.baseline_ref = Some(value.clone()),
            "veridactus-budget-limit" => {
                result.budget_limit = value.parse::<f64>().ok();
            }
            "veridactus-budget-strategy" => result.budget_strategy = Some(value.clone()),
            "veridactus-privacy-level" => result.privacy_level = Some(value.clone()),
            "veridactus-diff-output" => {
                result.diff_output = Some(value == "true");
            }
            "veridactus-focus-fields" => {
                result.focus_fields =
                    Some(value.split(',').map(|s| s.trim().to_string()).collect());
            }
            "veridactus-override-model" => result.override_model = Some(value.clone()),
            "veridactus-guardrails" => {
                result.guardrails = Some(value.split(',').map(|s| s.trim().to_string()).collect());
            }
            "veridactus-guardrails-strictness" => {
                result.guardrails_strictness = Some(value.clone())
            }
            "veridactus-instruction-hierarchy" => {
                result.instruction_hierarchy = Some(value.clone())
            }
            "veridactus-certified-guarantee" => result.certified_guarantee = Some(value.clone()),
            "veridactus-compliance-profile" => result.compliance_profile = Some(value.clone()),
            "veridactus-drift-suite-id" => result.drift_suite_id = Some(value.clone()),
            "veridactus-trust-delegation-token" => {
                result.trust_delegation_token = Some(value.clone())
            }
            "veridactus-audit-token" => result.audit_token = Some(value.clone()),
            "veridactus-proof-timeout" => {
                result.proof_timeout = value.parse::<u64>().ok();
            }
            "veridactus-proof-mode" => result.proof_mode = Some(value.clone()),
            _ => {}
        }
    }

    result.raw_headers = headers.clone();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试头部解析
    #[test]
    fn test_parse_headers() {
        let mut headers = BTreeMap::new();
        headers.insert("VERIDACTUS-Version".to_string(), "0.2".to_string());
        headers.insert("VERIDACTUS-Budget-Limit".to_string(), "0.05".to_string());
        headers.insert("VERIDACTUS-Privacy-Level".to_string(), "masked".to_string());
        headers.insert("VERIDACTUS-Guardrails".to_string(), "G1,G2".to_string());

        let parsed = parse_veridactus_headers(&headers);

        assert_eq!(parsed.version, Some("0.2".to_string()));
        assert!((parsed.budget_limit.unwrap() - 0.05).abs() < f64::EPSILON);
        assert_eq!(parsed.privacy_level, Some("masked".to_string()));
        assert_eq!(
            parsed.guardrails,
            Some(vec!["G1".to_string(), "G2".to_string()])
        );
    }

    /// 测试未知头部被安全忽略
    #[test]
    fn test_unknown_headers_ignored() {
        let mut headers = BTreeMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());
        headers.insert("VERIDACTUS-Version".to_string(), "0.2".to_string());

        let parsed = parse_veridactus_headers(&headers);
        assert_eq!(parsed.version, Some("0.2".to_string()));
    }

    /// 测试响应头部构建
    #[test]
    fn test_response_headers() {
        let resp = VeridactusResponseHeaders {
            version: "0.2".to_string(),
            trace_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            cost_consumed: Some(0.05),
            budget_remaining: Some(0.95),
            proof_levels: Some(vec!["L0".to_string()]),
            ..Default::default()
        };

        let headers = resp.to_headers();
        assert_eq!(headers.get("VERIDACTUS-Version").unwrap(), "0.2");
        assert_eq!(headers.get("VERIDACTUS-Cost-Consumed").unwrap(), "0.050000");
        assert_eq!(headers.get("VERIDACTUS-Proof-Levels").unwrap(), "L0");
    }
}
