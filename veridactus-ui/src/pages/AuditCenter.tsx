// VERIDACTUS 审计指挥舱 — 企业级风险大盘 + Trace 全息视角 + 会话分组
import { useEffect, useState, useCallback } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import ProofLevelBadge from '../components/atoms/ProofLevelBadge';
import ExecutionContract from '../components/viz/ExecutionContract';
import ObservationsPanel from '../components/viz/ObservationsPanel';
import StateMachineTimeline from '../components/viz/StateMachineTimeline';
import { useI18n } from '../i18n';
import { ConfirmDialog } from '../components/ui/Dialog';
import { MetricCard, VerificationBadge } from './AuditComponents';
import {
  getTracesFromDataPlane, getTracesGroupedBySession, getTraceDetail, replayTrace, verifyTraceSignature,
  getReplayBranches, createReplayBranch, deleteReplayBranch,
  batchExportTraces, batchDeleteTraces, getRealtimeMetrics,
} from '../api';
import type { TraceSummary, TraceDetail, VerificationResult, ReplayResult, ReplayBranch, RealTimeMetrics, SessionGroup } from '../types';
import {
  Activity, Search, Shield, ChevronRight, ChevronDown, FileText, Lock, Zap, GitBranch,
  AlertTriangle, RefreshCw, Play, CheckCircle, XCircle, Trash2, Download,
  Plus, BarChart3, Monitor, MessageSquare, Clock,
} from 'lucide-react';

type PanelView = 'traces' | 'sessions' | 'branches' | 'metrics';

export default function AuditCenter() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [traces, setTraces] = useState<TraceSummary[]>([]);
  const [sessionGroups, setSessionGroups] = useState<SessionGroup[]>([]);
  const [selectedTrace, setSelectedTrace] = useState<TraceDetail | null>(null);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [verificationResult, setVerificationResult] = useState<VerificationResult | null>(null);
  const [replayResult, setReplayResult] = useState<ReplayResult | null>(null);
  const [branches, setBranches] = useState<ReplayBranch[]>([]);
  const [metrics, setMetrics] = useState<RealTimeMetrics | null>(null);
  const [selectedTraces, setSelectedTraces] = useState<string[]>([]);
  const [activePanel, setActivePanel] = useState<PanelView>('traces');
  const [expandedSessions, setExpandedSessions] = useState<Set<string>>(new Set());
  const [deleteBranchId, setDeleteBranchId] = useState<string | null>(null);
  const [deleteTracesCount, setDeleteTracesCount] = useState<number>(0);
  const [isVerifying, setIsVerifying] = useState(false);
  const [isReplaying, setIsReplaying] = useState(false);
  const [newBranchName, setNewBranchName] = useState('');
  const [branchError, setBranchError] = useState('');
  const [replayError, setReplayError] = useState<string | null>(null);

  const loadTraces = async () => {
    setLoading(true); setError(null);
    try {
      const [tracesList, sessions] = await Promise.all([
        getTracesFromDataPlane(),
        getTracesGroupedBySession(),
      ]);
      setTraces(tracesList);
      setSessionGroups(sessions);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load traces');
    } finally { setLoading(false); }
  };
  const loadBranches = async () => { try { setBranches((await getReplayBranches()).branches); } catch {} };
  const loadMetrics = async () => { try { setMetrics(await getRealtimeMetrics()); } catch {} };

  useEffect(() => { loadTraces(); loadBranches(); loadMetrics(); const iv = setInterval(loadMetrics, 5000); return () => clearInterval(iv); }, []);
  useEffect(() => {
    const tid = searchParams.get('trace');
    if (tid) getTraceDetail(tid).then(setSelectedTrace).catch(console.error);
    else setSelectedTrace(null);
  }, [searchParams]);

  const filtered = traces.filter(t =>
    t.trace_id?.toLowerCase().includes(search.toLowerCase()) ||
    t.model?.toLowerCase().includes(search.toLowerCase())
  );

  const filteredSessions = sessionGroups.filter(s =>
    s.session_id.toLowerCase().includes(search.toLowerCase()) ||
    s.traces.some(t => t.model?.toLowerCase().includes(search.toLowerCase()))
  );

  const toggleSession = (sid: string) => {
    setExpandedSessions(prev => {
      const next = new Set(prev);
      if (next.has(sid)) next.delete(sid); else next.add(sid);
      return next;
    });
  };

  const handleVerify = async () => {
    if (!selectedTrace) return;
    setIsVerifying(true);
    try { setVerificationResult(await verifyTraceSignature(selectedTrace.trace_id)); } catch (err) {
      setVerificationResult({ trace_id: selectedTrace.trace_id, l0_passed: false, overall_passed: false, error: err instanceof Error ? err.message : 'Verification failed' });
    } finally { setIsVerifying(false); }
  };
  const handleReplay = async (mode = 'replay') => {
    if (!selectedTrace) return;
    setIsReplaying(true); setReplayError(null);
    try { setReplayResult(await replayTrace(selectedTrace.trace_id, mode)); } catch (err) {
      setReplayError(err instanceof Error ? err.message : 'Replay failed'); setReplayResult(null);
    } finally { setIsReplaying(false); }
  };
  const handleCreateBranch = async () => {
    if (!newBranchName.trim()) { setBranchError('Branch name is required'); return; }
    try { await createReplayBranch(newBranchName.trim()); setNewBranchName(''); setBranchError(''); loadBranches(); }
    catch (err) { setBranchError(err instanceof Error ? err.message : 'Failed to create branch'); }
  };

  const handleExportSelected = async () => {
    if (!selectedTraces.length) return;
    try {
      const r = await batchExportTraces(selectedTraces);
      const blob = new Blob([JSON.stringify(r.traces, null, 2)], { type: 'application/json' });
      const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = `traces-${Date.now()}.json`; a.click(); URL.revokeObjectURL(a.href);
    } catch {}
  };
  const confirmDeleteBranch = async () => {
    if (!deleteBranchId) return;
    try { await deleteReplayBranch(deleteBranchId); loadBranches(); } catch {} finally { setDeleteBranchId(null); }
  };
  const confirmDeleteTraces = async () => {
    if (!selectedTraces.length) return;
    try { await batchDeleteTraces(selectedTraces); setSelectedTraces([]); loadTraces(); } catch {} finally { setDeleteTracesCount(0); }
  };

  const toggleSelectAll = () => setSelectedTraces(selectedTraces.length === filtered.length ? [] : filtered.map(t => t.trace_id));
  const toggleSelect = (id: string) => setSelectedTraces(p => p.includes(id) ? p.filter(x => x !== id) : [...p, id]);

  const panelButtonClass = (panel: PanelView) =>
    `flex-1 py-2 px-3 rounded-lg border text-xs font-medium flex items-center justify-center gap-1 cursor-pointer transition-colors ${
      activePanel === panel
        ? 'border-purple-500/30 bg-purple-500/20 text-[var(--text-primary)]'
        : 'border-white/[0.06] text-[var(--text-tertiary)] hover:border-white/[0.12]'
    }`;

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-[var(--text-primary)]">{t('audit.title')}</h1>
        <p className="text-sm text-[var(--text-secondary)] mt-1">{t('audit.subtitle')}</p>
      </div>

      {/* 批量操作栏 */}
      {selectedTraces.length > 0 && (
        <motion.div initial={{ opacity: 0, y: -10 }} animate={{ opacity: 1, y: 0 }}
          className="flex items-center gap-3 p-3 rounded-btn mb-4 bg-purple-500/10">
          <span className="text-sm text-[var(--text-secondary)]">Selected {selectedTraces.length} trace(s)</span>
          <button onClick={handleExportSelected}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-medium cursor-pointer bg-emerald-500/20 border-emerald-500/30 text-emerald-400">
            <Download size={14} /> Export
          </button>
          <button onClick={() => setDeleteTracesCount(selectedTraces.length)}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-medium cursor-pointer bg-red-500/20 border-red-500/30 text-red-400">
            <Trash2 size={14} /> Delete
          </button>
        </motion.div>
      )}

      {/* 主布局 */}
      <div className="flex gap-5 min-h-[400px]">
        {/* 左侧面板 */}
        <div className="w-[380px] flex-shrink-0 flex flex-col gap-3">
          <div className="relative">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-tertiary)]" />
            <input className="input-field !pl-[34px]" placeholder={t('audit.search')} value={search} onChange={e => setSearch(e.target.value)} />
          </div>
          {/* 面板切换按钮 */}
          <div className="flex gap-2">
            <button onClick={() => setActivePanel('traces')} className={panelButtonClass('traces')}>
              Traces
            </button>
            <button onClick={() => setActivePanel('sessions')} className={panelButtonClass('sessions')}>
              <MessageSquare size={14} /> Sessions
            </button>
            <button onClick={() => setActivePanel('branches')} className={panelButtonClass('branches')}>
              <GitBranch size={14} /> Branches
            </button>
            <button onClick={() => setActivePanel('metrics')} className={panelButtonClass('metrics')}>
              <BarChart3 size={14} /> Metrics
            </button>
          </div>

          <div className="flex-1 overflow-y-auto flex flex-col gap-2">
            {/* ====== Trace 列表视图 ====== */}
            {activePanel === 'traces' && <>
              {filtered.length > 0 && (
                <div className="flex items-center px-2">
                  <input type="checkbox" checked={selectedTraces.length === filtered.length && filtered.length > 0} onChange={toggleSelectAll} className="mr-2" />
                  <span className="text-[11px] text-[var(--text-tertiary)]">Select All</span>
                </div>
              )}
              {loading ? <div className="text-center py-10 text-sm text-[var(--text-tertiary)]">{t('app.loading')}</div>
              : error ? (
                <GlassCard className="text-center p-8">
                  <AlertTriangle size={32} className="mx-auto mb-3" style={{ color: '#ff7675' }} />
                  <p className="text-sm text-[#ff7675] mb-4">{error}</p>
                  <button onClick={loadTraces}
                    className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg border text-xs cursor-pointer bg-purple-500/20 border-purple-500/30 text-purple-300">
                    <RefreshCw size={14} /> Retry
                  </button>
                </GlassCard>
              ) : filtered.length === 0 ? (
                <GlassCard className="text-center p-8">
                  <Activity size={32} className="mx-auto mb-3 opacity-30" />
                  <p className="text-sm text-[var(--text-tertiary)]">{t('audit.no_traces')}</p>
                </GlassCard>
              ) : filtered.map(t => (
                <GlassCard key={t.trace_id} className="p-3.5 cursor-pointer relative"
                  style={{ borderColor: selectedTrace?.trace_id === t.trace_id ? 'rgba(108,92,231,0.5)' : undefined }}
                  onClick={() => { getTraceDetail(t.trace_id).then(setSelectedTrace); setSearchParams({ trace: t.trace_id }); }}>
                  <input type="checkbox" checked={selectedTraces.includes(t.trace_id)} onChange={e => { e.stopPropagation(); toggleSelect(t.trace_id); }}
                    className="absolute top-3.5 right-3.5" />
                  <div className="flex justify-between items-start">
                    <div>
                      <p className="text-[13px] font-semibold text-[var(--text-primary)]">{t.model || 'Unknown'}</p>
                      <p className="text-[11px] text-[var(--text-tertiary)] mt-0.5 font-mono">{t.trace_id?.slice(0, 12)}...</p>
                      {t.session_id && <p className="text-[10px] text-purple-400/60 mt-0.5 font-mono">Session: {t.session_id.slice(0, 8)}...</p>}
                    </div>
                    <ChevronRight size={14} className="text-[var(--text-tertiary)]" />
                  </div>
                  <div className="flex gap-1.5 mt-2 flex-wrap">
                    {t.proof_levels?.map(pl => <ProofLevelBadge key={pl} level={pl} size="small" />)}
                    <span className="text-[10px] text-[var(--text-tertiary)] ml-auto self-center">{t.created_at ? new Date(t.created_at).toLocaleString() : ''}</span>
                  </div>
                </GlassCard>
              ))}
            </>}

            {/* ====== 会话分组视图 ====== */}
            {activePanel === 'sessions' && <>
              {loading ? <div className="text-center py-10 text-sm text-[var(--text-tertiary)]">{t('app.loading')}</div>
              : filteredSessions.length === 0 ? (
                <GlassCard className="text-center p-8">
                  <MessageSquare size={32} className="mx-auto mb-3 opacity-30" />
                  <p className="text-sm text-[var(--text-tertiary)]">No session groups found</p>
                  <p className="text-[11px] text-[var(--text-tertiary)] mt-1">Traces from multi-turn conversations will appear here</p>
                </GlassCard>
              ) : filteredSessions.map(session => {
                const isExpanded = expandedSessions.has(session.session_id);
                const isUngrouped = session.session_id === 'ungrouped';
                const sessionLabel = isUngrouped
                  ? 'Independent Traces (no session)'
                  : `Session ${session.session_id.slice(0, 8)}...`;
                return (
                  <div key={session.session_id}>
                    <GlassCard
                      className={`p-3.5 cursor-pointer transition-colors ${isExpanded ? 'border-purple-500/30' : ''}`}
                      onClick={() => toggleSession(session.session_id)}>
                      <div className="flex items-center gap-3">
                        {isExpanded
                          ? <ChevronDown size={16} className="text-purple-400 flex-shrink-0" />
                          : <ChevronRight size={16} className="text-[var(--text-tertiary)] flex-shrink-0" />
                        }
                        <MessageSquare size={14} className={isUngrouped ? 'text-[var(--text-tertiary)]' : 'text-purple-400'} />
                        <div className="flex-1 min-w-0">
                          <p className="text-[13px] font-semibold text-[var(--text-primary)] truncate">
                            {sessionLabel}
                          </p>
                          <div className="flex items-center gap-3 mt-0.5">
                            <span className="text-[11px] text-purple-400/70">{session.trace_count} trace(s)</span>
                            {session.traces.length > 0 && (
                              <span className="text-[10px] text-[var(--text-tertiary)] flex items-center gap-1">
                                <Clock size={10} />
                                {session.traces[0].created_at ? new Date(session.traces[0].created_at).toLocaleDateString() : ''}
                              </span>
                            )}
                          </div>
                        </div>
                      </div>
                    </GlassCard>
                    {/* 展开的 Trace 列表 */}
                    <AnimatePresence>
                      {isExpanded && (
                        <motion.div
                          initial={{ opacity: 0, height: 0 }}
                          animate={{ opacity: 1, height: 'auto' }}
                          exit={{ opacity: 0, height: 0 }}
                          transition={{ duration: 0.2 }}
                          className="overflow-hidden">
                          <div className="ml-6 mt-1 flex flex-col gap-1 border-l-2 border-purple-500/20 pl-3">
                            {session.traces.map(t => (
                              <GlassCard key={t.trace_id} className="p-3 cursor-pointer"
                                style={{ borderColor: selectedTrace?.trace_id === t.trace_id ? 'rgba(108,92,231,0.5)' : undefined }}
                                onClick={() => { getTraceDetail(t.trace_id).then(setSelectedTrace); setSearchParams({ trace: t.trace_id }); }}>
                                <div className="flex justify-between items-start">
                                  <div>
                                    <p className="text-[12px] font-semibold text-[var(--text-primary)]">{t.model || 'Unknown'}</p>
                                    <p className="text-[10px] text-[var(--text-tertiary)] mt-0.5 font-mono">{t.trace_id?.slice(0, 12)}...</p>
                                  </div>
                                  <ChevronRight size={12} className="text-[var(--text-tertiary)]" />
                                </div>
                                <div className="flex gap-1.5 mt-1.5 flex-wrap">
                                  {t.proof_levels?.map(pl => <ProofLevelBadge key={pl} level={pl} size="small" />)}
                                  <span className="text-[9px] text-[var(--text-tertiary)] ml-auto">{t.created_at ? new Date(t.created_at).toLocaleTimeString() : ''}</span>
                                </div>
                              </GlassCard>
                            ))}
                          </div>
                        </motion.div>
                      )}
                    </AnimatePresence>
                  </div>
                );
              })}
            </>}

            {/* ====== 分支管理面板 ====== */}
            {activePanel === 'branches' && (
              <div className="flex flex-col gap-3">
                <div className="flex gap-2">
                  <input type="text" placeholder="New branch name" value={newBranchName} onChange={e => setNewBranchName(e.target.value)}
                    onKeyDown={e => e.key === 'Enter' && handleCreateBranch()}
                    className="flex-1 py-2 px-3 rounded-lg border text-xs text-[var(--text-primary)] border-purple-500/30 bg-black/20" />
                  <button onClick={handleCreateBranch}
                    className="py-2 px-3 rounded-lg border text-xs flex items-center gap-1 cursor-pointer bg-purple-500/20 border-purple-500/30 text-purple-300">
                    <Plus size={14} />
                  </button>
                </div>
                {branchError && <p className="text-red-400 text-[11px]">{branchError}</p>}
                {!branches.length ? (
                  <GlassCard className="text-center p-6">
                    <GitBranch size={32} className="mx-auto mb-2 opacity-30" />
                    <p className="text-xs text-[var(--text-tertiary)]">No branches yet</p>
                  </GlassCard>
                ) : branches.map(b => (
                  <GlassCard key={b.branch_id} className="p-3">
                    <div className="flex justify-between items-center">
                      <div>
                        <p className="text-[13px] font-semibold text-[var(--text-primary)]">{b.name}</p>
                        <p className="text-[10px] text-[var(--text-tertiary)] font-mono">{b.branch_id.slice(0, 8)}...</p>
                      </div>
                      <button onClick={() => setDeleteBranchId(b.branch_id)}
                        className="p-1 rounded text-red-400 cursor-pointer bg-red-500/10">
                        <Trash2 size={14} />
                      </button>
                    </div>
                    <div className="flex gap-3 mt-2 text-[10px] text-[var(--text-tertiary)]">
                      <span>Snapshots: {b.snapshot_count}</span><span>{new Date(b.created_at).toLocaleDateString()}</span>
                    </div>
                  </GlassCard>
                ))}
              </div>
            )}

            {/* ====== 实时指标面板 ====== */}
            {activePanel === 'metrics' && metrics && (
              <div className="flex flex-col gap-3">
                <div className="flex items-center gap-2">
                  <Monitor size={16} className="text-purple-300" />
                  <span className="text-[13px] font-semibold text-[var(--text-primary)]">Real-time Metrics</span>
                  <span className="text-[10px] text-[var(--text-tertiary)] ml-auto">{new Date(metrics.timestamp).toLocaleTimeString()}</span>
                </div>
                <GlassCard className="p-4">
                  <div className="flex flex-col gap-3">
                    <MetricCard label="Total Requests" value={metrics.requests_total.toLocaleString()} icon={Activity} color="#00d4aa" />
                    <MetricCard label="Avg Latency" value={metrics.average_latency_ms.toFixed(2) + 'ms'} icon={RefreshCw as any} color="#74b9ff" />
                    <MetricCard label="Constraint Violations" value={metrics.constraint_violations_total.toLocaleString()} icon={AlertTriangle} color="#ffeaa7" />
                    <MetricCard label="Guardrail Activations" value={metrics.guardrail_activations_total.toLocaleString()} icon={Shield} color="#fd79a8" />
                    <MetricCard label="ASI Risks Flagged" value={metrics.asi_risks_flagged_total.toLocaleString()} icon={XCircle} color="#ff7675" />
                  </div>
                </GlassCard>
              </div>
            )}
          </div>
        </div>

        {/* 右侧详情面板 */}
        <div className="flex-1 overflow-y-auto pr-2">
          <AnimatePresence mode="wait">
            {selectedTrace ? (
              <motion.div key={selectedTrace.trace_id} initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }}
                transition={{ duration: 0.25 }} className="flex flex-col gap-4">
                {/* 操作按钮 */}
                <motion.div initial={{ opacity: 0, y: -10 }} animate={{ opacity: 1, y: 0 }} className="flex gap-3 flex-wrap">
                  {[
                    ['Replay', 'replay', 'bg-purple-500/20 border-purple-500/30 text-purple-300', Play],
                    ['Record', 'record', 'bg-emerald-500/20 border-emerald-500/30 text-emerald-400', Zap],
                    ['Branch Replay', 'branch', 'bg-sky-500/20 border-sky-500/30 text-sky-400', GitBranch],
                  ].map(([label, mode, className, Icon]) => (
                    <button key={label as string} onClick={() => handleReplay(mode as string)} disabled={isReplaying}
                      className={`inline-flex items-center gap-1.5 px-5 py-2.5 rounded-btn border text-[13px] font-medium cursor-pointer disabled:cursor-not-allowed ${className}`}>
                      {isReplaying ? <RefreshCw size={16} className="animate-spin" /> : <Icon size={16} />}
                      {isReplaying ? 'Replaying...' : label as string}
                    </button>
                  ))}
                  <button onClick={handleVerify} disabled={isVerifying}
                    className="inline-flex items-center gap-1.5 px-5 py-2.5 rounded-btn border text-[13px] font-medium cursor-pointer disabled:cursor-not-allowed bg-pink-500/20 border-pink-500/30 text-pink-400">
                    {isVerifying ? <RefreshCw size={16} className="animate-spin" /> : <CheckCircle size={16} />}
                    {isVerifying ? 'Verifying...' : 'Verify Signature'}
                  </button>
                </motion.div>

                {/* 验证结果 */}
                {verificationResult && (
                  <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }}>
                    <GlassCard className="p-5" style={{ borderColor: verificationResult.overall_passed ? 'rgba(0,212,170,0.3)' : 'rgba(255,118,117,0.3)' }}>
                      <div className="flex items-center gap-3 mb-4">
                        {verificationResult.overall_passed ? <CheckCircle size={32} className="text-emerald-400" /> : <XCircle size={32} className="text-red-400" />}
                        <div>
                          <h3 className={`text-sm font-semibold ${verificationResult.overall_passed ? 'text-emerald-400' : 'text-red-400'}`}>
                            {verificationResult.overall_passed ? 'Signature Verified' : 'Verification Failed'}
                          </h3>
                          <p className="text-[11px] text-[var(--text-tertiary)] mt-0.5">Trace: {verificationResult.trace_id.slice(0, 12)}...</p>
                        </div>
                      </div>
                      {verificationResult.error && <p className="text-red-400 text-xs mb-3">{verificationResult.error}</p>}
                      <div className="grid grid-cols-4 gap-3">
                        {(['L0','L1','L2A','L2B'] as const).map(lv => (
                          <VerificationBadge key={lv} level={lv} passed={verificationResult[`${lv.toLowerCase()}_passed` as keyof typeof verificationResult] as boolean|undefined} />
                        ))}
                      </div>
                    </GlassCard>
                  </motion.div>
                )}

                {/* 重放错误 */}
                {replayError && (
                  <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }}>
                    <GlassCard className="p-4 border-red-500/30 bg-red-500/5">
                      <div className="flex items-center gap-3">
                        <AlertTriangle size={24} className="text-red-400" />
                        <div>
                          <h3 className="text-[13px] font-semibold text-red-400">Replay Failed</h3>
                          <p className="text-xs text-[var(--text-secondary)] mt-1">{replayError}</p>
                        </div>
                      </div>
                    </GlassCard>
                  </motion.div>
                )}

                {/* 重放结果 */}
                {replayResult && (
                  <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }}>
                    <GlassCard className="p-5">
                      <div className="flex items-center gap-3 mb-4">
                        <Play size={32} className="text-purple-300" />
                        <div>
                          <h3 className="text-sm font-semibold text-purple-300">Replay Completed</h3>
                          <p className="text-[11px] text-[var(--text-tertiary)] mt-0.5">Mode: {replayResult.mode} | Duration: {replayResult.duration_ms}ms | Cache: {replayResult.cache_hit ? 'Hit' : 'Miss'}</p>
                        </div>
                      </div>
                      <div className="p-3 rounded-lg bg-black/20">
                        <h4 className="text-xs font-semibold text-[var(--text-secondary)] mb-3">Determinism Check</h4>
                        <div className="grid grid-cols-3 gap-3">
                          {[
                            [replayResult.determinism.is_identical ? 'Identical' : 'Different', replayResult.determinism.is_identical ? 'text-emerald-400' : 'text-yellow-400', 'Output Match'],
                            [(replayResult.determinism.similarity_score * 100).toFixed(1) + '%', 'text-purple-300', 'Similarity'],
                            [replayResult.determinism.hash_match ? 'Match' : 'Mismatch', replayResult.determinism.hash_match ? 'text-emerald-400' : 'text-red-400', 'Hash Check'],
                          ].map(([v, color, l]) => (
                            <div key={l as string} className="text-center">
                              <p className={`text-2xl font-bold ${color}`}>{v as string}</p>
                              <p className="text-[10px] text-[var(--text-tertiary)] mt-1">{l as string}</p>
                            </div>
                          ))}
                        </div>
                      </div>
                    </GlassCard>
                  </motion.div>
                )}

                <ExecutionContract trace={selectedTrace} />
                <StateMachineTimeline currentState={selectedTrace.execution_state} />

                {/* Input/Output 双栏 */}
                <div className="grid grid-cols-2 gap-4">
                  {[
                    { label: t('audit.input'), color: 'text-emerald-400', data: selectedTrace.input?.prompt ? (Array.isArray(selectedTrace.input.prompt) ? selectedTrace.input.prompt.map((it: any, i: number) => (
                      <div key={i} className="p-2 rounded-md mb-2 bg-white/5">
                        <span className="text-[10px] mr-2 text-purple-300">{it.role || 'user'}:</span><span>{it.content}</span>
                      </div>
                    )) : <p className="mb-3">{selectedTrace.input.prompt}</p>) : undefined },
                    { label: t('audit.output'), color: 'text-purple-300', data: typeof selectedTrace.output?.response === 'string' ? <p>{selectedTrace.output.response}</p>
                      : selectedTrace.output?.response?.choices?.[0]?.message?.content ? <p>{selectedTrace.output.response.choices[0].message.content}</p> : undefined },
                  ].map(({ label, color, data }, i) => (
                    <motion.div key={label} initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.1 + i * 0.05 }}>
                      <GlassCard className="p-5">
                        <h3 className="text-[13px] font-semibold text-[var(--text-secondary)] mb-2.5 flex items-center gap-1.5">
                          <Zap size={14} className={color} /> {label}
                        </h3>
                        <div className="text-[11px] text-[var(--text-primary)] whitespace-pre-wrap max-h-[280px] overflow-y-auto p-3 rounded-lg bg-black/20">
                          {data || <pre>{JSON.stringify(i === 0 ? selectedTrace.input : selectedTrace.output, null, 2)}</pre>}
                        </div>
                      </GlassCard>
                    </motion.div>
                  ))}
                </div>

                <ObservationsPanel observations={selectedTrace.observations} />

                {/* Proof Chain */}
                <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.3 }}>
                  <GlassCard className="p-5">
                    <h3 className="text-[13px] font-semibold text-[var(--text-secondary)] mb-3 flex items-center gap-1.5">
                      <Lock size={14} className="text-purple-300" /> {t('audit.proof_chain')}
                    </h3>
                    {(selectedTrace.proofs?.proof_chain || []).length ? (
                      <div className="flex flex-col gap-2">
                        {(selectedTrace.proofs?.proof_chain || []).map((p, i) => (
                          <motion.div key={i} initial={{ opacity: 0, x: -10 }} animate={{ opacity: 1, x: 0 }}
                            transition={{ delay: 0.35 + i * 0.1 }}
                            className="p-3.5 rounded-btn flex justify-between items-center bg-purple-500/8 border border-purple-500/15">
                            <div className="flex items-center gap-3">
                              <ProofLevelBadge level={p.level} />
                              <div>
                                <span className="text-xs font-semibold text-[var(--text-primary)]">{p.proof_type}</span>
                                {p.digest && <p className="text-[10px] text-[var(--text-tertiary)] font-mono mt-0.5">Digest: {p.digest.slice(0, 16)}...</p>}
                              </div>
                            </div>
                            <div className="text-right">
                              <span className="text-[10px] text-[var(--text-tertiary)] font-mono block max-w-[250px] truncate">{p.signature?.slice(0, 32)}...</span>
                              <span className="text-[9px] text-[var(--text-tertiary)] mt-1">{p.signature ? `${p.signature.length} chars` : '-'}</span>
                            </div>
                          </motion.div>
                        ))}
                      </div>
                    ) : (
                      <div className="text-center py-6">
                        <Lock size={32} className="mx-auto mb-2 opacity-20" />
                        <p className="text-sm text-[var(--text-tertiary)]">{t('audit.no_proofs')}</p>
                      </div>
                    )}
                  </GlassCard>
                </motion.div>

                {/* Raw JSON */}
                <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.4 }}>
                  <GlassCard className="p-5">
                    <h3 className="text-[13px] font-semibold text-[var(--text-secondary)] mb-2.5 flex items-center gap-1.5">
                      <FileText size={14} /> {t('audit.raw_json')}
                    </h3>
                    <pre className="text-[11px] text-[var(--text-primary)] whitespace-pre-wrap max-h-[300px] overflow-y-auto p-3 rounded-lg bg-black/20">
                      {JSON.stringify(selectedTrace, null, 2)}
                    </pre>
                  </GlassCard>
                </motion.div>
              </motion.div>
            ) : (
              <GlassCard className="flex flex-col items-center justify-center h-full p-10">
                <Shield size={48} className="mb-4 opacity-20" />
                <h3 className="text-lg font-semibold text-[var(--text-primary)] mb-2">{t('audit.title')}</h3>
                <p className="text-sm text-[var(--text-secondary)] text-center">{t('audit.select_hint')}</p>
              </GlassCard>
            )}
          </AnimatePresence>
        </div>
      </div>

      <ConfirmDialog open={!!deleteBranchId} onClose={() => setDeleteBranchId(null)} onConfirm={confirmDeleteBranch}
        title="Delete Branch" message="Are you sure you want to delete this branch?" confirmText="Delete" danger />
      <ConfirmDialog open={deleteTracesCount > 0} onClose={() => setDeleteTracesCount(0)} onConfirm={confirmDeleteTraces}
        title="Batch Delete Traces" message={`Are you sure you want to delete ${deleteTracesCount} trace(s)?`} confirmText="Delete" danger />
    </motion.div>
  );
}