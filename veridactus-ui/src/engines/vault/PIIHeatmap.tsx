// PII 检测热力图 — 从真实 Trace 数据中提取 PII 模式统计
import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Eye, AlertTriangle } from 'lucide-react';

interface PIIEvent { workspace: string; type: string; count: number; severity: 'low' | 'medium' | 'high' | 'critical' }

/** PII 检测规则（从 trace 的 input/output 文本中扫描） */
const PII_PATTERNS: { type: string; regex: RegExp }[] = [
  { type: 'email',      regex: /[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/g },
  { type: 'phone',      regex: /1[3-9]\d{9}/g },
  { type: 'credit_card',regex: /\b(?:\d[ -]*?){13,16}\b/g },
  { type: 'api_key',    regex: /(?:sk-[a-zA-Z0-9]{20,}|Bearer\s+[a-zA-Z0-9_-]{20,})/g },
  { type: 'id_card',    regex: /\b[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]\b/g },
  { type: 'ip_address', regex: /\b(?:\d{1,3}\.){3}\d{1,3}\b/g },
];

/** 从消息文本中扫描 PII 匹配 */
function scanPII(text: string): Map<string, number> {
  const found = new Map<string, number>();
  if (!text) return found;
  for (const pat of PII_PATTERNS) {
    const matches = text.match(pat.regex);
    if (matches?.length) found.set(pat.type, matches.length);
  }
  return found;
}

export default function PIIHeatmap() {
  const [events, setEvents] = useState<PIIEvent[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch('/v1/traces?limit=200')
      .then(r => r.json())
      .then(data => {
        const traces = data?.traces || data || [];
        const wsMap = new Map<string, Map<string, number>>();

        for (const t of traces) {
          const ws = t.tenant_id || t.workspace_id || 'unknown';
          if (!wsMap.has(ws)) wsMap.set(ws, new Map());
          const typeMap = wsMap.get(ws)!;

          // 从真实 input/output 文本中扫描 PII
          const texts: string[] = [];
          const input_content = t.input?.prompt || t.input;
          if (input_content) {
            if (typeof input_content === 'string') texts.push(input_content);
            else try { texts.push(JSON.stringify(input_content)); } catch {}
          }
          const output_content = t.output?.response;
          if (output_content) {
            if (typeof output_content === 'string') texts.push(output_content);
            else try { texts.push(JSON.stringify(output_content)); } catch {}
          }
          // 如果 trace_data 有 raw 文本也扫描
          if (t.trace_data) {
            try { texts.push(JSON.stringify(t.trace_data)); } catch {}
          }

          for (const text of texts) {
            const hits = scanPII(text);
            for (const [type, count] of hits) {
              typeMap.set(type, (typeMap.get(type) || 0) + count);
            }
          }
        }

        const result: PIIEvent[] = [];
        for (const [ws, typeMap] of wsMap) {
          for (const [type, count] of typeMap) {
            const severity =
              count > 20 ? 'critical' : count > 10 ? 'high' : count > 5 ? 'medium' : 'low';
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
          <p style={{ fontSize: 11, color: '#5a6a8a', margin: '2px 0 0' }}>
            基于真实 Trace 数据扫描的 PII 模式分布（{events.length} 项）
          </p>
        </div>
      </div>

      {events.length > 0 ? (
        <div style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${Math.min(6, new Set(events.map(e => e.type)).size)}, 1fr)`,
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
                title={`${e.workspace} / ${e.type}: ${e.count} 匹配`}
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
onMouseEnter={(e: any) => { e.currentTarget.style.transform = 'scale(1.1)'; }}
onMouseLeave={(e: any) => { e.currentTarget.style.transform = 'scale(1)'; }}
              >
                {e.count}
              </motion.div>
            );
          })}
        </div>
      ) : (
        <div style={{ textAlign: 'center', padding: 32, color: '#5a6a8a', fontSize: 13 }}>
          <AlertTriangle size={24} style={{ marginBottom: 8 }} />
          <div>暂无 PII 检测数据 — 发送更多消息后可看到统计</div>
        </div>
      )}

      {/* 图例 */}
      <div style={{ display: 'flex', gap: 16, marginTop: 16, flexWrap: 'wrap' }}>
        {(Object.entries(severityColors) as [string, string][]).map(([level, color]) => (
          <div key={level} style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 11, color: '#8892b0' }}>
            <div style={{ width: 10, height: 10, borderRadius: 3, background: color }} />
            {level === 'low' ? '低' : level === 'medium' ? '中' : level === 'high' ? '高' : '严重'}
          </div>
        ))}
      </div>
    </motion.div>
  );
}
