// Developer Hub — 三栏布局 Playground (AI-1.md 指令 7.1)
import { useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Play, Eye, Zap, Activity, DollarSign } from 'lucide-react';
import XRayPanel from './XRayPanel';

export default function PlaygroundPage() {
  const [prompt, setPrompt] = useState('');
  const [response, setResponse] = useState('');
  const [model, setModel] = useState('glm-5.1');
  const [streaming, setStreaming] = useState(false);
  const [tokens, setTokens] = useState(0);
  const [cost, setCost] = useState(0);
  const [latency, setLatency] = useState(0);

  const handleSend = useCallback(async () => {
    if (!prompt.trim()) return;
    setStreaming(true); setResponse(''); setTokens(0);
    const start = performance.now();

    try {
      const res = await fetch('/v1/chat/completions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ model, messages: [{ role: 'user', content: prompt }], stream: true, max_tokens: 512 }),
      });
      const reader = res.body?.getReader();
      if (!reader) return;
      const decoder = new TextDecoder();
      let content = '', tokCount = 0;
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        const chunk = decoder.decode(value, { stream: true });
        const lines = chunk.split('\n').filter(l => l.startsWith('data: '));
        for (const line of lines) {
          const data = line.slice(6).trim();
          if (data === '[DONE]') break;
          try { const d = JSON.parse(data); const delta = d.choices?.[0]?.delta?.content || ''; content += delta; tokCount++; setResponse(content); setTokens(tokCount); } catch {}
        }
      }
      setCost(parseFloat((tokCount * 0.000002).toFixed(6)));
      setLatency(Math.round(performance.now() - start));
    } catch {} finally { setStreaming(false); }
  }, [prompt, model]);

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} style={{ height: '100%', display: 'flex', gap: 0 }}>
      {/* Left: Prompt Editor */}
      <div style={{ width: 300, flexShrink: 0, display: 'flex', flexDirection: 'column', borderRight: '1px solid rgba(255,255,255,0.06)' }}>
        <div style={{ padding: '12px 16px', borderBottom: '1px solid rgba(255,255,255,0.06)', display: 'flex', alignItems: 'center', gap: 8 }}>
          <Play size={16} color="#6c5ce7" /><span style={{ fontWeight: 700, color: '#fff', fontSize: 13 }}>Prompt</span>
        </div>
        <select value={model} onChange={e => setModel(e.target.value)} style={{ margin: '12px 16px', padding: '8px 12px', borderRadius: 8, background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)', color: '#e0e6f0', fontSize: 13 }}>
          <option value="glm-5.1">GLM-5.1 (Zhipu)</option>
          <option value="deepseek-v3">DeepSeek V3</option>
          <option value="gpt-4o">GPT-4o</option>
          <option value="claude-3.5-sonnet">Claude 3.5</option>
        </select>
        <textarea value={prompt} onChange={e => setPrompt(e.target.value)} placeholder="输入你的 Prompt..." style={{
          flex: 1, margin: '0 16px 16px', padding: 14, borderRadius: 12,
          background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)',
          color: '#e0e6f0', fontSize: 13, resize: 'none', outline: 'none', fontFamily: 'monospace', lineHeight: 1.6,
        }} />
        <button onClick={handleSend} disabled={streaming || !prompt.trim()} style={{
          margin: '0 16px 16px', padding: '12px', borderRadius: 12,
          background: streaming ? 'rgba(255,255,255,0.05)' : 'linear-gradient(135deg, #6c5ce7, #00d4aa)',
          border: 'none', color: '#fff', fontWeight: 700, cursor: streaming ? 'not-allowed' : 'pointer', opacity: streaming ? 0.5 : 1,
        }}>{streaming ? 'Streaming...' : '▶  Run'}</button>
      </div>

      {/* Center: Output */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
        <div style={{ padding: '12px 16px', borderBottom: '1px solid rgba(255,255,255,0.06)', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Zap size={16} color="#00d4aa" /><span style={{ fontWeight: 700, color: '#fff', fontSize: 13 }}>Output</span>
          </div>
          <div style={{ display: 'flex', gap: 16, fontSize: 11, color: '#8892b0' }}>
            <span><Activity size={10} style={{verticalAlign:-1}} /> {tokens} tokens</span>
            <span><DollarSign size={10} style={{verticalAlign:-1}} /> ${cost.toFixed(6)}</span>
            <span>⏱ {latency}ms</span>
          </div>
        </div>
        <div style={{ flex: 1, padding: 20, overflow: 'auto', fontFamily: 'system-ui, sans-serif', fontSize: 14, color: '#e0e6f0', lineHeight: 1.8, whiteSpace: 'pre-wrap' }}>
          {response || (
            <span style={{ color: '#8892b0', fontStyle: 'italic' }}>输出将在这里显示...</span>
          )}
        </div>
      </div>

      {/* Right: X-Ray Panel */}
      <div style={{ width: 320, flexShrink: 0 }}>
        <XRayPanel tokensUsed={tokens} costUsd={cost} budgetRemaining={10.0} safetyScore={100} />
      </div>
    </motion.div>
  );
}
