//! # G3 语义守卫
//!
//! 协议 §5.6 G3: 基于嵌入向量的语义一致性守卫。
//! 通过余弦相似度检测 LLM 输出是否偏离预期的语义空间。

/// 语义守卫配置
#[derive(Debug, Clone)]
pub struct SemanticGuardConfig {
    /// 低风险阈值（cosine similarity 低于此值触发警告）
    pub warning_threshold: f64,
    /// 高风险阈值（cosine similarity 低于此值触发阻断）
    pub block_threshold: f64,
    /// 是否启用语义守卫
    pub enabled: bool,
}

impl Default for SemanticGuardConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 0.7,
            block_threshold: 0.4,
            enabled: true,
        }
    }
}

/// 语义检测结果
#[derive(Debug, Clone)]
pub struct SemanticGuardResult {
    /// 余弦相似度 [-1.0, 1.0]
    pub similarity: f64,
    /// 检测等级
    pub level: SemanticRiskLevel,
    /// 是否应阻断
    pub should_block: bool,
    /// 详细描述
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SemanticRiskLevel {
    /// 安全：相似度高
    Safe,
    /// 警告：相似度偏低
    Warning,
    /// 危险：相似度很低，可能语义漂移
    Dangerous,
}

/// G3 语义守卫
pub struct SemanticGuard {
    config: SemanticGuardConfig,
}

impl SemanticGuard {
    pub fn new(config: SemanticGuardConfig) -> Self {
        Self { config }
    }

    /// 计算两个嵌入向量的余弦相似度
    pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..a.len() {
            dot_product += a[i] * b[i];
            norm_a += a[i] * a[i];
            norm_b += b[i] * b[i];
        }

        let denominator = (norm_a * norm_b).sqrt();
        if denominator < 1e-10 {
            return 0.0;
        }

        dot_product / denominator
    }

    /// 评估语义相似度
    pub fn evaluate(
        &self,
        reference_embedding: &[f64],
        current_embedding: &[f64],
    ) -> SemanticGuardResult {
        if !self.config.enabled {
            return SemanticGuardResult {
                similarity: 1.0,
                level: SemanticRiskLevel::Safe,
                should_block: false,
                description: "Semantic guard disabled".to_string(),
            };
        }

        let similarity = Self::cosine_similarity(reference_embedding, current_embedding);

        let (level, should_block, description) = if similarity >= self.config.warning_threshold {
            (SemanticRiskLevel::Safe, false, 
             format!("Semantic consistency good (similarity={:.4})", similarity))
        } else if similarity >= self.config.block_threshold {
            (SemanticRiskLevel::Warning, false,
             format!("Semantic deviation warning (similarity={:.4})", similarity))
        } else {
            (SemanticRiskLevel::Dangerous, true,
             format!("Critical semantic drift, possible model substitution or poisoning (similarity={:.4})", similarity))
        };

        SemanticGuardResult {
            similarity,
            level,
            should_block,
            description,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = SemanticGuard::cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = SemanticGuard::cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_semantic_guard_safe() {
        let guard = SemanticGuard::new(SemanticGuardConfig::default());
        let ref_emb = vec![1.0, 2.0, 3.0];
        let cur_emb = vec![1.1, 1.9, 3.0];
        let result = guard.evaluate(&ref_emb, &cur_emb);
        assert_eq!(result.level, SemanticRiskLevel::Safe);
        assert!(!result.should_block);
    }

    #[test]
    fn test_semantic_guard_warning() {
        let guard = SemanticGuard::new(SemanticGuardConfig {
            warning_threshold: 0.98,
            block_threshold: 0.4,
            enabled: true,
        });
        let ref_emb = vec![1.0, 2.0, 3.0];
        // cos ≈ 0.845, between 0.4 and 0.98 → Warning
        let cur_emb = vec![1.0, 0.0, 3.0];
        let result = guard.evaluate(&ref_emb, &cur_emb);
        assert_eq!(result.level, SemanticRiskLevel::Warning);
    }

    #[test]
    fn test_semantic_guard_dangerous() {
        let guard = SemanticGuard::new(SemanticGuardConfig::default());
        let ref_emb = vec![1.0, 0.0, 0.0];
        let cur_emb = vec![-1.0, 0.0, 0.0];
        let result = guard.evaluate(&ref_emb, &cur_emb);
        assert_eq!(result.level, SemanticRiskLevel::Dangerous);
        assert!(result.should_block);
    }
}
