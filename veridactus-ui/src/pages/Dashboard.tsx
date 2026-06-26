import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import CircularProgress from '../components/viz/CircularProgress';
import GlassCard from '../components/ui/GlassCard';
import AnimatedMetric from '../components/viz/AnimatedMetric';
import ProofLevelBadge from '../components/atoms/ProofLevelBadge';
import { useMetricsStore } from '../store';
import { useI18n } from '../i18n';
import { getTracesFromDataPlane } from '../api';
import type { TraceSummary } from '../types';
import { Activity, GitBranch, Puzzle, Shield, CheckCircle, XCircle, Boxes } from 'lucide-react';

export default function Dashboard() {
  const navigate = useNavigate();
  const { t } = useI18n();
  const { traceCount, pipelineCount, pluginCount, policyCount, services } = useMetricsStore();
  const [recentTraces, setRecentTraces] = useState<TraceSummary[]>([]);

  useEffect(() => {
    getTracesFromDataPlane()
      .then(traces => setRecentTraces(traces.slice(-5).reverse()))
      .catch(err => console.warn('Failed to load traces for dashboard:', err));
  }, []);

  const allOk = services.dataPlane && services.controlPlane;
  const healthScore = services.dataPlane && services.controlPlane && services.pythonWorker ? 95
    : services.dataPlane && services.controlPlane ? 72
    : services.dataPlane ? 45 : 15;

  const statCards = [
    { label: t('dashboard.traces'), value: traceCount, icon: Activity, color: '#6c5ce7', path: '/audit' },
    { label: t('dashboard.pipelines'), value: pipelineCount, icon: GitBranch, color: '#00d4aa', path: '/pipelines' },
    { label: t('dashboard.plugins'), value: pluginCount, icon: Puzzle, color: '#74b9ff', path: '/plugins' },
    { label: t('dashboard.policies'), value: policyCount, icon: Shield, color: '#fdcb6e', path: '/api-keys' },
  ];

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 28 }}>
        <div>
          <h1 style={{ fontSize: 28, fontWeight: 700, background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
            {t('dashboard.title')}
          </h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: 14, marginTop: 4 }}>{t('dashboard.subtitle')}</p>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 14px', borderRadius: 20, background: allOk ? 'rgba(0,212,170,0.12)' : 'rgba(255,118,117,0.12)', border: `1px solid ${allOk ? 'rgba(0,212,170,0.3)' : 'rgba(255,118,117,0.3)'}` }}>
          {allOk ? <CheckCircle size={14} color="#00d4aa" /> : <XCircle size={14} color="#ff7675" />}
          <span style={{ fontSize: 13, fontWeight: 600, color: allOk ? '#00d4aa' : '#ff7675' }}>
            {allOk ? t('dashboard.healthy') : t('dashboard.degraded')}
          </span>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '280px 1fr', gap: 20, marginBottom: 28 }}>
        <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }}>
          <GlassCard style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', padding: 24 }}>
            <div style={{ width: 180, height: 180, position: 'relative' }}>
              <CircularProgress score={healthScore} color={healthScore >= 80 ? '#00d4aa' : healthScore >= 50 ? '#fdcb6e' : '#ff7675'} />
              <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
                <span style={{ fontSize: 36, fontWeight: 700, color: 'var(--text-primary)', fontFamily: "'JetBrains Mono', monospace" }}>{healthScore}%</span>
                <span style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 2 }}>{t('dashboard.health_score')}</span>
              </div>
            </div>
            <div style={{ width: '100%', marginTop: 16, display: 'flex', flexDirection: 'column', gap: 8 }}>
              {[
                { label: t('dashboard.data_plane'), ok: services.dataPlane },
                { label: t('dashboard.control_plane'), ok: services.controlPlane },
                { label: t('dashboard.python_worker'), ok: services.pythonWorker },
              ].map(s => (
                <div key={s.label} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '4px 0' }}>
                  <span style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{s.label}</span>
                  {s.ok ? <CheckCircle size={14} color="#00d4aa" /> : <XCircle size={14} color="#ff7675" />}
                </div>
              ))}
            </div>
          </GlassCard>
        </motion.div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
          {statCards.map((card, i) => (
            <motion.div key={card.label} initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: i * 0.05 }}>
              <GlassCard style={{ padding: 20, cursor: 'pointer' }} onClick={() => navigate(card.path)}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
                  <div style={{ width: 44, height: 44, borderRadius: 12, background: card.color + '20', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                    <card.icon size={22} color={card.color} />
                  </div>
                  <div>
                    <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 2 }}>{card.label}</p>
                    <AnimatedMetric value={card.value} suffix="" decimals={0} />
                  </div>
                </div>
              </GlassCard>
            </motion.div>
          ))}
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 20 }}>
        <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.1 }}>
          <GlassCard style={{ padding: 24 }}>
            <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 16, color: 'var(--text-primary)' }}>{t('dashboard.trust_state')}</h3>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
              {(['L0', 'L1', 'L2A', 'L2B'] as const).map(level => {
                const pct = level === 'L0' ? 100 : level === 'L1' ? 68 : level === 'L2A' ? 42 : 18;
                const color = { L0: '#74b9ff', L1: '#fdcb6e', L2A: '#a29bfe', L2B: '#00d4aa' }[level];
                return (
                  <div key={level} style={{ padding: 14, borderRadius: 12, background: 'rgba(255,255,255,0.04)' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 10 }}>
                      <ProofLevelBadge level={level} size="small" />
                      <span style={{ fontSize: 16, fontWeight: 700, color, fontFamily: "'JetBrains Mono', monospace" }}>{pct}%</span>
                    </div>
                    <div style={{ height: 4, borderRadius: 2, background: 'rgba(255,255,255,0.08)', overflow: 'hidden' }}>
                      <motion.div initial={{ width: 0 }} animate={{ width: `${pct}%` }} transition={{ duration: 0.8, ease: 'easeOut' }} style={{ height: '100%', borderRadius: 2, background: `linear-gradient(90deg, ${color}40, ${color})`, boxShadow: `0 0 8px ${color}60` }} />
                    </div>
                  </div>
                );
              })}
            </div>
          </GlassCard>
        </motion.div>

        <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.15 }}>
          <GlassCard style={{ padding: 24 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
              <h3 style={{ fontSize: 15, fontWeight: 600, color: 'var(--text-primary)' }}>{t('dashboard.recent_traces')}</h3>
              <button className="btn-secondary" style={{ padding: '6px 12px', fontSize: 12 }} onClick={() => navigate('/audit')}>{t('dashboard.view_all')}</button>
            </div>
            {recentTraces.length === 0 ? (
              <div style={{ textAlign: 'center', padding: 32, color: 'var(--text-tertiary)' }}>
                <Boxes size={32} style={{ opacity: 0.3, margin: '0 auto 12px' }} />
                <p style={{ fontSize: 13 }}>{t('dashboard.no_traces')}</p>
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {recentTraces.map((tr, i) => (
                  <div key={tr.trace_id}
                    style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '10px 12px', borderRadius: 10, background: 'rgba(255,255,255,0.03)', cursor: 'pointer' }}
                    onClick={() => navigate(`/audit?trace=${tr.trace_id}`)}
                  >
                    <div>
                      <p style={{ fontSize: 12, fontWeight: 600, color: 'var(--text-primary)' }}>{tr.model}</p>
                      <p style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 2 }}>
                        {tr.trace_id.slice(0, 8)}... · {new Date(tr.created_at).toLocaleTimeString()}
                      </p>
                    </div>
                    <div style={{ display: 'flex', gap: 6 }}>
                      {tr.proof_levels.map(pl => <ProofLevelBadge key={pl} level={pl} size="small" />)}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </GlassCard>
        </motion.div>
      </div>
    </motion.div>
  );
}
