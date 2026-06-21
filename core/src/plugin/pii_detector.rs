//! # PII 检测插件
//!
//! 生产级敏感信息检测插件，遵循 GDPR、PCI-DSS 等合规要求。
//! 支持检测：身份证、信用卡、电话、邮箱地址等敏感信息。

use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::plugin::governance::{
    AsyncContext, GovernancePlugin, PluginMetadata, PluginType, RequestContext, ResponseContext,
    StreamChunkContext,
};
use crate::types::journal::{ExecutionJournal, JournalEventType};
use crate::types::{Action, OwaspAsiRisk, SafetyAction, SafetyEvent, SafetyTrigger, Severity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PIIDetectorConfig {
    pub enabled: bool,
    pub detect_types: Vec<PIIType>,
    pub action_on_detect: PIIDetectAction,
    pub mask_character: String,
    pub log_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PIIType {
    ChinaIdCard,
    CreditCard,
    PhoneNumber,
    Email,
    BankAccount,
    Passport,
    SocialSecurity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PIIDetectAction {
    Block,
    Mask,
    Flag,
}

impl Default for PIIDetectorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detect_types: vec![
                PIIType::ChinaIdCard,
                PIIType::CreditCard,
                PIIType::PhoneNumber,
                PIIType::Email,
            ],
            action_on_detect: PIIDetectAction::Mask,
            mask_character: "*".to_string(),
            log_only: false,
        }
    }
}

pub struct PIIDetectorPlugin {
    config: PIIDetectorConfig,
    patterns: Vec<(PIIType, Regex, String)>,
}

impl PIIDetectorPlugin {
    pub fn new(config: PIIDetectorConfig) -> Self {
        let patterns = Self::build_patterns();
        Self { config, patterns }
    }

    fn build_patterns() -> Vec<(PIIType, Regex, String)> {
        vec![
            (PIIType::ChinaIdCard, Regex::new(r"\b[1-9]\d{5}(18|19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]\b").unwrap(), "ID card number".to_string()),
            (PIIType::CreditCard, Regex::new(r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\b").unwrap(), "Credit card number".to_string()),
            (PIIType::CreditCard, Regex::new(r"\b(?:\d{4}[- ]?){3}\d{4}\b").unwrap(), "Credit card number(带分隔符)".to_string()),
            (PIIType::PhoneNumber, Regex::new(r"\b1[3-9]\d{9}\b").unwrap(), "CN phone number".to_string()),
            (PIIType::Email, Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(), "Email address".to_string()),
            (PIIType::BankAccount, Regex::new(r"\b[0-9]{16,19}\b").unwrap(), "Bank account".to_string()),
            (PIIType::Passport, Regex::new(r"\b[A-Z]{1,2}[0-9]{6,9}\b").unwrap(), "Passport number".to_string()),
        ]
    }

    fn detect_pii(&self, content: &str) -> Vec<(PIIType, String, usize)> {
        let mut findings = Vec::new();
        for (pii_type, pattern, _) in &self.patterns {
            if !self.config.detect_types.contains(pii_type) {
                continue;
            }
            for mat in pattern.find_iter(content) {
                findings.push((pii_type.clone(), mat.as_str().to_string(), mat.start()));
            }
        }
        findings.sort_by_key(|f| f.2);
        findings
    }

    fn mask_content(&self, content: &str, findings: &[(PIIType, String, usize)]) -> String {
        if findings.is_empty() {
            return content.to_string();
        }

        let chars: Vec<char> = content.chars().collect();
        let char_count = chars.len();
        let mut result_chars: Vec<char> = chars.clone();
        let _offset: isize = 0;

        for (_, pii_str, byte_start) in findings {
            let pii_chars: Vec<char> = pii_str.chars().collect();
            let pii_char_len = pii_chars.len();

            let char_start = self.byte_offset_to_char_offset(content, *byte_start);
            let char_end = char_start + pii_char_len;

            if char_end <= char_count && pii_char_len >= 4 {
                let mask_len = pii_char_len.min(8).max(4);
                let prefix: String = pii_chars[..2.min(pii_char_len)].iter().collect();
                let suffix: String = pii_chars[pii_char_len.saturating_sub(2)..].iter().collect();
                let mask: String = format!(
                    "{}{}{}",
                    prefix,
                    self.config.mask_character.repeat(mask_len),
                    suffix
                );

                for (i, m) in mask.chars().enumerate() {
                    let pos = (char_start + i) as usize;
                    if pos < result_chars.len() {
                        result_chars[pos] = m;
                    }
                }
                for i in mask.len()..pii_char_len {
                    let pos = (char_start + i) as usize;
                    if pos < result_chars.len() {
                        result_chars[pos] = '*';
                    }
                }
            }
        }

        result_chars.iter().collect()
    }

    fn byte_offset_to_char_offset(&self, content: &str, byte_offset: usize) -> usize {
        content
            .char_indices()
            .position(|(idx, _)| idx == byte_offset)
            .unwrap_or(0)
    }

    fn create_safety_event(&self, _pii_type: &PIIType, content_hash: String) -> SafetyEvent {
        SafetyEvent {
            trigger_type: SafetyTrigger::G2OutputFilter,
            severity: Severity::High,
            action_taken: match self.config.action_on_detect {
                PIIDetectAction::Block => SafetyAction::Blocked,
                PIIDetectAction::Mask => SafetyAction::Rewritten,
                PIIDetectAction::Flag => SafetyAction::Flagged,
            },
            content_hash,
            asi_risk_id: Some(OwaspAsiRisk::SensitiveDataExfiltration),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[async_trait]
impl GovernancePlugin for PIIDetectorPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "pii-detector".to_string(),
            plugin_type: PluginType::Native,
            version: "0.2.1".to_string(),
            description: "PII detection plugin - masks ID/credit card/phone/email".to_string(),
            author: Some("VERIDACTUS Team".to_string()),
            supported_protocol_versions: crate::types::VersionRange {
                min: "0.2.0".to_string(),
                max: "0.2.1".to_string(),
            },
        }
    }

    async fn on_request(
        &self,
        ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, String> {
        if !self.config.enabled {
            return Ok(Action::Continue);
        }

        let body = ctx.body.as_deref().unwrap_or("");
        let findings = self.detect_pii(body);

        if !findings.is_empty() {
            let content_hash = crate::crypto::signature::compute_sha256_hex(body.as_bytes());
            let pii_types: Vec<String> = findings
                .iter()
                .map(|(t, _, _)| format!("{:?}", t))
                .collect();
            let pii_strs: Vec<String> = findings.iter().map(|(_, s, _)| s.clone()).collect();

            journal.append_event(JournalEventType::SafetyEvent(
                self.create_safety_event(&findings[0].0, content_hash.clone()),
            ));

            if self.config.log_only {
                tracing::warn!(
                    "PII detected (request): found {:?} - {:?}",
                    pii_types,
                    pii_strs
                );
            } else {
                match self.config.action_on_detect {
                    PIIDetectAction::Block => {
                        return Err(format!(
                            "PII block: sensitive info in request ({:?})",
                            pii_types
                        ));
                    }
                    PIIDetectAction::Mask | PIIDetectAction::Flag => {
                        let masked = self.mask_content(body, &findings);
                        ctx.body = Some(masked);
                        tracing::info!("PII masked (request)");
                    }
                }
            }
        }

        Ok(Action::Continue)
    }

    async fn on_stream_chunk(
        &self,
        _ctx: &mut StreamChunkContext,
        _journal: &mut ExecutionJournal,
    ) -> Result<Action, String> {
        Ok(Action::Continue)
    }

    async fn on_response(
        &self,
        ctx: &mut ResponseContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, String> {
        if !self.config.enabled {
            return Ok(Action::Continue);
        }

        let findings = self.detect_pii(&ctx.response);

        if !findings.is_empty() {
            let content_hash =
                crate::crypto::signature::compute_sha256_hex(ctx.response.as_bytes());
            let pii_types: Vec<String> = findings
                .iter()
                .map(|(t, _, _)| format!("{:?}", t))
                .collect();

            journal.append_event(JournalEventType::SafetyEvent(
                self.create_safety_event(&findings[0].0, content_hash),
            ));

            if self.config.log_only {
                tracing::warn!("PII detected (response): found {:?}", pii_types);
            } else {
                match self.config.action_on_detect {
                    PIIDetectAction::Block => {
                        return Err(format!(
                            "PII block: sensitive info in response ({:?})",
                            pii_types
                        ));
                    }
                    PIIDetectAction::Mask => {
                        let masked = self.mask_content(&ctx.response, &findings);
                        ctx.response = masked;
                        tracing::info!("PII 已遮蔽 (响应)");
                    }
                    PIIDetectAction::Flag => {
                        tracing::warn!("PII 已标记 (响应): {:?}", pii_types);
                    }
                }
            }
        }

        Ok(Action::Continue)
    }

    async fn on_async_finalize(
        &self,
        _ctx: &mut AsyncContext,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "plugin": "pii-detector",
            "status": "ok"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_china_id_card() {
        let plugin = PIIDetectorPlugin::new(PIIDetectorConfig::default());
        let mut ctx = RequestContext {
            headers: std::collections::HashMap::new(),
            body: Some("用户ID card number: 110101199003074532".to_string()),
            trace_id: uuid::Uuid::new_v4(),
            tenant_id: "test".to_string(),

            plugin_config: None,
        };
        let mut journal = ExecutionJournal::new(uuid::Uuid::new_v4(), "test");
        let result = plugin.on_request(&mut ctx, &mut journal).await;
        assert!(result.is_ok());
        let body = ctx.body.unwrap();
        assert!(body.contains("**"));
        assert!(!body.contains("110101199003074532"));
    }

    #[tokio::test]
    async fn test_detect_credit_card() {
        let plugin = PIIDetectorPlugin::new(PIIDetectorConfig::default());
        let mut ctx = RequestContext {
            headers: std::collections::HashMap::new(),
            body: Some("Credit card number: 4532015112830366".to_string()),
            trace_id: uuid::Uuid::new_v4(),
            tenant_id: "test".to_string(),

            plugin_config: None,
        };
        let mut journal = ExecutionJournal::new(uuid::Uuid::new_v4(), "test");
        let result = plugin.on_request(&mut ctx, &mut journal).await;
        assert!(result.is_ok());
        let body = ctx.body.unwrap();
        assert!(body.contains("*"));
    }

    #[tokio::test]
    async fn test_detect_phone_number() {
        let plugin = PIIDetectorPlugin::new(PIIDetectorConfig::default());
        let mut ctx = RequestContext {
            headers: std::collections::HashMap::new(),
            body: Some("联系电话: 13812345678".to_string()),
            trace_id: uuid::Uuid::new_v4(),
            tenant_id: "test".to_string(),

            plugin_config: None,
        };
        let mut journal = ExecutionJournal::new(uuid::Uuid::new_v4(), "test");
        let result = plugin.on_request(&mut ctx, &mut journal).await;
        assert!(result.is_ok());
        let body = ctx.body.unwrap();
        assert!(body.contains("*"));
        assert!(!body.contains("13812345678"));
    }

    #[tokio::test]
    async fn test_detect_email() {
        let plugin = PIIDetectorPlugin::new(PIIDetectorConfig::default());
        let mut ctx = RequestContext {
            headers: std::collections::HashMap::new(),
            body: Some("邮箱: zhangsan@example.com".to_string()),
            trace_id: uuid::Uuid::new_v4(),
            tenant_id: "test".to_string(),

            plugin_config: None,
        };
        let mut journal = ExecutionJournal::new(uuid::Uuid::new_v4(), "test");
        let result = plugin.on_request(&mut ctx, &mut journal).await;
        assert!(result.is_ok());
        let body = ctx.body.unwrap();
        assert!(body.contains("*"));
    }

    #[tokio::test]
    async fn test_safe_content() {
        let plugin = PIIDetectorPlugin::new(PIIDetectorConfig::default());
        let mut ctx = RequestContext {
            headers: std::collections::HashMap::new(),
            body: Some("今天天气很好".to_string()),
            trace_id: uuid::Uuid::new_v4(),
            tenant_id: "test".to_string(),

            plugin_config: None,
        };
        let mut journal = ExecutionJournal::new(uuid::Uuid::new_v4(), "test");
        let result = plugin.on_request(&mut ctx, &mut journal).await;
        assert!(result.is_ok());
        assert_eq!(ctx.body.unwrap(), "今天天气很好");
    }
}
