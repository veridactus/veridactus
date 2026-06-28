// VERIDACTUS Chat — 专业多会话安全沙箱（业界标准 UI）
import { useState, useRef, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Send, Shield, Zap, ChevronDown, Plus, Trash2, MessageSquare, PanelLeftClose, PanelLeft, CheckCircle, AlertTriangle, DollarSign, Fingerprint, Activity, ShieldCheck } from 'lucide-react';
import SafetyShield from './SafetyShield';
import { getStoredToken } from '../../auth/useAuth';
import { getWorkspaceId } from '../../auth/AuthGuard';
import { useNavSidebarStore } from '../../store';

// ==================== Types ====================
interface PipelineTrace {
  traceId: string;
  proofLevels: string[];
  costConsumed: number;
  budgetRemaining?: number;
  version: string;
  checksPassed?: string[];
}
interface Message {
  id: string; role: 'user'|'assistant'; content: string; model?: string; tokens?: number; timestamp: number;
  pipeline?: PipelineTrace;
}
interface AvailableModel { id: string; name: string; provider: string; color: string; is_default: boolean; }
interface Conversation { id: string; title: string; model: string; created_at: string; updated_at: string; }
/** 可选的流水线（有名称便于识别） */
interface PipelineOption { plan_id: string; name: string; stages: number; status: string; }

// ==================== Constants ====================
const MODEL_COLORS = ['#6c5ce7','#00d4aa','#74b9ff','#fdcb6e','#ff7675','#a29bfe','#fd79a8','#00cec9'];
const FALLBACK: AvailableModel[] = [{ id:'glm-5.1', name:'GLM-5.1', provider:'Zhipu', color:'#6c5ce7', is_default:true }];
const CACHE_MSGS = 'v_msgs', CACHE_CONV = 'v_conv', CACHE_STREAM = 'v_stream';
// DP 治理模式 API Key（与 .env 中 VERIDACTUS_ADMIN_KEY 一致）
const DP_API_KEY = 'veridactus-admin-dev-2026';

function gid(){ return Date.now().toString(36)+Math.random().toString(36).slice(2); }
/** 获取当前用户 ID 后缀，用于隔离多用户 localStorage 缓存 */
function uidSuffix(): string {
  try { const t = getStoredToken(); if (!t) return ''; const p = JSON.parse(atob(t.split('.')[1])); return '_' + (p.sub || ''); } catch { return ''; }
}
function load<T>(k:string,fallback:T):T{try{const r=localStorage.getItem(k+uidSuffix());return r?JSON.parse(r):fallback}catch{return fallback}}
function save(k:string,v:any){try{localStorage.setItem(k+uidSuffix(),JSON.stringify(v))}catch{}}

async function fetchModels(): Promise<AvailableModel[]> {
  try{const r=await fetch('/models');if(r.ok){const d=await r.json();return(d.data||[]).map((m:any,i:number)=>({id:m.id,name:m.id,provider:m.owned_by||'',color:MODEL_COLORS[i%8],is_default:!!m.is_default}));}}catch{}
  try{const r=await fetch('/api/v1/models');if(r.ok){const d=await r.json();return(d.models||[]).map((m:any,i:number)=>({id:m.name,name:m.name,provider:'',color:MODEL_COLORS[i%8],is_default:!!m.is_default}));}}catch{}
  return FALLBACK;
}

/** 获取已发布的流水线列表（用于选择器）需要 JWT 鉴权 */
async function fetchPipelines(token: string): Promise<PipelineOption[]> {
  const headers: Record<string,string> = token ? { Authorization: `Bearer ${token}` } : {};
  try{const r=await fetch('/api/v1/pipelines',{headers});if(r.ok){const d=await r.json();
    return (d.pipelines||[]).filter((p:any)=>p.status==='published'||p.status==='active').map((p:any)=>({
      plan_id:p.plan_id, name:p.name||p.plan_id?.slice(0,8)||'Unnamed', stages:p.stages?.length||0, status:p.status
    }));}}catch{}
  return [];
}

// ==================== Component ====================
export default function ChatPage() {
  // 恢复流式中断的消息
  const [messages, setMessages] = useState<Message[]>(()=>{
    const ms=load<Message[]>(CACHE_MSGS,[]);
    const st=load<any>(CACHE_STREAM,null);
    // 如果上次流式未完成，标记最后一条 assistant 消息为中断
    if(st&&ms.length>0){
      const last=ms[ms.length-1];
      if(last.role==='assistant'&&!last.pipeline){
        last.content=(last.content||'')+' ⚡ (流式中断，刷新后继续)';
      }
    }
    return ms;
  });
  const [input, setInput] = useState('');
  const [models, setModels] = useState<AvailableModel[]>(FALLBACK);
  const [selModel, setSelModel] = useState<AvailableModel>(FALLBACK[0]);
  const [streaming, setStreaming] = useState(false);
  const [showModels, setShowModels] = useState(false);
  const [convs, setConvs] = useState<Conversation[]>([]);
  const [activeId, setActiveId] = useState<string|null>(()=>load<string|null>(CACHE_CONV,null));
  const [sidebar, setSidebar] = useState(true);
  // 同步 activeId 到 ref（send 函数始终能读到最新值）
  useEffect(()=>{ activeIdRef.current = activeId; }, [activeId]);

  // 流水线选择器
  const [pipelines, setPipelines] = useState<PipelineOption[]>([]);
  const [selPipeline, setSelPipeline] = useState<PipelineOption|null>(null);
  const [showPipelines, setShowPipelines] = useState(false);
  const { navCollapsed, toggleNav } = useNavSidebarStore();
  const [hoverDel, setHoverDel] = useState<string|null>(null);
  const endRef = useRef<HTMLDivElement>(null);
  const abortRef = useRef<AbortController|null>(null);
  const modelsRef = useRef(models); modelsRef.current = models;
  const creatingRef = useRef(false);
  const activeIdRef = useRef<string|null>(null);
  const token = getStoredToken()||'';

  // Headers as plain Record to satisfy TypeScript
  const authHeaders: Record<string,string> = token ? { Authorization: `Bearer ${token}` } : {};
  const ctHeaders: Record<string,string> = token ? { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' } : { 'Content-Type': 'application/json' };

  useEffect(()=>{fetchModels().then(m=>{setModels(m);setSelModel(m.find(x=>x.is_default)||m[0]);}); fetchPipelines(token).then(ps=>{setPipelines(ps);if(ps.length>0)setSelPipeline(ps[0]);});},[token]);
  useEffect(()=>{
    if(!token)return;
    fetch('/api/v1/conversations',{headers:authHeaders}).then(r=>r.json()).then(d=>d.conversations&&setConvs(d.conversations)).catch(()=>{});
  },[token]);
  useEffect(()=>{endRef.current?.scrollIntoView({behavior:'smooth'});save(CACHE_MSGS,messages.slice(-80));},[messages]);
  // 流式状态持久化：streaming 时保存标记，完成时清除
  useEffect(()=>{if(streaming)save(CACHE_STREAM,{ts:Date.now()});else localStorage.removeItem(CACHE_STREAM);},[streaming]);

  const newChat = useCallback(async()=>{
    if(streaming||creatingRef.current)return;
    setMessages([]);setActiveId(null);save(CACHE_CONV,null);save(CACHE_MSGS,[]);
    // 不提前创建会话 — 第一条消息发送时自动创建（避免空对话）
  },[streaming]);

  const switchConv = useCallback(async(c:Conversation)=>{
    if(streaming)return;setActiveId(c.id);save(CACHE_CONV,c.id);
    const r=await fetch('/api/v1/conversations/'+c.id,{headers:authHeaders});
    if(!r.ok)return;const d=await r.json();
    const ms=d.messages.map((m:any)=>({id:m.id,role:m.role,content:m.content,model:m.model||'',tokens:m.tokens||0,timestamp:Date.now()}));
    setMessages(ms);save(CACHE_MSGS,ms);
    if(d.conversation?.model){const fm=modelsRef.current.find(x=>x.id===d.conversation.model);if(fm)setSelModel(fm);}
  },[streaming,token]);

  const delConv = useCallback(async(id:string,e:React.MouseEvent)=>{
    e.stopPropagation();if(!token)return;
    await fetch('/api/v1/conversations/'+id,{method:'DELETE',headers:authHeaders});
    setConvs(p=>p.filter(x=>x.id!==id));if(activeId===id){setMessages([]);setActiveId(null);save(CACHE_CONV,null);save(CACHE_MSGS,[]);}
  },[token,activeId]);

  const send = useCallback(async()=>{
    if(!input.trim()||streaming)return;const uc=input.trim();setInput('');
    const u:Message={id:gid(),role:'user',content:uc,timestamp:Date.now()};
    const a:Message={id:gid(),role:'assistant',content:'',model:selModel.id,timestamp:Date.now()};
    setMessages(p=>[...p,u,a]);setStreaming(true);

    let cid=activeIdRef.current; // 用 ref 避免闭包旧值
    // 确保有活跃会话：没有则自动创建
    if(!cid&&token&&!creatingRef.current){
      creatingRef.current=true;
      const title=uc.length>30?uc.slice(0,30)+'...':uc;
      try{
        const r=await fetch('/api/v1/conversations',{method:'POST',headers:ctHeaders,body:JSON.stringify({title,model:selModel.id})});
        if(r.ok){const c=await r.json();cid=c.id;setActiveId(cid);save(CACHE_CONV,cid);
          fetch('/api/v1/conversations',{headers:authHeaders}).then(r2=>r2.json()).then(d=>d.conversations&&setConvs(d.conversations)).catch(()=>{});}
        else{console.error('创建会话失败:',r.status);}
      }catch(e){console.error('创建会话异常:',e);}
      finally{creatingRef.current=false;}
    }
    // 保存用户消息（需有 token 和会话 ID）
    if(cid&&token){
      fetch('/api/v1/conversations/'+cid+'/messages',{method:'POST',headers:ctHeaders,body:JSON.stringify({id:u.id,role:'user',content:uc,model:selModel.id,tokens:0,timestamp:u.timestamp})})
        .catch(e=>console.error('保存用户消息失败:',e));
    } else if(!token) {
      console.warn('⚠️ 未登录，消息不会保存到服务端');
    }

    // VERIDACTUS 治理协议头 — 触发 DP 完整 pipeline 执行
    const govHeaders: Record<string,string> = {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${DP_API_KEY}`,
      'VERIDACTUS-Version': '0.2',
      'VERIDACTUS-Budget-Limit': '0.50',
      'VERIDACTUS-Privacy-Level': 'standard',
      'VERIDACTUS-Workspace-Id': getWorkspaceId(), // 多租户隔离：trace 关联到正确 workspace
    };
    // 如果选择了特定流水线，告知 DP
    if(selPipeline){ govHeaders['VERIDACTUS-Pipeline-Id'] = selPipeline.plan_id; }
    // 将会话 ID 传给数据面，用于 Trace 关联
    if(cid){ govHeaders['VERIDACTUS-Session-Id'] = cid; }

    const ctrl=new AbortController();abortRef.current=ctrl;
    try{
      const res=await fetch('/v1/chat/completions',{method:'POST',signal:ctrl.signal,headers:govHeaders,
        body:JSON.stringify({model:selModel.id,messages:messages.concat(u).map(m=>({role:m.role,content:m.content})),stream:true,max_tokens:4096})});
      if(!res.ok){
        const errBody=await res.json().catch(()=>({}));
        const errMsg=errBody?.error?.message||`HTTP ${res.status}`;
        throw new Error(errMsg);
      }
      // 捕获 VERIDACTUS 响应头
      const pipeTrace: PipelineTrace = {
        traceId: res.headers.get('VERIDACTUS-Trace-Id')||'',
        proofLevels: (res.headers.get('VERIDACTUS-Proof-Levels')||'').split(',').filter(Boolean),
        costConsumed: parseFloat(res.headers.get('VERIDACTUS-Cost-Consumed')||'0'),
        budgetRemaining: parseFloat(res.headers.get('VERIDACTUS-Budget-Remaining')||'0')||undefined,
        version: res.headers.get('VERIDACTUS-Version')||'',
      };
      const reader=res.body?.getReader();if(!reader)throw new Error('No reader');
      const dec=new TextDecoder();let fc='',df=false;
      while(true){const{value,done}=await reader.read();if(done)break;
        for(const line of dec.decode(value,{stream:true}).split('\n')){const d=line.replace(/^(data: )+/,'').trim();if(!d)continue;
          if(d==='[DONE]'){df=true;break;}if(d.startsWith('[BUDGET')){fc+='\n⚠️ 预算耗尽';df=true;break;}
          try{fc+=JSON.parse(d).choices?.[0]?.delta?.content||'';}catch{}}if(df)break;setMessages(p=>p.map(m=>m.id===a.id?{...m,content:fc}:m));}
      const tk=Math.ceil(fc.length/4);
      setMessages(p=>p.map(m=>m.id===a.id?{...m,content:fc,tokens:tk,pipeline:pipeTrace}:m));
      if(cid&&token){
        fetch('/api/v1/conversations/'+cid+'/messages',{method:'POST',headers:ctHeaders,
          body:JSON.stringify({id:a.id,role:'assistant',content:fc,model:selModel.id,tokens:tk,timestamp:Date.now()})})
          .catch(e=>console.error('保存助手消息失败:',e));
        // 标题用首条输入命名，不更新（首次创建时已设置）
        setConvs(p=>p.map(c=>c.id===cid?{...c,updated_at:new Date().toISOString()}:c));
      }
    }catch(e:any){if(e.name!=='AbortError')setMessages(p=>p.map(m=>m.id===a.id?{...m,content:`❌ ${e.message}`}:m));}
    finally{setStreaming(false);abortRef.current=null;}
  },[input,streaming,messages,selModel,activeId,token,selPipeline]);

  const kd=(e:React.KeyboardEvent)=>{if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();send();}};

  return (
    <div className="flex h-full font-sans antialiased" style={{background:'#0B0F19'}}>
      {/* === Sidebar (ChatGPT-style collapsible) === */}
      <div className={`flex-shrink-0 border-r border-white/[0.06] bg-[#0a0e1a] flex flex-col h-full transition-all duration-300 overflow-hidden ${sidebar?'w-[260px]':'w-0 border-r-0'}`}>
        <div className="flex items-center justify-between p-3 pb-2">
          <button onClick={newChat} className="flex-1 flex items-center gap-2.5 py-2.5 px-3 rounded-xl bg-white/[0.06] hover:bg-white/[0.1] border border-white/[0.08] text-white text-[13px] font-medium cursor-pointer transition-all duration-150">
            <Plus size={15}/> 新建对话
          </button>
          <button onClick={toggleNav} className="ml-1.5 p-1.5 rounded-lg hover:bg-white/[0.06] text-[#5a6a8a] hover:text-white transition-colors flex-shrink-0" title={navCollapsed?'展开导航':'收起导航'}>
            {navCollapsed?<PanelLeft size={14}/>:<PanelLeftClose size={14}/>}
          </button>
        </div>
        <div className="flex-1 overflow-y-auto px-2 pb-2 space-y-0.5">
          {convs.length===0&&(<p className="text-center text-[11px] text-[#4a5568] mt-12 px-4 leading-relaxed">暂无对话历史</p>)}
          {convs.map(c=>(
            <div key={c.id} onClick={()=>switchConv(c)} onMouseEnter={()=>setHoverDel(c.id)} onMouseLeave={()=>setHoverDel(null)}
              className={`group flex items-center gap-2.5 py-2 px-3 rounded-lg cursor-pointer transition-all duration-150 select-none ${
                c.id===activeId?'bg-white/[0.06] text-white':'text-[#8892b0] hover:bg-white/[0.03] hover:text-[#c8d2e0]'}`}>
              <MessageSquare size={14} className="flex-shrink-0" style={{opacity:c.id===activeId?0.7:0.35}}/>
              <span className="flex-1 truncate text-[12px] leading-tight">{c.title}</span>
              {hoverDel===c.id&&<Trash2 size={13} onClick={(e)=>delConv(c.id,e)} className="cursor-pointer text-[#4a5568] hover:text-[#ff7675] transition-colors flex-shrink-0"/>}
            </div>
          ))}
        </div>
      </div>

      {/* === Main Chat Area === */}
      <div className="flex-1 flex flex-col min-w-0 min-h-0">
        <header className="flex items-center justify-between h-12 px-4 border-b border-white/[0.05] bg-[#0b0f19]/90 backdrop-blur-md flex-shrink-0">
          <div className="flex items-center gap-2.5">
            <button onClick={()=>setSidebar(!sidebar)} className="p-1.5 rounded-lg hover:bg-white/[0.06] text-[#5a6a8a] hover:text-white transition-colors" title={sidebar?'收起对话栏':'展开对话栏'}>
              {sidebar?<PanelLeftClose size={16}/>:<PanelLeft size={16}/>}
            </button>
            <Shield size={16} color="#6c5ce7"/>
            <span className="font-semibold text-[13px] text-white tracking-tight">VERIDACTUS <span className="text-[#6c5ce7] font-medium">Chat</span></span>
          </div>
          <div className="flex items-center gap-2">
            {/* 流水线选择器 */}
            {pipelines.length > 0 && (
              <div className="relative">
                <button onClick={()=>setShowPipelines(!showPipelines)} className="flex items-center gap-1.5 h-8 px-3 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.06] text-white text-[11px] font-medium cursor-pointer transition-all">
                  <ShieldCheck size={12} color="#00d4aa"/>{selPipeline?.name||'默认'}<ChevronDown size={10}/>
                </button>
                <AnimatePresence>{showPipelines&&(
                  <motion.div initial={{opacity:0,y:-4}} animate={{opacity:1,y:0}} exit={{opacity:0,y:-4}}
                    className="absolute top-full right-0 mt-1.5 bg-[#0f1326] border border-white/[0.08] rounded-xl p-1.5 min-w-[200px] z-50 shadow-[0_16px_48px_rgba(0,0,0,0.6)]">
                    {pipelines.map(pl=>(<div key={pl.plan_id} onClick={async()=>{
                      setSelPipeline(pl);setShowPipelines(false);
                      // 推送选中的 pipeline 到 DP，确保 DP 有该配置
                      try{
                        const r=await fetch('/api/v1/pipelines/'+pl.plan_id);
                        if(r.ok){
                          const full=await r.json();
                          // 修复 config 字段：DP 期望 JSON 对象而非 JSON 字符串
                          const stages=(full.stages||[]).map((s:any)=>({
                            placement:s.placement, parallel:s.parallel,
                            plugins:(s.plugins||[]).map((p:any)=>({
                              name:p.name, type:p.type,
                              config: (()=>{try{return JSON.parse(p.config||'{}')}catch{return p.config||{}}})()
                            }))
                          }));
                          await fetch('/v1/admin/config/sync',{method:'POST',headers:{'Content-Type':'application/json'},
                            body:JSON.stringify({change_type:'pipeline',data:[{plan_id:full.plan_id,tenant:full.tenant,stages}]})}).catch(()=>{});
                        }
                      }catch{}
                    }}
                      className={`flex items-center gap-2 py-2 px-3 rounded-lg cursor-pointer text-[11px] transition-all ${
                        pl.plan_id===selPipeline?.plan_id?'bg-[rgba(0,212,170,0.1)] text-white':'text-[#8892b0] hover:bg-white/[0.04] hover:text-white'}`}>
                      <span className="w-1.5 h-1.5 rounded-full" style={{background:pl.status==='published'?'#6c5ce7':'#00d4aa'}}/>
                      <span className="flex-1 truncate">{pl.name}</span>
                      <span className="text-[10px] text-[#4a5568]">{pl.stages}阶段</span>
                    </div>))}
                  </motion.div>
                )}</AnimatePresence>
              </div>
            )}
            {/* 模型选择器 */}
            <div className="relative">
              <button onClick={()=>setShowModels(!showModels)} className="flex items-center gap-1.5 h-8 px-3 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.06] text-white text-[12px] font-medium cursor-pointer transition-all">
                <span className="w-2 h-2 rounded-full" style={{background:selModel.color}}/>{selModel.name}<ChevronDown size={11}/>
              </button>
              <AnimatePresence>{showModels&&(
                <motion.div initial={{opacity:0,y:-4,scale:0.96}} animate={{opacity:1,y:0,scale:1}} exit={{opacity:0,y:-4,scale:0.96}}
                  className="absolute top-full right-0 mt-1.5 bg-[#0f1326] border border-white/[0.08] rounded-xl p-1.5 min-w-[190px] z-50 shadow-[0_16px_48px_rgba(0,0,0,0.6)]">
                  {models.map(m=>(<div key={m.id} onClick={()=>{setSelModel(m);setShowModels(false)}}
                    className={`flex items-center gap-2.5 py-2 px-3 rounded-lg cursor-pointer text-[12px] transition-all ${
                      m.id===selModel.id?'bg-[rgba(108,92,231,0.12)] text-white':'text-[#8892b0] hover:bg-white/[0.04] hover:text-white'}`}>
                    <span className="w-2 h-2 rounded-full" style={{background:m.color}}/>{m.name}<span className="text-[10px] text-[#5a6a8a] ml-auto">{m.provider}</span>
                  </div>))}
                </motion.div>
              )}</AnimatePresence>
            </div>
          </div>
        </header>

        {/* Messages — 核心渲染区，完整防溢出 */}
        <div className="flex-1 overflow-y-auto overscroll-contain">
          <div className="max-w-[768px] mx-auto px-4 sm:px-6 py-6">
            {messages.length===0&&(
              <div className="flex flex-col items-center justify-center min-h-[50vh]">
                <motion.div animate={{y:[0,-6,0]}} transition={{duration:2.5,repeat:Infinity}}>
                  <Shield size={44} color="#6c5ce7" className="opacity-25"/>
                </motion.div>
                <h2 className="text-[17px] font-semibold text-white/60 mt-5 tracking-tight">VERIDACTUS 安全沙箱</h2>
                <p className="text-[12px] text-[#4a5568] mt-1.5 max-w-[320px] text-center leading-relaxed">每条对话经过 L0 密码学签名审计<br/>确保不可篡改可验证</p>
              </div>
            )}
            {messages.map(msg=>(
              <div key={msg.id} className={`flex flex-col gap-1.5 mb-6 ${msg.role==='user'?'items-end':''}`}>
                <div className={`flex gap-3 ${msg.role==='user'?'flex-row-reverse':''}`}>
                  <div className={`w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0 ${
                    msg.role==='user'?'bg-[rgba(0,212,170,0.12)]':'bg-[rgba(108,92,231,0.1)]'}`}>
                    {msg.role==='user'?<Zap size={12} color="#00d4aa"/>:<Shield size={12} color="#6c5ce7"/>}
                  </div>
                  <div className={`min-w-0 ${msg.role==='user'?'max-w-[80%]':'max-w-[85%]'}`}>
                    <div className={`px-4 py-3 rounded-2xl text-[14px] leading-[1.65] break-words [overflow-wrap:anywhere] ${
                      msg.role==='user'?'bg-[#6c5ce7]/15 border border-[#6c5ce7]/20 text-[#e2e8f0] rounded-tr-lg':'bg-white/[0.025] border border-white/[0.05] text-[#d4dce6] rounded-tl-lg'}`}>
                      <div className="whitespace-pre-wrap max-w-full">
                        {msg.content||(msg.role==='assistant'&&streaming
                          ?<span className="inline-flex gap-1.5 items-center"><span className="w-1.5 h-1.5 rounded-full bg-[#6c5ce7] animate-pulse"/><span className="w-1.5 h-1.5 rounded-full bg-[#6c5ce7] animate-pulse" style={{animationDelay:'0.15s'}}/><span className="w-1.5 h-1.5 rounded-full bg-[#6c5ce7] animate-pulse" style={{animationDelay:'0.3s'}}/></span>
                          :null)}
                      </div>
                    </div>
                    <div className="flex items-center gap-2 mt-1.5 px-1">
                      {msg.model&&<span className="text-[10px] text-[#4a5568] font-medium">{msg.model}</span>}
                      {msg.tokens?<span className="text-[10px] text-[#3a4568]">{msg.tokens} tokens</span>:null}
                    </div>
                  </div>
                </div>
                {/* VERIDACTUS 治理流水线执行面板 */}
                {msg.role==='assistant' && msg.pipeline && (
                  <motion.div initial={{opacity:0,y:4}} animate={{opacity:1,y:0}} transition={{delay:0.2}}
                    className="ml-10 mr-0 max-w-[85%]"
                    style={{
                      background:'linear-gradient(135deg, rgba(108,92,231,0.06) 0%, rgba(0,212,170,0.04) 100%)',
                      border:'1px solid rgba(108,92,231,0.15)', borderRadius:10, padding:'8px 12px',
                      display:'flex', flexWrap:'wrap', gap:10, alignItems:'center',
                    }}>
                    {/* 证明级别 */}
                    <div style={{display:'flex',alignItems:'center',gap:4}}>
                      <ShieldCheck size={12} color={msg.pipeline.proofLevels.length>0?'#00d4aa':'#4a5568'}/>
                      <span style={{fontSize:10,color:'#8892b0',fontWeight:600}}>L0 审计</span>
                      <span style={{fontSize:9,color:msg.pipeline.proofLevels.length>0?'#00d4aa':'#4a5568',fontWeight:700}}>
                        {msg.pipeline.proofLevels.length>0?'✓ 已签名':'—'}
                      </span>
                    </div>
                    {/* 预算消耗 */}
                    <div style={{display:'flex',alignItems:'center',gap:4}}>
                      <DollarSign size={12} color={msg.pipeline.costConsumed>0?'#fdcb6e':'#4a5568'}/>
                      <span style={{fontSize:10,color:'#8892b0',fontWeight:600}}>费用</span>
                      <span style={{fontSize:9,color:'#fdcb6e',fontWeight:700}}>
                        {msg.pipeline.costConsumed>0?`$${msg.pipeline.costConsumed.toFixed(6)}`:'—'}
                      </span>
                    </div>
                    {/* 预算剩余 */}
                    {msg.pipeline.budgetRemaining !== undefined && (
                      <div style={{display:'flex',alignItems:'center',gap:4}}>
                        <Activity size={12} color={msg.pipeline.budgetRemaining!>0?'#74b9ff':'#ff7675'}/>
                        <span style={{fontSize:10,color:'#8892b0',fontWeight:600}}>预算剩余</span>
                        <span style={{fontSize:9,color:msg.pipeline.budgetRemaining!>0?'#74b9ff':'#ff7675',fontWeight:700}}>
                          ${msg.pipeline.budgetRemaining!.toFixed(4)}
                        </span>
                      </div>
                    )}
                    {/* Trace ID (可点击查看) */}
                    <div style={{display:'flex',alignItems:'center',gap:4}}>
                      <Fingerprint size={12} color="#6c5ce7"/>
                      <span style={{fontSize:10,color:'#8892b0',fontWeight:600}}>Trace</span>
                      <a href={`/vault/${msg.pipeline.traceId}`} target="_blank" rel="noreferrer"
                        style={{fontSize:9,color:'#6c5ce7',fontWeight:700,textDecoration:'none',
                          fontFamily:'monospace',background:'rgba(108,92,231,0.1)',padding:'1px 6px',borderRadius:4}}>
                        {msg.pipeline.traceId.slice(0,12)}...
                      </a>
                    </div>
                    {/* 协议版本 */}
                    <div style={{display:'flex',alignItems:'center',gap:4}}>
                      <CheckCircle size={12} color="#a29bfe"/>
                      <span style={{fontSize:10,color:'#8892b0',fontWeight:600}}>协议</span>
                      <span style={{fontSize:9,color:'#a29bfe',fontWeight:700}}>v{msg.pipeline.version}</span>
                    </div>
                  </motion.div>
                )}
              </div>
            ))}
            <div ref={endRef} className="h-4"/>
          </div>
        </div>

        {/* Input Bar + Footer */}
        <div className="border-t border-white/[0.04] bg-[#0b0f19]/95 backdrop-blur-md px-4 py-3 flex-shrink-0">
          <div className="max-w-[768px] mx-auto flex items-end gap-2.5">
            <div className="flex-1 relative">
              <textarea value={input} onChange={e=>setInput(e.target.value)} onKeyDown={kd}
                placeholder="发送消息..."
                className="w-full py-2.5 px-4 rounded-2xl bg-white/[0.03] border border-white/[0.06] hover:border-white/[0.12] focus:border-[#6c5ce7]/40 text-[14px] text-[#e2e8f0] placeholder:text-[#4a5568] outline-none resize-none transition-all duration-150 font-sans"
                style={{minHeight:46,maxHeight:160}} rows={1}/>
              <SafetyShield text={input}/>
            </div>
            {streaming?(
              <button onClick={()=>{abortRef.current?.abort();setStreaming(false);}} className="w-[42px] h-[42px] rounded-xl bg-[rgba(255,107,107,0.1)] hover:bg-[rgba(255,107,107,0.18)] flex items-center justify-center flex-shrink-0 transition-colors cursor-pointer border-none"><div className="w-3 h-3 bg-[#ff7675] rounded-sm"/></button>
            ):(
              <button onClick={send} disabled={!input.trim()} className="w-[42px] h-[42px] rounded-xl bg-[#5b4ae0] hover:bg-[#6c5ce7] disabled:bg-white/[0.04] disabled:cursor-not-allowed flex items-center justify-center flex-shrink-0 transition-all duration-150 cursor-pointer border-none"><Send size={15} className="text-white"/></button>
            )}
          </div>
          <div className="max-w-[768px] mx-auto mt-2">
            <p className="text-[10px] text-[#3a4568] text-center">VERIDACTUS 安全沙箱 · L0 密码学签名审计</p>
          </div>
        </div>
      </div>
    </div>
  );
}
