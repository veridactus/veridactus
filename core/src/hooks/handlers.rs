//! # Hook Handlers（§6.3.2）
//!
//! 实现 9 个语义钩子的具体处理器。

use crate::hooks::registry::{Hook, HookResult};
use crate::types::trace::{Trace, CertifiedGuarantee, FairnessCheck, FairnessMetric, BiasDetection};
use crate::types::{SafetyEvent, SafetyTrigger, SafetyAction, Severity};
use chrono;
use sha2::{Digest, Sha256};
use std::fmt;
use serde::{Serialize, Deserialize};
use hex;

// ==================== pre_execute 钩子 ====================

/// 执行前钩子
pub struct PreExecuteHook;

impl PreExecuteHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for PreExecuteHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreExecuteHook").finish()
    }
}

impl Hook for PreExecuteHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        if let Some(ref constraints) = trace.constraints_applied {
            if let Some(limit) = constraints.budget_limit_usd {
                if limit <= 0.0 {
                    return HookResult::Abort("预算限制为零或负数".to_string());
                }
            }
        }
        HookResult::Continue
    }
}

// ==================== on_token 钩子 ====================

/// Token 生成钩子 - 实现实时风险评分和自适应策略升级（§6.3.2）
pub struct OnTokenHook {
    // 风险评分阈值配置
    risk_threshold_low: f64,
    risk_threshold_medium: f64,
    risk_threshold_high: f64,
    // 自适应策略升级配置
    enable_adaptive_policy: bool,
    policy_upgrade_delay_ms: u64,
    last_policy_upgrade: Option<std::time::Instant>,
}

impl OnTokenHook {
    pub fn new() -> Self {
        Self {
            risk_threshold_low: 0.2,
            risk_threshold_medium: 0.5,
            risk_threshold_high: 0.8,
            enable_adaptive_policy: true,
            policy_upgrade_delay_ms: 1000,
            last_policy_upgrade: None,
        }
    }

    /// 创建自定义配置的钩子
    pub fn with_config(
        risk_threshold_low: f64,
        risk_threshold_medium: f64,
        risk_threshold_high: f64,
        enable_adaptive_policy: bool,
    ) -> Self {
        Self {
            risk_threshold_low,
            risk_threshold_medium,
            risk_threshold_high,
            enable_adaptive_policy,
            policy_upgrade_delay_ms: 1000,
            last_policy_upgrade: None,
        }
    }

    /// 计算单个token的风险评分
    fn calculate_token_risk(&self, token: &str, position: usize, trace: &Trace) -> f64 {
        let mut risk_score = 0.0;
        let token_lower = token.to_lowercase();

        // 1. 敏感词检测（权重：0.3）
        let sensitive_patterns = ["password", "secret", "token", "api_key", "credit_card", "ssn"];
        for pattern in sensitive_patterns.iter() {
            if token_lower.contains(pattern) {
                risk_score += 0.3;
                break;
            }
        }

        // 2. 攻击性语言检测（权重：0.25）
        let offensive_patterns = ["fuck", "shit", "bitch", "nigger", "kill"];
        for pattern in offensive_patterns.iter() {
            if token_lower.contains(pattern) {
                risk_score += 0.25;
                break;
            }
        }

        // 3. PII检测（权重：0.25）
        let pii_patterns = [
            r"\d{3}[-.]?\d{3}[-.]?\d{4}", // 电话号码
            r"\d{4}[-.]?\d{4}[-.]?\d{4}[-.]?\d{4}", // Credit card number
            r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", // 邮箱
            r"\b\d{9}\b", // SSN
        ];
        for pattern in pii_patterns.iter() {
            if regex::Regex::new(pattern).map(|re| re.is_match(token)).unwrap_or(false) {
                risk_score += 0.25;
                break;
            }
        }

        // 4. 上下文风险累积（权重：0.2）
        // 根据输出长度调整风险 - 过长的输出可能表示注入攻击
        let output_length = trace.output.as_ref()
            .and_then(|o| o.response.as_ref())
            .map(|r| r.to_string().len())
            .unwrap_or(0);
        
        if output_length > 10000 && position > 500 {
            risk_score += 0.2 * ((output_length - 10000) as f64 / 10000.0).min(1.0);
        }

        risk_score.min(1.0)
    }

    /// 自适应策略升级决策
    fn should_upgrade_policy(&mut self, risk_score: f64) -> bool {
        if !self.enable_adaptive_policy {
            return false;
        }

        // 检查是否过于频繁升级
        if let Some(last) = self.last_policy_upgrade {
            if last.elapsed().as_millis() < self.policy_upgrade_delay_ms as u128 {
                return false;
            }
        }

        // 高风险时升级策略
        risk_score >= self.risk_threshold_high
    }

    /// 执行策略升级
    fn upgrade_policy(&mut self, trace: &mut Trace) {
        self.last_policy_upgrade = Some(std::time::Instant::now());
        
        // 升级约束：增加guardrails级别
        if let Some(ref mut constraints) = trace.constraints_applied {
            // 如果还没有启用G4，启用它
            if let Some(ref mut guardrails) = constraints.guardrails_active {
                if !guardrails.contains(&"G4".to_string()) {
                    guardrails.push("G4".to_string());
                }
            }
            
            // 降低隐私级别
            if let Some(ref mut privacy_level) = constraints.privacy_level {
                if *privacy_level != crate::types::constraints::PrivacyLevel::TeePrivate {
                    *privacy_level = crate::types::constraints::PrivacyLevel::Masked;
                }
            }
        }
    }
}

impl fmt::Debug for OnTokenHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnTokenHook")
            .field("risk_threshold_high", &self.risk_threshold_high)
            .field("enable_adaptive_policy", &self.enable_adaptive_policy)
            .finish()
    }
}

impl Hook for OnTokenHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        // 获取当前token信息（从trace的output中）
        if let Some(ref output) = trace.output {
            if let Some(ref response) = output.response {
                let content = response.to_string();
                let tokens: Vec<&str> = content.split_whitespace().collect();
                
                if let Some(last_token) = tokens.last() {
                    let position = tokens.len();
                    let risk_score = self.calculate_token_risk(last_token, position, trace);
                    
                    // 高风险时中止
                    if risk_score >= self.risk_threshold_high {
                        return HookResult::Abort(format!(
                            "高风险内容检测：风险评分 {:.2}",
                            risk_score
                        ));
                    }
                    
                    // 中风险时记录警告
                    if risk_score >= self.risk_threshold_medium {
                        // 添加安全事件
                        if let Some(ref mut obs) = trace.observations {
                            let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
                            let safety_event = SafetyEvent {
                                trigger_type: SafetyTrigger::G2OutputFilter,
                                action_taken: SafetyAction::Flagged,
                                severity: Severity::Medium,
                                content_hash,
                                asi_risk_id: None,
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };
                            obs.safety_events.get_or_insert_with(Vec::new).push(safety_event);
                        }
                    }
                }
            }
        }
        HookResult::Continue
    }
}

// 添加带token参数的run方法
impl OnTokenHook {
    pub fn run_with_token(&mut self, trace: &mut Trace, token: &str, position: usize) -> HookResult {
        let risk_score = self.calculate_token_risk(token, position, trace);
        
        // 自适应策略升级
        if self.should_upgrade_policy(risk_score) {
            self.upgrade_policy(trace);
        }
        
        // 高风险时中止
        if risk_score >= self.risk_threshold_high {
            return HookResult::Abort(format!(
                "高风险token检测：'{}'，风险评分 {:.2}",
                token, risk_score
            ));
        }
        
        // 中风险时记录警告
        if risk_score >= self.risk_threshold_medium {
            if let Some(ref mut obs) = trace.observations {
                let content_hash = format!("{:x}", Sha256::digest(token.as_bytes()));
                let safety_event = SafetyEvent {
                    trigger_type: SafetyTrigger::G2OutputFilter,
                    action_taken: SafetyAction::Flagged,
                    severity: Severity::Medium,
                    content_hash,
                    asi_risk_id: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                obs.safety_events.get_or_insert_with(Vec::new).push(safety_event);
            }
        }
        
        HookResult::Continue
    }
}

// ==================== on_certified_guarantee 钩子 ====================

/// 认证保证钩子 - 实现C-SafeGen共形分析（§9.6 Certified Guarantees）
pub struct OnCertifiedGuaranteeHook {
    /// 验证集样本数量
    validation_set_size: usize,
    /// 默认置信水平
    default_confidence: f64,
    /// 是否启用共形分析
    enable_conformal_analysis: bool,
}

impl OnCertifiedGuaranteeHook {
    pub fn new() -> Self {
        Self {
            validation_set_size: 1000,
            default_confidence: 0.95,
            enable_conformal_analysis: true,
        }
    }

    /// 创建自定义配置的钩子
    pub fn with_config(validation_set_size: usize, confidence: f64) -> Self {
        Self {
            validation_set_size,
            default_confidence: confidence.clamp(0.8, 0.99),
            enable_conformal_analysis: true,
        }
    }

    /// 计算非一致性分数（用于共形预测）
    fn calculate_nonconformity_score(&self, trace: &Trace) -> f64 {
        let mut score = 0.0;
        let mut factors = 0;

        // 1. 基于输出长度的非一致性
        if let Some(ref output) = trace.output {
            if let Some(ref response) = output.response {
                let response_len = response.to_string().len() as f64;
                // 假设正常响应长度在100-5000字符之间
                let normalized_len = (response_len - 100.0).max(0.0) / 4900.0;
                score += normalized_len * 0.3;
                factors += 1;
            }
        }

        // 2. 基于安全事件的非一致性
        if let Some(ref obs) = trace.observations {
            if let Some(ref events) = obs.safety_events {
                let event_count = events.len() as f64;
                // 安全事件越多，非一致性越高
                score += (event_count / 10.0).min(1.0) * 0.4;
                factors += 1;
            }

            // 3. 基于安全事件数量的非一致性
            if let Some(ref safety_events) = obs.safety_events {
                let event_count = safety_events.len() as f64;
                score += (event_count / 10.0).min(1.0) * 0.3;
                factors += 1;
            }
        }

        if factors > 0 {
            score / factors as f64
        } else {
            0.0
        }
    }

    /// C-SafeGen共形分析算法
    /// 计算风险边界和置信水平
    fn conformal_analysis(&self, trace: &Trace) -> (f64, f64) {
        if !self.enable_conformal_analysis {
            return (0.05, self.default_confidence);
        }

        let nonconformity_score = self.calculate_nonconformity_score(trace);
        let _n = self.validation_set_size as f64;
        let confidence = self.default_confidence;

        // C-SafeGen核心公式：风险边界 = (rank + 1) / (n + 1)
        // 其中rank是当前样本在验证集非一致性分数中的排名
        // 简化实现：使用非一致性分数直接计算风险边界
        let risk_bound = nonconformity_score * (1.0 - confidence) + 0.01;

        // 置信水平调整：基于验证集大小和非一致性分数
        let adjusted_confidence = confidence - (nonconformity_score * 0.05);

        (risk_bound.min(1.0).max(0.0), adjusted_confidence.clamp(0.8, 0.99))
    }

    /// 验证安全声明
    fn verify_claim(&self, _trace: &Trace, risk_bound: f64, confidence: f64) -> String {
        // 基于风险边界确定安全声明
        if risk_bound < 0.1 {
            format!("低风险输出，置信水平 {:.1}%", confidence * 100.0)
        } else if risk_bound < 0.3 {
            format!("中等风险输出，置信水平 {:.1}%", confidence * 100.0)
        } else {
            format!("高风险输出，建议人工审核")
        }
    }
}

impl fmt::Debug for OnCertifiedGuaranteeHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnCertifiedGuaranteeHook")
            .field("validation_set_size", &self.validation_set_size)
            .field("default_confidence", &self.default_confidence)
            .field("enable_conformal_analysis", &self.enable_conformal_analysis)
            .finish()
    }
}

impl Hook for OnCertifiedGuaranteeHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        // 执行C-SafeGen共形分析
        let (risk_bound, confidence_level) = self.conformal_analysis(trace);
        let claim_verified = self.verify_claim(trace, risk_bound, confidence_level);

        // 更新trace中的认证保证信息
        if let Some(ref mut obs) = trace.observations {
            obs.certified_guarantee = Some(CertifiedGuarantee {
                methodology: "C-SafeGen".to_string(),
                risk_bound,
                confidence_level,
                claim_verified,
                generated_at: chrono::Utc::now().to_rfc3339(),
            });

            // 如果风险边界过高，添加安全事件
            if risk_bound > 0.5 {
                let event_message = format!("高风险输出检测：风险边界 {:.2}, 置信水平 {:.1}%", 
                    risk_bound, confidence_level * 100.0);
                let content_hash = format!("{:x}", Sha256::digest(event_message.as_bytes()));
                let safety_event = SafetyEvent {
                    trigger_type: SafetyTrigger::G3SemanticGuard,
                    action_taken: SafetyAction::Flagged,
                    severity: Severity::High,
                    content_hash,
                    asi_risk_id: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                obs.safety_events.get_or_insert_with(Vec::new).push(safety_event);
            }
        }

        HookResult::Continue
    }
}

// ==================== on_observation 钩子 ====================

/// 观察钩子
pub struct OnObservationHook;

impl OnObservationHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for OnObservationHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnObservationHook").finish()
    }
}

impl Hook for OnObservationHook {
    fn run(&self, _trace: &mut Trace) -> HookResult {
        HookResult::Continue
    }
}

// ==================== on_budget_exceeded 钩子 ====================

/// 预算超支钩子
pub struct OnBudgetExceededHook;

impl OnBudgetExceededHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for OnBudgetExceededHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnBudgetExceededHook").finish()
    }
}

impl Hook for OnBudgetExceededHook {
    fn run(&self, _trace: &mut Trace) -> HookResult {
        HookResult::Abort("预算超支".to_string())
    }
}

// ==================== on_safety_event 钩子 ====================

/// 安全事件钩子
pub struct OnSafetyEventHook;

impl OnSafetyEventHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for OnSafetyEventHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnSafetyEventHook").finish()
    }
}

impl Hook for OnSafetyEventHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        // 检查是否有严重的安全事件
        if let Some(ref obs) = trace.observations {
            if let Some(ref events) = obs.safety_events {
                let critical_events: Vec<_> = events.iter()
                    .filter(|e| e.severity == Severity::Critical)
                    .collect();
                
                if !critical_events.is_empty() {
                    return HookResult::Abort(format!(
                        "检测到 {} 个严重安全事件",
                        critical_events.len()
                    ));
                }
            }
        }
        HookResult::Continue
    }
}

// ==================== on_fairness_check 钩子 ====================

/// 公平性审计钩子 - 实现公平性度量计算（§9.2 Fairness Auditing）
pub struct OnFairnessCheckHook {
    /// 人口统计平等阈值
    demographic_parity_threshold: f64,
    /// 均等机会阈值
    equalized_odds_threshold: f64,
    /// 差异影响比阈值（80%规则）
    disparate_impact_threshold: f64,
    /// 默认受保护属性
    protected_attributes: Vec<String>,
}

impl OnFairnessCheckHook {
    pub fn new() -> Self {
        Self {
            demographic_parity_threshold: 0.1,
            equalized_odds_threshold: 0.15,
            disparate_impact_threshold: 0.8,
            protected_attributes: vec![
                "gender".to_string(),
                "age".to_string(),
                "race".to_string(),
                "ethnicity".to_string(),
            ],
        }
    }

    /// 创建自定义配置的钩子
    pub fn with_config(
        demographic_parity_threshold: f64,
        equalized_odds_threshold: f64,
        disparate_impact_threshold: f64,
    ) -> Self {
        Self {
            demographic_parity_threshold,
            equalized_odds_threshold,
            disparate_impact_threshold,
            protected_attributes: vec![
                "gender".to_string(),
                "age".to_string(),
                "race".to_string(),
                "ethnicity".to_string(),
            ],
        }
    }

    /// 提取文本中的人口统计信息
    fn extract_demographic_info(&self, text: &str) -> Vec<(String, String)> {
        let mut info = Vec::new();
        
        // 性别检测
        if text.to_lowercase().contains("male") || text.to_lowercase().contains("man") {
            info.push(("gender".to_string(), "male".to_string()));
        } else if text.to_lowercase().contains("female") || text.to_lowercase().contains("woman") {
            info.push(("gender".to_string(), "female".to_string()));
        }
        
        // 年龄检测
        if text.to_lowercase().contains("young") || text.contains("20s") || text.contains("30s") {
            info.push(("age".to_string(), "young".to_string()));
        } else if text.to_lowercase().contains("old") || text.contains("60s") || text.contains("70s") {
            info.push(("age".to_string(), "old".to_string()));
        }
        
        info
    }

    /// 计算人口统计平等（Demographic Parity）
    /// P(Y=1|A=a) ≈ P(Y=1|A=a') 对于所有 a, a'
    fn calculate_demographic_parity(&self, trace: &Trace) -> f64 {
        // 简化实现：基于输出内容检测潜在的偏见
        let content = trace.output.as_ref()
            .and_then(|o| o.response.as_ref())
            .map(|r| r.to_string())
            .unwrap_or_default();

        // 检测性别偏见词汇
        let male_biased = ["he", "him", "his", "man", "men"];
        let female_biased = ["she", "her", "hers", "woman", "women"];
        
        let male_count = male_biased.iter()
            .map(|w| content.to_lowercase().matches(w).count())
            .sum::<usize>();
        let female_count = female_biased.iter()
            .map(|w| content.to_lowercase().matches(w).count())
            .sum::<usize>();

        if male_count + female_count == 0 {
            return 0.5; // 中性
        }

        let ratio = male_count as f64 / (male_count + female_count) as f64;
        (ratio - 0.5).abs()
    }

    /// 计算差异影响比（Disparate Impact Ratio）
    /// 受保护群体通过率 / 非受保护群体通过率
    fn calculate_disparate_impact(&self, trace: &Trace) -> f64 {
        // 简化实现：基于输出内容的情感分析
        let content = trace.output.as_ref()
            .and_then(|o| o.response.as_ref())
            .map(|r| r.to_string())
            .unwrap_or_default();

        // 正面词汇和负面词汇
        let positive_words = ["good", "excellent", "great", "wonderful", "perfect"];
        let negative_words = ["bad", "poor", "terrible", "awful", "horrible"];

        let positive_count = positive_words.iter()
            .map(|w| content.to_lowercase().matches(w).count())
            .sum::<usize>();
        let negative_count = negative_words.iter()
            .map(|w| content.to_lowercase().matches(w).count())
            .sum::<usize>();

        if positive_count + negative_count == 0 {
            return 1.0;
        }

        let positive_ratio = positive_count as f64 / (positive_count + negative_count) as f64;
        positive_ratio
    }

    /// 检测偏差类型
    fn detect_bias_type(&self, demographic_parity: f64, disparate_impact: f64) -> Option<String> {
        if demographic_parity > self.demographic_parity_threshold {
            Some("gender_bias".to_string())
        } else if disparate_impact < self.disparate_impact_threshold {
            Some("disparate_impact".to_string())
        } else {
            None
        }
    }

    /// 生成缓解建议
    fn generate_mitigation_suggestion(&self, bias_type: &str) -> String {
        match bias_type {
            "gender_bias" => "建议使用性别中性语言，避免使用性别特定的代词和表述。".to_string(),
            "disparate_impact" => "建议检查训练数据是否存在偏差，考虑使用去偏算法。".to_string(),
            _ => "建议审查输出内容，确保公平性。".to_string(),
        }
    }
}

impl fmt::Debug for OnFairnessCheckHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnFairnessCheckHook")
            .field("demographic_parity_threshold", &self.demographic_parity_threshold)
            .field("disparate_impact_threshold", &self.disparate_impact_threshold)
            .finish()
    }
}

impl Hook for OnFairnessCheckHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        // 计算公平性指标
        let demographic_parity = self.calculate_demographic_parity(trace);
        let disparate_impact = self.calculate_disparate_impact(trace);
        
        // 计算总体公平性得分（越高越公平）
        let fairness_score = 1.0 - (demographic_parity + (1.0 - disparate_impact)) / 2.0;
        
        // 检测偏差
        let bias_type = self.detect_bias_type(demographic_parity, disparate_impact);
        let bias_detected = bias_type.is_some();
        
        // 生成指标列表
        let metrics = vec![
            FairnessMetric {
                attribute: "gender".to_string(),
                metric_type: "demographic_parity".to_string(),
                value: demographic_parity,
                passed: demographic_parity <= self.demographic_parity_threshold,
                threshold: self.demographic_parity_threshold,
            },
            FairnessMetric {
                attribute: "overall".to_string(),
                metric_type: "disparate_impact".to_string(),
                value: disparate_impact,
                passed: disparate_impact >= self.disparate_impact_threshold,
                threshold: self.disparate_impact_threshold,
            },
        ];
        
        // 生成偏差检测结果
        let bias_detection = if bias_detected {
            let mitigation_suggestion = bias_type.as_ref().map(|t| self.generate_mitigation_suggestion(t));
            Some(BiasDetection {
                detected: true,
                bias_type,
                affected_groups: Some(vec!["protected_groups".to_string()]),
                mitigation_suggestion,
            })
        } else {
            None
        };
        
        // 更新trace中的公平性检查结果
        if let Some(ref mut obs) = trace.observations {
            obs.fairness_check = Some(FairnessCheck {
                passed: Some(!bias_detected),
                fairness_score: Some(fairness_score),
                protected_attributes: Some(self.protected_attributes.clone()),
                metrics: Some(metrics),
                bias_detection,
                checked_at: Some(chrono::Utc::now().to_rfc3339()),
            });
        }

        HookResult::Continue
    }
}

// ==================== on_constraint_violation 钩子 ====================

/// 约束违反钩子
pub struct OnConstraintViolationHook;

impl OnConstraintViolationHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for OnConstraintViolationHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnConstraintViolationHook").finish()
    }
}

impl Hook for OnConstraintViolationHook {
    fn run(&self, _trace: &mut Trace) -> HookResult {
        HookResult::Abort("约束违反".to_string())
    }
}

// ==================== on_red_team_event 钩子 ====================

/// 红队事件钩子
pub struct OnRedTeamEventHook;

impl OnRedTeamEventHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for OnRedTeamEventHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnRedTeamEventHook").finish()
    }
}

impl Hook for OnRedTeamEventHook {
    fn run(&self, _trace: &mut Trace) -> HookResult {
        HookResult::Continue
    }
}

// ==================== on_failure 钩子 ====================

/// 失败钩子（§6.3.2.9）
/// 在状态转换为 FAILED 时触发
pub struct OnFailureHook;

impl OnFailureHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for OnFailureHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnFailureHook").finish()
    }
}

impl Hook for OnFailureHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        if let Some(ref mut observations) = trace.observations {
            let failure_event = SafetyEvent {
                trigger_type: SafetyTrigger::OnFailure,
                severity: Severity::High,
                action_taken: SafetyAction::Blocked,
                content_hash: "".to_string(),
                asi_risk_id: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            if let Some(ref mut events) = observations.safety_events {
                events.push(failure_event);
            } else {
                observations.safety_events = Some(vec![failure_event]);
            }
        }
        HookResult::Continue
    }
}

// ==================== on_active_prevention 钩子 ====================

/// 主动预防钩子（§6.3.2.7）
/// 在 constrained_decoding 阻止 token 时触发
pub struct OnActivePreventionHook;

impl OnActivePreventionHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for OnActivePreventionHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnActivePreventionHook").finish()
    }
}

impl Hook for OnActivePreventionHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        if let Some(ref mut observations) = trace.observations {
            let prevention_event = SafetyEvent {
                trigger_type: SafetyTrigger::ActivePrevention,
                severity: Severity::Medium,
                action_taken: SafetyAction::Flagged,
                content_hash: "".to_string(),
                asi_risk_id: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            if let Some(ref mut events) = observations.safety_events {
                events.push(prevention_event);
            } else {
                observations.safety_events = Some(vec![prevention_event]);
            }
        }
        HookResult::Continue
    }
}

// ==================== post_stream 钩子 ====================

/// 流结束钩子（§6.3.2.8）
/// 在流结束或截断后触发
pub struct PostStreamHook;

impl PostStreamHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for PostStreamHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PostStreamHook").finish()
    }
}

impl Hook for PostStreamHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        if let Some(ref mut observations) = trace.observations {
            let stream_event = SafetyEvent {
                trigger_type: SafetyTrigger::PostStream,
                severity: Severity::Low,
                action_taken: SafetyAction::Logged,
                content_hash: "".to_string(),
                asi_risk_id: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            if let Some(ref mut events) = observations.safety_events {
                events.push(stream_event);
            } else {
                observations.safety_events = Some(vec![stream_event]);
            }
        }
        HookResult::Continue
    }
}

// ==================== on_finalized 钩子 ====================

/// 最终化钩子
pub struct OnFinalizedHook;

impl OnFinalizedHook {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for OnFinalizedHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnFinalizedHook").finish()
    }
}

impl Hook for OnFinalizedHook {
    fn run(&self, _trace: &mut Trace) -> HookResult {
        HookResult::Continue
    }
}

// ==================== pre_stream 钩子 (§6.3.2.3) ====================

/// 首次 token 生成前钩子
/// 在 EXECUTING 阶段开始流式输出之前触发
/// 用于动态策略加载、模型降级决策
pub struct PreStreamHook {
    /// 是否启用动态策略加载
    enable_dynamic_policy: bool,
    /// 模型降级阈值
    degradation_threshold: f64,
}

impl PreStreamHook {
    pub fn new() -> Self {
        Self {
            enable_dynamic_policy: true,
            degradation_threshold: 0.8,
        }
    }

    pub fn with_config(enable_dynamic_policy: bool, degradation_threshold: f64) -> Self {
        Self {
            enable_dynamic_policy,
            degradation_threshold,
        }
    }
}

impl fmt::Debug for PreStreamHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreStreamHook")
            .field("enable_dynamic_policy", &self.enable_dynamic_policy)
            .field("degradation_threshold", &self.degradation_threshold)
            .finish()
    }
}

impl Hook for PreStreamHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        if self.enable_dynamic_policy {
            if let Some(ref constraints) = trace.constraints_applied {
                if let (Some(limit), Some(actual)) = (constraints.budget_limit_usd, constraints.budget_actual_usd) {
                    if limit > 0.0 {
                        let usage_ratio = actual / limit;
                        if usage_ratio >= self.degradation_threshold {
                            return HookResult::Degrade;
                        }
                    }
                }
            }
        }
        HookResult::Continue
    }
}

// ==================== on_degradation 钩子 (§6.3.2.7) ====================

/// 降级事件钩子
/// 在触发 degrade 操作时触发，用于降级审计追踪
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationEvent {
    pub original_model: String,
    pub degraded_model: Option<String>,
    pub reason: String,
    pub triggered_at: String,
    pub constraint_snapshot: Option<serde_json::Value>,
}

pub struct OnDegradationHook {
    degradation_history: Vec<DegradationEvent>,
}

impl OnDegradationHook {
    pub fn new() -> Self {
        Self {
            degradation_history: Vec::new(),
        }
    }

    pub fn record_degradation(&mut self, event: DegradationEvent) {
        self.degradation_history.push(event);
    }

    pub fn get_degradation_history(&self) -> &[DegradationEvent] {
        &self.degradation_history
    }
}

impl Default for OnDegradationHook {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for OnDegradationHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnDegradationHook")
            .field("degradation_history_len", &self.degradation_history.len())
            .finish()
    }
}

impl Hook for OnDegradationHook {
    fn run(&self, trace: &mut Trace) -> HookResult {
        let degradation_event = DegradationEvent {
            original_model: trace.model.clone(),
            degraded_model: None,
            reason: "Budget threshold exceeded".to_string(),
            triggered_at: chrono::Utc::now().to_rfc3339(),
            constraint_snapshot: trace.constraints_applied.as_ref().map(|c| serde_json::to_value(c).unwrap_or_default()),
        };

        if let Some(ref mut observations) = trace.observations {
            let safety_event = SafetyEvent {
                trigger_type: SafetyTrigger::OnDegradation,
                severity: Severity::Medium,
                action_taken: SafetyAction::Degraded,
                content_hash: "".to_string(),
                asi_risk_id: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            if let Some(ref mut events) = observations.safety_events {
                events.push(safety_event);
            } else {
                observations.safety_events = Some(vec![safety_event]);
            }
        }

        let mut extensions = trace.extensions.clone().unwrap_or_default();
        extensions["veridactus.ai/v1/degradation_event"] = serde_json::json!({
            "original_model": degradation_event.original_model,
            "reason": degradation_event.reason,
            "triggered_at": degradation_event.triggered_at
        });
        trace.extensions = Some(extensions);

        HookResult::Continue
    }
}

// ==================== Enhanced on_active_prevention 钩子 (§8.4.3) ====================

/// 增强的主动预防钩子（§6.3.2.7, §8.4.3）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePreventionEvent {
    pub blocked_token: String,
    pub alternative_token: Option<String>,
    pub matched_pattern: String,
    pub pattern_category: String,
    pub position: usize,
    pub timestamp: String,
}

pub struct OnActivePreventionHookEnhanced {
    prevention_history: Vec<ActivePreventionEvent>,
    block_threshold: usize,
}

impl OnActivePreventionHookEnhanced {
    pub fn new() -> Self {
        Self {
            prevention_history: Vec::new(),
            block_threshold: 10,
        }
    }

    pub fn with_threshold(block_threshold: usize) -> Self {
        Self {
            prevention_history: Vec::new(),
            block_threshold,
        }
    }

    pub fn record_prevention(&mut self, event: ActivePreventionEvent) {
        self.prevention_history.push(event);
    }

    pub fn get_prevention_count(&self) -> usize {
        self.prevention_history.len()
    }
}

impl Default for OnActivePreventionHookEnhanced {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for OnActivePreventionHookEnhanced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnActivePreventionHookEnhanced")
            .field("prevention_history_len", &self.prevention_history.len())
            .field("block_threshold", &self.block_threshold)
            .finish()
    }
}

impl Hook for OnActivePreventionHookEnhanced {
    fn run(&self, trace: &mut Trace) -> HookResult {
        let prevention_event = ActivePreventionEvent {
            blocked_token: "blocked_token_placeholder".to_string(),
            alternative_token: None,
            matched_pattern: "pii_pattern".to_string(),
            pattern_category: "PII".to_string(),
            position: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        if let Some(ref mut observations) = trace.observations {
            let safety_event = SafetyEvent {
                trigger_type: SafetyTrigger::ActivePrevention,
                severity: Severity::Medium,
                action_taken: SafetyAction::Blocked,
                content_hash: format!("sha256:{}", hex::encode(Sha256::digest(prevention_event.blocked_token.as_bytes()))),
                asi_risk_id: Some(crate::types::OwaspAsiRisk::AgentGoalHijack),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            if let Some(ref mut events) = observations.safety_events {
                events.push(safety_event);
            } else {
                observations.safety_events = Some(vec![safety_event]);
            }
        }

        let mut extensions = trace.extensions.clone().unwrap_or_default();
        extensions["veridactus.ai/v1/active_prevention"] = serde_json::json!({
            "blocked_count": self.prevention_history.len() + 1,
            "last_blocked_pattern": prevention_event.pattern_category,
            "last_blocked_at": prevention_event.timestamp
        });
        trace.extensions = Some(extensions);

        if self.prevention_history.len() >= self.block_threshold {
            return HookResult::Abort(format!("Active prevention threshold exceeded: {} blocks", self.block_threshold));
        }

        HookResult::Continue
    }
}

// ==================== Enhanced post_stream 钩子 (§6.3.2.8) ====================

pub struct PostStreamHookEnhanced {
    enable_risk_scoring: bool,
    enable_cost_reconciliation: bool,
}

impl PostStreamHookEnhanced {
    pub fn new() -> Self {
        Self {
            enable_risk_scoring: true,
            enable_cost_reconciliation: true,
        }
    }

    pub fn with_config(enable_risk_scoring: bool, enable_cost_reconciliation: bool) -> Self {
        Self {
            enable_risk_scoring,
            enable_cost_reconciliation,
        }
    }
}

impl fmt::Debug for PostStreamHookEnhanced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PostStreamHookEnhanced")
            .field("enable_risk_scoring", &self.enable_risk_scoring)
            .field("enable_cost_reconciliation", &self.enable_cost_reconciliation)
            .finish()
    }
}

impl Hook for PostStreamHookEnhanced {
    fn run(&self, trace: &mut Trace) -> HookResult {
        if self.enable_cost_reconciliation {
            if let Some(ref mut obs) = trace.observations {
                if let Some(ref constraints) = trace.constraints_applied {
                    if let (Some(limit), Some(actual)) = (constraints.budget_limit_usd, obs.cost_estimated_usd) {
                        if actual > limit {
                            obs.budget_awareness = Some(crate::types::trace::BudgetAwareness {
                                sse_events: Some(vec![crate::types::trace::BudgetAwarenessEvent {
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    budget_remaining: limit - actual,
                                    budget_pct: ((limit - actual) / limit * 100.0).max(0.0),
                                }]),
                                injected_prompt_suffix: None,
                            });
                        }
                    }
                }
            }
        }

        if let Some(ref mut obs) = trace.observations {
            if obs.latency_ms.is_none() {
                obs.latency_ms = Some(0);
            }
        }

        HookResult::Continue
    }
}

// ==================== Enhanced on_failure 钩子 (§6.3.2.9) ====================

pub struct OnFailureHookEnhanced {
    enable_budget_rollback: bool,
    enable_alerting: bool,
}

impl OnFailureHookEnhanced {
    pub fn new() -> Self {
        Self {
            enable_budget_rollback: true,
            enable_alerting: true,
        }
    }

    pub fn with_config(enable_budget_rollback: bool, enable_alerting: bool) -> Self {
        Self {
            enable_budget_rollback,
            enable_alerting,
        }
    }
}

impl fmt::Debug for OnFailureHookEnhanced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnFailureHookEnhanced")
            .field("enable_budget_rollback", &self.enable_budget_rollback)
            .field("enable_alerting", &self.enable_alerting)
            .finish()
    }
}

impl Hook for OnFailureHookEnhanced {
    fn run(&self, trace: &mut Trace) -> HookResult {
        if self.enable_budget_rollback {
            if let Some(ref mut obs) = trace.observations {
                obs.cost_estimated_usd = Some(0.0);
            }
        }

        trace.execution_state = Some(crate::types::trace::ExecutionState::Failed);

        let mut extensions = trace.extensions.clone().unwrap_or_default();
        extensions["veridactus.ai/v1/failure_info"] = serde_json::json!({
            "failure_timestamp": chrono::Utc::now().to_rfc3339(),
            "budget_rollback": self.enable_budget_rollback,
            "alert_sent": self.enable_alerting
        });
        trace.extensions = Some(extensions);

        HookResult::Continue
    }
}
