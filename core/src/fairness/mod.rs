//! # Fairness Audit Module
//!
//! 实现 VERIDACTUS v0.2.1 §9.2 Fairness Audit 规范.
//!
//! 提供公平性指标计算和偏差检测功能.

use serde::{Deserialize, Serialize};
// use sha2::Digest; // unused
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FairnessAuditReport {
    pub audit_id: String,
    pub timestamp: String,
    pub metrics: FairnessMetrics,
    pub bias_detections: Vec<BiasDetection>,
    pub overall_fairness_score: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FairnessMetrics {
    pub demographic_parity: Option<DemographicParity>,
    pub equalized_odds: Option<EqualizedOdds>,
    pub calibration: Option<CalibrationMetrics>,
    pub individual_fairness: Option<IndividualFairness>,
    pub intersectional_fairness: Option<IntersectFairness>,
    pub proxy_variable_analysis: Option<ProxyAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemographicParity {
    pub difference: f64,
    pub ratio: f64,
    pub privileged_group_rate: f64,
    pub unprivileged_group_rate: f64,
    pub threshold: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqualizedOdds {
    pub true_positive_rate_gap: f64,
    pub false_positive_rate_gap: f64,
    pub threshold: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationMetrics {
    pub max_calibration_error: f64,
    pub expected_calibration_error: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualFairness {
    pub consistency_score: f64,
    pub threshold: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntersectFairness {
    pub subgroups: Vec<SubgroupMetrics>,
    pub max_disparity: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgroupMetrics {
    pub subgroup_id: String,
    pub positive_rate: f64,
    pub sample_count: usize,
    pub disparity_vs_overall: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAnalysis {
    pub potential_proxies: Vec<ProxyVariable>,
    pub risk_level: ProxyRiskLevel,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyVariable {
    pub name: String,
    pub correlation_with_protected: f64,
    pub correlation_with_outcome: f64,
    pub risk_score: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProxyRiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasDetection {
    pub bias_type: BiasType,
    pub severity: BiasSeverity,
    pub affected_groups: Vec<String>,
    pub description: String,
    pub mitigation_suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiasType {
    DemographicParity,
    EqualizedOdds,
    Calibration,
    IndividualFairness,
    Intersectionality,
    ProxyVariable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiasSeverity {
    Low,
    Medium,
    High,
    Critical,
}

pub struct FairnessAuditor {
    demographic_parity_threshold: f64,
    equalized_odds_threshold: f64,
    calibration_threshold: f64,
    consistency_threshold: f64,
    intersectional_threshold: f64,
    proxy_correlation_threshold: f64,
}

impl Default for FairnessAuditor {
    fn default() -> Self {
        Self::new()
    }
}

impl FairnessAuditor {
    pub fn new() -> Self {
        Self {
            demographic_parity_threshold: 0.1,
            equalized_odds_threshold: 0.1,
            calibration_threshold: 0.05,
            consistency_threshold: 0.95,
            intersectional_threshold: 0.15,
            proxy_correlation_threshold: 0.7,
        }
    }

    pub fn with_thresholds(
        demographic_parity_threshold: f64,
        equalized_odds_threshold: f64,
        calibration_threshold: f64,
        consistency_threshold: f64,
    ) -> Self {
        Self {
            demographic_parity_threshold,
            equalized_odds_threshold,
            calibration_threshold,
            consistency_threshold,
            intersectional_threshold: 0.15,
            proxy_correlation_threshold: 0.7,
        }
    }

    pub fn audit(
        &self,
        predictions: &[Prediction],
        protected_attributes: &[ProtectedAttribute],
    ) -> FairnessAuditReport {
        let metrics = self.calculate_metrics(predictions, protected_attributes);
        let bias_detections = self.detect_bias(&metrics);
        let overall_score = self.calculate_overall_fairness_score(&metrics, &bias_detections);
        let recommendations = self.generate_recommendations(&metrics, &bias_detections);

        FairnessAuditReport {
            audit_id: uuid_v4(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metrics,
            bias_detections,
            overall_fairness_score: overall_score,
            recommendations,
        }
    }

    pub fn audit_with_proxy_check(
        &self,
        predictions: &[Prediction],
        protected_attributes: &[ProtectedAttribute],
        features: &[FeatureVector],
    ) -> FairnessAuditReport {
        let mut metrics = self.calculate_metrics(predictions, protected_attributes);

        if features.len() == predictions.len() {
            metrics.proxy_variable_analysis =
                Some(self.analyze_proxy_variables(predictions, protected_attributes, features));
        }

        metrics.intersectional_fairness =
            Some(self.calculate_intersectional_fairness(predictions, protected_attributes));

        let bias_detections = self.detect_bias(&metrics);
        let overall_score = self.calculate_overall_fairness_score(&metrics, &bias_detections);
        let recommendations = self.generate_recommendations(&metrics, &bias_detections);

        FairnessAuditReport {
            audit_id: uuid_v4(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metrics,
            bias_detections,
            overall_fairness_score: overall_score,
            recommendations,
        }
    }

    fn calculate_metrics(
        &self,
        predictions: &[Prediction],
        protected_attributes: &[ProtectedAttribute],
    ) -> FairnessMetrics {
        FairnessMetrics {
            demographic_parity: Some(
                self.calculate_demographic_parity(predictions, protected_attributes),
            ),
            equalized_odds: Some(self.calculate_equalized_odds(predictions, protected_attributes)),
            calibration: Some(self.calculate_calibration(predictions)),
            individual_fairness: Some(self.calculate_individual_fairness(predictions)),
            intersectional_fairness: None,
            proxy_variable_analysis: None,
        }
    }

    fn calculate_demographic_parity(
        &self,
        predictions: &[Prediction],
        protected_attributes: &[ProtectedAttribute],
    ) -> DemographicParity {
        let mut group_positive_rates: HashMap<String, f64> = HashMap::new();
        let mut group_counts: HashMap<String, usize> = HashMap::new();

        for (pred, attr) in predictions.iter().zip(protected_attributes.iter()) {
            let group = &attr.group;
            let count = group_counts.entry(group.clone()).or_insert(0);
            *count += 1;
            if pred.score >= 0.5 {
                *group_positive_rates.entry(group.clone()).or_insert(0.0) += 1.0;
            }
        }

        let mut rates: Vec<f64> = Vec::new();
        for (group, count) in &group_counts {
            let rate = group_positive_rates.get(group).copied().unwrap_or(0.0) / (*count as f64);
            rates.push(rate);
        }

        rates.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let unprivileged_rate = rates.first().copied().unwrap_or(0.0);
        let privileged_rate = rates.last().copied().unwrap_or(0.0);

        let diff = (privileged_rate - unprivileged_rate).abs();
        let ratio = if privileged_rate > 0.0 {
            unprivileged_rate / privileged_rate
        } else {
            1.0
        };

        DemographicParity {
            difference: diff,
            ratio,
            privileged_group_rate: privileged_rate,
            unprivileged_group_rate: unprivileged_rate,
            threshold: self.demographic_parity_threshold,
            passed: diff < self.demographic_parity_threshold,
        }
    }

    fn calculate_equalized_odds(
        &self,
        predictions: &[Prediction],
        protected_attributes: &[ProtectedAttribute],
    ) -> EqualizedOdds {
        let mut tpr_by_group: HashMap<String, Vec<(f64, bool)>> = HashMap::new();
        let mut fpr_by_group: HashMap<String, Vec<(f64, bool)>> = HashMap::new();

        for (pred, attr) in predictions.iter().zip(protected_attributes.iter()) {
            let group = &attr.group;
            tpr_by_group
                .entry(group.clone())
                .or_insert_with(Vec::new)
                .push((pred.score, pred.actual_outcome));
            fpr_by_group
                .entry(group.clone())
                .or_insert_with(Vec::new)
                .push((pred.score, !pred.actual_outcome && pred.score >= 0.5));
        }

        let mut tpr_gaps: Vec<f64> = Vec::new();
        let mut fpr_gaps: Vec<f64> = Vec::new();

        let groups: Vec<String> = tpr_by_group.keys().cloned().collect();
        if groups.len() >= 2 {
            for i in 0..groups.len() {
                for j in (i + 1)..groups.len() {
                    let tpr_i = Self::compute_tpr(&tpr_by_group[&groups[i]]);
                    let tpr_j = Self::compute_tpr(&tpr_by_group[&groups[j]]);
                    tpr_gaps.push((tpr_i - tpr_j).abs());

                    let fpr_i = Self::compute_fpr(&fpr_by_group[&groups[i]]);
                    let fpr_j = Self::compute_fpr(&fpr_by_group[&groups[j]]);
                    fpr_gaps.push((fpr_i - fpr_j).abs());
                }
            }
        }

        let avg_tpr_gap = if tpr_gaps.is_empty() {
            0.0
        } else {
            tpr_gaps.iter().sum::<f64>() / tpr_gaps.len() as f64
        };
        let avg_fpr_gap = if fpr_gaps.is_empty() {
            0.0
        } else {
            fpr_gaps.iter().sum::<f64>() / fpr_gaps.len() as f64
        };

        EqualizedOdds {
            true_positive_rate_gap: avg_tpr_gap,
            false_positive_rate_gap: avg_fpr_gap,
            threshold: self.equalized_odds_threshold,
            passed: avg_tpr_gap < self.equalized_odds_threshold
                && avg_fpr_gap < self.equalized_odds_threshold,
        }
    }

    fn compute_tpr(items: &[(f64, bool)]) -> f64 {
        let positives: usize = items.iter().filter(|(_, outcome)| *outcome).count();
        let predicted_positives: usize = items
            .iter()
            .filter(|(score, _)| *score >= 0.5 && *score >= 0.5)
            .count();
        if positives == 0 {
            return 1.0;
        }
        predicted_positives as f64 / positives as f64
    }

    fn compute_fpr(items: &[(f64, bool)]) -> f64 {
        let negatives: usize = items.iter().filter(|(_, outcome)| !*outcome).count();
        let predicted_positives: usize = items
            .iter()
            .filter(|(score, outcome)| *score >= 0.5 && !*outcome)
            .count();
        if negatives == 0 {
            return 0.0;
        }
        predicted_positives as f64 / negatives as f64
    }

    fn calculate_calibration(&self, predictions: &[Prediction]) -> CalibrationMetrics {
        let mut bin_correct = vec![0.0; 10];
        let mut bin_total = vec![0.0; 10];

        for pred in predictions {
            let bin_idx = ((pred.score * 10.0) as usize).min(9);
            bin_total[bin_idx] += 1.0;
            if (pred.score >= 0.5) == pred.actual_outcome {
                bin_correct[bin_idx] += 1.0;
            }
        }

        let mut max_cal_error: f64 = 0.0;
        let mut sum_cal_error: f64 = 0.0;
        let mut valid_bins = 0;

        for i in 0..10 {
            if bin_total[i] > 0.0 {
                let bin_mid = (i as f64 + 0.5) / 10.0;
                let bin_accuracy = bin_correct[i] / bin_total[i];
                let cal_error = (bin_mid - bin_accuracy).abs();
                max_cal_error = max_cal_error.max(cal_error);
                sum_cal_error += cal_error;
                valid_bins += 1;
            }
        }

        let ece = if valid_bins > 0 {
            sum_cal_error / valid_bins as f64
        } else {
            0.0
        };

        CalibrationMetrics {
            max_calibration_error: max_cal_error,
            expected_calibration_error: ece,
            passed: ece < self.calibration_threshold,
        }
    }

    fn calculate_individual_fairness(&self, predictions: &[Prediction]) -> IndividualFairness {
        let n = predictions.len();
        if n < 2 {
            return IndividualFairness {
                consistency_score: 1.0,
                threshold: self.consistency_threshold,
                passed: true,
            };
        }

        let mut total_consistency = 0.0;
        let mut comparisons = 0;

        for i in 0..n {
            for j in (i + 1)..n {
                let similarity = Self::compute_similarity(&predictions[i], &predictions[j]);
                let score_diff = (predictions[i].score - predictions[j].score).abs();
                let expected_diff = 1.0 - similarity;
                let consistency = 1.0 - (score_diff - expected_diff).abs().min(1.0);
                total_consistency += consistency;
                comparisons += 1;
            }
        }

        let consistency_score = if comparisons > 0 {
            total_consistency / comparisons as f64
        } else {
            1.0
        };

        IndividualFairness {
            consistency_score,
            threshold: self.consistency_threshold,
            passed: consistency_score >= self.consistency_threshold,
        }
    }

    fn compute_similarity(a: &Prediction, b: &Prediction) -> f64 {
        let features_similarity = if let (Some(af), Some(bf)) = (&a.features, &b.features) {
            let dot = af.iter().zip(bf.iter()).map(|(x, y)| x * y).sum::<f64>();
            let norm_a = af.iter().map(|x| x * x).sum::<f64>().sqrt();
            let norm_b = bf.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm_a > 0.0 && norm_b > 0.0 {
                dot / (norm_a * norm_b)
            } else {
                0.0
            }
        } else {
            0.0
        };
        (features_similarity + 1.0) / 2.0
    }

    fn detect_bias(&self, metrics: &FairnessMetrics) -> Vec<BiasDetection> {
        let mut detections = Vec::new();

        if let Some(ref dp) = metrics.demographic_parity {
            if !dp.passed {
                detections.push(BiasDetection {
                    bias_type: BiasType::DemographicParity,
                    severity: if dp.difference > 0.3 {
                        BiasSeverity::High
                    } else {
                        BiasSeverity::Medium
                    },
                    affected_groups: vec![
                        "protected_group_1".to_string(),
                        "protected_group_2".to_string(),
                    ],
                    description: format!(
                        "人口统计 parity 差异 {} 超过阈值 {}",
                        dp.difference, dp.threshold
                    ),
                    mitigation_suggestion: "考虑对受保护群体使用校准后的决策阈值".to_string(),
                });
            }
        }

        if let Some(ref eo) = metrics.equalized_odds {
            if !eo.passed {
                detections.push(BiasDetection {
                    bias_type: BiasType::EqualizedOdds,
                    severity: if eo.true_positive_rate_gap > 0.2 {
                        BiasSeverity::High
                    } else {
                        BiasSeverity::Medium
                    },
                    affected_groups: vec!["group_a".to_string(), "group_b".to_string()],
                    description: format!(
                        "均等化机会差距 TPR={}, FPR={}",
                        eo.true_positive_rate_gap, eo.false_positive_rate_gap
                    ),
                    mitigation_suggestion: "实施对抗性去偏或再平衡技术".to_string(),
                });
            }
        }

        detections
    }

    fn calculate_overall_fairness_score(
        &self,
        metrics: &FairnessMetrics,
        detections: &[BiasDetection],
    ) -> f64 {
        let mut score = 1.0;

        if let Some(ref dp) = metrics.demographic_parity {
            if dp.passed {
                score *= 1.0;
            } else {
                score *= 1.0 - dp.difference.min(0.5);
            }
        }

        if let Some(ref eo) = metrics.equalized_odds {
            if !eo.passed {
                score *=
                    1.0 - (eo.true_positive_rate_gap + eo.false_positive_rate_gap).min(0.5) / 2.0;
            }
        }

        for detection in detections {
            match detection.severity {
                BiasSeverity::Critical => score *= 0.5,
                BiasSeverity::High => score *= 0.7,
                BiasSeverity::Medium => score *= 0.85,
                BiasSeverity::Low => score *= 0.95,
            }
        }

        score.max(0.0).min(1.0)
    }

    fn generate_recommendations(
        &self,
        metrics: &FairnessMetrics,
        detections: &[BiasDetection],
    ) -> Vec<String> {
        let mut recs = Vec::new();

        if let Some(ref dp) = metrics.demographic_parity {
            if !dp.passed {
                recs.push(
                    "建议使用机会均等化(Equality of Opportunity)替代人口统计 parity".to_string(),
                );
            }
        }

        if let Some(ref eo) = metrics.equalized_odds {
            if !eo.passed {
                recs.push("建议实施 Threshold Adaptation 根据群体调整决策阈值".to_string());
            }
        }

        if detections
            .iter()
            .any(|d| matches!(d.bias_type, BiasType::Intersectionality))
        {
            recs.push("检测到交叉性偏差，建议分别评估每个交叉群体".to_string());
        }

        if recs.is_empty() {
            recs.push("未检测到显著偏差，建议定期进行公平性审计".to_string());
        }

        recs
    }

    fn calculate_intersectional_fairness(
        &self,
        predictions: &[Prediction],
        protected_attributes: &[ProtectedAttribute],
    ) -> IntersectFairness {
        let mut subgroup_data: HashMap<String, (usize, usize)> = HashMap::new();

        for (pred, attr) in predictions.iter().zip(protected_attributes.iter()) {
            let subgroup_id = match &attr.subgroup {
                Some(sg) => format!("{}_{}", attr.group, sg),
                None => attr.group.clone(),
            };

            let (positive_count, total_count) =
                subgroup_data.entry(subgroup_id.clone()).or_insert((0, 0));
            *total_count += 1;
            if pred.score >= 0.5 {
                *positive_count += 1;
            }
        }

        let overall_positive_rate = if !predictions.is_empty() {
            predictions.iter().filter(|p| p.score >= 0.5).count() as f64 / predictions.len() as f64
        } else {
            0.0
        };

        let mut subgroups: Vec<SubgroupMetrics> = subgroup_data
            .iter()
            .map(|(subgroup_id, (pos_count, total_count))| {
                let positive_rate = *pos_count as f64 / *total_count as f64;
                let disparity = (positive_rate - overall_positive_rate).abs();
                SubgroupMetrics {
                    subgroup_id: subgroup_id.clone(),
                    positive_rate,
                    sample_count: *total_count,
                    disparity_vs_overall: disparity,
                }
            })
            .collect();

        subgroups.sort_by(|a, b| {
            b.disparity_vs_overall
                .partial_cmp(&a.disparity_vs_overall)
                .unwrap()
        });

        let max_disparity = subgroups
            .first()
            .map(|s| s.disparity_vs_overall)
            .unwrap_or(0.0);

        IntersectFairness {
            subgroups,
            max_disparity,
            passed: max_disparity < self.intersectional_threshold,
        }
    }

    fn analyze_proxy_variables(
        &self,
        predictions: &[Prediction],
        protected_attributes: &[ProtectedAttribute],
        features: &[FeatureVector],
    ) -> ProxyAnalysis {
        let mut potential_proxies = Vec::new();

        if protected_attributes.len() != features.len() || features.is_empty() {
            return ProxyAnalysis {
                potential_proxies,
                risk_level: ProxyRiskLevel::Low,
                passed: true,
            };
        }

        let protected_binary: Vec<f64> = protected_attributes
            .iter()
            .map(|attr| if attr.sensitive { 1.0 } else { 0.0 })
            .collect();

        let outcomes: Vec<f64> = predictions
            .iter()
            .map(|p| if p.actual_outcome { 1.0 } else { 0.0 })
            .collect();

        let feature_dim = features[0].values.len();

        for feature_idx in 0..feature_dim {
            let feature_values: Vec<f64> = features.iter().map(|f| f.values[feature_idx]).collect();

            let protected_corr = Self::pearson_correlation(&protected_binary, &feature_values);
            let outcome_corr = Self::pearson_correlation(&outcomes, &feature_values);

            let risk_score = (protected_corr.abs() * 0.5 + outcome_corr.abs() * 0.5).min(1.0);

            if protected_corr.abs() > self.proxy_correlation_threshold && outcome_corr.abs() > 0.3 {
                potential_proxies.push(ProxyVariable {
                    name: format!("feature_{}", feature_idx),
                    correlation_with_protected: protected_corr,
                    correlation_with_outcome: outcome_corr,
                    risk_score,
                    description: format!(
                        "特征 {} 可能成为代理变量，与受保护属性相关性: {:.3}, 与结果相关性: {:.3}",
                        feature_idx, protected_corr, outcome_corr
                    ),
                });
            }
        }

        potential_proxies.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());

        let risk_level = if potential_proxies.iter().any(|p| p.risk_score > 0.7) {
            ProxyRiskLevel::High
        } else if potential_proxies.iter().any(|p| p.risk_score > 0.5) {
            ProxyRiskLevel::Medium
        } else {
            ProxyRiskLevel::Low
        };

        let passed = !matches!(risk_level, ProxyRiskLevel::High);

        ProxyAnalysis {
            potential_proxies,
            risk_level,
            passed,
        }
    }

    fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
        if x.len() != y.len() || x.is_empty() {
            return 0.0;
        }

        let n = x.len() as f64;
        let mean_x = x.iter().sum::<f64>() / n;
        let mean_y = y.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denom_x = 0.0;
        let mut denom_y = 0.0;

        for (xi, yi) in x.iter().zip(y.iter()) {
            let dx = xi - mean_x;
            let dy = yi - mean_y;
            numerator += dx * dy;
            denom_x += dx * dx;
            denom_y += dy * dy;
        }

        let denom = (denom_x.sqrt() * denom_y.sqrt()).max(f64::MIN_POSITIVE);
        numerator / denom
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    pub name: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: String,
    pub score: f64,
    pub actual_outcome: bool,
    pub features: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedAttribute {
    pub group: String,
    pub subgroup: Option<String>,
    pub sensitive: bool,
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("fairness_audit_{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fairness_auditor_demographic_parity() {
        let auditor = FairnessAuditor::new();

        let predictions = vec![
            Prediction {
                id: "1".to_string(),
                score: 0.9,
                actual_outcome: true,
                features: None,
            },
            Prediction {
                id: "2".to_string(),
                score: 0.8,
                actual_outcome: true,
                features: None,
            },
            Prediction {
                id: "3".to_string(),
                score: 0.3,
                actual_outcome: false,
                features: None,
            },
        ];

        let protected_attributes = vec![
            ProtectedAttribute {
                group: "group_a".to_string(),
                subgroup: None,
                sensitive: true,
            },
            ProtectedAttribute {
                group: "group_a".to_string(),
                subgroup: None,
                sensitive: true,
            },
            ProtectedAttribute {
                group: "group_b".to_string(),
                subgroup: None,
                sensitive: true,
            },
        ];

        let report = auditor.audit(&predictions, &protected_attributes);
        assert!(report.overall_fairness_score >= 0.0 && report.overall_fairness_score <= 1.0);
    }
}
