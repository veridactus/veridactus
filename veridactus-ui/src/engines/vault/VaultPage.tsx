// Holo-Trace Vault — 全息证据金库列表
import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Shield, Activity, DollarSign, Search, ChevronRight, Sparkles } from 'lucide-react';
import { getTracesFromDataPlane } from '../../api';
import type { TraceSummary } from '../../types';

const SAFETY_COLORS: Record<string, string> = {
  safe: '#00d4aa', flagged: '#fdcb6e', blocked: '#ff7675',
};

export default function VaultPage() {
  const [traces, setTraces] = useState<TraceSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const navigate = useNavigate();

  useEffect(() => {
    getTracesFromDataPlane()
      .then(t => setTraces(t.reverse()))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const filtered = traces.filter(t =>
    t.trace_id?.includes(search) || t.model?.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        marginBottom: 24, flexWrap: 'wrap', gap: 12,
      }}>
        <div>
          <h1 style={{ fontSize: 22, fontWeight: 700, color: '#fff' }}>
            <Sparkles size={18} color="#6c5ce7" style={{ marginRight: 8 }} /> Holo-Trace Vault
          </h1>
          <p style={{ color: '#8892b0', fontSize: 13, marginTop: 4 }}>
            全息证据金库 — 每条 Trace 都有密码学审计签名
          </p>
        </div>
        <div style={{
          display: 'flex', gap: 12, alignItems: 'center',
          background: 'rgba(255,255,255,0.03)', borderRadius: 12,
          border: '1px solid rgba(255,255,255,0.08)', padding: '8px 16px',
        }}>
          <Search size={16} color="#8892b0" />
          <input
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="搜索 Trace ID 或模型..."
            style={{
              background: 'transparent', border: 'none', color: '#e0e6f0',
              fontSize: 13, outline: 'none', minWidth: 220,
            }}
          />
        </div>
      </div>

      {loading ? (
        <div style={{ textAlign: 'center', padding: 60, color: '#8892b0' }}>加载中...</div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {/* Table header */}
          <div style={{
            display: 'grid', gridTemplateColumns: '2fr 1fr 1fr 1fr 1fr 40px',
            gap: 12, padding: '10px 20px', fontSize: 11, fontWeight: 600,
            color: '#8892b0', textTransform: 'uppercase', letterSpacing: '0.05em',
          }}>
            <span>Trace ID</span>
            <span>🤖 Model</span>
            <span>💰 Cost</span>
            <span>🛡️ Safety</span>
            <span>📅 Time</span>
            <span></span>
          </div>

          {filtered.slice(0, 50).map((trace, idx) => (
            <motion.div
              key={trace.trace_id || idx}
              initial={{ opacity: 0, x: -8 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: idx * 0.02 }}
              onClick={() => navigate(`/vault/${trace.trace_id}`)}
              style={{
                display: 'grid', gridTemplateColumns: '2fr 1fr 1fr 1fr 1fr 40px',
                gap: 12, padding: '14px 20px', borderRadius: 12, cursor: 'pointer',
                background: 'rgba(255,255,255,0.02)', border: '1px solid rgba(255,255,255,0.05)',
                transition: 'all 0.15s',
                color: '#e0e6f0', fontSize: 13,
              }}
              onMouseEnter={e => {
                e.currentTarget.style.background = 'rgba(108,92,231,0.06)';
                e.currentTarget.style.borderColor = 'rgba(108,92,231,0.2)';
              }}
              onMouseLeave={e => {
                e.currentTarget.style.background = 'rgba(255,255,255,0.02)';
                e.currentTarget.style.borderColor = 'rgba(255,255,255,0.05)';
              }}
            >
              <span style={{ fontFamily: 'monospace', fontSize: 12, color: '#6c5ce7' }}>
                {trace.trace_id?.slice(0, 12)}...
              </span>
              <span>{trace.model || '—'}</span>
              <span style={{ color: '#00d4aa', fontFamily: 'monospace', fontSize: 12 }}>
                ${(trace.cost_estimated_usd || 0).toFixed(6)}
              </span>
              <span>
                <SafetyBadge status={trace.safety || 'safe'} />
              </span>
              <span style={{ color: '#8892b0', fontSize: 11 }}>
                {trace.created_at ? new Date(trace.created_at).toLocaleDateString() : '—'}
              </span>
              <ChevronRight size={16} color="#8892b0" />
            </motion.div>
          ))}

          {filtered.length === 0 && (
            <div style={{ textAlign: 'center', padding: 40, color: '#8892b0' }}>
              暂无 Trace 记录 — 去 Chat 页面发送第一条消息
            </div>
          )}
        </div>
      )}
    </motion.div>
  );
}

function SafetyBadge({ status }: { status: string }) {
  const color = SAFETY_COLORS[status] || '#8892b0';
  return (
    <span style={{
      fontSize: 10, padding: '2px 8px', borderRadius: 6, fontWeight: 600,
      background: `${color}18`, color, border: `1px solid ${color}30`,
    }}>
      {status === 'safe' ? '🟢 Safe' : status === 'flagged' ? '🟡 Flagged' : status === 'blocked' ? '🔴 Blocked' : status}
    </span>
  );
}
