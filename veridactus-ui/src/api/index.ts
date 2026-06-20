import type { TraceSummary, TraceDetail, Pipeline, PluginMeta, Policy, ModelInfo, ApiKey, ModelConfig, VerificationResult, ReplayResult, ReplayBranch, RealTimeMetrics } from '../types';
import { transformTraceList, transformTraceDetail } from './transform';

/// 用于控制面管理 API 的 Admin Key（从构建时环境变量或 localStorage 获取）
function getAdminKey(): string | null {
  // 优先从 URL 参数获取（开发环境）
  if (typeof window !== 'undefined') {
    const params = new URLSearchParams(window.location.search);
    const key = params.get('admin_key');
    if (key) {
      localStorage.setItem('veridactus_admin_key', key);
      return key;
    }
    // 回退到 localStorage
    return localStorage.getItem('veridactus_admin_key');
  }
  return null;
}

async function fetchJSON(url: string, isCpApi = false): Promise<any> {
  const headers: Record<string, string> = {};
  if (isCpApi) {
    const adminKey = getAdminKey();
    if (adminKey) headers['X-Admin-Key'] = adminKey;
  }
  const res = await fetch(url, { headers });
  if (!res.ok) throw new Error('HTTP ' + res.status);
  return res.json();
}

async function cpFetch(url: string, method: string, body?: any): Promise<any> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  const adminKey = getAdminKey();
  if (adminKey) headers['X-Admin-Key'] = adminKey;
  const res = await fetch(url, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error('HTTP ' + res.status);
  return res.json();
}

async function cpFetchNoBody(url: string, method: string): Promise<boolean> {
  const headers: Record<string, string> = {};
  const adminKey = getAdminKey();
  if (adminKey) headers['X-Admin-Key'] = adminKey;
  const res = await fetch(url, { method, headers });
  return res.ok;
}

export async function getDataPlaneHealth(): Promise<boolean> {
  try { const res = await fetch('/health'); return res.ok; } catch { return false; }
}

export async function getModels(): Promise<ModelInfo[]> {
  const data = await fetchJSON('/models');
  return data.data || [];
}

export async function getTracesFromDataPlane(): Promise<TraceSummary[]> {
  const data = await fetchJSON('/v1/traces');
  return transformTraceList(data.traces || []);
}

export async function getTraceDetail(traceId: string): Promise<TraceDetail> {
  const raw = await fetchJSON('/v1/traces/' + traceId);
  return transformTraceDetail(raw);
}

export async function getControlPlaneHealth(): Promise<boolean> {
  const d = await fetchJSON('/api/v1/health', true);
  return d.status === 'ok';
}

export async function getPipelines(): Promise<Pipeline[]> {
  const data = await fetchJSON('/api/v1/pipelines', true);
  return data.pipelines || [];
}

export async function getPipeline(id: string): Promise<Pipeline> {
  return await fetchJSON('/api/v1/pipelines/' + id, true);
}

export async function createPipeline(p: Partial<Pipeline>): Promise<Pipeline> {
  return cpFetch('/api/v1/pipelines', 'POST', p);
}

export async function updatePipeline(id: string, p: Partial<Pipeline>): Promise<Pipeline> {
  return cpFetch('/api/v1/pipelines/' + id, 'PUT', p);
}

export async function deletePipeline(id: string): Promise<boolean> {
  return cpFetchNoBody('/api/v1/pipelines/' + id, 'DELETE');
}

export async function getPlugins(): Promise<PluginMeta[]> {
  const data = await fetchJSON('/api/v1/plugins', true);
  return data.plugins || [];
}

export async function registerPlugin(p: Partial<PluginMeta>): Promise<PluginMeta> {
  return cpFetch('/api/v1/plugins', 'POST', p);
}

export async function getPolicies(): Promise<Policy[]> {
  const data = await fetchJSON('/api/v1/policies', true);
  return data.policies || [];
}

export async function getPythonWorkerHealth(): Promise<boolean> {
  try { const d = await fetchJSON('/pw/health'); return d.status === 'ok'; } catch { return false; }
}

export async function checkAllServices() {
  const [dp, cp, pw] = await Promise.all([
    getDataPlaneHealth(), getControlPlaneHealth(), getPythonWorkerHealth(),
  ]);
  return { dataPlane: dp, controlPlane: cp, pythonWorker: pw };
}

export async function sendChatCompletion(body: any, headers: Record<string, string> = {}) {
  const res = await fetch('/v1/chat/completions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...headers },
    body: JSON.stringify(body),
  });
  const data = await res.json();
  return {
    status: res.status,
    data,
    headers: {
      traceId: res.headers.get('VERIDACTUS-Trace-Id'),
      proofLevels: res.headers.get('VERIDACTUS-Proof-Levels'),
      costConsumed: res.headers.get('VERIDACTUS-Cost-Consumed'),
      version: res.headers.get('VERIDACTUS-Version'),
    },
  };
}

export async function getApiKeys(): Promise<ApiKey[]> {
  const data = await fetchJSON('/api/v1/apikeys', true);
  return data.keys || [];
}

export async function createApiKey(name: string, tenantId: string = 'default'): Promise<ApiKey> {
  return cpFetch('/api/v1/apikeys', 'POST', { name, tenant_id: tenantId });
}

export async function deleteApiKey(keyId: string): Promise<boolean> {
  return cpFetchNoBody('/api/v1/apikeys/' + keyId, 'DELETE');
}

export async function rotateApiKey(keyId: string): Promise<boolean> {
  return cpFetch('/api/v1/apikeys/' + keyId, 'PUT', { status: 'rotated' }).then(() => true);
}

export async function getModelsConfig(): Promise<ModelConfig[]> {
  const data = await fetchJSON('/api/v1/models', true);
  return data.models || [];
}

export async function createModel(config: Omit<ModelConfig, 'id'>): Promise<ModelConfig> {
  return cpFetch('/api/v1/models', 'POST', config);
}

export async function updateModel(id: string, config: Partial<ModelConfig>): Promise<ModelConfig> {
  return cpFetch('/api/v1/models/' + id, 'PUT', config);
}

export async function deleteModel(id: string): Promise<boolean> {
  return cpFetchNoBody('/api/v1/models/' + id, 'DELETE');
}

// ==================== 重放相关 API ====================

export async function replayTrace(traceId: string, mode: string = 'replay', branchPoint?: number, branchName?: string): Promise<ReplayResult> {
  const body: Record<string, any> = { mode };
  if (branchPoint !== undefined) body.branch_point = branchPoint;
  if (branchName) body.branch_name = branchName;
  
  const res = await fetch('/v1/traces/' + traceId + '/replay', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error('HTTP ' + res.status);
  return res.json();
}

export async function verifyTraceSignature(traceId: string): Promise<VerificationResult> {
  const res = await fetch('/v1/traces/' + traceId + '/verify', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
  });
  if (!res.ok) throw new Error('HTTP ' + res.status);
  return res.json();
}

export async function getReplayBranches(): Promise<{ branches: ReplayBranch[]; total: number }> {
  const data = await fetchJSON('/v1/replay/branches');
  return { branches: data.branches || [], total: data.total || 0 };
}

export async function createReplayBranch(name: string, parentId?: string): Promise<ReplayBranch> {
  const body: Record<string, any> = { name };
  if (parentId) body.parent_id = parentId;
  
  const res = await fetch('/v1/replay/branches', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error('HTTP ' + res.status);
  return res.json();
}

export async function getReplayBranch(branchId: string): Promise<ReplayBranch> {
  return await fetchJSON('/v1/replay/branches/' + branchId);
}

export async function deleteReplayBranch(branchId: string): Promise<boolean> {
  const res = await fetch('/v1/replay/branches/' + branchId, { method: 'DELETE' });
  return res.ok;
}

export async function mergeReplayBranch(sourceId: string, targetId: string): Promise<boolean> {
  const res = await fetch(`/v1/replay/branches/${sourceId}/merge/${targetId}`, { method: 'POST' });
  return res.ok;
}

// ==================== 批量操作 API ====================

export async function batchExportTraces(traceIds: string[]): Promise<{ operation: string; count: number; traces: any[]; exported_at: string }> {
  const res = await fetch('/v1/traces/batch', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ operation: 'export', trace_ids: traceIds }),
  });
  if (!res.ok) throw new Error('HTTP ' + res.status);
  return res.json();
}

export async function batchDeleteTraces(traceIds: string[]): Promise<{ operation: string; requested: number; deleted: number }> {
  const res = await fetch('/v1/traces/batch', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ operation: 'delete', trace_ids: traceIds }),
  });
  if (!res.ok) throw new Error('HTTP ' + res.status);
  return res.json();
}

// ==================== 实时指标 API ====================

export async function getRealtimeMetrics(): Promise<RealTimeMetrics> {
  const data = await fetchJSON('/v1/metrics/realtime');
  return data;
}