/**
 * VERIDACTUS 前后端数据格式转换器
 *
 * 后端(Rust)返回的 Trace 格式遵循 snake_case 命名,
 * 前端 UI 组件期望的格式使用 camelCase。
 * 此模块负责在 API 层进行转换。
 */
import type { TraceSummary, TraceDetail, Observations } from '../types';

/** 将后端 Trace 摘要转换为前端格式 */
export function transformTraceSummary(raw: any): TraceSummary {
  return {
    trace_id: raw.trace_id || '',
    model: raw.model || '',
    created_at: raw.created_at || '',
    execution_state: raw.execution_state
      ? typeof raw.execution_state === 'string'
        ? raw.execution_state
        : raw.execution_state.Init ? 'INIT'
        : raw.execution_state.Finalized ? 'FINALIZED'
        : raw.execution_state.Failed ? 'FAILED'
        : raw.execution_state.ConstraintEval ? 'CONSTRAINT_EVAL'
        : raw.execution_state.Executing ? 'EXECUTING'
        : raw.execution_state.Validation ? 'VALIDATION'
        : String(raw.execution_state)
      : '',
    proof_levels: Array.isArray(raw.proof_levels) ? raw.proof_levels : [],
    signature: raw.signature || undefined,
  };
}

/** 将后端 Trace 详情转换为前端格式 */
export function transformTraceDetail(raw: any): TraceDetail {
  return {
    trace_id: raw.trace_id || '',
    model: raw.model || '',
    tenant_id: raw.tenant_id || '',
    created_at: raw.created_at || '',
    execution_state: raw.execution_state
      ? typeof raw.execution_state === 'string'
        ? raw.execution_state
        : raw.execution_state.Finalized ? 'FINALIZED'
        : raw.execution_state.Failed ? 'FAILED'
        : 'ACTIVE'
      : '',
    input: raw.input
      ? {
          prompt: raw.input.prompt || undefined,
          params: raw.input.params || undefined,
          metadata: raw.input.metadata || undefined,
        }
      : undefined,
    output: raw.output
      ? {
          response: raw.output.response || undefined,
          truncated: !!raw.output.truncated,
          finish_reason: raw.output.finish_reason || undefined,
        }
      : undefined,
    proofs: raw.proofs
      ? {
          proof_chain: (raw.proofs.proof_chain || []).map((p: any) => ({
            level: p.level || '',
            proof_type: p.type || p.proof_type || 'sha256',
            signature: p.signature || undefined,
            timestamp: p.timestamp || undefined,
            digest: p.signature || undefined,
          })),
          aggregated_root: raw.proofs.aggregated_root || undefined,
        }
      : undefined,
    constraints_applied: raw.constraints_applied || undefined,
    observations: transformObservations(raw.observations),
    supply_chain: raw.supply_chain || undefined,
    engine_determinism: raw.engine_determinism || undefined,
  };
}

/** 将后端 Observations 转换为前端格式 */
function transformObservations(raw: any): Observations | undefined {
  if (!raw) return undefined;

  const obs: Observations = {};

  // 字段名映射: backend → frontend
  const fieldMap: Record<string, string> = {
    tokens_count: 'token_count',
    cost_estimated_usd: 'cost_usd',
    latency_ms: 'latency_ms',
  };

  // 映射顶层字段
  for (const [backendKey, frontendKey] of Object.entries(fieldMap)) {
    if (raw[backendKey] !== undefined && raw[backendKey] !== null) {
      (obs as any)[frontendKey] = raw[backendKey];
    }
  }

  // 如果有 usage 信息，提取 prompt_tokens 和 completion_tokens
  // (后端从 response.usage 提取，但存在 observations 中时只有 tokens_count)
  // 尝试从 output.response.usage 获取更细粒度的 token 信息

  // 转换 state_transitions 为 events
  if (raw.state_transitions && Array.isArray(raw.state_transitions)) {
    obs.events = raw.state_transitions.map((st: any, idx: number) => {
      const fromStr = typeof st.from === 'string' ? st.from : (st.from ? Object.keys(st.from)[0] : '');
      const toStr = typeof st.to === 'string' ? st.to : (st.to ? Object.keys(st.to)[0] : '');
      return {
        timestamp: st.timestamp || new Date().toISOString(),
        event_type: `${fromStr}_→_${toStr}`,
        details: { transition_index: st.transition_index || idx },
      };
    });
  }

  // 预算使用率 (从 constraints_applied 计算)
  // (由调用方在获得完整 trace 后自行填充)

  // safety_events → logs
  if (raw.safety_events && Array.isArray(raw.safety_events)) {
    obs.events = obs.events || [];
    for (const se of raw.safety_events) {
      obs.events.push({
        timestamp: se.timestamp || new Date().toISOString(),
        event_type: `safety:${se.trigger_type || 'unknown'}`,
        details: {
          severity: se.severity,
          action: se.action_taken,
          asi_risk_id: se.asi_risk_id,
        },
      });
    }
  }

  return obs;
}

/** 将后端 Trace 数组转换为前端 TraceSummary 数组 */
export function transformTraceList(rawList: any[]): TraceSummary[] {
  return (rawList || []).map(transformTraceSummary);
}
