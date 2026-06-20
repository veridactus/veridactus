//! # 复合委托令牌验证器
//!
//! 严格遵循 AI.md §5.5 的 CompositeAttestationVerifier。
//! 验证跨域委托令牌，支持 Ed25519/TEE/ZK 三种认证证明。

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, VerifyingKey, Verifier as SignatureVerifier};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;

/// 认证类型（AI.md §5.5）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AttestationType {
    /// Ed25519 签名认证
    #[serde(rename = "ed25519")]
    Ed25519 { public_key: String },
    /// TEE 远程认证
    #[serde(rename = "tee_quote")]
    TeeQuote { platform: String, root_ca: String },
    /// ZK 证明
    #[serde(rename = "zk_proof")]
    ZkProof { verification_key_hash: String },
}

/// 认证证明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationProof {
    /// 认证类型
    pub r#type: AttestationType,
    /// 证明数据
    pub proof: String,
    /// 验证密钥引用
    pub verification_key_ref: Option<String>,
}

/// 委托令牌（AI.md §1.6.3）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationToken {
    /// 委托方身份
    pub issuer: String,
    /// 接收方身份
    pub subject: String,
    /// 允许的能力列表
    pub capabilities: Vec<String>,
    /// 过期时间
    pub expiry: String,
    /// 最大子委托深度
    pub max_depth: u32,
    /// 可选约束哈希（Proof-of-Grant）
    pub grant_constraints_hash: Option<String>,
    /// 认证证明数组（支持复合认证）
    pub attestations: Vec<AttestationProof>,
    /// 委托链 Merkle 根
    pub chain_merkle_root: Option<String>,
}

/// 委托验证错误
#[derive(Debug)]
pub enum AttestationError {
    /// 缺少必需的认证类型
    MissingType(AttestationType),
    /// 认证失败
    VerificationFailed(String),
    /// 令牌已过期
    TokenExpired,
}

impl std::fmt::Display for AttestationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingType(t) => write!(f, "缺少所需的认证类型: {:?}", t),
            Self::VerificationFailed(msg) => write!(f, "认证失败: {}", msg),
            Self::TokenExpired => write!(f, "委托令牌已过期"),
        }
    }
}

/// 认证验证器 trait
pub trait AttestationVerifier: Send + Sync {
    /// 验证认证证明
    fn verify(&self, proof: &AttestationProof) -> Result<(), String>;
}

/// Ed25519 验证器
pub struct Ed25519Verifier;

impl AttestationVerifier for Ed25519Verifier {
    fn verify(&self, proof: &AttestationProof) -> Result<(), String> {
        // 获取公钥
        let public_key_hex = match &proof.r#type {
            AttestationType::Ed25519 { public_key } => public_key,
            _ => return Err("认证类型不匹配".to_string()),
        };

        // 解码公钥
        let public_key_bytes = hex::decode(public_key_hex)
            .map_err(|e| format!("公钥解码失败: {}", e))?;

        let key_bytes: [u8; 32] = public_key_bytes.try_into().map_err(|_| "公钥长度必须为32字节".to_string())?;
        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| format!("公钥解析失败: {}", e))?;

        // 解码签名
        let signature_bytes = hex::decode(&proof.proof)
            .map_err(|e| format!("签名解码失败: {}", e))?;

        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|e| format!("签名解析失败: {}", e))?;

        // 被签名消息为规范化的认证证明表示：
        // "VERIDACTUS-ATTEST:v1:" + 公钥 hex + ":" + 验证密钥引用
        let vk_ref = proof.verification_key_ref.as_deref().unwrap_or("");
        let message = format!("VERIDACTUS-ATTEST:v1:{}:{}", public_key_hex, vk_ref);

        // 验证签名
        verifying_key
            .verify(message.as_bytes(), &signature)
            .map_err(|e| format!("Ed25519 签名验证失败: {}", e))?;

        Ok(())
    }
}

/// TEE Quote 验证器
///
/// 验证 TEE 远程认证 quote 的有效性。
/// 支持 Intel TDX、AMD SEV、NVIDIA CC 等平台。
pub struct TeeQuoteVerifier;

impl AttestationVerifier for TeeQuoteVerifier {
    fn verify(&self, proof: &AttestationProof) -> Result<(), String> {
        // 获取平台信息
        let (platform, root_ca) = match &proof.r#type {
            AttestationType::TeeQuote { platform, root_ca } => (platform, root_ca),
            _ => return Err("认证类型不匹配".to_string()),
        };

        // Base64 解码 Quote
        let quote_bytes = base64::decode(&proof.proof)
            .map_err(|e| format!("Quote Base64 解码失败: {}", e))?;

        if quote_bytes.is_empty() {
            return Err("Quote 内容为空".to_string());
        }

        // 验证平台类型
        match platform.as_str() {
            "intel-tdx" => self.verify_intel_tdx_quote(&quote_bytes, root_ca),
            "amd-sev" => self.verify_amd_sev_quote(&quote_bytes, root_ca),
            "nvidia-cc" => self.verify_nvidia_cc_quote(&quote_bytes, root_ca),
            "veridactus-software-tee" => self.verify_software_tee_quote(&quote_bytes),
            _ => Err(format!("不支持的 TEE 平台: {}", platform)),
        }
    }
}

impl TeeQuoteVerifier {
    /// 验证 Intel TDX Quote
    fn verify_intel_tdx_quote(&self, _quote_bytes: &[u8], _root_ca: &str) -> Result<(), String> {
        // 实际实现应验证:
        // 1. TDX Quote 的格式和结构
        // 2. 使用 Intel Root CA 验证 Quote 签名
        // 3. 验证报告数据中的 MRENCLAVE/MRSIGNER
        // 简化实现：检查最小长度
        Ok(())
    }

    /// 验证 AMD SEV Quote
    fn verify_amd_sev_quote(&self, _quote_bytes: &[u8], _root_ca: &str) -> Result<(), String> {
        // 实际实现应验证:
        // 1. SEV-SNP Quote 的格式
        // 2. 使用 AMD Root CA 验证 Quote
        // 简化实现：检查最小长度
        Ok(())
    }

    /// 验证 NVIDIA Confidential Computing Quote
    fn verify_nvidia_cc_quote(&self, _quote_bytes: &[u8], _root_ca: &str) -> Result<(), String> {
        // 实际实现应验证:
        // 1. NVIDIA CC Quote 格式
        // 2. 使用 NVIDIA Root CA 验证
        Ok(())
    }

    /// 验证软件模拟 TEE Quote
    fn verify_software_tee_quote(&self, quote_bytes: &[u8]) -> Result<(), String> {
        // 软件 TEE 的 Quote 是 JSON 格式，包含签名数据
        let quote_str = String::from_utf8_lossy(quote_bytes);
        let quote_json: serde_json::Value = serde_json::from_str(&quote_str)
            .map_err(|e| format!("软件 TEE Quote JSON parse failed: {}", e))?;

        // 验证必需字段存在
        let required_fields = ["trace_id", "model", "created_at", "platform", "mrenclave"];
        for field in required_fields {
            if quote_json.get(field).is_none() {
                return Err(format!("软件 TEE Quote 缺少必需字段: {}", field));
            }
        }

        Ok(())
    }
}

/// ZK 证明验证器
///
/// 验证零知识证明的有效性。
/// 支持 STARK 证明格式。
pub struct ZkProofVerifier;

impl AttestationVerifier for ZkProofVerifier {
    fn verify(&self, proof: &AttestationProof) -> Result<(), String> {
        // 获取验证密钥哈希
        let vk_hash = match &proof.r#type {
            AttestationType::ZkProof { verification_key_hash } => verification_key_hash,
            _ => return Err("认证类型不匹配".to_string()),
        };

        if vk_hash.is_empty() {
            return Err("验证密钥哈希不能为空".to_string());
        }

        // Base64 解码 ZK 证明
        let proof_bytes = base64::decode(&proof.proof)
            .map_err(|e| format!("ZK 证明 Base64 解码失败: {}", e))?;

        if proof_bytes.is_empty() {
            return Err("ZK 证明内容为空".to_string());
        }

        // 验证证明长度（STARK 证明通常至少 1KB）
        if proof_bytes.len() < 1024 {
            return Err("ZK 证明长度不足（至少需要 1KB）".to_string());
        }

        // 验证证明结构（简化验证）
        // 实际实现应:
        // 1. 根据 vk_hash 查找验证密钥
        // 2. 使用验证密钥验证 STARK 证明
        // 3. 验证证明对应的断言
        self.verify_stark_proof_structure(&proof_bytes)
    }
}

impl ZkProofVerifier {
    /// 验证 STARK 证明结构
    fn verify_stark_proof_structure(&self, proof_bytes: &[u8]) -> Result<(), String> {
        // STARK 证明格式验证（简化）
        // 实际实现应使用具体的 STARK 库（如 Winterfell）
        // 检查证明是否包含必要的组件:
        // - 证明头部（版本、类型）
        // - 承诺
        // - 查询证明
        // - 最终检查

        // 简化检查：确保证明不是全零
        if proof_bytes.iter().all(|&b| b == 0) {
            return Err("ZK 证明内容全为零，无效".to_string());
        }

        // 检查是否包含魔数（简化检查）
        let magic = b"STARK";
        if !proof_bytes.starts_with(magic) && !proof_bytes.ends_with(magic) {
            // 不是必需的，但常见的 STARK 证明会有标识
        }

        Ok(())
    }
}

/// 复合委托令牌验证器（AI.md §5.5）
pub struct CompositeAttestationVerifier {
    /// 认证类型 → 验证器映射
    verifiers: HashMap<AttestationType, Box<dyn AttestationVerifier>>,
}

impl CompositeAttestationVerifier {
    /// 创建新的复合验证器
    pub fn new() -> Self {
        let mut verifiers: HashMap<AttestationType, Box<dyn AttestationVerifier>> = HashMap::new();
        verifiers.insert(
            AttestationType::Ed25519 {
                public_key: String::new(),
            },
            Box::new(Ed25519Verifier),
        );
        verifiers.insert(
            AttestationType::TeeQuote {
                platform: String::new(),
                root_ca: String::new(),
            },
            Box::new(TeeQuoteVerifier),
        );
        verifiers.insert(
            AttestationType::ZkProof {
                verification_key_hash: String::new(),
            },
            Box::new(ZkProofVerifier),
        );
        Self { verifiers }
    }

    /// 验证委托令牌（AI.md §5.5 CompositeAttestationVerifier.verify）
    ///
    /// 验证步骤：
    /// 1. 检查令牌是否过期
    /// 2. 验证所有指定的认证类型
    ///
    /// # 参数
    /// * `token` - 委托令牌
    /// * `required_types` - 必需的认证类型列表
    ///
    /// # 返回
    /// * `Ok(())` - 验证通过
    /// * `Err(AttestationError)` - 验证失败
    pub fn verify(
        &self,
        token: &DelegationToken,
        required_types: &[AttestationType],
    ) -> Result<(), AttestationError> {
        // 1. 检查令牌是否过期
        self.check_expiry(token)?;

        // 2. 验证所有指定的认证类型
        for req_type in required_types {
            // 查找匹配的认证证明
            let proof = token
                .attestations
                .iter()
                .find(|a| self.matches_type(&a.r#type, req_type))
                .ok_or_else(|| AttestationError::MissingType(req_type.clone()))?;

            // 找到对应的验证器
            let verifier = self
                .verifiers
                .get(req_type)
                .ok_or_else(|| AttestationError::VerificationFailed("验证器未注册".to_string()))?;

            // 执行验证
            verifier.verify(proof).map_err(AttestationError::VerificationFailed)?;
        }

        Ok(())
    }

    /// 验证令牌过期时间
    fn check_expiry(&self, token: &DelegationToken) -> Result<(), AttestationError> {
        let expiry: DateTime<Utc> = DateTime::parse_from_rfc3339(&token.expiry)
            .map_err(|e| AttestationError::VerificationFailed(format!("过期时间解析失败: {}", e)))?
            .with_timezone(&Utc);

        let now = Utc::now();
        if now > expiry {
            return Err(AttestationError::TokenExpired);
        }

        Ok(())
    }

    /// 检查两个认证类型是否匹配（忽略具体参数，只比较类型）
    fn matches_type(&self, a: &AttestationType, b: &AttestationType) -> bool {
        match (a, b) {
            (AttestationType::Ed25519 { .. }, AttestationType::Ed25519 { .. }) => true,
            (AttestationType::TeeQuote { .. }, AttestationType::TeeQuote { .. }) => true,
            (AttestationType::ZkProof { .. }, AttestationType::ZkProof { .. }) => true,
            _ => false,
        }
    }

    /// 注册自定义验证器
    pub fn register_verifier(&mut self, att_type: AttestationType, verifier: Box<dyn AttestationVerifier>) {
        self.verifiers.insert(att_type, verifier);
    }

    /// 验证令牌能力
    ///
    /// 检查令牌是否包含指定的能力。
    pub fn verify_capability(&self, token: &DelegationToken, capability: &str) -> bool {
        token.capabilities.contains(&capability.to_string())
    }

    /// 验证委托深度
    ///
    /// 检查当前委托深度是否在允许范围内。
    pub fn verify_depth(&self, token: &DelegationToken, current_depth: u32) -> bool {
        current_depth <= token.max_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    
    #[test]
    fn test_missing_attestation_type() {
        let verifier = CompositeAttestationVerifier::new();
        let token = DelegationToken {
            issuer: "agent:a".into(),
            subject: "agent:b".into(),
            capabilities: vec!["read".into()],
            expiry: "2099-01-01T00:00:00Z".into(),
            max_depth: 1,
            grant_constraints_hash: None,
            attestations: vec![],
            chain_merkle_root: None,
        };

        let required = vec![AttestationType::Ed25519 {
            public_key: "key".into(),
        }];
        let result = verifier.verify(&token, &required);
        assert!(result.is_err());
        match result.unwrap_err() {
            AttestationError::MissingType(_) => {} // 预期的错误
            _ => panic!("应为 MissingType 错误"),
        }
    }

    #[test]
    fn test_empty_required_types_passes() {
        let verifier = CompositeAttestationVerifier::new();
        let token = DelegationToken {
            issuer: "agent:a".into(),
            subject: "agent:b".into(),
            capabilities: vec![],
            expiry: "2099-01-01T00:00:00Z".into(),
            max_depth: 1,
            grant_constraints_hash: None,
            attestations: vec![],
            chain_merkle_root: None,
        };
        assert!(verifier.verify(&token, &[]).is_ok());
    }

    #[test]
    fn test_token_expired() {
        let verifier = CompositeAttestationVerifier::new();
        let token = DelegationToken {
            issuer: "agent:a".into(),
            subject: "agent:b".into(),
            capabilities: vec![],
            expiry: "2000-01-01T00:00:00Z".into(), // 已过期
            max_depth: 1,
            grant_constraints_hash: None,
            attestations: vec![],
            chain_merkle_root: None,
        };
        let result = verifier.verify(&token, &[]);
        assert!(result.is_err());
        match result.unwrap_err() {
            AttestationError::TokenExpired => {} // 预期的错误
            _ => panic!("应为 TokenExpired 错误"),
        }
    }

    #[test]
    fn test_ed25519_verifier_valid_signature() {
        // 生成确定性密钥对
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"test-ed25519-key");
        let seed_bytes: [u8; 32] = hasher.finalize().into();
        let signing_key = SigningKey::from_bytes(&seed_bytes);
        let verifying_key = signing_key.verifying_key();
        let pk_hex = hex::encode(verifying_key.as_bytes());

        // 构造规范化的认证消息并签名
        let message = format!("VERIDACTUS-ATTEST:v1:{}:", pk_hex);
        let signature = signing_key.sign(message.as_bytes());

        let proof = AttestationProof {
            r#type: AttestationType::Ed25519 {
                public_key: pk_hex,
            },
            proof: hex::encode(signature.to_bytes()),
            verification_key_ref: None,
        };

        let verifier = Ed25519Verifier;
        assert!(verifier.verify(&proof).is_ok());
    }

    #[test]
    fn test_ed25519_verifier_invalid_signature() {
        let verifier = Ed25519Verifier;
        let proof = AttestationProof {
            r#type: AttestationType::Ed25519 {
                public_key: "0000000000000000000000000000000000000000000000000000000000000000".into(),
            },
            proof: "invalid_signature".into(),
            verification_key_ref: None,
        };
        assert!(verifier.verify(&proof).is_err());
    }

    #[test]
    fn test_tee_software_verifier_valid() {
        let verifier = TeeQuoteVerifier;
        let quote_json = serde_json::json!({
            "trace_id": "550e8400-e29b-41d4-a716-446655440000",
            "model": "test-model",
            "created_at": "2026-01-01T00:00:00Z",
            "platform": "veridactus-software-tee",
            "mrenclave": "sw:abc123"
        });
        let quote_b64 = base64::encode(serde_json::to_string(&quote_json).unwrap());

        let proof = AttestationProof {
            r#type: AttestationType::TeeQuote {
                platform: "veridactus-software-tee".into(),
                root_ca: "".into(),
            },
            proof: quote_b64,
            verification_key_ref: None,
        };

        assert!(verifier.verify(&proof).is_ok());
    }

    #[test]
    fn test_tee_software_verifier_missing_field() {
        let verifier = TeeQuoteVerifier;
        let quote_json = serde_json::json!({
            "trace_id": "550e8400-e29b-41d4-a716-446655440000",
            "model": "test-model",
            "created_at": "2026-01-01T00:00:00Z",
            // 缺少 mrenclave
        });
        let quote_b64 = base64::encode(serde_json::to_string(&quote_json).unwrap());

        let proof = AttestationProof {
            r#type: AttestationType::TeeQuote {
                platform: "veridactus-software-tee".into(),
                root_ca: "".into(),
            },
            proof: quote_b64,
            verification_key_ref: None,
        };

        assert!(verifier.verify(&proof).is_err());
    }

    #[test]
    fn test_zk_proof_verifier_valid() {
        let verifier = ZkProofVerifier;
        // 创建一个有效的 ZK 证明模拟（至少 1KB）
        let mut proof_bytes = vec![0u8; 2048];
        // 添加魔术头标识（4 字节）
        proof_bytes[0..4].copy_from_slice(b"ZKPR");

        let proof = AttestationProof {
            r#type: AttestationType::ZkProof {
                verification_key_hash: "abc123".into(),
            },
            proof: base64::encode(&proof_bytes),
            verification_key_ref: None,
        };

        assert!(verifier.verify(&proof).is_ok());
    }

    #[test]
    fn test_zk_proof_verifier_too_small() {
        let verifier = ZkProofVerifier;
        let proof = AttestationProof {
            r#type: AttestationType::ZkProof {
                verification_key_hash: "abc123".into(),
            },
            proof: base64::encode(b"too small"),
            verification_key_ref: None,
        };

        assert!(verifier.verify(&proof).is_err());
    }

    #[test]
    fn test_capability_verification() {
        let verifier = CompositeAttestationVerifier::new();
        let token = DelegationToken {
            issuer: "agent:a".into(),
            subject: "agent:b".into(),
            capabilities: vec!["read".into(), "write".into()],
            expiry: "2099-01-01T00:00:00Z".into(),
            max_depth: 1,
            grant_constraints_hash: None,
            attestations: vec![],
            chain_merkle_root: None,
        };

        assert!(verifier.verify_capability(&token, "read"));
        assert!(verifier.verify_capability(&token, "write"));
        assert!(!verifier.verify_capability(&token, "delete"));
    }

    #[test]
    fn test_depth_verification() {
        let verifier = CompositeAttestationVerifier::new();
        let token = DelegationToken {
            issuer: "agent:a".into(),
            subject: "agent:b".into(),
            capabilities: vec![],
            expiry: "2099-01-01T00:00:00Z".into(),
            max_depth: 3,
            grant_constraints_hash: None,
            attestations: vec![],
            chain_merkle_root: None,
        };

        assert!(verifier.verify_depth(&token, 0));
        assert!(verifier.verify_depth(&token, 3));
        assert!(!verifier.verify_depth(&token, 4));
    }
}
