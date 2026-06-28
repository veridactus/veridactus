// Holo-Trace Vault — 审计中心：按 会话 分组展示 Traces
import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Search, ChevronRight, Sparkles, MessageSquare, Clock, Cpu, DollarSign, Activity, Shield, ChevronDown, List } from 'lucide-react';
import { getTracesFromCP } from '../../api';
import { getStoredToken } from '../../auth/useAuth';
import type { TraceSummary } from '../../types';

const SAFETY_COLORS: Record<string, string> = { safe: '#00d4aa', flagged: '#fdcb6e', blocked: '#ff7675' };
const STATE_CONFIG: Record<string, { label: string; color: string }> = {
  FINALIZED: { label: '已完成', color: '#00d4aa' },
  FAILED: { label: '失败', color: '#ff7675' },
  EXECUTING: { label: '进行中', color: '#74b9ff' },
  INIT: { label: '初始化', color: '#a29bfe' },
};

/** 格式化时间 HH:mm */
function fmtTime(iso: string): string {
  if (!iso) return '—';
  try { const d=new Date(iso); const pad=(n:number)=>String(n).padStart(2,'0'); return `${pad(d.getMonth()+1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`; }
  catch { return iso?.slice(0,16)||'—'; }
}

/** 按 session_id 分组 traces */
interface SessionGroup {
  sessionId: string;
  messages: TraceSummary[];
  firstTime: string;
  lastTime: string;
  model: string;
  totalTokens: number;
  totalCost: number;
  label: string; // 会话标签：优先用对话标题
}

function groupBySession(traces: TraceSummary[], convTitles: Map<string, string>): { sessions: SessionGroup[]; orphans: TraceSummary[] } {
  const map = new Map<string, TraceSummary[]>();
  const orphans: TraceSummary[] = [];
  for (const t of traces) {
    const sid = (t as any).session_id;
    if (sid) {
      if (!map.has(sid)) map.set(sid, []);
      map.get(sid)!.push(t);
    } else {
      orphans.push(t);
    }
  }
  const sessions: SessionGroup[] = [];
  for (const [sid, msgs] of map) {
    msgs.sort((a,b)=>new Date(a.created_at).getTime()-new Date(b.created_at).getTime());
    // 优先使用对话标题（从 conversations API 获取），否则用模型名兜底
    const title = convTitles.get(sid);
    const model = msgs[0]?.model||'';
    const label = title || (model ? `${model} · ${msgCount(msgs)}条消息` : `会话 · ${msgCount(msgs)}条消息`);
    sessions.push({
      sessionId: sid,
      messages: msgs,
      firstTime: msgs[0]?.created_at||'',
      lastTime: msgs[msgs.length-1]?.created_at||'',
      model,
      totalTokens: msgs.reduce((s,m)=>s+(m.tokens_count||0),0),
      totalCost: msgs.reduce((s,m)=>s+(m.cost_estimated_usd||0),0),
      label,
    });
  }
  sessions.sort((a,b)=>new Date(b.lastTime).getTime()-new Date(a.lastTime).getTime());
  orphans.sort((a,b)=>new Date(b.created_at).getTime()-new Date(a.created_at).getTime());
  return { sessions, orphans };

  function msgCount(msgs: TraceSummary[]) { return msgs.length; }
}

export default function VaultPage() {
  const [traces, setTraces] = useState<TraceSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [viewMode, setViewMode] = useState<'sessions' | 'traces'>('sessions');
  const [expandedSession, setExpandedSession] = useState<string | null>(null);
  /** session_id -> conversation title，从控制面拿 */
  const [convTitles, setConvTitles] = useState<Map<string,string>>(new Map());
  const navigate = useNavigate();

  useEffect(() => {
    const token = getStoredToken();
    Promise.all([
      getTracesFromCP()
        .then(t => { t.sort((a,b)=>new Date(b.created_at).getTime()-new Date(a.created_at).getTime()); setTraces(t); })
        .catch(()=>{}),
      // 拿对话标题：仅当已登录时
      token
        ? fetch('/api/v1/conversations', { headers: { Authorization: `Bearer ${token}` } })
            .then(r => r.ok ? r.json() : Promise.reject(r))
            .then(d => {
              const map = new Map<string,string>();
              for (const c of d.conversations || []) { map.set(c.id, c.title||''); }
              setConvTitles(map);
            })
            .catch(() => {})
        : Promise.resolve(),
    ]).finally(() => setLoading(false));
  }, []);

  const { sessions, orphans } = groupBySession(traces, convTitles);

  // 搜索过滤
  const filteredOrphans = orphans.filter(t =>
    t.trace_id?.includes(search) || t.model?.toLowerCase().includes(search.toLowerCase())
  );

  const filteredSessions = sessions.filter(s =>
    s.label?.toLowerCase().includes(search.toLowerCase()) ||
    s.sessionId?.includes(search) ||
    s.messages.some(m=>m.trace_id?.includes(search)||m.model?.toLowerCase().includes(search.toLowerCase()))
  );

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      {/* Header */}
      <div style={{ display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:24, flexWrap:'wrap', gap:12 }}>
        <div>
          <h1 style={{ fontSize:22, fontWeight:700, color:'#fff' }}>
            <Sparkles size={18} color="#6c5ce7" style={{ marginRight:8 }} /> Holo-Trace Vault
          </h1>
          <p style={{ color:'#8892b0', fontSize:13, marginTop:4 }}>
            审计中心 — {sessions.length} 个对话 · {orphans.length} 条独立 Trace
          </p>
        </div>
        <div style={{ display:'flex', gap:8, alignItems:'center' }}>
          {/* View toggle */}
          <div style={{ display:'flex', gap:0, background:'rgba(255,255,255,0.03)', borderRadius:10, padding:2, border:'1px solid rgba(255,255,255,0.08)' }}>
            {[['sessions','按对话'],['traces','按记录']].map(([k,label])=>(
              <button key={k} onClick={()=>setViewMode(k as any)}
                style={{
                  padding:'6px 14px', borderRadius:8, border:'none', cursor:'pointer',
                  fontSize:12, fontWeight:600,
                  background: viewMode===k?'rgba(108,92,231,0.2)':'transparent',
                  color: viewMode===k?'#fff':'#8892b0',
                  transition:'all 0.15s',
                }}>
                {label}
              </button>
            ))}
          </div>
          {/* Search */}
          <div style={{ display:'flex', gap:8, alignItems:'center', background:'rgba(255,255,255,0.03)', borderRadius:12, border:'1px solid rgba(255,255,255,0.08)', padding:'6px 14px' }}>
            <Search size={14} color="#8892b0"/>
            <input value={search} onChange={e=>setSearch(e.target.value)}
              placeholder="搜索…" style={{ background:'transparent', border:'none', color:'#e0e6f0', fontSize:12, outline:'none', width:150 }}/>
          </div>
        </div>
      </div>

      {loading ? (
        <div style={{ textAlign:'center', padding:60, color:'#8892b0' }}>加载中...</div>
      ) : viewMode === 'sessions' ? (
        /* ====== 会话视图 ====== */
        <div style={{ display:'flex', flexDirection:'column', gap:8 }}>
          {/* Table header */}
          <div style={{ display:'grid', gridTemplateColumns:'1.5fr 1fr 0.8fr 0.8fr 1fr 0.5fr', gap:12, padding:'10px 20px', fontSize:11, fontWeight:600, color:'#8892b0', textTransform:'uppercase' }}>
            <span>对话</span><span>模型</span><span>消息数</span><span>Tokens</span><span>最后活跃</span><span></span>
          </div>

          {filteredSessions.length === 0 && filteredOrphans.length === 0 && (
            <div style={{ textAlign:'center', padding:40, color:'#8892b0' }}>暂无 Trace 记录</div>
          )}

          {filteredSessions.map((s) => {
            const isExpanded = expandedSession === s.sessionId;
            const msgCount = s.messages.length;
            return (
              <div key={s.sessionId}>
                <motion.div
                  initial={{ opacity:0, x:-8 }} animate={{ opacity:1, x:0 }}
                  onClick={() => setExpandedSession(isExpanded ? null : s.sessionId)}
                  style={{
                    display:'grid', gridTemplateColumns:'1.5fr 1fr 0.8fr 0.8fr 1fr 0.5fr', gap:12,
                    padding:'14px 20px', borderRadius:12, cursor:'pointer',
                    background: isExpanded ? 'rgba(108,92,231,0.06)' : 'rgba(255,255,255,0.02)',
                    border: isExpanded ? '1px solid rgba(108,92,231,0.2)' : '1px solid rgba(255,255,255,0.05)',
                    color:'#e0e6f0', fontSize:13, transition:'all 0.15s',
                  }}
                  onMouseEnter={e => {if(!isExpanded){e.currentTarget.style.background='rgba(108,92,231,0.04)';e.currentTarget.style.borderColor='rgba(108,92,231,0.15)';}}}
                  onMouseLeave={e => {if(!isExpanded){e.currentTarget.style.background='rgba(255,255,255,0.02)';e.currentTarget.style.borderColor='rgba(255,255,255,0.05)';}}}
                >
                  <div style={{ display:'flex', alignItems:'center', gap:8 }}>
                    <MessageSquare size={14} color="#6c5ce7"/>
                    <span style={{ fontWeight:600 }}>{s.label}</span>
                  </div>
                  <span style={{ fontSize:12 }}>{s.model||'—'}</span>
                  <span style={{ color:'#a29bfe', fontWeight:600, fontSize:12 }}>{msgCount} 条</span>
                  <span style={{ fontSize:12, color:'#8892b0' }}>{s.totalTokens}</span>
                  <span style={{ display:'flex', alignItems:'center', gap:4, fontSize:12, color:'#8892b0' }}>
                    <Clock size={11}/> {fmtTime(s.lastTime)}
                  </span>
                  <ChevronDown size={14} style={{ color:'#8892b0', transform:isExpanded?'rotate(180deg)':'none', transition:'0.2s' }}/>
                </motion.div>

                {/* 展开的对话详情 */}
                {isExpanded && (
                  <div style={{ marginLeft:20, marginTop:4, marginBottom:8, display:'flex', flexDirection:'column', gap:4 }}>
                    {s.messages.map((msg, i) => {
                      const state = msg.execution_state?.toUpperCase?.()||'';
                      const stateCfg = STATE_CONFIG[state];
                      return (
                        <motion.div key={msg.trace_id||i} initial={{ opacity:0, y:-4 }} animate={{ opacity:1, y:0 }} transition={{ delay:i*0.03 }}
                          onClick={()=>navigate(`/vault/${msg.trace_id}`)}
                          style={{
                            display:'grid', gridTemplateColumns:'1.5fr 1fr 0.8fr 0.8fr 1fr 40px', gap:12,
                            padding:'10px 16px', borderRadius:10, cursor:'pointer',
                            background:'rgba(255,255,255,0.01)', border:'1px solid rgba(255,255,255,0.03)',
                            fontSize:12, color:'#a0aec0', transition:'all 0.1s',
                          }}
                          onMouseEnter={e=>{e.currentTarget.style.background='rgba(108,92,231,0.04)';e.currentTarget.style.borderColor='rgba(108,92,231,0.1)';}}
                          onMouseLeave={e=>{e.currentTarget.style.background='rgba(255,255,255,0.01)';e.currentTarget.style.borderColor='rgba(255,255,255,0.03)';}}
                        >
                          <span style={{ fontFamily:'monospace', fontSize:11, color:'#6c5ce7' }}>{msg.trace_id?.slice(0,12)}...</span>
                          <span>{msg.model||'—'}</span>
                          <span>{stateCfg ? <span style={{ fontSize:10, color:stateCfg.color }}>{stateCfg.label}</span> : state||'—'}</span>
                          <span><SafetyBadge status={msg.safety||'safe'}/></span>
                          <span style={{ fontSize:11, color:'#8892b0' }}>{fmtTime(msg.created_at)}</span>
                          <ChevronRight size={12} color="#8892b0"/>
                        </motion.div>
                      );
                    })}
                  </div>
                )}
              </div>
            );
          })}

          {/* 无 session 的独立 traces */}
          {filteredOrphans.length > 0 && (
            <div style={{ marginTop:16 }}>
              <div style={{ fontSize:12, color:'#8892b0', fontWeight:600, marginBottom:8, padding:'0 4px' }}>
                <List size={12} style={{ verticalAlign:-2, marginRight:4 }}/> 独立 Trace（无关联对话）
              </div>
              {filteredOrphans.slice(0,30).map((msg) => {
                const state = msg.execution_state?.toUpperCase?.()||'';
                const stateCfg = STATE_CONFIG[state];
                return (
                  <motion.div key={msg.trace_id} initial={{ opacity:0 }} animate={{ opacity:1 }}
                    onClick={()=>navigate(`/vault/${msg.trace_id}`)}
                    style={{
                      display:'grid', gridTemplateColumns:'1.5fr 1fr 0.8fr 0.8fr 1fr 40px', gap:12,
                      padding:'10px 16px', borderRadius:10, cursor:'pointer', marginBottom:4,
                      background:'rgba(255,255,255,0.01)', border:'1px solid rgba(255,255,255,0.03)',
                      fontSize:12, color:'#a0aec0', transition:'all 0.1s',
                    }}
                    onMouseEnter={e=>{e.currentTarget.style.background='rgba(108,92,231,0.04)';e.currentTarget.style.borderColor='rgba(108,92,231,0.1)';}}
                    onMouseLeave={e=>{e.currentTarget.style.background='rgba(255,255,255,0.01)';e.currentTarget.style.borderColor='rgba(255,255,255,0.03)';}}
                  >
                    <span style={{ fontFamily:'monospace', fontSize:11, color:'#6c5ce7' }}>{msg.trace_id?.slice(0,12)}...</span>
                    <span>{msg.model||'—'}</span>
                    <span>{stateCfg ? <span style={{ fontSize:10, color:stateCfg.color }}>{stateCfg.label}</span> : state||'—'}</span>
                    <span><SafetyBadge status={msg.safety||'safe'}/></span>
                    <span style={{ fontSize:11, color:'#8892b0' }}>{fmtTime(msg.created_at)}</span>
                    <ChevronRight size={12} color="#8892b0"/>
                  </motion.div>
                );
              })}
            </div>
          )}
        </div>
      ) : (
        /* ====== 单条 Trace 视图 ====== */
        <div style={{ display:'flex', flexDirection:'column', gap:8 }}>
          <div style={{ display:'grid', gridTemplateColumns:'1.5fr 1fr 0.8fr 0.8fr 1fr 1fr 40px', gap:12, padding:'10px 20px', fontSize:11, fontWeight:600, color:'#8892b0', textTransform:'uppercase' }}>
            <span>Trace ID</span><span>🤖 Model</span><span>📊 状态</span><span>🛡️ 安全</span><span>💰 费用</span><span>🕐 时间</span><span></span>
          </div>
          {filteredOrphans.map((trace, idx) => {
            const state = trace.execution_state?.toUpperCase?.()||'';
            const stateCfg = STATE_CONFIG[state];
            return (
              <motion.div key={trace.trace_id||idx} initial={{ opacity:0, x:-8 }} animate={{ opacity:1, x:0 }} transition={{ delay:idx*0.02 }}
                onClick={()=>navigate(`/vault/${trace.trace_id}`)}
                style={{ display:'grid', gridTemplateColumns:'1.5fr 1fr 0.8fr 0.8fr 1fr 1fr 40px', gap:12, padding:'14px 20px', borderRadius:12, cursor:'pointer', background:'rgba(255,255,255,0.02)', border:'1px solid rgba(255,255,255,0.05)', color:'#e0e6f0', fontSize:13, transition:'all 0.15s' }}
                onMouseEnter={e=>{e.currentTarget.style.background='rgba(108,92,231,0.06)';e.currentTarget.style.borderColor='rgba(108,92,231,0.2)';}}
                onMouseLeave={e=>{e.currentTarget.style.background='rgba(255,255,255,0.02)';e.currentTarget.style.borderColor='rgba(255,255,255,0.05)';}}
              >
                <span style={{ fontFamily:'monospace', fontSize:12, color:'#6c5ce7' }}>{trace.trace_id?.slice(0,12)}...</span>
                <span>{trace.model||'—'}</span>
                <span>{stateCfg ? <span style={{ fontSize:10, padding:'2px 8px', borderRadius:6, fontWeight:600, background:`${stateCfg.color}15`, color:stateCfg.color, border:`1px solid ${stateCfg.color}30` }}>{stateCfg.label}</span> : state||'—'}</span>
                <span><SafetyBadge status={trace.safety||'safe'}/></span>
                <span style={{ color:'#00d4aa', fontFamily:'monospace', fontSize:12 }}>${(trace.cost_estimated_usd||0).toFixed(6)}</span>
                <span style={{ color:'#8892b0', fontSize:11 }}>{fmtTime(trace.created_at)}</span>
                <ChevronRight size={16} color="#8892b0"/>
              </motion.div>
            );
          })}
        </div>
      )}
    </motion.div>
  );
}

function SafetyBadge({ status }: { status: string }) {
  const color = SAFETY_COLORS[status] || '#8892b0';
  return <span style={{ fontSize:10, padding:'2px 8px', borderRadius:6, fontWeight:600, background:`${color}18`, color, border:`1px solid ${color}30` }}>{status==='safe'?'🟢 Safe':status==='flagged'?'🟡 Flagged':status==='blocked'?'🔴 Blocked':status}</span>;
}
