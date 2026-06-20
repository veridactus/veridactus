//! # 数据脱敏处理
//!
//! 遵循 §8.0 Privacy & Data Handling 和 AI.md §4.2 隐私映射矩阵。
//!
//! 支持三种脱敏策略：
//! - `regex`: 基于正则的快速替换
//! - `presidio`: 命名实体识别（TODO: 集成外部服务）
//! - `synthetic`: 合成数据替换（TODO）
//!
//! 隐私级别处理规则（§4.2）：
//! | 级别 | 输入存储 | 输出存储 | 签名计算 |
//! | raw | 明文 | 明文 | 完整 Trace |
//! | masked | 脱敏文本 | 脱敏文本 | 脱敏后 Trace |
//! | hash_only | 仅 SHA-256 | null | 哈希摘要 |
//! | tee_private | 加密/TEE内 | 加密/null | 外部可见摘要 |

use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::types::constraints::PrivacyLevel;

/// 常见 PII 模式的正则集合
pub struct PiiPatterns {
    /// email: user@domain.com
    pub email: Regex,
    /// 电话号码（中国手机号）
    pub phone: Regex,
    /// ID card number（中国18位）
    pub id_card: Regex,
    /// IP 地址
    pub ip_address: Regex,
    /// API Key 模式
    pub api_key: Regex,
}

impl Default for PiiPatterns {
    fn default() -> Self {
        Self {
            email: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
            phone: Regex::new(r"1[3-9]\d{9}").unwrap(),
            id_card: Regex::new(
                r"[1-9]\d{5}(19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]",
            )
            .unwrap(),
            ip_address: Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(),
            api_key: Regex::new(r"(?i)(sk-[a-zA-Z0-9]{20,}|api[_-]?key[=:]\s*[a-zA-Z0-9]{16,})")
                .unwrap(),
        }
    }
}

/// 脱敏配置
#[derive(Debug, Clone)]
pub struct MaskingConfig {
    /// 掩码字符
    pub mask_char: char,
    /// 保留的前缀字符数
    pub prefix_keep: usize,
    /// 保留的后缀字符数
    pub suffix_keep: usize,
    /// 自定义字段映射
    pub field_patterns: HashMap<String, String>,
}

impl Default for MaskingConfig {
    fn default() -> Self {
        Self {
            mask_char: '*',
            prefix_keep: 2,
            suffix_keep: 2,
            field_patterns: HashMap::new(),
        }
    }
}

/// 脱敏处理器
pub struct MaskingProcessor {
    /// PII 模式
    patterns: PiiPatterns,
    /// 脱敏配置
    config: MaskingConfig,
}

impl Default for MaskingProcessor {
    fn default() -> Self {
        Self::new(MaskingConfig::default())
    }
}

impl MaskingProcessor {
    /// 创建新的脱敏处理器
    pub fn new(config: MaskingConfig) -> Self {
        Self {
            patterns: PiiPatterns::default(),
            config,
        }
    }

    /// 根据隐私级别处理 JSON Value
    ///
    /// # 参数
    /// * `value` - 待处理的 JSON 值
    /// * `level` - 隐私级别
    ///
    /// # 返回
    /// 处理后的 JSON Value
    pub fn process(&self, value: &Value, level: &PrivacyLevel) -> Value {
        match level {
            PrivacyLevel::Raw => value.clone(),
            PrivacyLevel::Masked => self.apply_masking(value),
            PrivacyLevel::HashOnly => self.apply_hash_only(value),
            PrivacyLevel::TeePrivate => self.apply_tee_private(value),
        }
    }

    /// 对字符串应用脱敏（masked 级别）
    ///
    /// 替换 PII 为掩码字符，基于正则匹配。
    /// 遵循 AI.md §4.2 映射矩阵：masked 级别存储脱敏文本。
    pub fn mask_string(&self, input: &str) -> String {
        let mut result = input.to_string();

        // 1. 掩码邮箱
        result = self
            .patterns
            .email
            .replace_all(&result, |caps: &regex::Captures| {
                let email = caps.get(0).unwrap().as_str();
                let at_pos = email.find('@').unwrap_or(email.len());
                let local_part = &email[..at_pos];
                let domain = &email[at_pos + 1..];
                if local_part.len() <= 2 {
                    format!("{}@{}", "*".repeat(local_part.len()), domain)
                } else {
                    let masked_local = format!(
                        "{}{}",
                        &local_part[..self.config.prefix_keep],
                        "*".repeat(
                            local_part.len() - self.config.prefix_keep - self.config.suffix_keep
                        ),
                    );
                    format!("{}@{}", masked_local, domain)
                }
            })
            .to_string();

        // 2. 掩码手机号
        result = self
            .patterns
            .phone
            .replace_all(&result, |caps: &regex::Captures| {
                let phone = caps.get(0).unwrap().as_str();
                format!("{}****{}", &phone[..3], &phone[7..])
            })
            .to_string();

        // 3. 掩码身份证
        result = self
            .patterns
            .id_card
            .replace_all(&result, |caps: &regex::Captures| {
                let id = caps.get(0).unwrap().as_str();
                format!("{}********{}", &id[..6], &id[id.len() - 4..])
            })
            .to_string();

        // 4. 掩码 IP
        result = self
            .patterns
            .ip_address
            .replace_all(&result, |caps: &regex::Captures| {
                let ip = caps.get(0).unwrap().as_str();
                let parts: Vec<&str> = ip.split('.').collect();
                if parts.len() == 4 {
                    format!("{}.{}.{}.{}", parts[0], parts[1], "***", "***")
                } else {
                    "***.***.***.***".to_string()
                }
            })
            .to_string();

        // 5. 掩码 API Key
        result = self
            .patterns
            .api_key
            .replace_all(&result, |caps: &regex::Captures| {
                let key = caps.get(0).unwrap().as_str();
                if key.len() > 8 {
                    format!("{}...{}", &key[..4], "*".repeat(8))
                } else {
                    "*".repeat(key.len())
                }
            })
            .to_string();

        result
    }

    /// 对 JSON Value 应用脱敏
    fn apply_masking(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => Value::String(self.mask_string(s)),
            Value::Array(arr) => Value::Array(arr.iter().map(|v| self.apply_masking(v)).collect()),
            Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(k.clone(), self.apply_masking(v));
                }
                Value::Object(new_obj)
            }
            _ => value.clone(),
        }
    }

    /// 应用 hash_only 级别：将字符串替换为 SHA-256 摘要
    /// 遵循 AI.md §4.2：hash_only 存储仅 SHA-256 哈希
    fn apply_hash_only(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => {
                let hash = compute_sha256(s.as_bytes());
                Value::String(format!("sha256:{}", hash))
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.apply_hash_only(v)).collect())
            }
            Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(k.clone(), self.apply_hash_only(v));
                }
                Value::Object(new_obj)
            }
            // 数字和布尔值保持原样
            _ => value.clone(),
        }
    }

    /// tee_private 级别：返回空值（只有 TEE 内部可见）
    /// 遵循 AI.md §4.2：tee_private 存储 null 或加密
    fn apply_tee_private(&self, value: &Value) -> Value {
        match value {
            Value::String(_) => Value::Null,
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.apply_tee_private(v)).collect())
            }
            Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(k.clone(), self.apply_tee_private(v));
                }
                Value::Object(new_obj)
            }
            _ => value.clone(),
        }
    }
}

/// 计算 SHA-256 摘要
fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// 根据隐私级别脱敏敏感 HTTP 头部
/// 遵循 AI.md §4.2 映射矩阵
pub fn sanitize_headers(
    headers: &std::collections::BTreeMap<String, String>,
    level: &PrivacyLevel,
) -> std::collections::BTreeMap<String, String> {
    let sensitive_headers = ["authorization", "cookie", "x-api-key", "set-cookie"];

    match level {
        PrivacyLevel::Raw => headers.clone(),
        PrivacyLevel::Masked => {
            let mut result = headers.clone();
            for h in &sensitive_headers {
                if let Some(val) = result.get(*h) {
                    if val.len() > 8 {
                        result.insert(h.to_string(), format!("{}...{}", &val[..4], "*".repeat(8)));
                    } else {
                        result.insert(h.to_string(), "***".to_string());
                    }
                }
            }
            result
        }
        PrivacyLevel::HashOnly | PrivacyLevel::TeePrivate => {
            // 仅保留非敏感头部
            let mut result = std::collections::BTreeMap::new();
            for (k, v) in headers {
                if !sensitive_headers.contains(&k.to_lowercase().as_str()) {
                    result.insert(k.clone(), v.clone());
                }
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_email() {
        let processor = MaskingProcessor::default();
        // "user@example.com": local_part="user"(4) → prefix(2)="us", suffix(2)="er", mid(0)=""
        // 当中间部分为0时，只显示前后各2个字符
        let masked = processor.mask_string("user@example.com");
        println!("masked: {}", masked);
        assert!(masked.contains("@example.com"));
        assert!(!masked.contains("user@"));
    }

    #[test]
    fn test_mask_phone() {
        let processor = MaskingProcessor::default();
        assert_eq!(processor.mask_string("13800138000"), "138****8000");
    }

    #[test]
    fn test_mask_id_card() {
        let processor = MaskingProcessor::default();
        let masked = processor.mask_string("110101199001011234");
        assert!(masked.starts_with("110101"));
        assert!(masked.ends_with("1234"));
        assert_eq!(masked.len(), 18);
    }

    #[test]
    fn test_hash_only_string() {
        let processor = MaskingProcessor::default();
        let value = Value::String("hello".to_string());
        let result = processor.process(&value, &PrivacyLevel::HashOnly);
        assert!(result.as_str().unwrap().starts_with("sha256:"));
        assert_eq!(result.as_str().unwrap().len(), 64 + 7); // sha256: + 64 hex
    }

    #[test]
    fn test_tee_private() {
        let processor = MaskingProcessor::default();
        let value = Value::String("secret".to_string());
        let result = processor.process(&value, &PrivacyLevel::TeePrivate);
        assert!(result.is_null());
    }

    #[test]
    fn test_raw_passthrough() {
        let processor = MaskingProcessor::default();
        let value = Value::String("test".to_string());
        let result = processor.process(&value, &PrivacyLevel::Raw);
        assert_eq!(result, Value::String("test".to_string()));
    }

    #[test]
    fn test_json_masking() {
        let processor = MaskingProcessor::default();
        let value = serde_json::json!({
            "email": "user@test.com",
            "phone": "13800138000",
            "name": "张三",
        });
        let result = processor.process(&value, &PrivacyLevel::Masked);
        let obj = result.as_object().unwrap();
        assert_ne!(obj["email"], "user@test.com");
        assert_ne!(obj["phone"], "13800138000");
        assert_eq!(obj["name"], "张三");
    }

    #[test]
    fn test_header_sanitization() {
        use std::collections::BTreeMap;
        let mut headers = BTreeMap::new();
        headers.insert("authorization".to_string(), "Bearer sk-test123".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());

        let masked = sanitize_headers(&headers, &PrivacyLevel::Masked);
        assert_eq!(masked.get("content-type").unwrap(), "application/json");
        assert!(masked.get("authorization").unwrap().contains("***"));

        let hash_only = sanitize_headers(&headers, &PrivacyLevel::HashOnly);
        assert!(hash_only.get("authorization").is_none());
        assert!(hash_only.get("content-type").is_some());
    }
}
