//! # 软件级 L1 认证 (Ed25519 替代 TEE)
//!
//! 协议 §7.1.3 规定 L1 为 TEE 硬件认证 (Intel TDX/AMD SEV/NVIDIA CC)。
//! 在无 TEE 硬件时，使用 Ed25519 签名提供**等价的 API 接口**。
//!
//! 工作原理:
//! - 生成 Ed25519 密钥对 (签名密钥模拟 TEE 的飞地身份)
//! - 对 Trace 的核心字段签名 (模拟 TEE 的 attestation quote)
//! - 验证方用公钥验证签名 (模拟 TEE 的 PKI 验证)
//!
//! 限制: 这是软件级证明，安全性依赖于签名密钥的保护，
//!       而非硬件飞地的物理隔离。适合开发/测试/演示环境。
//!       生产环境应替换为真实的 TEE 认证。

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use sha2::{Digest, Sha256};

use crate::types::proof::{ProofChainEntry, ProofLevel, ProofType};
use crate::types::trace::Trace;

/// 软件 TEE 认证器
///
/// 模拟 TEE attestation 的软件实现。
/// 使用 Ed25519 签名替代硬件 quote。
pub struct SoftwareAttestation {
    /// 签名密钥 (模拟 TEE 的飞地私钥)
    signing_key: SigningKey,
    /// 平台标识
    platform: String,
    /// MRENCLAVE 模拟 (飞地测量哈希)
    mrenclave: String,
}

impl SoftwareAttestation {
    /// 创建新的认证器 (自动生成随机密钥)
    pub fn new() -> Self {
        // 使用 OsRng 生成随机密钥（安全随机，每次启动不同）
        use rand::rngs::OsRng;
        use rand::RngCore;
        let mut seed_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut seed_bytes);
        let signing_key = SigningKey::from_bytes(&seed_bytes);
        let verifying_key = signing_key.verifying_key();

        // MRENCLAVE = SHA-256(公钥) 模拟飞地身份
        let mrenclave = format!(
            "sw:{}",
            hex::encode(Sha256::digest(verifying_key.as_bytes()))
        );

        Self {
            signing_key,
            platform: "veridactus-software-tee-v1".to_string(),
            mrenclave,
        }
    }

    /// 使用指定密钥创建 (用于可复现测试)
    pub fn with_key(signing_key: SigningKey, platform: &str) -> Self {
        let verifying_key = signing_key.verifying_key();
        let mrenclave = format!(
            "sw:{}",
            hex::encode(Sha256::digest(verifying_key.as_bytes()))
        );
        Self {
            signing_key,
            platform: platform.to_string(),
            mrenclave,
        }
    }

    /// 生成 L1 级证明
    ///
    /// 对 Trace 的核心字段签名，生成软件 attestation。
    /// 对应协议 §7.1.3 L1: Hardware Attestation (TEE) 的软件等价实现。
    ///
    /// # 参数
    /// * `trace` - 待认证的 Trace
    ///
    /// # 返回
    /// L1 ProofChainEntry
    pub fn attest(&self, trace: &Trace) -> ProofChainEntry {
        // 构建认证数据 (模拟 TEE quote 的内容)
        let quote_data = serde_json::json!({
            "trace_id": trace.trace_id,
            "model": trace.model,
            "created_at": trace.created_at,
            "model_fingerprint": trace.supply_chain
                .as_ref()
                .and_then(|s| s.model.as_ref())
                .and_then(|m| m.get("hash"))
                .and_then(|h| h.as_str()),
            "platform": self.platform,
            "mrenclave": self.mrenclave,
        });

        let quote_str = serde_json::to_string(&quote_data).unwrap_or_default();
        let signature: Signature = self.signing_key.sign(quote_str.as_bytes());

        ProofChainEntry {
            level: ProofLevel::L1,
            r#type: ProofType::TeeAttestation,
            signature: Some(hex::encode(signature.to_bytes())),
            signature_pq: None,
            attestation_quote: Some(base64_encode(quote_str.as_bytes())),
            model_fingerprint: trace
                .supply_chain
                .as_ref()
                .and_then(|s| s.model.as_ref())
                .and_then(|m| m.get("hash"))
                .and_then(|h| h.as_str())
                .map(|s| s.to_string()),
            platform: Some(self.platform.clone()),
            mrenclave: Some(self.mrenclave.clone()),
            merkle_root: None,
            sampling_paths: None,
            zk_proof: None,
            verification_key_hash: None,
            proof_aggregation_root: None,
            canonicalization_method: "rfc8785".to_string(),
            canonical_json: None,
        }
    }

    /// 验证 L1 证明
    ///
    /// # 参数
    /// * `proof` - L1 证明条目
    /// * `trace` - 关联 Trace
    ///
    /// # 返回
    /// `Ok(())` 表示验证通过
    pub fn verify(&self, proof: &ProofChainEntry, trace: &Trace) -> Result<(), String> {
        let signature_hex = proof
            .signature
            .as_ref()
            .ok_or_else(|| "L1 签名缺失".to_string())?;

        let signature_bytes =
            hex::decode(signature_hex).map_err(|e| format!("签名 hex 解码失败: {}", e))?;

        let signature =
            Signature::from_slice(&signature_bytes).map_err(|e| format!("签名解析失败: {}", e))?;

        // 重建认证数据
        let quote_data = serde_json::json!({
            "trace_id": trace.trace_id,
            "model": trace.model,
            "created_at": trace.created_at,
            "model_fingerprint": proof.model_fingerprint,
            "platform": proof.platform,
            "mrenclave": proof.mrenclave,
        });

        let quote_str = serde_json::to_string(&quote_data).unwrap_or_default();
        let verifying_key = self.signing_key.verifying_key();

        verifying_key
            .verify(quote_str.as_bytes(), &signature)
            .map_err(|e| format!("L1 签名验证失败: {}", e))?;

        Ok(())
    }

    /// 获取公钥 (十六进制)
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().as_bytes())
    }

    /// 获取平台标识
    pub fn platform(&self) -> &str {
        &self.platform
    }

    /// 获取 MRENCLAVE
    pub fn mrenclave(&self) -> &str {
        &self.mrenclave
    }
}

fn base64_encode(data: &[u8]) -> String {
    // 简单 base64 编码 (无额外依赖)
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize]);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize]);
        result.push(if chunk.len() > 1 {
            CHARS[((triple >> 6) & 0x3F) as usize]
        } else {
            b'='
        });
        result.push(if chunk.len() > 2 {
            CHARS[(triple & 0x3F) as usize]
        } else {
            b'='
        });
    }
    String::from_utf8(result).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::proof::Proofs;
    use uuid::Uuid;

    fn create_test_trace() -> Trace {
        Trace {
            trace_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            parent_id: None,
            session_id: None,
            tenant_id: Some("test".to_string()),
            execution_state: None,
            model: "openai/gpt-4o".to_string(),
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
            created_at: "2026-01-01T00:00:00Z".to_string(),
            ttl_expire_at: None,
            extensions: None,
        }
    }

    #[test]
    fn test_software_attestation_generation() {
        let attestor = SoftwareAttestation::new();
        let trace = create_test_trace();
        let proof = attestor.attest(&trace);

        assert_eq!(proof.level, ProofLevel::L1);
        assert_eq!(proof.r#type, ProofType::TeeAttestation);
        assert!(proof.signature.is_some());
        assert_eq!(proof.signature.as_ref().unwrap().len(), 128); // Ed25519 sig is 64 bytes = 128 hex chars
        assert!(proof.attestation_quote.is_some());
        assert_eq!(
            proof.platform.as_deref(),
            Some("veridactus-software-tee-v1")
        );
    }

    #[test]
    fn test_software_attestation_verification() {
        let attestor = SoftwareAttestation::new();
        let trace = create_test_trace();
        let proof = attestor.attest(&trace);

        assert!(attestor.verify(&proof, &trace).is_ok());
    }

    #[test]
    fn test_software_attestation_tamper_detection() {
        let attestor = SoftwareAttestation::new();
        let mut trace = create_test_trace();
        let proof = attestor.attest(&trace);

        // 篡改模型名称
        trace.model = "tampered-model".to_string();
        assert!(attestor.verify(&proof, &trace).is_err());
    }

    #[test]
    fn test_mrenclave_consistency() {
        let attestor = SoftwareAttestation::new();
        let mrenclave = attestor.mrenclave();
        assert!(mrenclave.starts_with("sw:"));
        assert!(
            mrenclave.len() > 10,
            "mrenclave 应有合理长度: {}",
            mrenclave.len()
        );
    }
}
