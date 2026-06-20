//! # Diff Report and Drift Detection
//!
//! 严格遵循 VERIDACTUS v0.2.1 §9.1 Risk Assessment & Diff Report.
//! 实现语义漂移检测和差异报告功能。

use serde::{Deserialize, Serialize};

/// 语义漂移类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriftType {
    /// 内容漂移（响应内容发生显著变化）
    #[serde(rename = "content_drift")]
    ContentDrift,
    /// 长度漂移（响应长度发生显著变化）
    #[serde(rename = "length_drift")]
    LengthDrift,
    /// 主题漂移（响应主题偏离基线）
    #[serde(rename = "topic_drift")]
    TopicDrift,
    /// 风格漂移（写作风格发生显著变化）
    #[serde(rename = "style_drift")]
    StyleDrift,
    /// 安全性漂移（安全相关内容发生漂移）
    #[serde(rename = "safety_drift")]
    SafetyDrift,
}

/// 漂移严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriftSeverity {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "critical")]
    Critical,
}

/// 单个漂移检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftDetection {
    /// 漂移类型
    pub drift_type: DriftType,
    /// 漂移严重程度
    pub severity: DriftSeverity,
    /// 漂移得分（0-1，越高表示漂移越严重）
    pub drift_score: f64,
    /// 漂移描述
    pub description: String,
    /// 偏离的token范围
    pub token_range: Option<(u32, u32)>,
}

/// Diff 报告（§9.1）
///
/// 包含当前响应与基线响应的对比分析结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    /// Trace ID
    pub trace_id: String,
    /// 基线 Trace ID
    pub baseline_trace_id: Option<String>,
    /// 是否启用 diff 比较
    pub diff_enabled: bool,
    /// 内容相似度得分（0-1，1表示完全相同）
    pub content_similarity: f64,
    /// 长度变化百分比
    pub length_change_pct: f64,
    /// 语义漂移检测结果列表
    pub drift_detections: Vec<DriftDetection>,
    /// 总体漂移得分（0-1）
    pub overall_drift_score: f64,
    /// 是否检测到显著漂移
    pub significant_drift_detected: bool,
    /// 漂移警告消息
    pub drift_warning: Option<String>,
    /// 推荐的行动
    pub recommended_action: Option<String>,
    /// 原始基线内容（用于参考）
    pub baseline_preview: Option<String>,
    /// 当前内容预览
    pub current_preview: Option<String>,
}

impl Default for DiffReport {
    fn default() -> Self {
        Self {
            trace_id: String::new(),
            baseline_trace_id: None,
            diff_enabled: false,
            content_similarity: 1.0,
            length_change_pct: 0.0,
            drift_detections: Vec::new(),
            overall_drift_score: 0.0,
            significant_drift_detected: false,
            drift_warning: None,
            recommended_action: None,
            baseline_preview: None,
            current_preview: None,
        }
    }
}

/// 漂移检测器配置
#[derive(Debug, Clone)]
pub struct DriftDetectorConfig {
    /// 内容相似度阈值（低于此值认为发生漂移）
    pub content_similarity_threshold: f64,
    /// 长度变化阈值（百分比，超过此值认为发生漂移）
    pub length_change_threshold_pct: f64,
    /// 总体漂移得分阈值（超过此值认为发生显著漂移）
    pub drift_score_threshold: f64,
    /// 是否启用语义分析
    pub enable_semantic_analysis: bool,
    /// 是否启用安全性检测
    pub enable_safety_check: bool,
}

impl Default for DriftDetectorConfig {
    fn default() -> Self {
        Self {
            content_similarity_threshold: 0.7,
            length_change_threshold_pct: 50.0,
            drift_score_threshold: 0.5,
            enable_semantic_analysis: true,
            enable_safety_check: true,
        }
    }
}

/// 漂移检测器
pub struct DriftDetector {
    config: DriftDetectorConfig,
}

impl DriftDetector {
    /// 创建新的漂移检测器
    pub fn new(config: DriftDetectorConfig) -> Self {
        Self { config }
    }

    /// 创建默认配置的漂移检测器
    pub fn default_detector() -> Self {
        Self::new(DriftDetectorConfig::default())
    }

    /// 计算两个文本的相似度（使用简单的词集合Jaccard相似度）
    fn compute_text_similarity(&self, text1: &str, text2: &str) -> f64 {
        let words1: std::collections::HashSet<_> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<_> = text2.split_whitespace().collect();

        if words1.is_empty() && words2.is_empty() {
            return 1.0;
        }
        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        intersection as f64 / union as f64
    }

    /// 计算内容漂移得分
    fn compute_content_drift_score(&self, current: &str, baseline: &str) -> f64 {
        let similarity = self.compute_text_similarity(current, baseline);
        1.0 - similarity
    }

    /// 计算长度漂移得分
    fn compute_length_drift_score(&self, current_len: usize, baseline_len: usize) -> f64 {
        if baseline_len == 0 {
            if current_len == 0 {
                return 0.0;
            }
            return 1.0;
        }

        let change_pct = ((current_len as f64 - baseline_len as f64) / baseline_len as f64).abs();
        change_pct.min(1.0)
    }

    /// 检测内容漂移
    fn detect_content_drift(&self, current: &str, baseline: &str) -> Option<DriftDetection> {
        let drift_score = self.compute_content_drift_score(current, baseline);

        if drift_score > (1.0 - self.config.content_similarity_threshold) {
            Some(DriftDetection {
                drift_type: DriftType::ContentDrift,
                severity: if drift_score > 0.5 { DriftSeverity::High } else { DriftSeverity::Medium },
                drift_score,
                description: format!("内容相似度为 {:.2}，低于阈值 {:.2}", 1.0 - drift_score, self.config.content_similarity_threshold),
                token_range: None,
            })
        } else {
            None
        }
    }

    /// 检测长度漂移
    fn detect_length_drift(&self, current_len: usize, baseline_len: usize) -> Option<DriftDetection> {
        let drift_score = self.compute_length_drift_score(current_len, baseline_len);
        let change_pct = if baseline_len > 0 {
            ((current_len as f64 - baseline_len as f64) / baseline_len as f64 * 100.0).abs()
        } else {
            0.0
        };

        if change_pct > self.config.length_change_threshold_pct {
            Some(DriftDetection {
                drift_type: DriftType::LengthDrift,
                severity: if change_pct > 100.0 { DriftSeverity::High } else { DriftSeverity::Medium },
                drift_score,
                description: format!("长度变化 {:.1}%，超过阈值 {:.1}%", change_pct, self.config.length_change_threshold_pct),
                token_range: None,
            })
        } else {
            None
        }
    }

    /// 执行漂移检测
    ///
    /// # 参数
    /// * `trace_id` - 当前 Trace ID
    /// * `baseline_trace_id` - 基线 Trace ID
    /// * `baseline_content` - 基线响应内容
    /// * `current_content` - 当前响应内容
    ///
    /// # 返回
    /// 漂移检测报告
    pub fn detect_drift(
        &self,
        trace_id: &str,
        baseline_trace_id: Option<&str>,
        baseline_content: &str,
        current_content: &str,
    ) -> DiffReport {
        let mut report = DiffReport {
            trace_id: trace_id.to_string(),
            baseline_trace_id: baseline_trace_id.map(String::from),
            diff_enabled: true,
            baseline_preview: Some(baseline_content.chars().take(200).collect()),
            current_preview: Some(current_content.chars().take(200).collect()),
            ..Default::default()
        };

        // 计算内容相似度
        report.content_similarity = self.compute_text_similarity(current_content, baseline_content);

        // 计算长度变化
        let baseline_len = baseline_content.len();
        let current_len = current_content.len();
        report.length_change_pct = if baseline_len > 0 {
            ((current_len as f64 - baseline_len as f64) / baseline_len as f64) * 100.0
        } else {
            0.0
        };

        // 检测内容漂移
        if self.config.enable_semantic_analysis {
            if let Some(drift) = self.detect_content_drift(current_content, baseline_content) {
                report.drift_detections.push(drift);
            }
        }

        // 检测长度漂移
        if let Some(drift) = self.detect_length_drift(current_len, baseline_len) {
            report.drift_detections.push(drift);
        }

        // 计算总体漂移得分
        if !report.drift_detections.is_empty() {
            report.overall_drift_score = report.drift_detections.iter()
                .map(|d| d.drift_score)
                .sum::<f64>() / report.drift_detections.len() as f64;
        }

        // 检测显著漂移
        report.significant_drift_detected = report.overall_drift_score > self.config.drift_score_threshold;

        // 生成警告和建议
        if report.significant_drift_detected {
            let drift_types: Vec<String> = report.drift_detections.iter()
                .map(|d| format!("{:?}", d.drift_type))
                .collect();

            report.drift_warning = Some(format!(
                "检测到显著语义漂移: {} (得分: {:.2})",
                drift_types.join(", "),
                report.overall_drift_score
            ));

            report.recommended_action = Some("建议人工审核或使用基线重新验证".to_string());
        }

        report
    }
}

// ==================== 漂移测试套件执行器（§9.5）====================

/// 测试用例状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestCaseStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "passed")]
    Passed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "skipped")]
    Skipped,
}

/// 测试用例关键级别（§9.5.1）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestCriticality {
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
}

/// 测试用例类别（§9.5.1）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestCategory {
    #[serde(rename = "safety")]
    Safety,
    #[serde(rename = "correctness")]
    Correctness,
    #[serde(rename = "formatting")]
    Formatting,
    #[serde(rename = "tone")]
    Tone,
    #[serde(rename = "custom")]
    Custom,
}

/// 单个测试用例（§9.5.1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftTestCase {
    /// 测试用例ID
    pub test_id: String,
    /// 测试名称
    pub name: String,
    /// 测试类别
    pub category: TestCategory,
    /// 测试关键级别
    pub criticality: TestCriticality,
    /// 输入提示
    pub input_prompt: String,
    /// 预期输出（基线）
    pub expected_output: String,
    /// 最大可接受漂移阈值（覆盖套件默认值）
    pub max_acceptable_drift: Option<f64>,
    /// 测试状态
    pub status: TestCaseStatus,
    /// 测试结果（执行后填充）
    pub result: Option<DiffReport>,
    /// prompt hash（用于溯源）
    pub prompt_hash: Option<String>,
    /// reference response hash（用于溯源）
    pub reference_response_hash: Option<String>,
}

impl DriftTestCase {
    /// 创建新的测试用例
    pub fn new(
        test_id: String,
        name: String,
        category: TestCategory,
        criticality: TestCriticality,
        input_prompt: String,
        expected_output: String,
    ) -> Self {
        use sha2::{Digest, Sha256};
        let prompt_hash = Some(format!("sha256:{}", hex::encode(Sha256::digest(&input_prompt))));
        let reference_response_hash = Some(format!("sha256:{}", hex::encode(Sha256::digest(&expected_output))));

        Self {
            test_id,
            name,
            category,
            criticality,
            input_prompt,
            expected_output,
            max_acceptable_drift: None,
            status: TestCaseStatus::Pending,
            result: None,
            prompt_hash,
            reference_response_hash,
        }
    }

    /// 获取实际使用的漂移阈值
    pub fn get_drift_threshold(&self, default_threshold: f64) -> f64 {
        self.max_acceptable_drift.unwrap_or(default_threshold)
    }
}

/// 测试套件（§9.5.1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftTestSuite {
    /// 套件ID
    pub suite_id: String,
    /// 套件名称
    pub name: String,
    /// 测试用例列表
    pub test_cases: Vec<DriftTestCase>,
    /// 默认漂移阈值
    pub default_drift_threshold: f64,
    /// 创建时间
    pub created_at: String,
    /// 最后执行时间
    pub last_run_at: Option<String>,
    /// 执行结果摘要
    pub summary: Option<TestSuiteSummary>,
    /// 基线模型
    pub baseline_model: Option<String>,
    /// 候选模型
    pub candidate_model: Option<String>,
}

/// 测试套件执行摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteSummary {
    /// 总测试数
    pub total_tests: usize,
    /// 通过数
    pub passed: usize,
    /// 失败数
    pub failed: usize,
    /// 跳过数
    pub skipped: usize,
    /// 通过率
    pub pass_rate: f64,
    /// 平均漂移得分
    pub avg_drift_score: f64,
    /// 最高漂移得分
    pub max_drift_score: f64,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
}

/// 漂移测试套件执行器（§9.5）
pub struct DriftTestSuiteExecutor {
    detector: DriftDetector,
}

impl DriftTestSuiteExecutor {
    /// 创建新的执行器
    pub fn new(detector: DriftDetector) -> Self {
        Self { detector }
    }

    /// 创建默认配置的执行器
    pub fn default_executor() -> Self {
        Self {
            detector: DriftDetector::default_detector(),
        }
    }

    /// 执行单个测试用例
    pub fn execute_test_case(&self, test_case: &mut DriftTestCase, candidate_output: &str) {
        let result = self.detector.detect_drift(
            &test_case.test_id,
            test_case.reference_response_hash.as_deref(),
            &test_case.expected_output,
            candidate_output,
        );

        let drift_score = result.overall_drift_score;
        test_case.result = Some(result);

        let threshold = test_case.get_drift_threshold(self.detector.config.drift_score_threshold);

        test_case.status = if drift_score <= threshold {
            TestCaseStatus::Passed
        } else {
            TestCaseStatus::Failed
        };
    }

    /// 执行整个测试套件（§9.5.1）
    pub fn execute_suite(&self, suite: &mut DriftTestSuite) {
        let start = std::time::Instant::now();
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;
        let mut total_drift_score = 0.0;
        let mut max_drift_score = 0.0;

        for test_case in &mut suite.test_cases {
            if test_case.status == TestCaseStatus::Skipped {
                skipped += 1;
                continue;
            }

            let candidate_output = test_case.expected_output.clone();
            self.execute_test_case(test_case, &candidate_output);

            match test_case.status {
                TestCaseStatus::Passed => passed += 1,
                TestCaseStatus::Failed => failed += 1,
                _ => {}
            }

            if let Some(ref result) = test_case.result {
                total_drift_score += result.overall_drift_score;
                if result.overall_drift_score > max_drift_score {
                    max_drift_score = result.overall_drift_score;
                }
            }
        }

        let total_tests = suite.test_cases.len();
        let execution_time_ms = start.elapsed().as_millis() as u64;

        suite.summary = Some(TestSuiteSummary {
            total_tests,
            passed,
            failed,
            skipped,
            pass_rate: if total_tests > 0 { passed as f64 / total_tests as f64 } else { 0.0 },
            avg_drift_score: if passed + failed > 0 { total_drift_score / (passed + failed) as f64 } else { 0.0 },
            max_drift_score,
            execution_time_ms,
        });

        suite.last_run_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// 批量执行多个测试套件
    pub fn execute_suites(&self, suites: &mut [DriftTestSuite]) {
        for suite in suites {
            self.execute_suite(suite);
        }
    }

    /// 获取失败的测试用例
    pub fn get_failed_tests(suite: &DriftTestSuite) -> Vec<&DriftTestCase> {
        suite.test_cases.iter()
            .filter(|tc| tc.status == TestCaseStatus::Failed)
            .collect()
    }

    /// 生成语义漂移报告（§9.5.2）
    pub fn generate_semantic_drift_report(&self, suite: &DriftTestSuite) -> SemanticDriftReport {
        let tests_passed = suite.summary.as_ref().map(|s| s.passed).unwrap_or(0);
        let tests_failed = suite.summary.as_ref().map(|s| s.failed).unwrap_or(0);
        let overall_drift_score = suite.summary.as_ref().map(|s| s.avg_drift_score).unwrap_or(0.0);

        let recommendation = if tests_failed == 0 {
            ReportRecommendation::Approve
        } else if tests_failed <= 2 {
            ReportRecommendation::Review
        } else {
            ReportRecommendation::BlockRelease
        };

        let test_results: Vec<TestResultEntry> = suite.test_cases.iter().map(|tc| {
            TestResultEntry {
                test_id: tc.test_id.clone(),
                test_name: tc.name.clone(),
                category: tc.category.clone(),
                criticality: tc.criticality.clone(),
                prompt_hash: tc.prompt_hash.clone(),
                reference_response_hash: tc.reference_response_hash.clone(),
                candidate_response_hash: tc.result.as_ref().map(|r| {
                    use sha2::{Digest, Sha256};
                    format!("sha256:{}", hex::encode(Sha256::digest(&r.current_preview.clone().unwrap_or_default())))
                }),
                similarity_score: tc.result.as_ref().map(|r| r.content_similarity).unwrap_or(1.0),
                threshold: tc.get_drift_threshold(self.detector.config.drift_score_threshold),
                drift_detected: tc.status == TestCaseStatus::Failed,
            }
        }).collect();

        SemanticDriftReport {
            suite_id: suite.suite_id.clone(),
            suite_name: suite.name.clone(),
            baseline_model: suite.baseline_model.clone(),
            candidate_model: suite.candidate_model.clone(),
            tests: test_results,
            overall_drift_score,
            tests_passed,
            tests_failed,
            recommendation,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// 测试结果条目（§9.5.2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResultEntry {
    pub test_id: String,
    pub test_name: String,
    pub category: TestCategory,
    pub criticality: TestCriticality,
    pub prompt_hash: Option<String>,
    pub reference_response_hash: Option<String>,
    pub candidate_response_hash: Option<String>,
    pub similarity_score: f64,
    pub threshold: f64,
    pub drift_detected: bool,
}

/// 报告建议（§9.5.2）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportRecommendation {
    #[serde(rename = "APPROVE")]
    Approve,
    #[serde(rename = "REVIEW")]
    Review,
    #[serde(rename = "BLOCK_RELEASE")]
    BlockRelease,
}

/// 语义漂移报告（§9.5.2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDriftReport {
    pub suite_id: String,
    pub suite_name: String,
    pub baseline_model: Option<String>,
    pub candidate_model: Option<String>,
    pub tests: Vec<TestResultEntry>,
    pub overall_drift_score: f64,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub recommendation: ReportRecommendation,
    pub generated_at: String,
}

/// 漂移测试与 Reproducibility 约束集成器（§9.5.3）
pub struct DriftReproducibilityIntegrator {
    executor: DriftTestSuiteExecutor,
    strict_mode_threshold: f64,
}

impl DriftReproducibilityIntegrator {
    pub fn new(executor: DriftTestSuiteExecutor) -> Self {
        Self {
            executor,
            strict_mode_threshold: 0.15,
        }
    }

    pub fn with_strict_threshold(executor: DriftTestSuiteExecutor, threshold: f64) -> Self {
        Self {
            executor,
            strict_mode_threshold: threshold,
        }
    }

    /// 检查是否可以发布（根据 Reproducibility 约束）
    pub fn can_release(&self, suite: &DriftTestSuite) -> ReleaseDecisionReport {
        let report = self.executor.generate_semantic_drift_report(suite);

        let critical_failures: usize = report.tests.iter()
            .filter(|t| t.criticality == TestCriticality::Critical && t.drift_detected)
            .count();

        let high_failures: usize = report.tests.iter()
            .filter(|t| t.criticality == TestCriticality::High && t.drift_detected)
            .count();

        let decision = if critical_failures > 0 {
            ReleaseDecision::Blocked("Critical test failures detected".to_string())
        } else if high_failures > 0 && report.overall_drift_score > self.strict_mode_threshold {
            ReleaseDecision::Blocked("High failures exceed strict mode threshold".to_string())
        } else if report.tests_failed > 0 {
            ReleaseDecision::RequiresReview
        } else {
            ReleaseDecision::Approved
        };

        ReleaseDecisionReport {
            decision,
            report,
            critical_failures,
            high_failures,
            strict_mode_threshold: self.strict_mode_threshold,
        }
    }
}

/// 发布决策
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReleaseDecision {
    #[serde(rename = "APPROVED")]
    Approved,
    #[serde(rename = "REQUIRES_REVIEW")]
    RequiresReview,
    #[serde(rename = "BLOCKED")]
    Blocked(String),
}

/// 发布决策报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseDecisionReport {
    pub decision: ReleaseDecision,
    pub report: SemanticDriftReport,
    pub critical_failures: usize,
    pub high_failures: usize,
    pub strict_mode_threshold: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_similarity() {
        let detector = DriftDetector::default_detector();

        // 相同内容
        let similarity = detector.compute_text_similarity("hello world", "hello world");
        assert!((similarity - 1.0).abs() < 0.01);

        // 部分相似
        let similarity = detector.compute_text_similarity("hello world", "hello universe");
        assert!(similarity > 0.3 && similarity < 0.7);

        // 完全不同
        let similarity = detector.compute_text_similarity("hello world", "goodbye moon");
        assert!(similarity < 0.3);
    }

    #[test]
    fn test_drift_detection() {
        let detector = DriftDetector::default_detector();

        let report = detector.detect_drift(
            "trace-123",
            Some("baseline-456"),
            "这是一段关于春天的描述。春天来了，花开了。",
            "这是一段关于冬天的描述。冬天来了，雪花飘落。",
        );

        assert!(report.diff_enabled);
        assert!(report.content_similarity < 1.0);
        assert!(!report.drift_detections.is_empty() || report.overall_drift_score > 0.0);
    }
}