// ⚔️ A/B 对比模式 — 双模型并发 SSE 同步输出（生产级）
import { useState, useRef, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Columns, X, Activity } from 'lucide-react';
import { getStoredToken } from '../../auth/useAuth';

interface ABCompareProps {
  prompt: string;
  modelA: string;
  modelB: string;
  onClose: () => void;
}

function useSSEStream(url: string, body: any, token?: string) {
  const [text, setText] = useState('');
  const [tokens, setTokens] = useState(0);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    abortRef.current = controller;

    setText(''); setTokens(0);
    let content = '';
    let tokCount = 0;

    fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) },
      body: JSON.stringify({ ...body, stream: true }),
      signal: controller.signal,
    }).then(async res => {
      const reader = res.body?.getReader();
      if (!reader) return;
      const decoder = new TextDecoder();
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        const chunk = decoder.decode(value, { stream: true });
        const lines = chunk.split('\n').filter(l => l.startsWith('data: '));
        for (const line of lines) {
          const data = line.slice(6).trim();
          if (data === '[DONE]' || data.startsWith('[VERIDACTUS:')) break;
          try {
            const delta = JSON.parse(data).choices?.[0]?.delta?.content || '';
            content += delta; tokCount++;
            setText(content);
            setTokens(tokCount);
          } catch {}
        }
      }
    }).catch(() => {});

    return () => { controller.abort(); };
  }, [url, JSON.stringify(body), token]);

  return { text, tokens, abort: () => abortRef.current?.abort() };
}

export default function ABCompare({ prompt, modelA, modelB, onClose }: ABCompareProps) {
  const token = getStoredToken();
  const body = { messages: [{ role: 'user', content: prompt }], max_tokens: 512 };

  const streamA = useSSEStream('/v1/chat/completions', { ...body, model: modelA }, token || undefined);
  const streamB = useSSEStream('/v1/chat/completions', { ...body, model: modelB }, token || undefined);

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      style={{
        position: 'fixed', inset: 0, zIndex: 999,
        background: 'rgba(11,15,25,0.98)', backdropFilter: 'blur(20px)',
        display: 'flex', flexDirection: 'column',
      }}
    >
      {/* Header */}
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '16px 24px', borderBottom: '1px solid rgba(255,255,255,0.06)',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <Columns size={20} color="#6c5ce7" />
          <span style={{ fontWeight: 700, color: '#fff', fontSize: 15 }}>
            ⚔️ A/B Compare: <span style={{ color: '#6c5ce7' }}>{modelA}</span> vs <span style={{ color: '#00d4aa' }}>{modelB}</span>
          </span>
        </div>
        <button onClick={onClose} style={{
          background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)',
          borderRadius: 10, padding: '8px 12px', color: '#8892b0', cursor: 'pointer',
        }}>
          <X size={18} />
        </button>
      </div>

      {/* Split screen */}
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 0 }}>
        {/* Model A */}
        <div style={{
          display: 'flex', flexDirection: 'column',
          borderRight: '1px solid rgba(255,255,255,0.06)',
        }}>
          <div style={{
            padding: '10px 20px', background: 'rgba(108,92,231,0.1)',
            borderBottom: '1px solid rgba(108,92,231,0.2)',
            display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          }}>
            <span style={{ fontWeight: 700, color: '#6c5ce7', fontSize: 13 }}>🅰️ {modelA}</span>
            <span style={{ fontSize: 11, color: '#8892b0', display: 'flex', alignItems: 'center', gap: 4 }}>
              <Activity size={12} /> {streamA.tokens} tokens
            </span>
          </div>
          <div style={{
            flex: 1, padding: 20, overflow: 'auto',
            fontFamily: 'system-ui, sans-serif', fontSize: 14,
            color: '#e0e6f0', lineHeight: 1.8, whiteSpace: 'pre-wrap',
          }}>
            {streamA.text || (
              <span style={{ color: '#6c5ce7', display: 'flex', gap: 4 }}>
                <motion.span animate={{ opacity: [0.3,1,0.3] }} transition={{ duration:1, repeat:Infinity }}>●</motion.span>
                <motion.span animate={{ opacity: [0.3,1,0.3] }} transition={{ duration:1, delay:0.2, repeat:Infinity }}>●</motion.span>
                <motion.span animate={{ opacity: [0.3,1,0.3] }} transition={{ duration:1, delay:0.4, repeat:Infinity }}>●</motion.span>
              </span>
            )}
          </div>
        </div>

        {/* Model B */}
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          <div style={{
            padding: '10px 20px', background: 'rgba(0,212,170,0.1)',
            borderBottom: '1px solid rgba(0,212,170,0.2)',
            display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          }}>
            <span style={{ fontWeight: 700, color: '#00d4aa', fontSize: 13 }}>🅱️ {modelB}</span>
            <span style={{ fontSize: 11, color: '#8892b0', display: 'flex', alignItems: 'center', gap: 4 }}>
              <Activity size={12} /> {streamB.tokens} tokens
            </span>
          </div>
          <div style={{
            flex: 1, padding: 20, overflow: 'auto',
            fontFamily: 'system-ui, sans-serif', fontSize: 14,
            color: '#e0e6f0', lineHeight: 1.8, whiteSpace: 'pre-wrap',
          }}>
            {streamB.text || (
              <span style={{ color: '#00d4aa', display: 'flex', gap: 4 }}>
                <motion.span animate={{ opacity: [0.3,1,0.3] }} transition={{ duration:1, repeat:Infinity }}>●</motion.span>
                <motion.span animate={{ opacity: [0.3,1,0.3] }} transition={{ duration:1, delay:0.2, repeat:Infinity }}>●</motion.span>
                <motion.span animate={{ opacity: [0.3,1,0.3] }} transition={{ duration:1, delay:0.4, repeat:Infinity }}>●</motion.span>
              </span>
            )}
          </div>
        </div>
      </div>
    </motion.div>
  );
}
