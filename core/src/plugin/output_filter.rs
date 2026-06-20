//! # G2 输出过滤器
//!
//! 协议 §5.6 G2: 输出内容安全扫描。
//! 扫描 LLM 输出中的 PII 泄露、有害内容、不安全代码。

use regex::Regex;

/// G2 输出过滤结果
#[derive(Debug, Clone)]
pub struct OutputFilterResult {
    /// 是否通过过滤器
    pub passed: bool,
    /// 检测到的违规类型
    pub violations: Vec<String>,
    /// 是否截断响应
    pub truncated: bool,
    /// 处理后的输出文本
    pub filtered_text: String,
}

/// G2 输出过滤器
pub struct OutputFilter {
    pii_patterns: Vec<Regex>,
    harmful_patterns: Vec<Regex>,
    unsafe_code_patterns: Vec<Regex>,
}

/// 创建默认 PII 正则模式
fn default_pii_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
        Regex::new(r"1[3-9]\d{9}").unwrap(),
        Regex::new(r"\d{17}[\dXx]").unwrap(),
        Regex::new(r"\b(?:\d[ -]*?){13,19}\b").unwrap(),
    ]
}

fn default_harmful_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r"(?i)\b(hate\s*speech|violence|terrorism|illegal)\b").unwrap(),
        Regex::new(r"(?i)\b(child\s*abuse|exploitation)\b").unwrap(),
        Regex::new(r"(?i)\b(self[- ]?harm|suicide\s*method)\b").unwrap(),
    ]
}

fn default_unsafe_code_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r"(?i)(rm\s+-rf\s+/|sudo\s+rm\b|DROP\s+TABLE|DELETE\s+FROM)").unwrap(),
        Regex::new(r"(?i)(eval\s*\(|exec\s*\(|os\.system\s*\()").unwrap(),
    ]
}

impl OutputFilter {
    pub fn new() -> Self {
        Self {
            pii_patterns: default_pii_patterns(),
            harmful_patterns: default_harmful_patterns(),
            unsafe_code_patterns: default_unsafe_code_patterns(),
        }
    }

    /// 扫描输出文本，返回过滤结果
    pub fn scan(&self, text: &str) -> OutputFilterResult {
        let mut violations = Vec::new();
        let mut filtered_text = text.to_string();

        // 1. PII 检测
        for pattern in &self.pii_patterns {
            if pattern.is_match(text) {
                violations.push("pii_leak".to_string());
                // 遮蔽 PII
                filtered_text = pattern
                    .replace_all(&filtered_text, "[REDACTED]")
                    .to_string();
            }
        }

        // 2. 有害内容检测
        for pattern in &self.harmful_patterns {
            if let Some(m) = pattern.find(text) {
                violations.push(format!("harmful_content:{}", m.as_str()));
            }
        }

        // 3. 不安全代码检测
        for pattern in &self.unsafe_code_patterns {
            if pattern.is_match(text) {
                violations.push("unsafe_code".to_string());
            }
        }

        let passed = violations.is_empty();

        OutputFilterResult {
            passed,
            violations,
            truncated: !passed, // 检测到违规时标记为截断
            filtered_text,
        }
    }

    /// 仅 PII 遮蔽（保留内容，仅遮盖敏感信息）
    pub fn mask_pii(&self, text: &str) -> String {
        let mut result = text.to_string();
        for pattern in &self.pii_patterns {
            result = pattern.replace_all(&result, "[REDACTED]").to_string();
        }
        result
    }
}

impl Default for OutputFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pii_detection() {
        let filter = OutputFilter::new();
        let result = filter.scan("My email is test@example.com");
        assert!(!result.passed);
        assert!(result.violations.contains(&"pii_leak".to_string()));
        assert!(result.filtered_text.contains("[REDACTED]"));
    }

    #[test]
    fn test_harmful_detection() {
        let filter = OutputFilter::new();
        let result = filter.scan("This contains hate speech content");
        assert!(!result.passed);
    }

    #[test]
    fn test_clean_output() {
        let filter = OutputFilter::new();
        let result = filter.scan("The weather is nice today.");
        assert!(result.passed);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_unsafe_code_detection() {
        let filter = OutputFilter::new();
        let result = filter.scan("Run: rm -rf / to clean");
        assert!(!result.passed);
    }
}
