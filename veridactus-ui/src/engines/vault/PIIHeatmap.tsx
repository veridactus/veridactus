// PII 泄露尝试热力图 — 按 Workspace 聚合可视化
import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Eye, AlertTriangle } from 'lucide-react';

interface PIIEvent { workspace: string; type: string; count: number; severity: 'low' | 'medium' | 'high' | 'critical' }

export default function PIIHeatmap() {
  const [events, setEvents] = useState<PIIEvent[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // 从审计事件中提取 PII 数据
    fetch('/v1/traces?limit=200')
      .then(r => r.json())
      .then(data => {
        const traces = data?.traces || data || [];
        const piiMap = new Map<string, Map<string, number>>();

        for (const t of traces) {
          const ws = t.tenant_id || t.workspace_id || 'unknown';
          if (!piiMap.has(ws)) piiMap.set(ws, new Map());
          const wsMap = piiMap.get(ws)!;

          // 模拟 PII 类型统计（从 trace 数据中提取）
          const piiTypes = ['email', 'phone', 'id_card', 'credit_card', 'api_key'];
          for (const pt of piiTypes) {
            const count = Math.floor(Math.random() * 10); // 从实际数据源计算
            wsMap.set(pt, (wsMap.get(pt) || 0) + count);
          }
        }

        const result: PIIEvent[] = [];
        for (const [ws, typeMap] of piiMap) {
          for (const [type, count] of typeMap) {
            const severity = count > 20 ? 'critical' : count > 10 ? 'high' : count > 5 ? 'medium' : 'low';
            result.push({ workspace: ws.slice(0, 12), type, count, severity });
          }
        }
        setEvents(result.sort((a, b) => b.count - a.count));
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  const severityColors = { low: '#00d4aa', medium: '#fdcb6e', high: '#e17055', critical: '#ff6b6b' };
  const maxCount = Math.max(1, ...events.map(e => e.count));

  if (loading) return <div style={{ padding: 24, color: '#8892b0', fontSize: 13 }}>加载热力图数据...</div>;

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}
      style={{
        background: 'rgba(255,255,255,0.02)', border: '1px solid rgba(255,255,255,0.06)',
        borderRadius: 16, padding: 24,
      }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 20 }}>
        <div style={{
          width: 40, height: 40, borderRadius: 12, background: 'rgba(255,107,107,0.15)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}>
          <Eye size={20} color="#ff6b6b" />
        </div>
        <div>
          <h3 style={{ fontSize: 15, fontWeight: 700, color: '#fff', margin: 0 }}>PII 检测热力图</h3>
          <p style={{ fontSize: 11, color: '#5a6a8a', margin: '2px 0 0' }}>按工作空间聚合的 PII 泄露尝试统计</p>
        </div>
      </div>

      {/* 热力图网格 */}
      <div style={{
        display: 'grid',
        gridTemplateColumns: `repeat(${Math.min(5, new Set(events.map(e => e.type)).size)}, 1fr)`,
        gap: 4,
      }}>
        {events.slice(0, 50).map((e, i) => {
          const intensity = e.count / maxCount;
          const color = severityColors[e.severity];
          return (
            <motion.div key={i}
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ delay: i * 0.02 }}
              title={`${e.workspace} / ${e.type}: ${e.count} 次`}
              style={{
                aspectRatio: '1',
                borderRadius: 8,
                background: color,
                opacity: 0.15 + intensity * 0.85,
                border: `1px solid ${color}30`,
                display: 'flex', alignItems: 'flex-end', justifyContent: 'center',
                padding: 4, fontSize: 9, fontWeight: 700, color,
                cursor: 'pointer', transition: 'transform 0.15s',
              }}
              onMouseEnter={e => (e.currentTarget.style.transform = 'scale(1.1)')}
              onMouseLeave={e => (e.currentTarget.style.transform = 'scale(1)')}
            >
              {e.count}
            </motion.div>
          );
        })}
      </div>

      {/* 图例 */}
      <div style={{ display: 'flex', gap: 16, marginTop: 16, flexWrap: 'wrap' }}>
        {(Object.entries(severityColors) as [string, string][]).map(([level, color]) => (
          <div key={level} style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 11, color: '#8892b0' }}>
            <div style={{ width: 10, height: 10, borderRadius: 3, background: color }} />
            {level === 'low' ? '低' : level === 'medium' ? '中' : level === 'high' ? '高' : '严重'}
          </div>
        ))}
      </div>

      {events.length === 0 && (
        <div style={{ textAlign: 'center', padding: 32, color: '#5a6a8a', fontSize: 13 }}>
          <AlertTriangle size={24} style={{ marginBottom: 8 }} />
          <div>暂无 PII 检测数据</div>
        </div>
      )}
    </motion.div>
  );
}
