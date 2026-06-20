//! # Differential Privacy Budget
//!
//! 严格遵循 VERIDACTUS v0.2.1 §8.6 Differential Privacy Budget.
//! 实现 ε-δ 隐私预算管理，防止成员推理攻击和训练数据提取攻击。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialPrivacyBudget {
    pub epsilon: f64,
    pub delta: f64,
    pub mechanism: PrivacyMechanism,
    pub total_budget: BudgetLimit,
    pub consumed_epsilon: f64,
    pub consumed_delta: f64,
    pub remaining_epsilon: f64,
    pub remaining_delta: f64,
    pub budget_exhausted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyMechanism {
    Gaussian,
    Laplace,
    Exponential,
    Rounded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetLimit {
    pub epsilon: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyBudgetSnapshot {
    pub timestamp: String,
    pub total_epsilon: f64,
    pub total_delta: f64,
    pub consumed_epsilon: f64,
    pub consumed_delta: f64,
    pub remaining_epsilon: f64,
    pub remaining_delta: f64,
    pub budget_status: BudgetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStatus {
    Healthy,
    Warning,
    Exhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConsumption {
    pub request_id: String,
    pub consumed_epsilon: f64,
    pub consumed_delta: f64,
    pub timestamp: String,
    pub privacy_level: String,
    pub model_id: String,
}

pub struct DifferentialPrivacyManager {
    /// 存储 Arc<RwLock<>> 以确保所有修改都是对同一对象的原子操作
    budgets: RwLock<HashMap<String, Arc<RwLock<DifferentialPrivacyBudget>>>>,
    default_budget: BudgetLimit,
    warning_threshold: f64,
}

impl DifferentialPrivacyManager {
    pub fn new() -> Self {
        Self {
            budgets: RwLock::new(HashMap::new()),
            default_budget: BudgetLimit {
                epsilon: 5.0,
                delta: 1e-5,
            },
            warning_threshold: 0.8,
        }
    }

    pub fn with_default_budget(mut self, budget: BudgetLimit) -> Self {
        self.default_budget = budget;
        self
    }

    /// 原子获取或创建预算 — 返回 Arc 确保所有修改指向同一对象
    pub fn get_or_create_budget(&self, key: &str) -> Arc<RwLock<DifferentialPrivacyBudget>> {
        // 先用读锁检查
        {
            let budgets = self.budgets.read().unwrap();
            if let Some(b) = budgets.get(key) {
                return b.clone();
            }
        }
        // 用写锁创建
        let mut budgets = self.budgets.write().unwrap();
        budgets.entry(key.to_string()).or_insert_with(|| {
            Arc::new(RwLock::new(DifferentialPrivacyBudget {
                epsilon: self.default_budget.epsilon,
                delta: self.default_budget.delta,
                mechanism: PrivacyMechanism::Gaussian,
                total_budget: self.default_budget.clone(),
                consumed_epsilon: 0.0,
                consumed_delta: 0.0,
                remaining_epsilon: self.default_budget.epsilon,
                remaining_delta: self.default_budget.delta,
                budget_exhausted: false,
            }))
        }).clone()
    }

    pub fn consume_budget(
        &self,
        key: &str,
        epsilon_consumed: f64,
        delta_consumed: f64,
    ) -> Result<PrivacyConsumption, BudgetError> {
        let budget = self.get_or_create_budget(key);
        let mut budget_mut = budget.write().unwrap();

        if budget_mut.budget_exhausted {
            return Err(BudgetError::Exhausted);
        }

        if budget_mut.consumed_epsilon + epsilon_consumed > budget_mut.total_budget.epsilon {
            return Err(BudgetError::InsufficientEpsilon);
        }

        if budget_mut.consumed_delta + delta_consumed > budget_mut.total_budget.delta {
            return Err(BudgetError::InsufficientDelta);
        }

        budget_mut.consumed_epsilon += epsilon_consumed;
        budget_mut.consumed_delta += delta_consumed;
        budget_mut.remaining_epsilon = budget_mut.total_budget.epsilon - budget_mut.consumed_epsilon;
        budget_mut.remaining_delta = budget_mut.total_budget.delta - budget_mut.consumed_delta;

        if budget_mut.remaining_epsilon <= 0.0 || budget_mut.remaining_delta <= 0.0 {
            budget_mut.budget_exhausted = true;
        }

        Ok(PrivacyConsumption {
            request_id: format!("req_{}", uuid::Uuid::new_v4()),
            consumed_epsilon: epsilon_consumed,
            consumed_delta: delta_consumed,
            timestamp: chrono::Utc::now().to_rfc3339(),
            privacy_level: "tee_private".to_string(),
            model_id: "unknown".to_string(),
        })
    }

    pub fn get_budget_snapshot(&self, key: &str) -> Option<PrivacyBudgetSnapshot> {
        let budgets = self.budgets.read().unwrap();
        budgets.get(key).map(|arc_budget| {
            let budget = arc_budget.read().unwrap();
            let usage_ratio = budget.consumed_epsilon / budget.total_budget.epsilon;
            let status = if budget.budget_exhausted {
                BudgetStatus::Exhausted
            } else if usage_ratio >= self.warning_threshold {
                BudgetStatus::Warning
            } else {
                BudgetStatus::Healthy
            };

            PrivacyBudgetSnapshot {
                timestamp: chrono::Utc::now().to_rfc3339(),
                total_epsilon: budget.total_budget.epsilon,
                total_delta: budget.total_budget.delta,
                consumed_epsilon: budget.consumed_epsilon,
                consumed_delta: budget.consumed_delta,
                remaining_epsilon: budget.remaining_epsilon,
                remaining_delta: budget.remaining_delta,
                budget_status: status,
            }
        })
    }

    pub fn reset_budget(&self, key: &str) -> Result<(), BudgetError> {
        let budgets = self.budgets.read().unwrap();
        if let Some(budget) = budgets.get(key) {
            let mut b = budget.write().unwrap();
            b.consumed_epsilon = 0.0;
            b.consumed_delta = 0.0;
            b.remaining_epsilon = b.total_budget.epsilon;
            b.remaining_delta = b.total_budget.delta;
            b.budget_exhausted = false;
            Ok(())
        } else {
            Err(BudgetError::NotFound)
        }
    }
}

#[derive(Debug)]
pub enum BudgetError {
    Exhausted,
    InsufficientEpsilon,
    InsufficientDelta,
    NotFound,
    InvalidAmount,
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetError::Exhausted => write!(f, "Privacy budget exhausted"),
            BudgetError::InsufficientEpsilon => write!(f, "Insufficient epsilon budget"),
            BudgetError::InsufficientDelta => write!(f, "Insufficient delta budget"),
            BudgetError::NotFound => write!(f, "Budget not found"),
            BudgetError::InvalidAmount => write!(f, "Invalid consumption amount"),
        }
    }
}

impl std::error::Error for BudgetError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_budget() {
        let manager = DifferentialPrivacyManager::new();
        let result = manager.consume_budget("user123", 0.5, 1e-7);
        
        assert!(result.is_ok());
        let consumption = result.unwrap();
        assert_eq!(consumption.consumed_epsilon, 0.5);
    }

    #[test]
    fn test_budget_exhausted() {
        let manager = DifferentialPrivacyManager::new();
        
        // 消耗全部 epsilon 预算（10 × 0.5 = 5.0，等于默认预算上限）
        // 使用 delta=0.0 避免 delta 预算先耗尽
        for _ in 0..10 {
            let _ = manager.consume_budget("user123", 0.5, 0.0);
        }
        
        // 第 11 次调用应失败（epsilon 已耗尽，budget_exhausted=true）
        let result = manager.consume_budget("user123", 0.5, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_budget_status() {
        let manager = DifferentialPrivacyManager::new();
        
        for _ in 0..8 {
            let _ = manager.consume_budget("user123", 0.5, 1e-7);
        }
        
        let snapshot = manager.get_budget_snapshot("user123").unwrap();
        assert_eq!(snapshot.budget_status, BudgetStatus::Warning);
    }
}