//! # Merkle Tree for L2A Sampling Verification (§7.1.4)
//!
//! 基于 IMMACULATE 框架的概率采样验证。
//! 对 Trace 数据进行 Merkle 树承诺，支持随机路径开放和验证。

use sha2::{Digest, Sha256};

/// Merkle 树节点哈希
pub type NodeHash = [u8; 32];

/// Merkle 树
pub struct MerkleTree {
    /// 叶子节点哈希（原始数据的 SHA-256）
    leaves: Vec<NodeHash>,
    /// 树的所有层级 [level][index]
    levels: Vec<Vec<NodeHash>>,
    /// 根哈希
    root: NodeHash,
}

/// 采样路径（Merkle proof）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SamplingPath {
    /// 叶子索引
    pub leaf_index: usize,
    /// 叶子数据哈希
    pub leaf_hash: String,
    /// 证明路径（从叶子到根的兄弟节点哈希）
    pub proof: Vec<ProofStep>,
    /// 根哈希
    pub root_hash: String,
}

/// 证明步骤
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProofStep {
    /// 兄弟节点哈希
    pub sibling_hash: String,
    /// 兄弟在父节点中的位置（left 或 right）
    pub position: String,
}

impl MerkleTree {
    /// 从数据块构建 Merkle 树
    ///
    /// # 参数
    /// * `data_chunks` - 待承诺的数据块
    pub fn from_chunks(data_chunks: &[&[u8]]) -> Self {
        if data_chunks.is_empty() {
            // 空树：使用零哈希
            let zero = [0u8; 32];
            return Self {
                leaves: vec![zero],
                levels: vec![vec![zero]],
                root: zero,
            };
        }

        // 1. 计算叶子哈希
        let mut leaves: Vec<NodeHash> = data_chunks
            .iter()
            .map(|chunk| {
                let mut hasher = Sha256::new();
                hasher.update(b"VERIDACTUS_LEAF:");
                hasher.update(chunk);
                hasher.finalize().into()
            })
            .collect();

        // 确保叶子数为 2 的幂（填充零哈希）
        let target_len = leaves.len().next_power_of_two();
        while leaves.len() < target_len {
            let mut hasher = Sha256::new();
            hasher.update(b"VERIDACTUS_PAD:");
            leaves.push(hasher.finalize().into());
        }

        // 2. 逐层构建
        let mut levels = vec![leaves.clone()];
        let mut current = leaves;

        while current.len() > 1 {
            let mut next_level = Vec::with_capacity(current.len() / 2);
            for pair in current.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(b"VERIDACTUS_NODE:");
                hasher.update(&pair[0]);
                hasher.update(&pair[1]);
                next_level.push(hasher.finalize().into());
            }
            levels.push(next_level.clone());
            current = next_level;
        }

        let root = current[0];

        Self {
            leaves: levels[0].clone(),
            levels,
            root,
        }
    }

    /// 将 Trace JSON 分块构建 Merkle 树
    pub fn from_trace_json(trace_json: &str) -> Self {
        let bytes = trace_json.as_bytes();
        // 按 256 字节分块
        let chunks: Vec<&[u8]> = bytes.chunks(256).collect();
        Self::from_chunks(&chunks)
    }

    /// 获取根哈希（十六进制字符串）
    pub fn root_hex(&self) -> String {
        hex::encode(self.root)
    }

    /// 获取根哈希字节
    pub fn root_bytes(&self) -> NodeHash {
        self.root
    }

    /// 获取叶子数量
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    /// 生成指定叶子的采样路径（Merkle proof）
    pub fn generate_path(&self, leaf_index: usize) -> Option<SamplingPath> {
        if leaf_index >= self.leaves.len() {
            return None;
        }

        let mut proof_steps = Vec::new();
        let mut current_index = leaf_index;

        for level in 0..self.levels.len() - 1 {
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < self.levels[level].len() {
                proof_steps.push(ProofStep {
                    sibling_hash: hex::encode(self.levels[level][sibling_index]),
                    position: if current_index % 2 == 0 {
                        "right".to_string()
                    } else {
                        "left".to_string()
                    },
                });
            }

            current_index /= 2;
        }

        Some(SamplingPath {
            leaf_index,
            leaf_hash: hex::encode(self.leaves[leaf_index]),
            proof: proof_steps,
            root_hash: self.root_hex(),
        })
    }

    /// 验证采样路径
    pub fn verify_path(path: &SamplingPath) -> bool {
        let leaf_hash = match hex::decode(&path.leaf_hash) {
            Ok(h) => {
                let mut arr = [0u8; 32];
                if h.len() == 32 {
                    arr.copy_from_slice(&h);
                    arr
                } else {
                    return false;
                }
            }
            Err(_) => return false,
        };

        let mut current_hash = leaf_hash;
        for step in &path.proof {
            let sibling = match hex::decode(&step.sibling_hash) {
                Ok(h) => {
                    let mut arr = [0u8; 32];
                    if h.len() == 32 {
                        arr.copy_from_slice(&h);
                        arr
                    } else {
                        return false;
                    }
                }
                Err(_) => return false,
            };

            let mut hasher = Sha256::new();
            hasher.update(b"VERIDACTUS_NODE:");
            match step.position.as_str() {
                "left" => {
                    hasher.update(&sibling);
                    hasher.update(&current_hash);
                }
                "right" => {
                    hasher.update(&current_hash);
                    hasher.update(&sibling);
                }
                _ => return false,
            }
            current_hash = hasher.finalize().into();
        }

        hex::encode(current_hash) == path.root_hash
    }

    /// 随机采样验证：采样指定比例
    pub fn sample_verify(&self, sample_rate: f64) -> (bool, Vec<SamplingPath>) {
        let total = self.leaves.len();
        let sample_count = ((total as f64) * sample_rate).ceil() as usize;
        let sample_count = sample_count.max(1).min(total);

        let mut paths = Vec::new();
        let mut verified = 0;
        let mut total_sampled = 0;

        // 均匀间隔采样
        let step = total / sample_count;
        for i in (0..total).step_by(step.max(1)) {
            if total_sampled >= sample_count {
                break;
            }
            if let Some(path) = self.generate_path(i) {
                total_sampled += 1;
                if Self::verify_path(&path) {
                    verified += 1;
                }
                paths.push(path);
            }
        }

        (verified == total_sampled, paths)
    }
}

/// L2A 采样验证结果（§7.1.4，嵌入 Trace）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct L2ASamplingResult {
    /// Merkle 根
    pub merkle_root: String,
    /// 总叶子数
    pub total_leaves: usize,
    /// 采样率
    pub sample_rate: f64,
    /// 采样路径数
    pub sampled_count: usize,
    /// 全部路径验证通过
    pub all_verified: bool,
    /// 采样路径列表
    pub sampling_paths: Vec<SamplingPath>,
    /// 生成时间
    pub generated_at: String,
}

/// 生成 L2A 证明条目
pub fn generate_l2a_proof(
    trace_json: &str,
    sample_rate: f64,
) -> crate::types::proof::ProofChainEntry {
    let tree = MerkleTree::from_trace_json(trace_json);
    let (verified, paths) = tree.sample_verify(sample_rate);

    let result = L2ASamplingResult {
        merkle_root: tree.root_hex(),
        total_leaves: tree.leaf_count(),
        sample_rate,
        sampled_count: paths.len(),
        all_verified: verified,
        sampling_paths: paths,
        generated_at: chrono::Utc::now().to_rfc3339(),
    };

    crate::types::proof::ProofChainEntry {
        level: crate::types::proof::ProofLevel::L2A,
        r#type: crate::types::proof::ProofType::SamplingVerification,
        signature: None,
        signature_pq: None,
        attestation_quote: None,
        model_fingerprint: None,
        platform: None,
        mrenclave: None,
        merkle_root: Some(result.merkle_root.clone()),
        sampling_paths: Some(
            result
                .sampling_paths
                .iter()
                .map(|p| serde_json::to_string(p).unwrap_or_default())
                .collect(),
        ),
        zk_proof: None,
        verification_key_hash: None,
        proof_aggregation_root: None,
        canonicalization_method: "rfc8785".to_string(),
        canonical_json: None,
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_basic() {
        let chunks: Vec<&[u8]> = vec![b"hello", b"world", b"veridactus"];
        let tree = MerkleTree::from_chunks(&chunks);
        assert!(!tree.root_hex().is_empty());
        assert_eq!(tree.leaf_count(), 4); // padded to power of 2
    }

    #[test]
    fn test_generate_and_verify_path() {
        let data = "This is a test trace for VERIDACTUS L2A verification".as_bytes();
        let chunks: Vec<&[u8]> = data.chunks(8).collect();
        let tree = MerkleTree::from_chunks(&chunks);

        for i in 0..tree.leaf_count() {
            let path = tree.generate_path(i).unwrap();
            assert!(
                MerkleTree::verify_path(&path),
                "Path {} verification failed",
                i
            );
        }
    }

    #[test]
    fn test_sample_verify() {
        let data = std::iter::repeat("VERIDACTUS_TRACE_DATA_CHUNK_")
            .take(16)
            .enumerate()
            .map(|(i, s)| format!("{}{}", s, i))
            .collect::<Vec<_>>();
        let chunks: Vec<&[u8]> = data.iter().map(|s| s.as_bytes()).collect();
        let tree = MerkleTree::from_chunks(&chunks);

        let (verified, paths) = tree.sample_verify(0.5);
        assert!(verified, "Sample verification should pass");
        assert!(!paths.is_empty(), "Should have sampled paths");
    }

    #[test]
    fn test_invalid_path_rejected() {
        let chunks: Vec<&[u8]> = vec![b"data1", b"data2"];
        let tree = MerkleTree::from_chunks(&chunks);
        let mut path = tree.generate_path(0).unwrap();
        // Tamper with the leaf hash
        path.leaf_hash =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        assert!(
            !MerkleTree::verify_path(&path),
            "Tampered path should be rejected"
        );
    }

    #[test]
    fn test_large_trace() {
        // Simulate a large trace (100 chunks)
        let large_data = "X".repeat(25600);
        let tree = MerkleTree::from_trace_json(&large_data);
        assert!(tree.leaf_count() >= 100);
        let (verified, _) = tree.sample_verify(0.1);
        assert!(verified);
    }
}
