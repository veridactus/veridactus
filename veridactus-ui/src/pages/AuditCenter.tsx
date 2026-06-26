import { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import ProofLevelBadge from '../components/atoms/ProofLevelBadge';
import ExecutionContract from '../components/viz/ExecutionContract';
import ObservationsPanel from '../components/viz/ObservationsPanel';
import StateMachineTimeline from '../components/viz/StateMachineTimeline';
import { useI18n } from '../i18n';
import { ConfirmDialog } from '../components/ui/Dialog';
import { 
  getTracesFromDataPlane, 
  getTraceDetail,
  replayTrace,
  verifyTraceSignature,
  getReplayBranches,
  createReplayBranch,
  deleteReplayBranch,
  batchExportTraces,
  batchDeleteTraces,
  getRealtimeMetrics,
} from '../api';
import type { TraceSummary, TraceDetail, VerificationResult, ReplayResult, ReplayBranch, RealTimeMetrics } from '../types';
import { 
  Activity, Search, Shield, Clock, ChevronRight, Hash, FileText, Settings, Lock, Zap, 
  AlertTriangle, RefreshCw, Play, CheckCircle, XCircle, GitBranch, Trash2, Download, 
  Plus, BarChart3, Monitor, Check, AlertCircle
} from 'lucide-react';

export default function AuditCenter() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [traces, setTraces] = useState<TraceSummary[]>([]);
  const [selectedTrace, setSelectedTrace] = useState<TraceDetail | null>(null);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  // 新状态
  const [verificationResult, setVerificationResult] = useState<VerificationResult | null>(null);
  const [replayResult, setReplayResult] = useState<ReplayResult | null>(null);
  const [branches, setBranches] = useState<ReplayBranch[]>([]);
  const [metrics, setMetrics] = useState<RealTimeMetrics | null>(null);
  const [selectedTraces, setSelectedTraces] = useState<string[]>([]);
  const [showBranchPanel, setShowBranchPanel] = useState(false);
  const [showMetricsPanel, setShowMetricsPanel] = useState(false);
  const [deleteBranchId, setDeleteBranchId] = useState<string | null>(null);
  const [deleteTracesCount, setDeleteTracesCount] = useState<number>(0);
  const [isVerifying, setIsVerifying] = useState(false);
  const [isReplaying, setIsReplaying] = useState(false);
  const [newBranchName, setNewBranchName] = useState('');
  const [branchError, setBranchError] = useState('');
  const [replayError, setReplayError] = useState<string | null>(null);

  const loadTraces = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await getTracesFromDataPlane();
      setTraces(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load traces';
      setError(message);
      console.error('Failed to load traces:', err);
    } finally {
      setLoading(false);
    }
  };

  const loadBranches = async () => {
    try {
      const data = await getReplayBranches();
      setBranches(data.branches);
    } catch (err) {
      console.error('Failed to load branches:', err);
    }
  };

  const loadMetrics = async () => {
    try {
      const data = await getRealtimeMetrics();
      setMetrics(data);
    } catch (err) {
      console.error('Failed to load metrics:', err);
    }
  };

  useEffect(() => {
    loadTraces();
    loadBranches();
    loadMetrics();
    
    const interval = setInterval(loadMetrics, 5000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    const traceId = searchParams.get('trace');
    if (traceId) {
      getTraceDetail(traceId)
        .then(setSelectedTrace)
        .catch(err => console.error('Failed to load trace detail:', err));
    } else {
      setSelectedTrace(null);
    }
  }, [searchParams]);

  const filtered = traces.filter(t =>
    t.trace_id?.toLowerCase().includes(search.toLowerCase()) ||
    t.model?.toLowerCase().includes(search.toLowerCase())
  );

  const handleVerify = async () => {
    if (!selectedTrace) return;
    setIsVerifying(true);
    try {
      const result = await verifyTraceSignature(selectedTrace.trace_id);
      setVerificationResult(result);
    } catch (err) {
      console.error('Verification failed:', err);
      setVerificationResult({
        trace_id: selectedTrace.trace_id,
        l0_passed: false,
        overall_passed: false,
        error: err instanceof Error ? err.message : 'Verification failed'
      });
    } finally {
      setIsVerifying(false);
    }
  };

  const handleReplay = async (mode: string = 'replay') => {
    if (!selectedTrace) return;
    setIsReplaying(true);
    setReplayError(null);
    try {
      const result = await replayTrace(selectedTrace.trace_id, mode);
      setReplayResult(result);
      setReplayError(null);
    } catch (err) {
      console.error('Replay failed:', err);
      setReplayError(err instanceof Error ? err.message : 'Failed to replay trace');
      setReplayResult(null);
    } finally {
      setIsReplaying(false);
    }
  };

  const handleCreateBranch = async () => {
    if (!newBranchName.trim()) {
      setBranchError('Branch name is required');
      return;
    }
    try {
      await createReplayBranch(newBranchName.trim());
      setNewBranchName('');
      setBranchError('');
      loadBranches();
    } catch (err) {
      setBranchError(err instanceof Error ? err.message : 'Failed to create branch');
    }
  };

  const handleDeleteBranch = (branchId: string) => { setDeleteBranchId(branchId); };

  const handleExportSelected = async () => {
    if (selectedTraces.length === 0) return;
    try {
      const result = await batchExportTraces(selectedTraces);
      const blob = new Blob([JSON.stringify(result.traces, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `traces-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Export failed:', err);
    }
  };

  const handleDeleteSelected = () => {
    if (selectedTraces.length === 0) return;
    setDeleteTracesCount(selectedTraces.length);
  };

  const confirmDeleteBranch = async () => {
    if (!deleteBranchId) return;
    try {
      await deleteReplayBranch(deleteBranchId);
      loadBranches();
    } catch (err) {
      console.error('Failed to delete branch:', err);
    } finally {
      setDeleteBranchId(null);
    }
  };

  const confirmDeleteTraces = async () => {
    if (selectedTraces.length === 0) return;
    try {
      await batchDeleteTraces(selectedTraces);
      setSelectedTraces([]);
      loadTraces();
    } catch (err) {
      console.error('Delete failed:', err);
    } finally {
      setDeleteTracesCount(0);
    }
  };

  const toggleSelectAll = () => {
    if (selectedTraces.length === filtered.length) {
      setSelectedTraces([]);
    } else {
      setSelectedTraces(filtered.map(t => t.trace_id));
    }
  };

  const toggleSelect = (traceId: string) => {
    setSelectedTraces(prev => 
      prev.includes(traceId) 
        ? prev.filter(id => id !== traceId)
        : [...prev, traceId]
    );
  };

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div style={{ marginBottom: 24 }}>
        <h1 style={{ fontSize: 24, fontWeight: 700, color: 'var(--text-primary)' }}>{t('audit.title')}</h1>
        <p style={{ color: 'var(--text-secondary)', fontSize: 13, marginTop: 4 }}>{t('audit.subtitle')}</p>
      </div>

      {/* 批量操作栏 */}
      {selectedTraces.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          style={{
            display: 'flex',
            gap: 12,
            padding: 12,
            background: 'rgba(108,92,231,0.1)',
            borderRadius: 10,
            marginBottom: 16,
          }}
        >
          <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
            Selected {selectedTraces.length} trace(s)
          </span>
          <button
            onClick={handleExportSelected}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 12px',
              background: 'rgba(0,212,170,0.2)',
              border: '1px solid rgba(0,212,170,0.3)',
              borderRadius: 8,
              color: '#00d4aa',
              cursor: 'pointer',
              fontSize: 12,
            }}
          >
            <Download size={14} /> Export
          </button>
          <button
            onClick={handleDeleteSelected}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 12px',
              background: 'rgba(255,118,117,0.2)',
              border: '1px solid rgba(255,118,117,0.3)',
              borderRadius: 8,
              color: '#ff7675',
              cursor: 'pointer',
              fontSize: 12,
            }}
          >
            <Trash2 size={14} /> Delete
          </button>
        </motion.div>
      )}

      <div style={{ display: 'flex', gap: 20, height: '100%', minHeight: 400 }}>
        {/* 左侧面板 - Trace列表和分支管理 */}
        <div style={{ width: 380, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 12 }}>
          <div style={{ position: 'relative' }}>
            <Search size={14} style={{ position: 'absolute', left: 12, top: '50%', transform: 'translateY(-50%)', color: 'var(--text-tertiary)' }} />
            <input className="input-field" placeholder={t('audit.search')} value={search} onChange={e => setSearch(e.target.value)} style={{ paddingLeft: 34 }} />
          </div>

          {/* 切换面板按钮 */}
          <div style={{ display: 'flex', gap: 8 }}>
            <button
              onClick={() => { setShowBranchPanel(false); setShowMetricsPanel(false); }}
              style={{
                flex: 1,
                padding: '8px 12px',
                borderRadius: 8,
                border: '1px solid rgba(108,92,231,0.3)',
                background: !showBranchPanel && !showMetricsPanel ? 'rgba(108,92,231,0.2)' : 'transparent',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                fontSize: 12,
              }}
            >
              Traces ({filtered.length})
            </button>
            <button
              onClick={() => { setShowBranchPanel(!showBranchPanel); setShowMetricsPanel(false); }}
              style={{
                flex: 1,
                padding: '8px 12px',
                borderRadius: 8,
                border: '1px solid rgba(108,92,231,0.3)',
                background: showBranchPanel ? 'rgba(108,92,231,0.2)' : 'transparent',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                fontSize: 12,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 4,
              }}
            >
              <GitBranch size={14} /> Branches
            </button>
            <button
              onClick={() => { setShowMetricsPanel(!showMetricsPanel); setShowBranchPanel(false); }}
              style={{
                flex: 1,
                padding: '8px 12px',
                borderRadius: 8,
                border: '1px solid rgba(108,92,231,0.3)',
                background: showMetricsPanel ? 'rgba(108,92,231,0.2)' : 'transparent',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                fontSize: 12,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 4,
              }}
            >
              <BarChart3 size={14} /> Metrics
            </button>
          </div>

          <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 8 }}>
            {/* Trace列表 */}
            {!showBranchPanel && !showMetricsPanel && (
              <>
                {/* 全选 checkbox */}
                {filtered.length > 0 && (
                  <div style={{ display: 'flex', alignItems: 'center', padding: '0 8px' }}>
                    <input
                      type="checkbox"
                      checked={selectedTraces.length === filtered.length && filtered.length > 0}
                      onChange={toggleSelectAll}
                      style={{ marginRight: 8 }}
                    />
                    <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>Select All</span>
                  </div>
                )}

                {loading ? (
                  <div style={{ textAlign: 'center', padding: 40, color: 'var(--text-tertiary)', fontSize: 13 }}>{t('app.loading')}</div>
                ) : error ? (
                  <GlassCard style={{ textAlign: 'center', padding: 32 }}>
                    <AlertTriangle size={32} style={{ color: '#ff7675', margin: '0 auto 12px' }} />
                    <p style={{ color: '#ff7675', fontSize: 13, marginBottom: 16 }}>{error}</p>
                    <button
                      onClick={loadTraces}
                      style={{
                        display: 'inline-flex',
                        alignItems: 'center',
                        gap: 6,
                        padding: '8px 16px',
                        background: 'rgba(108,92,231,0.2)',
                        border: '1px solid rgba(108,92,231,0.3)',
                        borderRadius: 8,
                        color: '#a29bfe',
                        cursor: 'pointer',
                        fontSize: 12,
                      }}
                    >
                      <RefreshCw size={14} /> Retry
                    </button>
                  </GlassCard>
                ) : filtered.length === 0 ? (
                  <GlassCard style={{ textAlign: 'center', padding: 32 }}>
                    <Activity size={32} style={{ opacity: 0.3, margin: '0 auto 12px' }} />
                    <p style={{ color: 'var(--text-tertiary)', fontSize: 13 }}>{t('audit.no_traces')}</p>
                  </GlassCard>
                ) : (
                  filtered.map((trace, i) => (
                    <GlassCard
                      key={trace.trace_id}
                      style={{ 
                        padding: 14, 
                        cursor: 'pointer', 
                        borderColor: selectedTrace?.trace_id === trace.trace_id ? 'rgba(108,92,231,0.5)' : undefined,
                        position: 'relative',
                      }}
                      onClick={() => { 
                        getTraceDetail(trace.trace_id).then(setSelectedTrace); 
                        setSearchParams({ trace: trace.trace_id });
                      }}
                    >
                      <input
                        type="checkbox"
                        checked={selectedTraces.includes(trace.trace_id)}
                        onChange={(e) => { e.stopPropagation(); toggleSelect(trace.trace_id); }}
                        style={{ position: 'absolute', top: 14, right: 14 }}
                      />
                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                        <div>
                          <p style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>{trace.model || t('app.unknown')}</p>
                          <p style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 2, fontFamily: "'JetBrains Mono', monospace" }}>{trace.trace_id?.slice(0, 12)}...</p>
                        </div>
                        <ChevronRight size={14} color="var(--text-tertiary)" />
                      </div>
                      <div style={{ display: 'flex', gap: 6, marginTop: 8, flexWrap: 'wrap' }}>
                        {trace.proof_levels?.map(pl => <ProofLevelBadge key={pl} level={pl} size="small" />)}
                        <span style={{ fontSize: 10, color: 'var(--text-tertiary)', marginLeft: 'auto', alignSelf: 'center' }}>
                          {trace.created_at ? new Date(trace.created_at).toLocaleString() : ''}
                        </span>
                      </div>
                    </GlassCard>
                  ))
                )}
              </>
            )}

            {/* 分支管理面板 */}
            {showBranchPanel && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                <div style={{ display: 'flex', gap: 8 }}>
                  <input
                    type="text"
                    placeholder="New branch name"
                    value={newBranchName}
                    onChange={(e) => setNewBranchName(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleCreateBranch()}
                    style={{
                      flex: 1,
                      padding: '8px 12px',
                      borderRadius: 8,
                      border: '1px solid rgba(108,92,231,0.3)',
                      background: 'rgba(0,0,0,0.2)',
                      color: 'var(--text-primary)',
                      fontSize: 12,
                    }}
                  />
                  <button
                    onClick={handleCreateBranch}
                    style={{
                      padding: '8px 12px',
                      borderRadius: 8,
                      background: 'rgba(108,92,231,0.2)',
                      border: '1px solid rgba(108,92,231,0.3)',
                      color: '#a29bfe',
                      cursor: 'pointer',
                      fontSize: 12,
                      display: 'flex',
                      alignItems: 'center',
                      gap: 4,
                    }}
                  >
                    <Plus size={14} />
                  </button>
                </div>
                {branchError && (
                  <p style={{ color: '#ff7675', fontSize: 11 }}>{branchError}</p>
                )}
                {branches.length === 0 ? (
                  <GlassCard style={{ textAlign: 'center', padding: 24 }}>
                    <GitBranch size={32} style={{ opacity: 0.3, margin: '0 auto 8px' }} />
                    <p style={{ color: 'var(--text-tertiary)', fontSize: 12 }}>No branches yet</p>
                  </GlassCard>
                ) : (
                  branches.map(branch => (
                    <GlassCard key={branch.branch_id} style={{ padding: 12 }}>
                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div>
                          <p style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>{branch.name}</p>
                          <p style={{ fontSize: 10, color: 'var(--text-tertiary)', fontFamily: "'JetBrains Mono', monospace" }}>
                            {branch.branch_id.slice(0, 8)}...
                          </p>
                        </div>
                        <button
                          onClick={() => handleDeleteBranch(branch.branch_id)}
                          style={{
                            padding: 4,
                            borderRadius: 4,
                            background: 'rgba(255,118,117,0.1)',
                            border: 'none',
                            color: '#ff7675',
                            cursor: 'pointer',
                          }}
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                      <div style={{ display: 'flex', gap: 12, marginTop: 8, fontSize: 10, color: 'var(--text-tertiary)' }}>
                        <span>Snapshots: {branch.snapshot_count}</span>
                        <span>{new Date(branch.created_at).toLocaleDateString()}</span>
                      </div>
                    </GlassCard>
                  ))
                )}
              </div>
            )}

            {/* 实时指标面板 */}
            {showMetricsPanel && metrics && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Monitor size={16} style={{ color: '#a29bfe' }} />
                  <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>Real-time Metrics</span>
                  <span style={{ fontSize: 10, color: 'var(--text-tertiary)', marginLeft: 'auto' }}>
                    {new Date(metrics.timestamp).toLocaleTimeString()}
                  </span>
                </div>
                <GlassCard style={{ padding: 16 }}>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                    <MetricCard label="Total Requests" value={metrics.requests_total.toLocaleString()} icon={Activity} color="#00d4aa" />
                    <MetricCard label="Avg Latency" value={metrics.average_latency_ms.toFixed(2) + 'ms'} icon={Clock} color="#74b9ff" />
                    <MetricCard label="Constraint Violations" value={metrics.constraint_violations_total.toLocaleString()} icon={AlertCircle} color="#ffeaa7" />
                    <MetricCard label="Guardrail Activations" value={metrics.guardrail_activations_total.toLocaleString()} icon={Shield} color="#fd79a8" />
                    <MetricCard label="ASI Risks Flagged" value={metrics.asi_risks_flagged_total.toLocaleString()} icon={XCircle} color="#ff7675" />
                  </div>
                </GlassCard>
              </div>
            )}
          </div>
        </div>

        {/* 右侧面板 - Trace详情 */}
        <div style={{ flex: 1, overflowY: 'auto', paddingRight: 8 }}>
          <AnimatePresence mode="wait">
            {selectedTrace ? (
              <motion.div
                key={selectedTrace.trace_id}
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                transition={{ duration: 0.25 }}
                style={{ display: 'flex', flexDirection: 'column', gap: 16 }}
              >
                {/* 操作按钮栏 */}
                <motion.div
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  style={{ display: 'flex', gap: 12 }}
                >
                  <button
                    onClick={() => handleReplay('replay')}
                    disabled={isReplaying}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 6,
                      padding: '10px 20px',
                      background: 'rgba(108,92,231,0.2)',
                      border: '1px solid rgba(108,92,231,0.3)',
                      borderRadius: 10,
                      color: '#a29bfe',
                      cursor: isReplaying ? 'not-allowed' : 'pointer',
                      fontSize: 13,
                      fontWeight: 500,
                    }}
                  >
                    {isReplaying ? <RefreshCw size={16} style={{ animation: 'spin 1s linear infinite' }} /> : <Play size={16} />}
                    {isReplaying ? 'Replaying...' : 'Replay'}
                  </button>
                  <button
                    onClick={() => handleReplay('record')}
                    disabled={isReplaying}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 6,
                      padding: '10px 20px',
                      background: 'rgba(0,212,170,0.2)',
                      border: '1px solid rgba(0,212,170,0.3)',
                      borderRadius: 10,
                      color: '#00d4aa',
                      cursor: isReplaying ? 'not-allowed' : 'pointer',
                      fontSize: 13,
                      fontWeight: 500,
                    }}
                  >
                    <Zap size={16} /> Record
                  </button>
                  <button
                    onClick={() => handleReplay('branch')}
                    disabled={isReplaying}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 6,
                      padding: '10px 20px',
                      background: 'rgba(116,185,255,0.2)',
                      border: '1px solid rgba(116,185,255,0.3)',
                      borderRadius: 10,
                      color: '#74b9ff',
                      cursor: isReplaying ? 'not-allowed' : 'pointer',
                      fontSize: 13,
                      fontWeight: 500,
                    }}
                  >
                    <GitBranch size={16} /> Branch Replay
                  </button>
                  <button
                    onClick={handleVerify}
                    disabled={isVerifying}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 6,
                      padding: '10px 20px',
                      background: 'rgba(253,121,168,0.2)',
                      border: '1px solid rgba(253,121,168,0.3)',
                      borderRadius: 10,
                      color: '#fd79a8',
                      cursor: isVerifying ? 'not-allowed' : 'pointer',
                      fontSize: 13,
                      fontWeight: 500,
                    }}
                  >
                    {isVerifying ? <RefreshCw size={16} style={{ animation: 'spin 1s linear infinite' }} /> : <CheckCircle size={16} />}
                    {isVerifying ? 'Verifying...' : 'Verify Signature'}
                  </button>
                </motion.div>

                {/* 验证结果 */}
                {verificationResult && (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    style={{ marginTop: 8 }}
                  >
                    <GlassCard style={{ padding: 20, borderColor: verificationResult.overall_passed ? 'rgba(0,212,170,0.3)' : 'rgba(255,118,117,0.3)' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
                        {verificationResult.overall_passed ? (
                          <CheckCircle size={32} style={{ color: '#00d4aa' }} />
                        ) : (
                          <XCircle size={32} style={{ color: '#ff7675' }} />
                        )}
                        <div>
                          <h3 style={{ fontSize: 14, fontWeight: 600, color: verificationResult.overall_passed ? '#00d4aa' : '#ff7675' }}>
                            {verificationResult.overall_passed ? 'Signature Verified' : 'Verification Failed'}
                          </h3>
                          <p style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 2 }}>
                            Trace: {verificationResult.trace_id.slice(0, 12)}...
                          </p>
                        </div>
                      </div>
                      {verificationResult.error && (
                        <p style={{ color: '#ff7675', fontSize: 12, marginBottom: 12 }}>{verificationResult.error}</p>
                      )}
                      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 12 }}>
                        <VerificationBadge level="L0" passed={verificationResult.l0_passed} />
                        <VerificationBadge level="L1" passed={verificationResult.l1_passed} />
                        <VerificationBadge level="L2A" passed={verificationResult.l2a_passed} />
                        <VerificationBadge level="L2B" passed={verificationResult.l2b_passed} />
                      </div>
                    </GlassCard>
                  </motion.div>
                )}

                {/* 重放错误提示 */}
                {replayError && (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    style={{ marginTop: 8 }}
                  >
                    <GlassCard style={{ padding: 16, borderColor: 'rgba(255,118,117,0.3)', background: 'rgba(255,118,117,0.05)' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                        <AlertTriangle size={24} style={{ color: '#ff7675' }} />
                        <div>
                          <h3 style={{ fontSize: 13, fontWeight: 600, color: '#ff7675' }}>Replay Failed</h3>
                          <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 4 }}>{replayError}</p>
                        </div>
                      </div>
                    </GlassCard>
                  </motion.div>
                )}

                {/* 重放结果 */}
                {replayResult && (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    style={{ marginTop: 8 }}
                  >
                    <GlassCard style={{ padding: 20 }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
                        <Play size={32} style={{ color: '#a29bfe' }} />
                        <div>
                          <h3 style={{ fontSize: 14, fontWeight: 600, color: '#a29bfe' }}>Replay Completed</h3>
                          <p style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 2 }}>
                            Mode: {replayResult.mode} | Duration: {replayResult.duration_ms}ms | Cache: {replayResult.cache_hit ? 'Hit' : 'Miss'}
                          </p>
                        </div>
                      </div>
                      <div style={{ background: 'rgba(0,0,0,0.2)', padding: 12, borderRadius: 8 }}>
                        <h4 style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 12 }}>Determinism Check</h4>
                        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12 }}>
                          <div style={{ textAlign: 'center' }}>
                            <p style={{ fontSize: 24, fontWeight: 700, color: replayResult.determinism.is_identical ? '#00d4aa' : '#ffeaa7' }}>
                              {replayResult.determinism.is_identical ? 'Identical' : 'Different'}
                            </p>
                            <p style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 4 }}>Output Match</p>
                          </div>
                          <div style={{ textAlign: 'center' }}>
                            <p style={{ fontSize: 24, fontWeight: 700, color: '#a29bfe' }}>
                              {(replayResult.determinism.similarity_score * 100).toFixed(1)}%
                            </p>
                            <p style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 4 }}>Similarity</p>
                          </div>
                          <div style={{ textAlign: 'center' }}>
                            <p style={{ fontSize: 24, fontWeight: 700, color: replayResult.determinism.hash_match ? '#00d4aa' : '#ff7675' }}>
                              {replayResult.determinism.hash_match ? 'Match' : 'Mismatch'}
                            </p>
                            <p style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 4 }}>Hash Check</p>
                          </div>
                        </div>
                        {replayResult.determinism.token_diff_count > 0 && (
                          <div style={{ marginTop: 12, padding: 8, background: 'rgba(255,118,117,0.1)', borderRadius: 6 }}>
                            <p style={{ fontSize: 11, color: '#ff7675' }}>
                              Token differences: {replayResult.determinism.token_diff_count} | 
                              Byte differences: {replayResult.determinism.byte_diff_count}
                            </p>
                          </div>
                        )}
                      </div>
                    </GlassCard>
                  </motion.div>
                )}

                {/* 执行契约概览 */}
                <ExecutionContract trace={selectedTrace} />

                {/* 状态机时间线 */}
                <StateMachineTimeline currentState={selectedTrace.execution_state} />

                {/* Input/Output 双栏 */}
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.1 }}
                  >
                    <GlassCard style={{ padding: 20 }}>
                      <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 10, display: 'flex', alignItems: 'center', gap: 6 }}>
                        <Zap size={14} style={{ color: '#00d4aa' }} /> {t('audit.input')}
                      </h3>
                      <div style={{ fontSize: 11, color: 'var(--text-primary)', whiteSpace: 'pre-wrap', maxHeight: 280, overflowY: 'auto', background: 'rgba(0,0,0,0.2)', padding: 12, borderRadius: 8 }}>
                        {selectedTrace.input?.prompt ? (
                          <div>
                            <p style={{ marginBottom: 8, color: 'var(--text-tertiary)', fontSize: 10 }}>Prompt:</p>
                            {Array.isArray(selectedTrace.input.prompt) ? (
                              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                                {selectedTrace.input.prompt.map((item: any, index: number) => (
                                  <div key={index} style={{ background: 'rgba(255,255,255,0.05)', padding: 8, borderRadius: 6 }}>
                                    <span style={{ fontSize: 10, color: '#a29bfe', marginRight: 8 }}>{item.role || 'user'}:</span>
                                    <span>{item.content}</span>
                                  </div>
                                ))}
                              </div>
                            ) : (
                              <p style={{ marginBottom: 12 }}>{selectedTrace.input.prompt}</p>
                            )}
                            {selectedTrace.input.params && (
                              <>
                                <p style={{ marginBottom: 8, color: 'var(--text-tertiary)', fontSize: 10 }}>Params:</p>
                                <pre>{JSON.stringify(selectedTrace.input.params, null, 2)}</pre>
                              </>
                            )}
                          </div>
                        ) : (
                          <pre>{JSON.stringify(selectedTrace.input, null, 2)}</pre>
                        )}
                      </div>
                    </GlassCard>
                  </motion.div>

                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.15 }}
                  >
                    <GlassCard style={{ padding: 20 }}>
                      <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 10, display: 'flex', alignItems: 'center', gap: 6 }}>
                        <Zap size={14} style={{ color: '#a29bfe' }} /> {t('audit.output')}
                      </h3>
                      <div style={{ fontSize: 11, color: 'var(--text-primary)', whiteSpace: 'pre-wrap', maxHeight: 280, overflowY: 'auto', background: 'rgba(0,0,0,0.2)', padding: 12, borderRadius: 8 }}>
                        {typeof selectedTrace.output?.response === 'string' ? (
                          <p>{selectedTrace.output.response}</p>
                        ) : selectedTrace.output?.response?.choices?.[0]?.message?.content ? (
                          <div>
                            <p>{selectedTrace.output.response.choices[0].message.content}</p>
                            {selectedTrace.output.finish_reason && (
                              <p style={{ marginTop: 12, color: 'var(--text-tertiary)', fontSize: 10 }}>
                                Finish Reason: {selectedTrace.output.finish_reason}
                              </p>
                            )}
                          </div>
                        ) : (
                          <pre>{JSON.stringify(selectedTrace.output, null, 2)}</pre>
                        )}
                      </div>
                    </GlassCard>
                  </motion.div>
                </div>

                {/* Observations 面板 */}
                <ObservationsPanel observations={selectedTrace.observations} />

                {/* 约束应用 */}
                {selectedTrace.constraints_applied && typeof selectedTrace.constraints_applied === 'object' && Object.keys(selectedTrace.constraints_applied).length > 0 && (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.25 }}
                  >
                    <GlassCard style={{ padding: 20 }}>
                      <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 12, display: 'flex', alignItems: 'center', gap: 6 }}>
                        <Settings size={14} style={{ color: '#74b9ff' }} /> Constraints Applied
                      </h3>
                      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
                        {Object.entries(selectedTrace.constraints_applied || {}).map(([key, value]: [string, any], i: number) => (
                          <motion.div
                            key={key}
                            initial={{ opacity: 0, scale: 0.9 }}
                            animate={{ opacity: 1, scale: 1 }}
                            transition={{ delay: 0.3 + i * 0.05 }}
                            style={{
                              padding: 10,
                              borderRadius: 10,
                              background: 'rgba(116, 185, 255, 0.1)',
                              border: '1px solid rgba(116, 185, 255, 0.2)',
                            }}
                          >
                            <span style={{ fontSize: 12, fontWeight: 600, color: '#74b9ff' }}>
                              {key}
                            </span>
                            {value !== null && value !== undefined && (
                              <span style={{
                                fontSize: 10,
                                marginLeft: 8,
                                padding: '2px 6px',
                                borderRadius: 4,
                                background: typeof value === 'boolean' && value ? 'rgba(0,212,170,0.2)' : 'rgba(255,118,117,0.2)',
                                color: typeof value === 'boolean' && value ? '#00d4aa' : '#ff7675',
                              }}>
                                {String(value)}
                              </span>
                            )}
                          </motion.div>
                        ))}
                      </div>
                    </GlassCard>
                  </motion.div>
                )}

                {/* Proof Chain */}
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.3 }}
                >
                  <GlassCard style={{ padding: 20 }}>
                    <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 12, display: 'flex', alignItems: 'center', gap: 6 }}>
                      <Lock size={14} style={{ color: '#a29bfe' }} /> {t('audit.proof_chain')}
                    </h3>
                    {((selectedTrace.proofs?.proof_chain || []).length || 0) > 0 ? (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                        {(selectedTrace.proofs?.proof_chain || []).map((p, i) => (
                          <motion.div
                            key={i}
                            initial={{ opacity: 0, x: -10 }}
                            animate={{ opacity: 1, x: 0 }}
                            transition={{ delay: 0.35 + i * 0.1 }}
                            style={{ padding: 14, borderRadius: 10, background: 'rgba(162, 155, 254, 0.08)', border: '1px solid rgba(162, 155, 254, 0.15)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}
                          >
                            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                              <ProofLevelBadge level={p.level} />
                              <div>
                                <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-primary)' }}>{p.proof_type}</span>
                                {p.digest && (
                                  <p style={{ fontSize: 10, color: 'var(--text-tertiary)', fontFamily: "'JetBrains Mono', monospace", marginTop: 2 }}>
                                    Digest: {p.digest.slice(0, 16)}...
                                  </p>
                                )}
                              </div>
                            </div>
                            <div style={{ textAlign: 'right' }}>
                              <span style={{ fontSize: 10, color: 'var(--text-tertiary)', fontFamily: "'JetBrains Mono', monospace", maxWidth: 250, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', display: 'block' }}>
                                {p.signature?.slice(0, 32)}...
                              </span>
                              <span style={{ fontSize: 9, color: 'var(--text-tertiary)', marginTop: 4 }}>
                                {p.signature ? `${p.signature.length} chars` : '-'}
                              </span>
                            </div>
                          </motion.div>
                        ))}
                      </div>
                    ) : (
                      <div style={{ textAlign: 'center', padding: 24 }}>
                        <Lock size={32} style={{ opacity: 0.2, marginBottom: 8 }} />
                        <p style={{ color: 'var(--text-tertiary)', fontSize: 13 }}>{t('audit.no_proofs')}</p>
                      </div>
                    )}
                  </GlassCard>
                </motion.div>

                {/* Raw JSON */}
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.4 }}
                >
                  <GlassCard style={{ padding: 20 }}>
                    <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)', marginBottom: 10, display: 'flex', alignItems: 'center', gap: 6 }}>
                      <FileText size={14} /> {t('audit.raw_json')}
                    </h3>
                    <pre style={{ fontSize: 11, color: 'var(--text-primary)', whiteSpace: 'pre-wrap', maxHeight: 300, overflowY: 'auto', background: 'rgba(0,0,0,0.2)', padding: 12, borderRadius: 8 }}>
                      {JSON.stringify(selectedTrace, null, 2)}
                    </pre>
                  </GlassCard>
                </motion.div>
              </motion.div>
            ) : (
            <GlassCard style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', padding: 40 }}>
              <Shield size={48} style={{ opacity: 0.2, marginBottom: 16 }} />
              <h3 style={{ fontSize: 18, fontWeight: 600, color: 'var(--text-primary)', marginBottom: 8 }}>{t('audit.title')}</h3>
              <p style={{ color: 'var(--text-secondary)', fontSize: 13, textAlign: 'center' }}>{t('audit.select_hint')}</p>
            </GlassCard>
          )}
            </AnimatePresence>
        </div>
      </div>

      <ConfirmDialog
        open={!!deleteBranchId}
        onClose={() => setDeleteBranchId(null)}
        onConfirm={confirmDeleteBranch}
        title="删除分支"
        message="确定要删除这个分支吗？"
        confirmText="删除"
        danger
      />
      <ConfirmDialog
        open={deleteTracesCount > 0}
        onClose={() => setDeleteTracesCount(0)}
        onConfirm={confirmDeleteTraces}
        title="批量删除 Traces"
        message={`确定要删除 ${deleteTracesCount} 条 Trace 记录吗？`}
        confirmText="删除"
        danger
      />
    </motion.div>
  );
}

// 辅助组件
function MetricCard({ label, value, icon: Icon, color }: { label: string; value: string; icon: typeof Activity; color: string }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <div style={{ padding: 8, borderRadius: 8, background: `${color}20` }}>
        <Icon size={16} style={{ color }} />
      </div>
      <div>
        <p style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>{value}</p>
        <p style={{ fontSize: 10, color: 'var(--text-tertiary)' }}>{label}</p>
      </div>
    </div>
  );
}

function VerificationBadge({ level, passed }: { level: string; passed?: boolean }) {
  return (
    <div style={{ textAlign: 'center', padding: 10, borderRadius: 8, background: 'rgba(0,0,0,0.2)' }}>
      {passed === true ? (
        <CheckCircle size={20} style={{ color: '#00d4aa' }} />
      ) : passed === false ? (
        <XCircle size={20} style={{ color: '#ff7675' }} />
      ) : (
        <AlertCircle size={20} style={{ color: 'var(--text-tertiary)' }} />
      )}
      <p style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-primary)', marginTop: 4 }}>{level}</p>
      <p style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 2 }}>
        {passed === true ? 'Passed' : passed === false ? 'Failed' : 'Not Available'}
      </p>
    </div>
  );
}