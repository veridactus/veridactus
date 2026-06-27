// 审计指挥舱 — 企业级风险大盘
import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Shield, Activity, Key, Eye, DollarSign, BarChart3, GitBranch, Download } from 'lucide-react';

interface AuditEvent {
  type: string;
  label: string;
  value?: number;
  icon: string;
  levels?: { level: string; count: number; color: string }[];
}

const ICONS: Record<string, any> = {
  Activity, Key, Shield, Eye, DollarSign, BarChart3, GitBranch,
};

export default function AuditCommandCenter() {
  const [events, setEvents] = useState<AuditEvent[]>([]);
  const [period, setPeriod] = useState('today');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    fetch(`/api/v1/audit/events?period=${period}`)
      .then(r => r.json())
      .then(d => setEvents(d.events || []))
      .catch(() => setEvents([]))
      .finally(() => setLoading(false));
  }, [period]);

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 28 }}>
        <div>
          <h1 style={{
            fontSize: 24, fontWeight: 700, color: '#fff',
            display: 'flex', alignItems: 'center', gap: 10,
          }}>
            <Shield size={28} color="#6c5ce7" />
            Auditor Command Center
          </h1>
          <p style={{ color: '#8892b0', fontSize: 13, marginTop: 4 }}>
            全局风险大盘 — 实时安全态势感知
          </p>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          {['today', '7d', '30d'].map(p => (
            <motion.button
              key={p}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              onClick={() => setPeriod(p)}
              style={{
                padding: '8px 16px', borderRadius: 10, fontSize: 12, fontWeight: 600,
                border: '1px solid ' + (period === p ? 'rgba(108,92,231,0.4)' : 'rgba(255,255,255,0.1)'),
                background: period === p ? 'rgba(108,92,231,0.15)' : 'transparent',
                color: period === p ? '#6c5ce7' : '#8892b0', cursor: 'pointer',
              }}
            >
              {p === 'today' ? '今日' : p === '7d' ? '7天' : '30天'}
            </motion.button>
          ))}
          <motion.button
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            style={{
              padding: '8px 16px', borderRadius: 10,
              background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)',
              border: 'none', color: '#fff', fontSize: 12, fontWeight: 600,
              cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6,
            }}
          >
            <Download size={14} /> 导出报告
          </motion.button>
        </div>
      </div>

      {loading ? (
        <div style={{ textAlign: 'center', padding: 60, color: '#8892b0' }}>加载中...</div>
      ) : (
        <>
          {/* Metric Cards */}
          <div style={{
            display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
            gap: 16, marginBottom: 24,
          }}>
            {events.filter(e => e.type !== 'risk_distribution').map(evt => {
              const Icon = ICONS[evt.icon] || Activity;
              return (
                <motion.div
                  key={evt.type}
                  initial={{ opacity: 0, y: 12 }}
                  animate={{ opacity: 1, y: 0 }}
                  style={{
                    background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)',
                    borderRadius: 16, padding: 20,
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
                    <div style={{
                      width: 36, height: 36, borderRadius: 10,
                      background: 'rgba(108,92,231,0.15)', display: 'flex', alignItems: 'center', justifyContent: 'center',
                    }}>
                      <Icon size={18} color="#6c5ce7" />
                    </div>
                    <span style={{ fontSize: 12, color: '#8892b0', fontWeight: 500 }}>{evt.label}</span>
                  </div>
                  <div style={{ fontSize: 28, fontWeight: 800, color: '#fff' }}>
                    {((evt.value || 0) as number).toLocaleString()}
                  </div>
                </motion.div>
              );
            })}
          </div>

          {/* Risk Distribution */}
          {events.find(e => e.type === 'risk_distribution')?.levels && (
            <motion.div
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              style={{
                background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)',
                borderRadius: 16, padding: 24, marginBottom: 24,
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
                <BarChart3 size={18} color="#6c5ce7" />
                <span style={{ fontSize: 14, fontWeight: 600, color: '#fff' }}>风险级别分布</span>
              </div>
              <div style={{ display: 'flex', gap: 12 }}>
                {(events.find(e => e.type === 'risk_distribution')?.levels || []).map((lvl: any) => {
                  const total = (events.find(e => e.type === 'risk_distribution')?.levels || []).reduce((s: number, l: any) => s + l.count, 0) || 1;
                  const pct = Math.round((lvl.count / total) * 100);
                  return (
                    <div key={lvl.level} style={{ flex: 1 }}>
                      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
                        <span style={{ fontSize: 11, color: '#8892b0' }}>{lvl.level.toUpperCase()}</span>
                        <span style={{ fontSize: 11, color: lvl.color, fontWeight: 600 }}>{lvl.count}</span>
                      </div>
                      <div style={{
                        height: 6, borderRadius: 3, background: 'rgba(255,255,255,0.05)', overflow: 'hidden',
                      }}>
                        <motion.div
                          initial={{ width: 0 }}
                          animate={{ width: `${pct}%` }}
                          transition={{ duration: 0.8, delay: 0.2 }}
                          style={{ height: '100%', borderRadius: 3, background: lvl.color }}
                        />
                      </div>
                    </div>
                  );
                })}
              </div>
            </motion.div>
          )}
        </>
      )}
    </motion.div>
  );
}
