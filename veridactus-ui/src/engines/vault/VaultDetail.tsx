// Holo-Trace Vault — 上帝视角分屏详情
import { useState, useEffect, useRef, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { ArrowLeft, Shield, Activity, DollarSign, Clock, Cpu } from 'lucide-react';
import { getTraceDetail } from '../../api';
import type { TraceDetail } from '../../types';
import CryptoVerify from './CryptoVerify';

export default function VaultDetail() {
  const { traceId } = useParams<{ traceId: string }>();
  const navigate = useNavigate();
  const [trace, setTrace] = useState<TraceDetail | null>(null);
  const [loading, setLoading] = useState(true);

  const leftRef = useRef<HTMLDivElement>(null);
  const rightRef = useRef<HTMLDivElement>(null);
  const syncing = useRef(false);

  useEffect(() => {
    if (!traceId) return;
    getTraceDetail(traceId)
      .then(setTrace)
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [traceId]);

  // 分屏滚动同步
  const handleScroll = useCallback((source: 'left' | 'right') => {
    if (syncing.current) return;
    syncing.current = true;
    const sourceEl = source === 'left' ? leftRef.current : rightRef.current;
    const targetEl = source === 'left' ? rightRef.current : leftRef.current;
    if (sourceEl && targetEl) {
      const pct = sourceEl.scrollTop / (sourceEl.scrollHeight - sourceEl.clientHeight);
      targetEl.scrollTop = pct * (targetEl.scrollHeight - targetEl.clientHeight);
    }
    requestAnimationFrame(() => { syncing.current = false; });
  }, []);

  if (loading) return <div style={{ textAlign: 'center', padding: 60, color: '#8892b0' }}>加载中...</div>;
  if (!trace) return <div style={{ textAlign: 'center', padding: 60, color: '#ff7675' }}>Trace 未找到</div>;

  const inputText = JSON.stringify(trace.input?.prompt || trace.input, null, 2) || '—';
  const outputText = typeof trace.output?.response === 'string'
    ? trace.output.response
    : JSON.stringify(trace.output?.response || trace.output, null, 2) || '—';
  // 模拟 sanitized 版本
  const sanitizedInput = inputText.replace(
    /([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}|1[3-9]\d{9}|\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4})/g,
    '[REDACTED]'
  );

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 24 }}>
        <motion.button
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
          onClick={() => navigate('/vault')}
          style={{
            padding: '10px 14px', borderRadius: 10,
            background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)',
            color: '#e0e6f0', cursor: 'pointer',
          }}
        >
          <ArrowLeft size={18} />
        </motion.button>
        <div>
          <h1 style={{ fontSize: 20, fontWeight: 700, color: '#fff' }}>
            Trace <span style={{ color: '#6c5ce7', fontFamily: 'monospace' }}>{traceId?.slice(0, 16)}...</span>
          </h1>
          <div style={{ display: 'flex', gap: 16, marginTop: 4, fontSize: 12, color: '#8892b0' }}>
            <span><Cpu size={12} style={{ verticalAlign: -1 }} /> {trace.model}</span>
            <span><Clock size={12} style={{ verticalAlign: -1 }} /> {trace.created_at}</span>
            <span><DollarSign size={12} style={{ verticalAlign: -1 }} /> ${(trace.cost_estimated_usd || 0).toFixed(6)}</span>
            <span><Activity size={12} style={{ verticalAlign: -1 }} /> {trace.tokens_count || 0} tokens</span>
          </div>
        </div>
      </div>

      {/* Split-screen: Raw vs Sanitized */}
      <div style={{
        display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16,
        marginBottom: 24, height: '100%', minHeight: 300,
      }}>
        {/* Left: Raw */}
        <div style={{
          background: 'rgba(255,255,255,0.02)', border: '1px solid rgba(255,255,255,0.06)',
          borderRadius: 16, overflow: 'hidden', display: 'flex', flexDirection: 'column',
        }}>
          <div style={{
            padding: '12px 16px', borderBottom: '1px solid rgba(255,255,255,0.05)',
            fontWeight: 600, fontSize: 13, color: '#e0e6f0',
            display: 'flex', justifyContent: 'space-between',
          }}>
            <span>📥 Raw Input</span>
            <span style={{ fontSize: 10, color: '#ff7675' }}>原始请求</span>
          </div>
          <div ref={leftRef} onScroll={() => handleScroll('left')} style={{
            flex: 1, overflow: 'auto', padding: 16,
            fontFamily: 'monospace', fontSize: 12, color: '#e0e6f0',
            lineHeight: 1.6, whiteSpace: 'pre-wrap', wordBreak: 'break-all',
          }}>
            {inputText}
          </div>
        </div>

        {/* Right: Sanitized */}
        <div style={{
          background: 'rgba(0,212,170,0.03)', border: '1px solid rgba(0,212,170,0.15)',
          borderRadius: 16, overflow: 'hidden', display: 'flex', flexDirection: 'column',
        }}>
          <div style={{
            padding: '12px 16px', borderBottom: '1px solid rgba(0,212,170,0.1)',
            fontWeight: 600, fontSize: 13, color: '#e0e6f0',
            display: 'flex', justifyContent: 'space-between',
          }}>
            <span>🛡️ Sanitized Input</span>
            <span style={{ fontSize: 10, color: '#00d4aa' }}>已脱敏</span>
          </div>
          <div ref={rightRef} onScroll={() => handleScroll('right')} style={{
            flex: 1, overflow: 'auto', padding: 16,
            fontFamily: 'monospace', fontSize: 12, color: '#e0e6f0',
            lineHeight: 1.6, whiteSpace: 'pre-wrap', wordBreak: 'break-all',
          }}>
            {sanitizedInput}
          </div>
        </div>
      </div>

      {/* Crypto Verify */}
      {trace && <CryptoVerify trace={trace} auditSignature={trace.signature} />}
    </motion.div>
  );
}
