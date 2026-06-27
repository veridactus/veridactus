// Developer Hub — VERIDACTUS 治理流水线 Playground
import { useState, useEffect, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Play, Zap, Activity, DollarSign, ShieldCheck, ChevronDown, Fingerprint, RefreshCw } from 'lucide-react';
import XRayPanel from './XRayPanel';
import { getModels } from '../../api';
import { getStoredToken } from '../../auth/useAuth';
import { getWorkspaceId } from '../../auth/AuthGuard';
import type { ModelInfo } from '../../types';

// DP 治理模式 API Key
const DP_API_KEY = 'veridactus-admin-dev-2026';

/** 流水线选项 */
interface PipelineOption { plan_id: string; name: string; stages: number; status: string; }

/** 从 CP 获取已发布流水线 */
async function fetchPipelines(token: string): Promise<PipelineOption[]> {
  if (!token) return [];
  try {
    const r = await fetch('/api/v1/pipelines', { headers: { Authorization: `Bearer ${token}` } });
    if (!r.ok) return [];
    const d = await r.json();
    return (d.pipelines || [])
      .filter((p: any) => p.status === 'published' || p.status === 'active')
      .map((p: any) => ({ plan_id: p.plan_id, name: p.name || p.plan_id?.slice(0, 8) || 'Unnamed', stages: p.stages?.length || 0, status: p.status }));
  } catch { return []; }
}

export default function PlaygroundPage() {
  const [prompt, setPrompt] = useState('');
  const [response, setResponse] = useState('');
  const [model, setModel] = useState('');
  const [streaming, setStreaming] = useState(false);
  const [tokens, setTokens] = useState(0);
  const [cost, setCost] = useState(0);
  const [latency, setLatency] = useState(0);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [modelsLoading, setModelsLoading] = useState(true);
  const [error, setError] = useState('');

  // 流水线选择器
  const [pipelines, setPipelines] = useState<PipelineOption[]>([]);
  const [selPipeline, setSelPipeline] = useState<PipelineOption | null>(null);
  const [showPipelines, setShowPipelines] = useState(false);

  const [traceId, setTraceId] = useState('');
  const token = getStoredToken() || '';

  // 从数据面获取可用模型列表，并轮询重试
  const loadModels = useCallback(() => {
    setModelsLoading(true);
    getModels()
      .then(list => {
        setModels(list);
        if (list.length > 0 && !model) {
          const dm = list.find(m => m.is_default) || list[0];
          setModel(dm.id);
        }
      })
      .catch(() => setModels([]))
      .finally(() => setModelsLoading(false));
  }, []); // eslint-disable-line

  useEffect(() => { loadModels(); fetchPipelines(token).then(ps => { setPipelines(ps); if (ps.length > 0) setSelPipeline(ps[0]); }); }, [token]);

  const handleSend = useCallback(async () => {
    if (!prompt.trim() || !model) return;
    setStreaming(true); setResponse(''); setTokens(0); setCost(0); setError(''); setTraceId('');
    const start = performance.now();

    // VERIDACTUS 治理协议头
    const govHeaders: Record<string, string> = {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${DP_API_KEY}`,
      'VERIDACTUS-Version': '0.2',
      'VERIDACTUS-Budget-Limit': '0.50',
      'VERIDACTUS-Privacy-Level': 'standard',
      'VERIDACTUS-Workspace-Id': getWorkspaceId(), // 多租户隔离
    };
    if (selPipeline) govHeaders['VERIDACTUS-Pipeline-Id'] = selPipeline.plan_id;

    const ctrl = new AbortController();
    try {
      const res = await fetch('/v1/chat/completions', {
        method: 'POST', signal: ctrl.signal, headers: govHeaders,
        body: JSON.stringify({ model, messages: [{ role: 'user', content: prompt }], stream: true, max_tokens: 4096 }),
      });
      if (!res.ok) {
        const eb = await res.json().catch(() => ({}));
        throw new Error(eb?.error?.message || `HTTP ${res.status}`);
      }
      // 捕获 VERIDACTUS 响应头
      setTraceId(res.headers.get('VERIDACTUS-Trace-Id') || '');
      const costConsumed = parseFloat(res.headers.get('VERIDACTUS-Cost-Consumed') || '0');

      const reader = res.body?.getReader();
      if (!reader) { setStreaming(false); return; }
      const decoder = new TextDecoder();
      let content = '';
      let tokCount = 0;
      let finalUsage: { total_tokens?: number; completion_tokens?: number; prompt_tokens?: number } | null = null;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        const lines = decoder.decode(value, { stream: true }).split('\n').filter(l => l.startsWith('data: '));
        for (const line of lines) {
          const data = line.slice(6).trim();
          if (data === '[DONE]') break;
          try {
            const d = JSON.parse(data);
            const deltaContent = d.choices?.[0]?.delta?.content || '';
            content += deltaContent;
            tokCount++;
            setResponse(content); setTokens(tokCount);
            if (d.usage) { finalUsage = d.usage; setTokens(d.usage.total_tokens || d.usage.completion_tokens || tokCount); }
          } catch { /* skip malformed */ }
        }
      }

      const actualTokens = finalUsage?.total_tokens || tokCount;
      setTokens(actualTokens);
      setCost(costConsumed);
      setLatency(Math.round(performance.now() - start));
    } catch (e: any) {
      if (e.name === 'AbortError') return;
      setError(e.message || '请求失败，请检查服务状态');
    } finally {
      setStreaming(false);
    }
  }, [prompt, model, selPipeline]);

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="h-full flex gap-0">
      {/* Left: Prompt Editor */}
      <div className="w-[320px] flex-shrink-0 flex flex-col border-r border-white/[0.06]">
        <div className="py-3 px-4 border-b border-white/[0.06] flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Play size={16} className="text-purple-500" />
            <span className="font-bold text-white text-[13px]">VERIDACTUS Playground</span>
          </div>
          <button onClick={loadModels} title="刷新模型列表"
            className="p-1.5 rounded-lg hover:bg-white/[0.06] text-[#5a6a8a] hover:text-white transition-colors cursor-pointer border-none bg-transparent">
            <RefreshCw size={13} className={modelsLoading ? 'animate-spin' : ''} />
          </button>
        </div>

        {/* 流水线选择器 */}
        {pipelines.length > 0 && (
          <div className="mx-4 mt-3 relative">
            <button onClick={() => setShowPipelines(!showPipelines)}
              className="w-full flex items-center justify-between gap-1.5 h-9 px-3 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.06] text-white text-[11px] font-medium cursor-pointer transition-all">
              <div className="flex items-center gap-1.5">
                <ShieldCheck size={12} color="#00d4aa" />
                <span className="truncate">{selPipeline?.name || '默认流水线'}</span>
              </div>
              <ChevronDown size={10} />
            </button>
            {showPipelines && (
              <motion.div initial={{ opacity: 0, y: -4 }} animate={{ opacity: 1, y: 0 }}
                className="absolute top-full left-0 right-0 mt-1.5 bg-[#0f1326] border border-white/[0.08] rounded-xl p-1.5 z-50 shadow-[0_16px_48px_rgba(0,0,0,0.6)]">
                {pipelines.map(pl => (
                  <div key={pl.plan_id} onClick={() => { setSelPipeline(pl); setShowPipelines(false); }}
                    className={`flex items-center gap-2 py-2 px-3 rounded-lg cursor-pointer text-[11px] transition-all ${
                      pl.plan_id === selPipeline?.plan_id ? 'bg-[rgba(0,212,170,0.1)] text-white' : 'text-[#8892b0] hover:bg-white/[0.04] hover:text-white'}`}>
                    <span className="w-1.5 h-1.5 rounded-full" style={{ background: pl.status === 'published' ? '#6c5ce7' : '#00d4aa' }} />
                    <span className="flex-1 truncate">{pl.name}</span>
                    <span className="text-[10px] text-[#4a5568]">{pl.stages}阶段</span>
                  </div>
                ))}
              </motion.div>
            )}
          </div>
        )}

        {/* 模型选择器 */}
        <div className="mx-4 mt-3 relative">
          <select
            value={model}
            onChange={e => setModel(e.target.value)}
            disabled={modelsLoading}
            className="w-full py-2 px-3 rounded-lg text-[13px] text-[#e0e6f0] bg-white/5 border border-white/10 disabled:opacity-50 outline-none cursor-pointer"
          >
            {modelsLoading ? (
              <option value="">Loading models...</option>
            ) : models.length === 0 ? (
              <option value="">No models available</option>
            ) : (
              models.map(m => (
                <option key={m.id} value={m.id}>
                  {m.id}{m.owned_by ? ` (${m.owned_by})` : ''}{m.is_default ? ' [Default]' : ''}
                </option>
              ))
            )}
          </select>
        </div>

        {/* Prompt 输入框 */}
        <textarea
          value={prompt}
          onChange={e => setPrompt(e.target.value)}
          placeholder="Enter your prompt..."
          className="flex-1 mx-4 mb-4 mt-3 p-3.5 rounded-xl text-[13px] text-[#e0e6f0] resize-none outline-none font-mono leading-relaxed bg-white/[0.03] border border-white/[0.06]"
        />

        {/* Run 按钮 */}
        <button
          onClick={handleSend}
          disabled={streaming || !prompt.trim() || !model}
          className="mx-4 mb-4 py-3 rounded-xl border-none text-white font-bold disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer"
          style={{ background: streaming ? 'rgba(255,255,255,0.05)' : 'linear-gradient(135deg, #6c5ce7, #00d4aa)' }}
        >
          {streaming ? 'Streaming...' : '▶  Run'}
        </button>
      </div>

      {/* Center: Output */}
      <div className="flex-1 flex flex-col min-w-0">
        <div className="py-3 px-4 border-b border-white/[0.06] flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Zap size={16} className="text-emerald-400" />
            <span className="font-bold text-white text-[13px]">Output</span>
            {traceId && (
              <a href={`/vault/${traceId}`} target="_blank" rel="noreferrer"
                className="flex items-center gap-1 text-[10px] text-[#6c5ce7] no-underline ml-2 px-2 py-0.5 rounded-md bg-[rgba(108,92,231,0.1)]">
                <Fingerprint size={10} /> Trace
              </a>
            )}
          </div>
          <div className="flex gap-4 text-[11px] text-[#8892b0]">
            <span><Activity size={10} className="inline align-sub" /> {tokens} tokens</span>
            <span><DollarSign size={10} className="inline align-sub" /> ${cost.toFixed(6)}</span>
            <span>⏱ {latency}ms</span>
          </div>
        </div>
        <div className="flex-1 p-5 overflow-auto font-sans text-sm text-[#e0e6f0] leading-relaxed whitespace-pre-wrap">
          {error ? (
            <div className="text-[#ff7675] bg-[rgba(255,118,117,0.06)] border border-[rgba(255,118,117,0.15)] rounded-xl p-4 text-[13px]">
              ❌ {error}
            </div>
          ) : streaming ? (
            <span>{response}<span className="inline-block w-1.5 h-4 bg-[#6c5ce7] ml-0.5 animate-pulse align-middle" /></span>
          ) : response ? (
            response
          ) : (
            <span className="text-[#8892b0] italic">Output will appear here...</span>
          )}
        </div>
      </div>

      {/* Right: X-Ray Panel */}
      <div className="w-[320px] flex-shrink-0">
        <XRayPanel
          tokensUsed={tokens}
          costUsd={cost}
          budgetRemaining={selPipeline ? 0.5 - cost : 0}
          safetyScore={100}
        />
      </div>
    </motion.div>
  );
}
