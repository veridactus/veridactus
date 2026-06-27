// Holo-Trace Vault — 详情页：Input/Output 分屏 + 多轮对话上下文
import { useState, useEffect, useRef, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { ArrowLeft, Cpu, Clock, DollarSign, Activity, MessageSquare, ChevronDown } from 'lucide-react';
import { getTraceDetail } from '../../api';
import type { TraceDetail } from '../../types';
import CryptoVerify from './CryptoVerify';

/** 会话中的单条 Trace（用于多轮对话列表） */
interface SessionTrace {
  trace_id: string;
  role: 'user' | 'assistant';
  content: string;
  model?: string;
  tokens?: number;
  cost?: number;
  created_at: string;
  execution_state: string;
}
/** 格式化时间为 YYYY-MM-DD HH:mm */
function fmtTime(iso: string): string {
  if (!iso) return '—';
  try {
    const d = new Date(iso);
    const pad = (n:number) => String(n).padStart(2,'0');
    return `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  } catch { return iso?.slice(0,16) || '—'; }
}

/** 从 Trace 数据中提取消息内容（支持多种 input/output 格式） */
function extractContent(trace: any, type: 'input' | 'output'): string {
  try {
    if (type === 'input') {
      const prompt = trace.input?.prompt || trace.input;
      if (Array.isArray(prompt)) {
        // messages 数组：取最后一条 user 消息
        const msgs = prompt.slice().reverse();
        const userMsg = msgs.find((m: any) => m.role === 'user');
        return userMsg?.content || JSON.stringify(prompt);
      }
      return typeof prompt === 'string' ? prompt : JSON.stringify(prompt, null, 2);
    } else {
      const resp = trace.output?.response;
      if (!resp) return '';
      if (typeof resp === 'string') return resp;
      // 处理 streaming_content / choices 等格式
      if (resp.streaming_content) return resp.streaming_content;
      if (resp.choices?.[0]?.message?.content) return resp.choices[0].message.content;
      const txt = resp.choices?.[0]?.delta?.content || resp.choices?.[0]?.text || '';
      if (txt) return txt;
      return JSON.stringify(resp, null, 2);
    }
  } catch { return ''; }
}

export default function VaultDetail() {
  const { traceId } = useParams<{ traceId: string }>();
  const navigate = useNavigate();
  const [trace, setTrace] = useState<TraceDetail | null>(null);
  const [rawTrace, setRawTrace] = useState<Record<string,any> | null>(null);
  const [sessionTraces, setSessionTraces] = useState<SessionTrace[]>([]);
  const [sessionExpanded, setSessionExpanded] = useState(true);
  const [loading, setLoading] = useState(true);

  const leftRef = useRef<HTMLDivElement>(null);
  const rightRef = useRef<HTMLDivElement>(null);
  const syncing = useRef(false);

  useEffect(() => {
    if (!traceId) return;
    Promise.all([
      getTraceDetail(traceId),
      fetch('/v1/traces/' + traceId).then(r=>r.json()),
    ]).then(([detail, raw]) => {
      setTrace(detail);
      setRawTrace(raw);
      // 加载同 session 的其他 trace
      const sid = raw?.session_id;
      if (sid) {
        fetch('/v1/traces?session_id=' + sid).then(r=>r.json()).then(d => {
          const traces = (d.traces || []).sort(
            (a:any,b:any) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
          );
          const msgs: SessionTrace[] = [];
          for (const t of traces) {
            const inp = extractContent(t, 'input');
            const out = extractContent(t, 'output');
            if (inp) msgs.push({
              trace_id: t.trace_id, role: 'user',
              content: inp, model: t.model,
              tokens: t.tokens_count || t.observations?.token_count,
              cost: t.cost_estimated_usd || t.observations?.cost_usd,
              created_at: t.created_at,
              execution_state: typeof t.execution_state === 'string' ? t.execution_state : '',
            });
            if (out) msgs.push({
              trace_id: t.trace_id, role: 'assistant',
              content: out, model: t.model,
              tokens: t.tokens_count || t.observations?.token_count,
              cost: t.cost_estimated_usd || t.observations?.cost_usd,
              created_at: t.created_at,
              execution_state: typeof t.execution_state === 'string' ? t.execution_state : '',
            });
          }
          setSessionTraces(msgs);
        }).catch(() => {});
      }
    }).catch(() => {}).finally(() => setLoading(false));
  }, [traceId]);

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
  const hasOutput = trace.output?.response && outputText !== '—' && outputText !== '{}';

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 24, flexWrap: 'wrap' }}>
        <motion.button whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}
          onClick={() => navigate('/vault')}
          style={{ padding: '10px 14px', borderRadius: 10, background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.1)', color: '#e0e6f0', cursor: 'pointer' }}>
          <ArrowLeft size={18} />
        </motion.button>
        <div style={{ flex: 1 }}>
          <h1 style={{ fontSize: 20, fontWeight: 700, color: '#fff' }}>
            Trace <span style={{ color: '#6c5ce7', fontFamily: 'monospace' }}>{traceId?.slice(0, 16)}...</span>
          </h1>
          <div style={{ display: 'flex', gap: 16, marginTop: 4, fontSize: 12, color: '#8892b0', flexWrap: 'wrap' }}>
            <span><Cpu size={12} style={{ verticalAlign: -1 }} /> {trace.model}</span>
            <span><Clock size={12} style={{ verticalAlign: -1 }} /> {fmtTime(trace.created_at)}</span>
            <span><DollarSign size={12} style={{ verticalAlign: -1 }} /> ${(trace.cost_estimated_usd || 0).toFixed(6)}</span>
            <span><Activity size={12} style={{ verticalAlign: -1 }} /> {trace.tokens_count || 0} tokens</span>
            {rawTrace?.session_id && (
              <span style={{ fontFamily:'monospace', fontSize:11, color:'#a29bfe' }}>
                session: {rawTrace.session_id.slice(0,12)}...
              </span>
            )}
          </div>
        </div>
      </div>

      {/* 多轮对话上下文（同 session 的所有消息） */}
      {sessionTraces.length > 1 && (
        <div style={{
          background: 'rgba(108,92,231,0.04)', border: '1px solid rgba(108,92,231,0.12)',
          borderRadius: 16, padding: 16, marginBottom: 20,
        }}>
          <button onClick={()=>setSessionExpanded(!sessionExpanded)}
            style={{
              display:'flex', alignItems:'center', gap:8, width:'100%', background:'none', border:'none',
              color:'#e0e6f0', cursor:'pointer', fontSize:13, fontWeight:600, padding:0, marginBottom: sessionExpanded?12:0,
            }}>
            <MessageSquare size={14} color="#6c5ce7"/>
            <span>会话对话历史</span>
            <span style={{ fontSize:11, color:'#8892b0' }}>（{sessionTraces.length} 条消息，{Math.ceil(sessionTraces.length/2)} 轮交互）</span>
            <ChevronDown size={14} style={{ marginLeft:'auto', transform:sessionExpanded?'rotate(180deg)':'none', transition:'0.2s' }} />
          </button>
          {sessionExpanded && (
            <div style={{ display:'flex', flexDirection:'column', gap:8, maxHeight:360, overflowY:'auto' }}>
              {sessionTraces.map((msg, i) => (
                <div key={msg.trace_id + i} style={{
                  display:'flex', gap:10, padding:'8px 12px', borderRadius:10,
                  background: msg.trace_id === traceId ? 'rgba(108,92,231,0.1)' : 'rgba(255,255,255,0.02)',
                  border: msg.trace_id === traceId ? '1px solid rgba(108,92,231,0.3)' : '1px solid rgba(255,255,255,0.04)',
                }}>
                  <span style={{
                    width:22,height:22, borderRadius:6, display:'flex', alignItems:'center', justifyContent:'center',
                    flexShrink:0, fontSize:10,
                    background: msg.role==='user'?'rgba(0,212,170,0.15)':'rgba(108,92,231,0.12)',
                    color: msg.role==='user'?'#00d4aa':'#6c5ce7',
                  }}>
                    {msg.role==='user'?'Q':'A'}
                  </span>
                  <div style={{ flex:1, minWidth:0 }}>
                    <div style={{ display:'flex', alignItems:'center', gap:8, marginBottom:2 }}>
                      <span style={{ fontSize:10, color: msg.role==='user'?'#00d4aa':'#6c5ce7', fontWeight:600 }}>
                        {msg.role==='user'?'用户':'AI'}
                      </span>
                      {msg.trace_id === traceId && (
                        <span style={{ fontSize:9, color:'#6c5ce7', background:'rgba(108,92,231,0.15)', padding:'1px 6px', borderRadius:4 }}>当前查看</span>
                      )}
                      <span style={{ fontSize:9, color:'#4a5568', marginLeft:'auto' }}>{msg.tokens||0}t</span>
                    </div>
                    <div style={{ fontSize:12, color:'#a0aec0', lineHeight:1.5, whiteSpace:'pre-wrap', wordBreak:'break-word',
                      maxHeight:80, overflow:'hidden', position:'relative' }}>
                      {msg.content?.slice(0, 200)}
                      {msg.content?.length > 200 && (
                        <span style={{ position:'absolute', bottom:0, right:0, background:'linear-gradient(90deg,transparent,rgba(15,19,38,0.9))', paddingLeft:20 }}>
                          ...
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Split-screen: Input | Output */}
      <div style={{
        display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16,
        marginBottom: 24, minHeight: 300,
      }}>
        {/* Left: Input */}
        <div style={{
          background: 'rgba(255,255,255,0.02)', border: '1px solid rgba(255,255,255,0.06)',
          borderRadius: 16, overflow: 'hidden', display: 'flex', flexDirection: 'column',
        }}>
          <div style={{
            padding: '12px 16px', borderBottom: '1px solid rgba(255,255,255,0.05)',
            fontWeight: 600, fontSize: 13, color: '#e0e6f0',
            display: 'flex', justifyContent: 'space-between',
          }}>
            <span>📥 请求 Input</span>
            <span style={{ fontSize: 10, color: '#8892b0' }}>原始请求数据</span>
          </div>
          <div ref={leftRef} onScroll={() => handleScroll('left')} style={{
            flex: 1, overflow: 'auto', padding: 16,
            fontFamily: 'monospace', fontSize: 12, color: '#e0e6f0',
            lineHeight: 1.6, whiteSpace: 'pre-wrap', wordBreak: 'break-all',
          }}>
            {inputText}
          </div>
        </div>

        {/* Right: Output */}
        <div style={{
          background: hasOutput ? 'rgba(0,212,170,0.03)' : 'rgba(255,255,255,0.02)',
          border: hasOutput ? '1px solid rgba(0,212,170,0.15)' : '1px solid rgba(255,255,255,0.06)',
          borderRadius: 16, overflow: 'hidden', display: 'flex', flexDirection: 'column',
        }}>
          <div style={{
            padding: '12px 16px', borderBottom: hasOutput ? '1px solid rgba(0,212,170,0.1)' : '1px solid rgba(255,255,255,0.05)',
            fontWeight: 600, fontSize: 13, color: '#e0e6f0',
            display: 'flex', justifyContent: 'space-between',
          }}>
            <span>📤 响应 Output</span>
            <span style={{ fontSize: 10, color: hasOutput ? '#00d4aa' : '#8892b0' }}>
              {hasOutput ? 'LLM 响应' : '无输出（流式未捕获或处理中）'}
            </span>
          </div>
          <div ref={rightRef} onScroll={() => handleScroll('right')} style={{
            flex: 1, overflow: 'auto', padding: 16,
            fontFamily: 'monospace', fontSize: 12, color: '#e0e6f0',
            lineHeight: 1.6, whiteSpace: 'pre-wrap', wordBreak: 'break-all',
          }}>
            {outputText}
          </div>
        </div>
      </div>

      {/* Crypto Verify */}
      {rawTrace && <CryptoVerify trace={rawTrace}
        auditSignature={rawTrace.proofs?.proof_chain?.find((p:any)=>p.level==='L0')?.signature} />}
    </motion.div>
  );
}
