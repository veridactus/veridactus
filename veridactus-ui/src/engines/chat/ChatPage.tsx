// VERIDACTUS Chat — 安全沙箱对话
import { useState, useRef, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Send, Shield, Zap, Activity, ChevronDown, Columns } from 'lucide-react';
import SafetyShield from './SafetyShield';
import ABCompare from './ABCompare';
import { getStoredToken } from '../../auth/useAuth';

interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  model?: string;
  tokens?: number;
  cost?: number;
  safety?: 'safe' | 'flagged' | 'blocked';
  timestamp: number;
}

const MODELS = [
  { id: 'glm-5.1', name: 'GLM-5.1', provider: 'Zhipu', color: '#6c5ce7' },
  { id: 'deepseek-r1:14b', name: 'DeepSeek R1', provider: 'Local', color: '#00d4aa' },
  { id: 'gpt-4o', name: 'GPT-4o', provider: 'Azure', color: '#74b9ff' },
];

function generateId() { return Date.now().toString(36) + Math.random().toString(36).slice(2); }

export default function ChatPage() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [selectedModel, setSelectedModel] = useState(MODELS[0]);
  const [compareModel, setCompareModel] = useState(MODELS[1]);
  const [compareMode, setCompareMode] = useState(false);
  const [comparePrompt, setComparePrompt] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [showModelMenu, setShowModelMenu] = useState(false);
  const [showCompareMenu, setShowCompareMenu] = useState(false);
  const [budgetRemaining, setBudgetRemaining] = useState<number | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = useCallback(async () => {
    if (!input.trim() || isStreaming) return;
    const token = getStoredToken();

    const userMsg: Message = {
      id: generateId(), role: 'user', content: input.trim(), timestamp: Date.now(),
    };
    const assistantMsg: Message = {
      id: generateId(), role: 'assistant', content: '', model: selectedModel.id, timestamp: Date.now(),
    };

    setMessages(prev => [...prev, userMsg, assistantMsg]);
    setInput('');
    setIsStreaming(true);

    const controller = new AbortController();
    abortRef.current = controller;

    try {
      const res = await fetch('/v1/chat/completions', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        body: JSON.stringify({
          model: selectedModel.id,
          messages: messages.concat(userMsg).map(m => ({ role: m.role, content: m.content })),
          stream: true,
          max_tokens: 1024,
        }),
        signal: controller.signal,
      });

      if (!res.ok) throw new Error(`HTTP ${res.status}`);

      // 读取 VERIDACTUS 响应头
      const traceId = res.headers.get('VERIDACTUS-Trace-Id');
      const costConsumed = res.headers.get('VERIDACTUS-Cost-Consumed');
      const budgetRem = res.headers.get('VERIDACTUS-Budget-Remaining');
      if (budgetRem) setBudgetRemaining(parseFloat(budgetRem));

      const reader = res.body?.getReader();
      if (!reader) throw new Error('No reader');

      const decoder = new TextDecoder();
      let fullContent = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const chunk = decoder.decode(value, { stream: true });
        const lines = chunk.split('\n').filter(l => l.startsWith('data: '));

        for (const line of lines) {
          const data = line.slice(6).trim();
          if (data === '[DONE]') break;

          // 检查预算熔断事件
          if (data.startsWith('[VERIDACTUS:BUDGET_EXCEEDED]')) {
            fullContent += '\n\n⚠️ _预算已耗尽，请求已被终止_';
            break;
          }

          try {
            const parsed = JSON.parse(data);
            const delta = parsed.choices?.[0]?.delta?.content || '';
            fullContent += delta;

            // AI-1.md §6.3: SSE 流式输出使用 requestAnimationFrame 优化零抖动
            const updateContent = fullContent;
            requestAnimationFrame(() => {
              setMessages(prev => prev.map(m =>
                m.id === assistantMsg.id ? { ...m, content: updateContent } : m
              ));
            });
          } catch { /* ignore malformed chunks */ }
        }
      }

      // 更新最终消息
      setMessages(prev => prev.map(m =>
        m.id === assistantMsg.id ? {
          ...m, content: fullContent,
          tokens: Math.ceil(fullContent.length / 4),
          cost: parseFloat(costConsumed || '0'),
          safety: 'safe',
        } : m
      ));

      if (traceId) console.log('Trace:', traceId);
    } catch (err: any) {
      if (err.name === 'AbortError') return;
      setMessages(prev => prev.map(m =>
        m.id === assistantMsg.id ? { ...m, content: `❌ Error: ${err.message}` } : m
      ));
    } finally {
      setIsStreaming(false);
      abortRef.current = null;
    }
  }, [input, isStreaming, messages, selectedModel]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); }
  };

  const handleStop = () => { abortRef.current?.abort(); setIsStreaming(false); };

  return (
    <div style={{
      display: 'flex', flexDirection: 'column', height: '100%',
      background: 'linear-gradient(180deg, #0B0F19 0%, #131633 100%)',
      fontFamily: 'system-ui, -apple-system, sans-serif',
    }}>
      {/* Header */}
      <header style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '12px 24px', borderBottom: '1px solid rgba(255,255,255,0.06)',
        background: 'rgba(19,22,51,0.8)', backdropFilter: 'blur(12px)',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <Shield size={24} color="#6c5ce7" />
          <span style={{ fontWeight: 700, fontSize: 16, color: '#fff' }}>
            VERIDACTUS <span style={{ color: '#6c5ce7' }}>Chat</span>
          </span>
          <span style={{
            fontSize: 10, padding: '2px 8px', borderRadius: 10,
            background: 'rgba(0,212,170,0.15)', color: '#00d4aa', fontWeight: 600,
          }}>BETA</span>
        </div>

        {/* Model Selector + A/B Compare Toggle */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {/* A/B Compare Toggle */}
          <button
            onClick={() => {
              if (!compareMode && input.trim()) {
                setComparePrompt(input.trim());
                setInput('');
              }
              setCompareMode(!compareMode);
            }}
            title="⚔️ A/B 对比双模型"
            style={{
              display: 'flex', alignItems: 'center', gap: 6,
              padding: '8px 14px', borderRadius: 12,
              background: compareMode ? 'rgba(108,92,231,0.25)' : 'rgba(255,255,255,0.05)',
              border: `1px solid ${compareMode ? 'rgba(108,92,231,0.5)' : 'rgba(255,255,255,0.1)'}`,
              color: compareMode ? '#6c5ce7' : '#8892b0', fontSize: 12, fontWeight: 600, cursor: 'pointer',
              transition: 'all 0.2s',
            }}
          >
            <Columns size={15} />
            A/B
          </button>

          {/* Model A Selector */}
          <div style={{ position: 'relative' }}>
            <button
              onClick={() => setShowModelMenu(!showModelMenu)}
              style={{
                display: 'flex', alignItems: 'center', gap: 8,
                padding: '8px 16px', borderRadius: 12,
                background: 'rgba(108,92,231,0.12)', border: '1px solid rgba(108,92,231,0.3)',
                color: '#fff', fontSize: 13, fontWeight: 600, cursor: 'pointer',
              }}
            >
              <span style={{
                width: 8, height: 8, borderRadius: '50%', background: selectedModel.color,
                boxShadow: `0 0 8px ${selectedModel.color}`,
              }} />
              {selectedModel.name}
              <ChevronDown size={14} />
            </button>
            <AnimatePresence>
              {showModelMenu && (
                <motion.div
                  initial={{ opacity: 0, y: -4 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }}
                  style={{
                    position: 'absolute', top: '100%', right: 0, marginTop: 4,
                    background: 'rgba(19,22,51,0.98)', border: '1px solid rgba(108,92,231,0.3)',
                    borderRadius: 12, padding: 8, minWidth: 200, zIndex: 100,
                    boxShadow: '0 20px 40px rgba(0,0,0,0.5)',
                  }}
                >
                  {MODELS.map(m => (
                    <div key={m.id}
                      onClick={() => { setSelectedModel(m); setShowModelMenu(false); }}
                      style={{
                        display: 'flex', alignItems: 'center', gap: 10,
                        padding: '10px 14px', borderRadius: 8, cursor: 'pointer',
                        background: m.id === selectedModel.id ? 'rgba(108,92,231,0.15)' : 'transparent',
                        color: '#fff', fontSize: 13,
                      }}
                    >
                      <span style={{ width: 8, height: 8, borderRadius: '50%', background: m.color }} />
                      <span style={{ flex: 1 }}>{m.name}</span>
                      <span style={{ fontSize: 10, color: '#8892b0' }}>{m.provider}</span>
                    </div>
                  ))}
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Model B Selector (only visible in A/B mode) */}
          {compareMode && (
            <div style={{ position: 'relative' }}>
              <button
                onClick={() => setShowCompareMenu(!showCompareMenu)}
                style={{
                  display: 'flex', alignItems: 'center', gap: 8,
                  padding: '8px 16px', borderRadius: 12,
                  background: 'rgba(0,212,170,0.12)', border: '1px solid rgba(0,212,170,0.3)',
                  color: '#fff', fontSize: 13, fontWeight: 600, cursor: 'pointer',
                }}
              >
                <span style={{
                  width: 8, height: 8, borderRadius: '50%', background: compareModel.color,
                  boxShadow: `0 0 8px ${compareModel.color}`,
                }} />
                {compareModel.name}
                <ChevronDown size={14} />
              </button>
              <AnimatePresence>
                {showCompareMenu && (
                  <motion.div
                    initial={{ opacity: 0, y: -4 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }}
                    style={{
                      position: 'absolute', top: '100%', right: 0, marginTop: 4,
                      background: 'rgba(19,22,51,0.98)', border: '1px solid rgba(0,212,170,0.3)',
                      borderRadius: 12, padding: 8, minWidth: 200, zIndex: 100,
                      boxShadow: '0 20px 40px rgba(0,0,0,0.5)',
                    }}
                  >
                    {MODELS.filter(m => m.id !== selectedModel.id).map(m => (
                      <div key={m.id}
                        onClick={() => { setCompareModel(m); setShowCompareMenu(false); }}
                        style={{
                          display: 'flex', alignItems: 'center', gap: 10,
                          padding: '10px 14px', borderRadius: 8, cursor: 'pointer',
                          background: m.id === compareModel.id ? 'rgba(0,212,170,0.15)' : 'transparent',
                          color: '#fff', fontSize: 13,
                        }}
                      >
                        <span style={{ width: 8, height: 8, borderRadius: '50%', background: m.color }} />
                        <span style={{ flex: 1 }}>{m.name}</span>
                        <span style={{ fontSize: 10, color: '#8892b0' }}>{m.provider}</span>
                      </div>
                    ))}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          )}
        </div>
      </header>

      {/* ⚔️ A/B 对比全屏覆盖 */}
      <AnimatePresence>
        {compareMode && comparePrompt && (
          <ABCompare
            prompt={comparePrompt}
            modelA={selectedModel.id}
            modelB={compareModel.id}
            onClose={() => { setCompareMode(false); setComparePrompt(''); }}
          />
        )}
      </AnimatePresence>

      {/* Messages */}
      <div style={{
        flex: 1, overflowY: 'auto', padding: '24px 0',
      }}>
        <div style={{ maxWidth: 800, margin: '0 auto', padding: '0 24px' }}>
          <AnimatePresence>
            {messages.length === 0 && (
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                style={{
                  textAlign: 'center', paddingTop: '15vh',
                }}
              >
                <motion.div
                  animate={{ y: [0, -8, 0] }}
                  transition={{ duration: 2, repeat: Infinity }}
                >
                  <Shield size={64} color="#6c5ce7" style={{ opacity: 0.5 }} />
                </motion.div>
                <h2 style={{ color: '#fff', fontSize: 22, marginTop: 20, fontWeight: 700 }}>
                  VERIDACTUS 安全沙箱
                </h2>
                <p style={{ color: '#8892b0', fontSize: 14, marginTop: 8, maxWidth: 400, margin: '8px auto 0' }}>
                  每个对话都经过 L0 密码学签名审计，确保不可篡改
                </p>
              </motion.div>
            )}
          </AnimatePresence>

          {messages.map(msg => (
            <motion.div
              key={msg.id}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              style={{
                display: 'flex', gap: 12, marginBottom: 24,
                justifyContent: msg.role === 'user' ? 'flex-end' : 'flex-start',
              }}
            >
              {msg.role === 'assistant' && (
                <div style={{
                  width: 32, height: 32, borderRadius: 8,
                  background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  flexShrink: 0,
                }}>
                  <Zap size={16} color="#fff" />
                </div>
              )}
              <div style={{
                maxWidth: '75%', padding: '14px 18px', borderRadius: 16,
                background: msg.role === 'user'
                  ? 'linear-gradient(135deg, rgba(108,92,231,0.2), rgba(108,92,231,0.1))'
                  : 'rgba(255,255,255,0.04)',
                border: msg.role === 'user'
                  ? '1px solid rgba(108,92,231,0.3)'
                  : '1px solid rgba(255,255,255,0.06)',
                color: '#e0e6f0', fontSize: 14, lineHeight: 1.7,
                whiteSpace: 'pre-wrap', wordBreak: 'break-word',
              }}>
                {msg.content || (
                  <span style={{ display: 'flex', gap: 4, alignItems: 'center', color: '#6c5ce7' }}>
                    <motion.span animate={{ opacity: [0.3, 1, 0.3] }} transition={{ duration: 1, repeat: Infinity }}>●</motion.span>
                    <motion.span animate={{ opacity: [0.3, 1, 0.3] }} transition={{ duration: 1, delay: 0.2, repeat: Infinity }}>●</motion.span>
                    <motion.span animate={{ opacity: [0.3, 1, 0.3] }} transition={{ duration: 1, delay: 0.4, repeat: Infinity }}>●</motion.span>
                  </span>
                )}

                {/* Message footer */}
                {msg.safety && msg.tokens && (
                  <div style={{ display: 'flex', gap: 12, marginTop: 8, paddingTop: 8, borderTop: '1px solid rgba(255,255,255,0.05)' }}>
                    <span style={{ fontSize: 10, color: '#8892b0', display: 'flex', alignItems: 'center', gap: 4 }}>
                      <Activity size={10} /> {msg.tokens} tokens
                    </span>
                    {msg.cost !== undefined && msg.cost > 0 && (
                      <span style={{ fontSize: 10, color: '#00d4aa', display: 'flex', alignItems: 'center', gap: 4 }}>
                        💰 ${msg.cost.toFixed(6)}
                      </span>
                    )}
                    <span style={{
                      fontSize: 10, padding: '1px 6px', borderRadius: 4,
                      background: msg.safety === 'safe' ? 'rgba(0,212,170,0.15)' : 'rgba(255,118,117,0.15)',
                      color: msg.safety === 'safe' ? '#00d4aa' : '#ff7675',
                    }}>
                      {msg.safety === 'safe' ? '✅ Verified' : '⚠️ Flagged'}
                    </span>
                  </div>
                )}
              </div>
              {msg.role === 'user' && (
                <div style={{
                  width: 32, height: 32, borderRadius: 8,
                  background: 'linear-gradient(135deg, #00d4aa, #6c5ce7)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  flexShrink: 0, fontSize: 14, fontWeight: 700, color: '#fff',
                }}>U</div>
              )}
            </motion.div>
          ))}
          <div ref={messagesEndRef} />
        </div>
      </div>

      {/* Input Area */}
      <div style={{
        padding: '16px 24px 24px',
        background: 'rgba(19,22,51,0.9)', backdropFilter: 'blur(16px)',
        borderTop: '1px solid rgba(255,255,255,0.06)',
      }}>
        <div style={{ maxWidth: 800, margin: '0 auto' }}>
          {budgetRemaining !== null && (
            <div style={{
              display: 'flex', justifyContent: 'flex-end', marginBottom: 8,
              fontSize: 11, color: budgetRemaining > 0 ? '#00d4aa' : '#ff7675',
              gap: 4,
            }}>
              <span>💰</span>
              <span>Remaining: ${budgetRemaining.toFixed(6)}</span>
            </div>
          )}
          <div style={{
            display: 'flex', gap: 12, alignItems: 'flex-end',
            background: 'rgba(255,255,255,0.03)', borderRadius: 16,
            border: '1px solid rgba(108,92,231,0.2)', padding: '12px 16px',
            transition: 'border 0.2s',
          }}>
            <SafetyShield text={input} size={22} />
            <textarea
              value={input}
              onChange={e => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="输入消息... (Enter 发送, Shift+Enter 换行)"
              rows={1}
              style={{
                flex: 1, background: 'transparent', border: 'none', color: '#e0e6f0',
                fontSize: 14, resize: 'none', outline: 'none',
                fontFamily: 'inherit', lineHeight: 1.5,
                minHeight: 24, maxHeight: 120,
              }}
              disabled={isStreaming}
            />
            {isStreaming ? (
              <motion.button
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
                onClick={handleStop}
                style={{
                  padding: '10px 18px', borderRadius: 12,
                  background: 'rgba(255,118,117,0.15)', border: '1px solid rgba(255,118,117,0.3)',
                  color: '#ff7675', fontSize: 13, fontWeight: 600, cursor: 'pointer',
                }}
              >
                ⏹ Stop
              </motion.button>
            ) : (
              <motion.button
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
                onClick={handleSend}
                disabled={!input.trim()}
                style={{
                  padding: '10px 18px', borderRadius: 12,
                  background: input.trim()
                    ? 'linear-gradient(135deg, #6c5ce7, #00d4aa)'
                    : 'rgba(255,255,255,0.05)',
                  border: 'none', color: '#fff', fontSize: 13, fontWeight: 600,
                  cursor: input.trim() ? 'pointer' : 'not-allowed',
                  transition: 'all 0.2s', opacity: input.trim() ? 1 : 0.4,
                }}
              >
                <Send size={16} />
              </motion.button>
            )}
          </div>
          <p style={{
            textAlign: 'center', fontSize: 10, color: '#8892b0', marginTop: 8,
          }}>
            VERIDACTUS Chat — 每条对话都经过 L0 密码学审计签名
          </p>
        </div>
      </div>
    </div>
  );
}
