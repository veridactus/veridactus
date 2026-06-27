//! # Proof 证明链数据类型
//!
//! 严格遵循 VERIDACTUS v0.2.1 §7.0 Cryptographic Audit, Attestation & Provenance。
//! 实现分层证明架构（L0-L2B）。

use serde::{Deserialize, Serialize};

/// 证明级别（§7.1.1）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProofLevel {
    /// L0: 存储完整性（哈希链）
    #[serde(rename = "L0")]
    L0,
    /// L1: 硬件认证（TEE）
    #[serde(rename = "L1")]
    L1,
    /// L2A: 概率采样验证
    #[serde(rename = "L2A")]
    L2A,
    /// L2B: 实用零知识证明
    #[serde(rename = "L2B")]
    L2B,
}

/// 证明类型（§7.1.1）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProofType {
    /// 哈希链证明
    #[serde(rename = "hash_chain")]
    HashChain,
    /// TEE 硬件认证
    #[serde(rename = "tee_attestation")]
    TeeAttestation,
    /// 采样验证
    #[serde(rename = "sampling_verification")]
    SamplingVerification,
    /// 零知识证明
    #[serde(rename = "zk_stark")]
    ZkStark,
}

/// 证明链条目（§7.1.1）
///
/// 每个条目对应一个证明层级，支持 L0 到 L2B 的完整证明栈。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofChainEntry {
    /// 证明级别
    pub level: ProofLevel,
    /// 证明类型
    pub r#type: ProofType,
    /// 签名（L0 必填，64 字符十六进制 SHA-256 摘要）
    pub signature: Option<String>,
    /// 可选后量子签名
    pub signature_pq: Option<String>,
    /// TEE 认证报告（Base64 编码，L1 专用）
    pub attestation_quote: Option<String>,
    /// 模型指纹（SHA3-256，L1/L2）
    pub model_fingerprint: Option<String>,
    /// TEE 平台标识符
    pub platform: Option<String>,
    /// 飞地测量哈希
    pub mrenclave: Option<String>,
    /// Merkle 根（L2A 专用）
    pub merkle_root: Option<String>,
    /// 采样路径（L2A）
    pub sampling_paths: Option<Vec<String>>,
    /// ZK 证明数据（Base64 编码，L2B 专用）
    pub zk_proof: Option<String>,
    /// 验证密钥哈希（L2B 专用）
    pub verification_key_hash: Option<String>,
    /// 证明聚合根
    pub proof_aggregation_root: Option<String>,
    /// 规范化方法
    #[serde(default = "default_canonicalization")]
    pub canonicalization_method: String,
    /// 签名时的规范 JSON 字符串（用于跨存储验证，避免 JSONB 反序列化差异）
    #[serde(default)]
    pub canonical_json: Option<String>,
}

fn default_canonicalization() -> String {
    "rfc8785".to_string()
}

/// Proofs 证明对象（§3.0 proofs）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proofs {
    /// 证明链数组，按层级排序
    pub proof_chain: Vec<ProofChainEntry>,
    /// 所有证明条目的 Merkle 根
    pub aggregated_root: Option<String>,
}

impl Default for Proofs {
    fn default() -> Self {
        Self {
            proof_chain: Vec::new(),
            aggregated_root: None,
        }
    }
}

impl Proofs {
    /// 创建一个新的空 Proofs
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加证明链条目
    pub fn add_entry(&mut self, entry: ProofChainEntry) {
        self.proof_chain.push(entry);
    }

    /// 获取指定级别的证明
    pub fn get_proof(&self, level: &ProofLevel) -> Option<&ProofChainEntry> {
        self.proof_chain.iter().find(|e| e.level == *level)
    }

    /// 检查是否包含指定级别的证明
    pub fn has_level(&self, level: &ProofLevel) -> bool {
        self.proof_chain.iter().any(|e| e.level == *level)
    }

    /// 获取最高证明级别（返回克隆值以避免生命周期问题）
    pub fn max_level(&self) -> Option<ProofLevel> {
        const ORDER: [ProofLevel; 4] = [
            ProofLevel::L0,
            ProofLevel::L1,
            ProofLevel::L2A,
            ProofLevel::L2B,
        ];
        for level in ORDER.iter().rev() {
            if self.has_level(level) {
                return Some(level.clone());
            }
        }
        None
    }
}
