export interface TraceSummary {
  trace_id: string;
  model: string;
  created_at: string;
  execution_state: string;
  proof_levels: string[];
  signature?: string;
}

export interface TraceDetail {
  trace_id: string;
  model?: string;
  tenant_id?: string;
  created_at: string;
  execution_state?: string;
  input?: { prompt?: any; params?: any; metadata?: any };
  output?: { response?: any; truncated: boolean; finish_reason?: string };
  proofs?: { proof_chain: Proof[]; aggregated_root?: string };
  constraints_applied?: any;
  observations?: Observations;
  supply_chain?: any;
  engine_determinism?: any;
}

export interface Observations {
  token_count?: number;
  prompt_tokens?: number;
  completion_tokens?: number;
  latency_ms?: number;
  cost_usd?: number;
  budget_used?: number;
  budget_limit?: number;
  risk_score?: number;
  drift_score?: number;
  events?: ObservationEvent[];
}

export interface ObservationEvent {
  timestamp: string;
  event_type: string;
  details?: any;
}

export interface Proof {
  level: string;
  proof_type: string;
  signature?: string;
  timestamp?: string;
  digest?: string;
}

export interface Pipeline {
  plan_id: string;
  tenant: string;
  stages: StageConfig[];
  created_at: string;
}

export interface StageConfig {
  placement: string;
  parallel: boolean;
  plugins: PluginConf[];
}

export interface PluginConf {
  name: string;
  type: string;
  config: string;
  enabled: boolean;
}

export interface PluginMeta {
  id: string;
  name: string;
  type: string;
  version: string;
  description: string;
  config?: string;
}

export interface Policy {
  id: string;
  name: string;
  type: string;
  content: string;
  created_at: string;
}

export interface ModelInfo {
  id: string;
  object: string;
  created: number;
  owned_by: string;
  upstream_endpoint: string;
  is_default: boolean;
}

export interface ApiKey {
  id: string;
  name: string;
  key: string;
  tenant_id: string;
  status: string;
  created_at: string;
  last_used?: string;
}

export interface ModelConfig {
  id: string;
  name: string;
  upstream_url: string;
  upstream_model: string;
  is_default: boolean;
  supported_versions?: string[];
  status: string;
  api_key?: string;
  api_key_header?: string;
  use_proxy?: boolean;
  proxy_url?: string;
}

// 验证结果类型
export interface VerificationResult {
  trace_id: string;
  l0_passed: boolean;
  l1_passed?: boolean;
  l2a_passed?: boolean;
  l2b_passed?: boolean;
  error?: string;
  canonical_json?: string;
  overall_passed: boolean;
}

// 重放结果类型
export interface ReplayResult {
  success: boolean;
  trace_id: string;
  cache_hit: boolean;
  duration_ms: number;
  mode: string;
  branch_id?: string;
  determinism: {
    is_identical: boolean;
    similarity_score: number;
    hash_match: boolean;
    token_diff_count: number;
    byte_diff_count: number;
  };
}

// 重放分支类型
export interface ReplayBranch {
  branch_id: string;
  parent_branch_id?: string;
  name: string;
  created_at: string;
  snapshot_count: number;
}

// 实时指标类型
export interface RealTimeMetrics {
  requests_total: number;
  latency_sum_seconds: number;
  latency_count: number;
  constraint_violations_total: number;
  guardrail_activations_total: number;
  asi_risks_flagged_total: number;
  average_latency_ms: number;
  timestamp: string;
}
