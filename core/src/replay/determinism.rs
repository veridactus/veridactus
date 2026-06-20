//! # 引擎确定性验证器（M13）
//!
//! 严格遵循 AI.md §5.3。
//! 验证推理引擎的确定性声明是否与运行环境匹配。

use std::collections::{HashMap, HashSet};

use crate::types::trace::{Trace, EngineDeterminism};

/// 确定性策略标识
const STRATEGIES: &[&str] = &[
    "batch_invariant_kernels",
    "melody_determinism",
    "fixed_seed_only",
    "none",
];

/// 确定性错误
#[derive(Debug)]
pub enum DeterminismError {
    /// 不支持的策略
    UnsupportedStrategy(String),
    /// 版本不匹配
    VersionMismatch { required: String, available: String },
    /// 比特级保证不可用
    BitwiseGuaranteeNotSupported,
    /// 框架不可用
    FrameworkNotAvailable(String),
}

impl std::fmt::Display for DeterminismError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedStrategy(s) => write!(f, "不支持确定性策略: {}", s),
            Self::VersionMismatch { required, available } => {
                write!(f, "版本不匹配: 需要={}, 可用={}", required, available)
            }
            Self::BitwiseGuaranteeNotSupported => write!(f, "当前环境不支持比特级确定性"),
            Self::FrameworkNotAvailable(s) => write!(f, "框架不可用: {}", s),
        }
    }
}

/// 运行时环境信息
#[derive(Debug, Clone)]
pub struct RuntimeEnv {
    /// 可用框架及其版本
    pub frameworks: HashMap<String, String>,
    /// 支持的策略
    pub supported_strategies: HashSet<String>,
    /// 是否支持比特级确定性
    pub supports_bitwise_determinism: bool,
}

impl Default for RuntimeEnv {
    fn default() -> Self {
        let mut frameworks = HashMap::new();
        frameworks.insert("batch_invariant_kernels".to_string(), "1.2".to_string());
        frameworks.insert("melody_determinism".to_string(), "0.9".to_string());
        frameworks.insert("fixed_seed_only".to_string(), "1.0".to_string());
        frameworks.insert("none".to_string(), "1.0".to_string());

        let mut strategies = HashSet::new();
        for s in STRATEGIES {
            strategies.insert(s.to_string());
        }

        Self {
            frameworks,
            supported_strategies: strategies,
            supports_bitwise_determinism: false,
        }
    }
}

/// 引擎确定性验证器（AI.md §5.3）
pub struct DeterminismChecker {
    /// 支持的策略
    supported_strategies: HashSet<String>,
    /// 框架版本要求
    framework_versions: HashMap<String, String>,
}

impl Default for DeterminismChecker {
    fn default() -> Self {
        let mut framework_versions = HashMap::new();
        framework_versions.insert("batch_invariant_kernels".to_string(), "1.0".to_string());
        framework_versions.insert("melody_determinism".to_string(), "0.8".to_string());

        Self {
            supported_strategies: STRATEGIES.iter().map(|s| s.to_string()).collect(),
            framework_versions,
        }
    }
}

impl DeterminismChecker {
    /// 创建新的验证器
    pub fn new() -> Self {
        Self::default()
    }

    /// 验证 Trace 的引擎确定性声明
    ///
    /// # 参数
    /// * `trace` - 待验证的 Trace
    /// * `current_env` - 当前运行时环境
    ///
    /// # 返回
    /// * `Ok(())` - 验证通过
    /// * `Err(DeterminismError)` - 验证失败
    pub fn verify(&self, trace: &Trace, current_env: &RuntimeEnv) -> Result<(), DeterminismError> {
        let Some(det) = &trace.engine_determinism else {
            return Ok(()); // 未声明则跳过
        };

        // 策略匹配
        if !self.supported_strategies.contains(&det.strategy) {
            return Err(DeterminismError::UnsupportedStrategy(det.strategy.clone()));
        }

        // 版本兼容
        let available_version = current_env
            .frameworks
            .get(&det.strategy)
            .ok_or_else(|| DeterminismError::FrameworkNotAvailable(det.strategy.clone()))?;

        let required_version = det
            .framework_version
            .as_ref()
            .ok_or_else(|| DeterminismError::VersionMismatch {
                required: "any".to_string(),
                available: available_version.clone(),
            })?;

        if required_version > available_version {
            return Err(DeterminismError::VersionMismatch {
                required: required_version.clone(),
                available: available_version.clone(),
            });
        }

        // 比特级保证检查
        if det.bitwise_guarantee.unwrap_or(false) && !current_env.supports_bitwise_determinism {
            return Err(DeterminismError::BitwiseGuaranteeNotSupported);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_trace_with_determinism(strategy: &str, version: &str, bitwise: bool) -> Trace {
        let mut t = Trace::new("test/model");
        t.trace_id = Uuid::new_v4();
        t.engine_determinism = Some(EngineDeterminism {
            strategy: strategy.to_string(),
            framework_version: Some(version.to_string()),
            bitwise_guarantee: Some(bitwise),
        });
        t
    }

    #[test]
    fn test_determinism_accepted() {
        let checker = DeterminismChecker::new();
        let env = RuntimeEnv::default();
        let trace = create_trace_with_determinism("fixed_seed_only", "1.0", false);
        assert!(checker.verify(&trace, &env).is_ok());
    }

    #[test]
    fn test_unsupported_strategy() {
        let checker = DeterminismChecker::new();
        let env = RuntimeEnv::default();
        let trace = create_trace_with_determinism("unknown_strat", "1.0", false);
        assert!(checker.verify(&trace, &env).is_err());
    }

    #[test]
    fn test_version_mismatch() {
        let checker = DeterminismChecker::new();
        let env = RuntimeEnv::default();
        let trace = create_trace_with_determinism("batch_invariant_kernels", "9.9", false);
        assert!(checker.verify(&trace, &env).is_err());
    }

    #[test]
    fn test_bitwise_not_supported() {
        let checker = DeterminismChecker::new();
        let env = RuntimeEnv::default(); // supports_bitwise_determinism = false
        let trace = create_trace_with_determinism("fixed_seed_only", "1.0", true);
        assert!(checker.verify(&trace, &env).is_err());
    }

    #[test]
    fn test_no_declaration_skips() {
        let checker = DeterminismChecker::new();
        let env = RuntimeEnv::default();
        let trace = Trace::new("test/model");
        assert!(checker.verify(&trace, &env).is_ok());
    }
}
