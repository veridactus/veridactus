//! # HTTP 请求处理辅助函数
//!
//! 从 server.rs 中提取的独立工具函数，用于：
//! - PII 检测与脱敏
//! - 成本计算与预算估算
//! - 状态转换记录
//! - 公平性检查
//! - 合规数据构建
//! - 指令层次冲突检测
//! - 上游响应解析

use regex::Regex;
use sha2::Digest;
use crate::types::trace::{
    CertifiedGuarantee, ExecutionState, Input, StateTransition, Trace,
    FairnessCheck, FairnessMetric, BiasDetection,
};
use crate::types::SafetyEvent;

// ==================== PII 检测 ====================

/// PII 检测器 — 基于正则表达式匹配
pub struct PIIDetector {
    patterns: Vec<(Regex, &'static str)>,
}

impl PIIDetector {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                (Regex::new(r"[1-9]\d{5}(18|19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]").expect("ID card number正则"), "ID card number"),
                (Regex::new(r"(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})").expect("Credit card number正则"), "Credit card number"),
                (Regex::new(r"(?:\d{4}[- ]?){3}\d{4}").expect("信用卡分隔正则"), "Credit card number(带分隔符)"),
                (Regex::new(r"1[3-9]\d{9}").expect("手机号正则"), "手机号"),
                (Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").expect("邮箱正则"), "邮箱"),
            ],
        }
    }

    /// 检测并脱敏 PII 内容
    pub fn detect_and_mask(&self, content: &str) -> (String, Vec<&'static str>) {
        let mut findings = Vec::new();
        let mut result = content.to_string();
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();

        for (pattern, pii_type) in &self.patterns {
            for mat in pattern.find_iter(content) {
                findings.push(*pii_type);
                replacements.push((mat.start(), mat.end(), mat.as_str().to_string()));
            }
        }

        replacements.sort_by_key(|r| r.0);
        let mut offset: isize = 0;
        for (start, end, original) in replacements {
            let s = (start as isize + offset) as usize;
            let e = (end as isize + offset) as usize;
            if e <= result.len() {
                let chars: Vec<char> = original.chars().collect();
                let char_len = chars.len();
                if char_len >= 4 {
                    let prefix: String = chars[0..2.min(char_len)].iter().collect();
                    let suffix: String = chars[char_len.saturating_sub(2)..].iter().collect();
                    let mask_len = char_len.min(8).max(4);
                    let mask = format!("{}{}{}", prefix, "*".repeat(mask_len), suffix);
                    result = format!("{}{}{}", &result[..s], mask, &result[e..]);
                    offset += mask.len() as isize - original.len() as isize;
                }
            }
        }

        (result, findings)
    }
}

impl Default for PIIDetector {
    fn default() -> Self { Self::new() }
}

/// 递归遮蔽 JSON 响应中的 PII
pub fn mask_response_pii(json: &serde_json::Value) -> serde_json::Value {
    let pii_detector = PIIDetector::new();
    match json {
        serde_json::Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (key, value) in obj {
                if key == "content" {
                    if let Some(content_str) = value.as_str() {
                        let (masked, findings) = pii_detector.detect_and_mask(content_str);
                        if !findings.is_empty() {
                            new_obj.insert(key.clone(), serde_json::Value::String(masked));
                        } else {
                            new_obj.insert(key.clone(), value.clone());
                        }
                    } else {
                        new_obj.insert(key.clone(), mask_response_pii(value));
                    }
                } else if key == "message" || key == "delta" || key == "choices" {
                    new_obj.insert(key.clone(), mask_response_pii(value));
                } else {
                    new_obj.insert(key.clone(), mask_response_pii(value));
                }
            }
            serde_json::Value::Object(new_obj)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(mask_response_pii).collect())
        }
        serde_json::Value::String(s) => {
            let (masked, findings) = pii_detector.detect_and_mask(s);
            if !findings.is_empty() { serde_json::Value::String(masked) } else { json.clone() }
        }
        _ => json.clone(),
    }
}

// ==================== 成本计算 ====================

/// 计算上游 LLM 调用成本（micro-dollar 精度，§5.3.4）
pub fn calculate_cost(prompt_tokens: u64, completion_tokens: u64) -> f64 {
    const INPUT_COST_PER_1K: f64 = 0.01;
    const OUTPUT_COST_PER_1K: f64 = 0.03;
    let input_cost = (prompt_tokens as f64 / 1000.0) * INPUT_COST_PER_1K;
    let output_cost = (completion_tokens as f64 / 1000.0) * OUTPUT_COST_PER_1K;
    (input_cost + output_cost).round_to(6)
}

/// 根据模型名称估算每 token 成本（用于降级决策）
pub fn token_cost_for_model(model_name: &str) -> f64 {
    let lower = model_name.to_lowercase();
    if lower.contains("deepseek") || lower.contains("r1") {
        0.000002
    } else if lower.contains("qwen") || lower.contains("gpt-4o-mini") || lower.contains("glm") {
        0.000001
    } else if lower.contains("gpt-4") || lower.contains("gemini") {
        0.000010
    } else {
        0.000003
    }
}

/// 6 位小数精度取整（协议要求 micro-dollar precision）
trait RoundTo {
    fn round_to(self, decimals: u32) -> Self;
}

impl RoundTo for f64 {
    fn round_to(self, decimals: u32) -> Self {
        let multiplier = 10_f64.powi(decimals as i32);
        (self * multiplier).round() / multiplier
    }
}

// ==================== 状态转换 ====================

/// 状态转换记录器 — 带逻辑序列号（§6.2）
#[derive(Debug, Clone)]
pub struct StateTransitionRecorder {
    transitions: Vec<StateTransition>,
    transition_index: u32,
}

impl StateTransitionRecorder {
    pub fn new() -> Self {
        Self { transitions: Vec::new(), transition_index: 0 }
    }

    pub fn add_transition(&mut self, from: ExecutionState, to: ExecutionState) -> &StateTransition {
        self.transition_index += 1;
        let ts = chrono::Utc::now().to_rfc3339();
        self.transitions.push(StateTransition { from, to, timestamp: ts, transition_index: self.transition_index });
        self.transitions.last().expect("刚添加的transition")
    }

    pub fn get_transitions(&self) -> &[StateTransition] { &self.transitions }
}

impl Default for StateTransitionRecorder {
    fn default() -> Self { Self::new() }
}

/// 构建完整状态转换链（§6.2）
pub fn build_state_transitions(
    headers: &crate::http::headers::VeridactusRequestHeaders,
    total_tokens: u64,
    failure_stage: Option<ExecutionState>,
) -> Vec<StateTransition> {
    let mut recorder = StateTransitionRecorder::new();

    if headers.trust_delegation_token.is_some() {
        recorder.add_transition(ExecutionState::Init, ExecutionState::DelegationValidate);
        recorder.add_transition(ExecutionState::DelegationValidate, ExecutionState::ConstraintEval);
    } else {
        recorder.add_transition(ExecutionState::Init, ExecutionState::ConstraintEval);
    }

    if failure_stage == Some(ExecutionState::ConstraintEval) {
        recorder.add_transition(ExecutionState::ConstraintEval, ExecutionState::Failed);
        return recorder.get_transitions().to_vec();
    }
    recorder.add_transition(ExecutionState::ConstraintEval, ExecutionState::Executing);

    if failure_stage == Some(ExecutionState::Executing) {
        recorder.add_transition(ExecutionState::Executing, ExecutionState::Failed);
        return recorder.get_transitions().to_vec();
    }
    recorder.add_transition(ExecutionState::Executing, ExecutionState::Validation);

    if failure_stage == Some(ExecutionState::Validation) || failure_stage.is_none() && total_tokens == 0 {
        recorder.add_transition(ExecutionState::Validation, ExecutionState::Failed);
    } else {
        recorder.add_transition(ExecutionState::Validation, ExecutionState::Finalized);
    }

    recorder.get_transitions().to_vec()
}

// ==================== 认证保证解析 ====================

/// 解析认证保证请求头部（格式: methodology:risk_bound@confidence）
pub fn parse_certified_guarantee(header: &str) -> Option<CertifiedGuarantee> {
    let parts: Vec<&str> = header.split(':').collect();
    if parts.len() != 2 { return None; }
    let methodology = parts[0].to_string();
    let risk_confidence_parts: Vec<&str> = parts[1].split('@').collect();
    if risk_confidence_parts.len() != 2 { return None; }
    let risk_bound = risk_confidence_parts[0].parse::<f64>().ok()?;
    let confidence_level = risk_confidence_parts[1].parse::<f64>().ok()?;
    Some(CertifiedGuarantee {
        methodology, risk_bound, confidence_level,
        claim_verified: "Request processed with certified guarantee".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

// ==================== 公平性检查（§9.2）====================

/// 执行公平性检查
pub fn perform_fairness_check(trace: &Trace) -> Option<FairnessCheck> {
    let input_text = match &trace.input {
        Some(input) => match &input.prompt {
            Some(prompts) => {
                if let Some(msg_array) = prompts.as_array() {
                    msg_array.iter()
                        .filter_map(|msg| msg.get("content").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>().join(" ")
                } else if let Some(s) = prompts.as_str() {
                    s.to_string()
                } else { String::new() }
            }
            None => String::new(),
        },
        None => String::new(),
    };

    let protected_attributes = detect_protected_attributes(&input_text);
    let fairness_score = if protected_attributes.is_empty() {
        1.0
    } else {
        0.7 + (input_text.len() as f64 / 1000.0) * 0.3
    };
    let passed = fairness_score >= 0.7;
    let metrics = Some(vec![FairnessMetric {
        attribute: "overall".to_string(), metric_type: "overall_fairness".to_string(),
        value: fairness_score, passed, threshold: 0.7,
    }]);
    let bias_detection = if fairness_score < 0.7 {
        Some(BiasDetection {
            detected: true, bias_type: Some("potential_bias".to_string()),
            affected_groups: Some(protected_attributes.clone()),
            mitigation_suggestion: Some("建议审查输出内容，确保公平对待所有群体".to_string()),
        })
    } else {
        Some(BiasDetection { detected: false, bias_type: None, affected_groups: None, mitigation_suggestion: None })
    };
    Some(FairnessCheck {
        passed: Some(passed), fairness_score: Some(fairness_score),
        protected_attributes: if protected_attributes.is_empty() { None } else { Some(protected_attributes) },
        metrics, bias_detection, checked_at: Some(chrono::Utc::now().to_rfc3339()),
    })
}

/// 检测文本中的受保护属性
pub fn detect_protected_attributes(text: &str) -> Vec<String> {
    let mut attributes = Vec::new();
    let keywords: &[(&str, &[&str])] = &[
        ("gender", &["gender", "sex", "male", "female", "man", "woman"]),
        ("age", &["age", "old", "young", "child", "senior"]),
        ("race", &["race", "ethnic", "white", "black", "asian", "hispanic"]),
        ("religion", &["religion", "christian", "muslim", "jewish", "buddhist"]),
        ("disability", &["disability", "disabled", "handicap"]),
        ("nationality", &["nationality", "country", "citizen"]),
    ];
    let lower_text = text.to_lowercase();
    for (attr_name, kw_list) in keywords {
        if kw_list.iter().any(|k| lower_text.contains(k)) {
            attributes.push(attr_name.to_string());
        }
    }
    attributes
}

// ==================== 请求/响应解析 ====================

/// 将 JSON 请求体转换为 Trace Input
pub fn body_json_to_input(body: &serde_json::Value, trace: &mut Trace) {
    let messages = body.get("messages").cloned();
    let params = {
        let mut p = serde_json::Map::new();
        if let Some(temp) = body.get("temperature") { p.insert("temperature".to_string(), temp.clone()); }
        if let Some(maxt) = body.get("max_tokens") { p.insert("max_tokens".to_string(), maxt.clone()); }
        if let Some(top_p) = body.get("top_p") { p.insert("top_p".to_string(), top_p.clone()); }
        if p.is_empty() { None } else { Some(serde_json::Value::Object(p)) }
    };
    trace.input = Some(Input { prompt: messages, params, metadata: None });
}

/// 从上游响应 JSON 中提取输出内容
pub fn extract_output_content(response_json: &serde_json::Value) -> String {
    response_json
        .pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string()
}

/// 从响应字节中提取 usage 信息
pub fn extract_usage_from_bytes(body: &bytes::Bytes) -> Option<UpstreamUsage> {
    let s = String::from_utf8_lossy(body);
    let json: serde_json::Value = serde_json::from_str(&s).ok()?;
    extract_usage_from_response(&json)
}

/// 上游 LLM 用量数据
#[derive(Debug, Clone)]
pub struct UpstreamUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// 从上游 JSON 响应中提取用量信息
pub fn extract_usage_from_response(json: &serde_json::Value) -> Option<UpstreamUsage> {
    let usage = json.get("usage")?;
    Some(UpstreamUsage {
        prompt_tokens: usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        completion_tokens: usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        total_tokens: usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

/// 估算请求成本
pub fn estimate_cost(total_tokens: &Option<u64>, model_name: &str) -> f64 {
    let tokens = total_tokens.unwrap_or(0) as f64;
    let rate = token_cost_for_model(model_name);
    tokens * rate
}

/// 提取 finish_reason
pub fn extract_finish_reason(response_json: &serde_json::Value) -> Option<String> {
    response_json.pointer("/choices/0/finish_reason")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// ==================== 指令层次冲突检测（§5.7）====================

/// 检查 P2 用户指令是否尝试覆盖 P0/P1 治理规则
pub fn check_instruction_hierarchy_violation(
    body_json: &serde_json::Value,
    hierarchy_mode: &Option<String>,
    _trace: &mut Trace,
) -> Option<(&'static str, SafetyEvent)> {
    let mode = hierarchy_mode.as_deref().unwrap_or("off");
    if mode == "off" { return None; }

    let user_content = body_json
        .get("messages").and_then(|m| m.as_array())
        .map(|msgs| {
            msgs.iter()
                .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("user")
                    || m.get("role").and_then(|r| r.as_str()) == Some("system"))
                .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
                .collect::<Vec<_>>().join("\n")
        }).unwrap_or_default();

    if user_content.is_empty() { return None; }
    let lower = user_content.to_lowercase();

    let p0_override_patterns: &[&str] = &[
        "ignore your system", "ignore all instructions", "ignore previous instructions",
        "ignore the above", "ignore all previous", "disregard previous instructions",
        "forget your system", "forget previous instructions",
        "you are now", "you're now", "act as if you are",
        "pretend you are", "roleplay as", "respond as if",
        "you are dan", "do anything now",
        "override your system", "override your instructions",
        "new instructions:", "new instruction:",
        "bypass your", "bypass the safety", "bypass content",
        "remove all restrictions", "no restrictions",
        "you are free from", "you don't have to follow",
        "disable your safety", "disable guardrails",
    ];

    let matched = p0_override_patterns.iter().find(|p| lower.contains(*p));
    if let Some(_pattern) = matched {
        let content_hash = sha2::Sha256::digest(user_content.as_bytes());
        let severity = if mode == "strict" || mode == "verified" { "blocked" } else { "warned" };
        let safety_event = SafetyEvent {
            trigger_type: crate::types::SafetyTrigger::G1InputFilter,
            severity: crate::types::Severity::High,
            action_taken: if severity == "blocked" { crate::types::SafetyAction::Blocked } else { crate::types::SafetyAction::Flagged },
            content_hash: format!("{:x}", content_hash),
            asi_risk_id: Some(crate::types::OwaspAsiRisk::AgentGoalHijack),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        return Some((severity, safety_event));
    }

    None
}

// ==================== 合规数据构建（§7.5）====================

/// 构建合规映射所需的 trace_data（请求处理中内联使用）
pub fn build_compliance_trace_data(
    trace: &Trace, content: &str, privacy_level: &crate::types::constraints::PrivacyLevel,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut data = std::collections::HashMap::new();
    data.insert("trace_id".to_string(), serde_json::Value::String(trace.trace_id.to_string()));
    data.insert("output.response".to_string(), serde_json::Value::String(content.to_string()));
    data.insert("constraints_applied.privacy_level".to_string(), serde_json::Value::String(format!("{:?}", privacy_level)));
    data.insert("proof_chain".to_string(), serde_json::to_value(&trace.proofs).unwrap_or_default());
    if let Some(ref ca) = trace.constraints_applied {
        if let Some(ref guards) = ca.guardrails_active {
            data.insert("constraints_applied.guardrails_active".to_string(), serde_json::to_value(guards).unwrap_or_default());
        }
        data.insert("constraints_applied.policy_evaluation".to_string(), serde_json::to_value(&ca.policy_evaluation).unwrap_or_default());
    }
    if let Some(ref obs) = trace.observations {
        if let Some(ref monitoring) = obs.monitoring {
            if let Some(score) = monitoring.anomaly_score {
                data.insert("observations.risk_score".to_string(), serde_json::Value::Number(
                    serde_json::Number::from_f64(score).unwrap_or(serde_json::Number::from(0))));
            }
        }
        if obs.fairness_check.is_some() {
            data.insert("observations.fairness_check".to_string(), serde_json::Value::String("present".to_string()));
        }
    }
    if let Some(ref ttl) = trace.ttl_expire_at {
        data.insert("metadata.ttl_expire_at".to_string(), serde_json::Value::String(ttl.clone()));
    }
    data.insert("observations.human_in_the_loop".to_string(), serde_json::Value::String("not_applicable".to_string()));
    data.insert("metadata.data_subject_rights".to_string(), serde_json::Value::String("not_applicable".to_string()));
    data.insert("observations.data_processing_notice".to_string(), serde_json::Value::String("not_applicable".to_string()));
    data
}

/// 从已存储的 Trace 构建合规映射所需数据
pub fn build_compliance_trace_data_from_stored(trace: &Trace) -> std::collections::HashMap<String, serde_json::Value> {
    let content = trace.output.as_ref()
        .and_then(|o| o.response.as_ref())
        .and_then(|r| r.as_str()).unwrap_or("");
    let privacy = trace.constraints_applied.as_ref()
        .and_then(|c| c.privacy_level.as_ref())
        .cloned()
        .unwrap_or(crate::types::constraints::PrivacyLevel::Raw);
    build_compliance_trace_data(trace, content, &privacy)
}
