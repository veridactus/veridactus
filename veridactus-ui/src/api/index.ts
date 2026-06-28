import type { TraceSummary, TraceDetail, Pipeline, PluginMeta, Policy, ModelInfo, ApiKey, ModelConfig, VerificationResult, ReplayResult, ReplayBranch, RealTimeMetrics, SessionGroup } from '../types';
import { transformTraceList, transformTraceDetail } from './transform';

// ==================== 统一错误类型 ====================

/** API 错误 — 携带结构化错误信息，上游可直接用于 Toast 展示 */
export class ApiError extends Error {
  public status: number;
  public code: string;
  public detail: string;
  public traceId?: string;

  constructor(status: number, code: string, detail: string, traceId?: string) {
    super(`[${status}] ${code}: ${detail}`);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
    this.detail = detail;
    this.traceId = traceId;
  }

  /** 用户友好的错误消息 */
  get userMessage(): string {
    if (this.status === 401) return '登录已过期，请重新登录';
    if (this.status === 403) return '没有权限执行此操作';
    if (this.status === 429) return '请求过于频繁，请稍后重试';
    if (this.status >= 500) return '服务器错误，请稍后重试';
    return this.detail || `请求失败 (${this.status})`;
  }
}

// ==================== Admin Key ====================

/// 用于控制面管理 API 的 Admin Key（从 URL 参数或 localStorage 获取）
function getAdminKey(): string | null {
  if (typeof window !== 'undefined') {
    const params = new URLSearchParams(window.location.search);
    const key = params.get('admin_key');
    if (key) {
      localStorage.setItem('veridactus_admin_key', key);
      return key;
    }
    return localStorage.getItem('veridactus_admin_key');
  }
  return null;
}

/** 获取 JWT token（优先 localStorage，回退 cookie） */
function getJwtToken(): string | null {
  if (typeof window === 'undefined') return null;
  try { return localStorage.getItem('veridactus_token'); } catch { return null; }
}

// ==================== 内部请求工具 ====================

async function fetchJSON(url: string, isCpApi = false): Promise<any> {
  const headers: Record<string, string> = {};
  if (isCpApi) {
    const adminKey = getAdminKey();
    if (adminKey) { headers['X-Admin-Key'] = adminKey; }
    // 同时传 JWT token — 已登录用户通过 JWT 鉴权，无需 admin key
    const jwt = getJwtToken();
    if (jwt) { headers['Authorization'] = `Bearer ${jwt}`; }
  }
  const res = await fetch(url, { headers });
  if (!res.ok) {
    let code = 'http_error';
    let detail = res.statusText;
    try {
      const body = await res.json();
      if (body.error) code = body.error;
      if (body.message) detail = body.message;
    } catch {}
    throw new ApiError(res.status, code, detail);
  }
  return res.json();
}

async function cpFetch(url: string, method: string, body?: any): Promise<any> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  const adminKey = getAdminKey();
  if (adminKey) headers['X-Admin-Key'] = adminKey;
  // 同时传 JWT token — 已登录用户通过 JWT 鉴权，无需 admin key
  const jwt = getJwtToken();
  if (jwt) { headers['Authorization'] = `Bearer ${jwt}`; }
  const res = await fetch(url, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    let code = 'http_error';
    let detail = res.statusText;
    try {
      const body = await res.json();
      if (body.error) code = body.error;
      if (body.message) detail = body.message;
    } catch {}
    throw new ApiError(res.status, code, detail);
  }
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

/** 通过控制面 API 获取 traces（带 JWT 认证 + workspace 隔离） */
export async function getTracesFromCP(): Promise<TraceSummary[]> {
  const data = await fetchJSON('/api/v1/traces', true);
  return transformTraceList(data.traces || []);
}

export async function getTracesGroupedBySession(): Promise<SessionGroup[]> {
  const data = await fetchJSON('/v1/traces?group_by=session');
  return (data.sessions || []).map((s: any) => ({
    session_id: s.session_id,
    trace_count: s.trace_count,
    traces: transformTraceList(s.traces || []),
  }));
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

export async function publishPipeline(id: string): Promise<{ status: string; plan_id: string }> {
  return cpFetch('/api/v1/pipelines/' + id + '/publish', 'POST');
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

// ==================== 系统设置 API ====================

/** 获取系统设置（从 localStorage 和远程合并） */
export async function getSystemSettings(): Promise<Record<string, string>> {
  try {
    const data = await fetchJSON('/api/v1/settings', true);
    return data.settings || {};
  } catch {
    // 控制面不可用时返回空对象，前端用 localStorage
    return {};
  }
}

/** 同步系统设置到控制面 */
export async function updateSystemSettings(settings: Record<string, string>): Promise<boolean> {
  try {
    await cpFetch('/api/v1/settings', 'POST', { settings });
    return true;
  } catch {
    return false;
  }
}