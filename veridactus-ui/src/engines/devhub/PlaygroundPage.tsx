// Developer Hub — 三栏布局 Playground (AI-1.md 指令 7.1)
import { useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Play, Eye, Zap, Activity, DollarSign } from 'lucide-react';
import XRayPanel from './XRayPanel';

export default function PlaygroundPage() {
  const [prompt, setPrompt] = useState(''); const [response, setResponse] = useState('');
  const [model, setModel] = useState('glm-5.1'); const [streaming, setStreaming] = useState(false);
  const [tokens, setTokens] = useState(0); const [cost, setCost] = useState(0); const [latency, setLatency] = useState(0);

  const handleSend = useCallback(async () => {
    if (!prompt.trim()) return;
    setStreaming(true); setResponse(''); setTokens(0);
    const start = performance.now();
    try {
      const res = await fetch('/v1/chat/completions', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ model, messages: [{ role: 'user', content: prompt }], stream: true, max_tokens: 512 }) });
      const reader = res.body?.getReader(); if (!reader) return;
      const decoder = new TextDecoder(); let content = '', tokCount = 0;
      while (true) { const { done, value } = await reader.read(); if (done) break;
        for (const line of decoder.decode(value, { stream: true }).split('\n').filter(l => l.startsWith('data: '))) {
          const data = line.slice(6).trim(); if (data === '[DONE]') break;
          try { const d = JSON.parse(data); content += d.choices?.[0]?.delta?.content || ''; tokCount++; setResponse(content); setTokens(tokCount); } catch {}
        }
      }
      setCost(parseFloat((tokCount * 0.000002).toFixed(6))); setLatency(Math.round(performance.now() - start));
    } catch {} finally { setStreaming(false); }
  }, [prompt, model]);

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="h-full flex gap-0">
      {/* Left: Prompt Editor */}
      <div className="w-[300px] flex-shrink-0 flex flex-col border-r border-[rgba(255,255,255,0.06)]">
        <div className="py-3 px-4 border-b border-[rgba(255,255,255,0.06)] flex items-center gap-2">
          <Play size={16} color="#6c5ce7" /><span className="font-bold text-white text-[13px]">Prompt</span>
        </div>
        <select value={model} onChange={e => setModel(e.target.value)}
          className="mx-4 mt-3 py-2 px-3 rounded-lg text-[13px] text-[#e0e6f0]" style={{ background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)' }}>
          <option value="glm-5.1">GLM-5.1 (Zhipu)</option>
          <option value="deepseek-v3">DeepSeek V3</option>
          <option value="gpt-4o">GPT-4o</option>
          <option value="claude-3.5-sonnet">Claude 3.5</option>
        </select>
        <textarea value={prompt} onChange={e => setPrompt(e.target.value)} placeholder="输入你的 Prompt..."
          className="flex-1 mx-4 mb-4 mt-3 p-3.5 rounded-xl text-[13px] text-[#e0e6f0] resize-none outline-none font-mono leading-relaxed"
          style={{ background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)' }} />
        <button onClick={handleSend} disabled={streaming || !prompt.trim()}
          className="mx-4 mb-4 py-3 rounded-xl border-none text-white font-bold disabled:opacity-50 disabled:cursor-not-allowed"
          style={{ background: streaming ? 'rgba(255,255,255,0.05)' : 'linear-gradient(135deg, #6c5ce7, #00d4aa)' }}>
          {streaming ? 'Streaming...' : '▶  Run'}
        </button>
      </div>

      {/* Center: Output */}
      <div className="flex-1 flex flex-col">
        <div className="py-3 px-4 border-b border-[rgba(255,255,255,0.06)] flex items-center justify-between">
          <div className="flex items-center gap-2"><Zap size={16} color="#00d4aa" /><span className="font-bold text-white text-[13px]">Output</span></div>
          <div className="flex gap-4 text-[11px] text-[#8892b0]">
            <span><Activity size={10} className="inline align-sub" /> {tokens} tokens</span>
            <span><DollarSign size={10} className="inline align-sub" /> ${cost.toFixed(6)}</span>
            <span>⏱ {latency}ms</span>
          </div>
        </div>
        <div className="flex-1 p-5 overflow-auto font-sans text-sm text-[#e0e6f0] leading-relaxed whitespace-pre-wrap">
          {response || <span className="text-[#8892b0] italic">输出将在这里显示...</span>}
        </div>
      </div>

      {/* Right: X-Ray Panel */}
      <div className="w-[320px] flex-shrink-0">
        <XRayPanel tokensUsed={tokens} costUsd={cost} budgetRemaining={10.0} safetyScore={100} />
      </div>
    </motion.div>
  );
}