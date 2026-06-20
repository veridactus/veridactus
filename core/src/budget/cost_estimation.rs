//! # 预算成本预估
//!
//! 协议 §5.0 要求：在转发请求前预估 Token 成本。
//! 使用模型定价表 + 提示词长度做预检（pre-flight cost estimation）。

use std::collections::HashMap;
use tracing::info;

/// 模型定价（美元/1000 token）
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub prompt_price_per_1k: f64,
    pub completion_price_per_1k: f64,
}

/// 成本预估结果
#[derive(Debug, Clone)]
pub struct CostEstimate {
    /// 预估提示词 Token 数
    pub estimated_prompt_tokens: usize,
    /// 预估完成 Token 数
    pub estimated_completion_tokens: usize,
    /// 预估总成本（美元）
    pub estimated_cost_usd: f64,
    /// 是否在预算内
    pub within_budget: bool,
    /// 剩余预算
    pub remaining_budget: f64,
}

/// 默认定价表（常用模型）
pub fn default_pricing() -> HashMap<&'static str, ModelPricing> {
    let mut m = HashMap::new();
    // OpenAI/Azure
    m.insert("gpt-4o", ModelPricing { prompt_price_per_1k: 0.0025, completion_price_per_1k: 0.01 });
    m.insert("gpt-4o-mini", ModelPricing { prompt_price_per_1k: 0.00015, completion_price_per_1k: 0.0006 });
    // Zhipu
    m.insert("glm-5.1", ModelPricing { prompt_price_per_1k: 0.001, completion_price_per_1k: 0.001 });
    m.insert("glm-4-flash", ModelPricing { prompt_price_per_1k: 0.0001, completion_price_per_1k: 0.0001 });
    // DeepSeek
    m.insert("deepseek-r1:14b", ModelPricing { prompt_price_per_1k: 0.0, completion_price_per_1k: 0.0 });
    // Gemini
    m.insert("gemini-flash", ModelPricing { prompt_price_per_1k: 0.000075, completion_price_per_1k: 0.0003 });
    m
}

/// 预检成本预估
///
/// # 参数
/// - `model_name`: 模型名称
/// - `prompt_text`: 提示词文本（用于估算 token 数）
/// - `max_tokens`: 客户端请求的最大 token 数
/// - `budget_limit_usd`: 预算上限（美元），None = 无限制
pub fn estimate_cost(
    model_name: &str,
    prompt_text: &str,
    max_tokens: Option<usize>,
    budget_limit_usd: Option<f64>,
) -> CostEstimate {
    let pricing_table = default_pricing();
    let model_pricing = pricing_table
        .get(model_name)
        .unwrap_or(&ModelPricing { prompt_price_per_1k: 0.001, completion_price_per_1k: 0.002 });

    // 简单 token 估算：英文约 4 字符/token，中文约 2 字符/token
    let char_count = prompt_text.chars().count();
    let is_cjk = prompt_text.chars().any(|c| c as u32 > 0x2E80);
    let chars_per_token = if is_cjk { 2.0 } else { 4.0 };
    let estimated_prompt_tokens = (char_count as f64 / chars_per_token).ceil() as usize;

    // 预估完成 token = 请求 max_tokens 或默认 256
    let default_max = 256usize;
    let estimated_completion_tokens = max_tokens.unwrap_or(default_max).min(default_max);

    let cost = (estimated_prompt_tokens as f64 / 1000.0) * model_pricing.prompt_price_per_1k
        + (estimated_completion_tokens as f64 / 1000.0) * model_pricing.completion_price_per_1k;

    let within_budget = budget_limit_usd.map_or(true, |limit| cost <= limit);
    let remaining = budget_limit_usd.map_or(f64::MAX, |limit| limit - cost);

    if let Some(limit) = budget_limit_usd {
        info!(
            "预算预检: model={}, prompt_tokens≈{}, completion_tokens≈{}, cost≈${:.6}, limit=${}, within_budget={}",
            model_name, estimated_prompt_tokens, estimated_completion_tokens, cost, limit, within_budget
        );
    }

    CostEstimate {
        estimated_prompt_tokens,
        estimated_completion_tokens,
        estimated_cost_usd: cost,
        within_budget,
        remaining_budget: remaining.max(0.0),
    }
}

/// 应用 buffer_ratio 缓冲比例（协议 §5.2.1）
/// 在预算限制之上预留安全缓冲，防止微小浮动导致超额
pub fn apply_buffer_ratio(limit: f64, buffer_ratio: Option<f64>) -> f64 {
    let ratio = buffer_ratio.unwrap_or(0.001);
    limit * (1.0 - ratio)
}

/// 计算实际成本（基于上游返回的 usage 信息）
pub fn calculate_actual_cost(
    model_name: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
) -> f64 {
    let pricing_table = default_pricing();
    let model_pricing = pricing_table
        .get(model_name)
        .unwrap_or(&ModelPricing { prompt_price_per_1k: 0.001, completion_price_per_1k: 0.002 });

    (prompt_tokens as f64 / 1000.0) * model_pricing.prompt_price_per_1k
        + (completion_tokens as f64 / 1000.0) * model_pricing.completion_price_per_1k
}
