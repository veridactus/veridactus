// VERIDACTUS Chat — 专业多会话安全沙箱（业界标准 UI）
import { useState, useRef, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Send, Shield, Zap, ChevronDown, Plus, Trash2, MessageSquare, PanelLeftClose, PanelLeft } from 'lucide-react';
import SafetyShield from './SafetyShield';
import { getStoredToken } from '../../auth/useAuth';
import { useNavSidebarStore } from '../../store';

// ==================== Types ====================
interface Message { id: string; role: 'user'|'assistant'; content: string; model?: string; tokens?: number; timestamp: number; }
interface AvailableModel { id: string; name: string; provider: string; color: string; is_default: boolean; }
interface Conversation { id: string; title: string; model: string; created_at: string; updated_at: string; }

// ==================== Constants ====================
const MODEL_COLORS = ['#6c5ce7','#00d4aa','#74b9ff','#fdcb6e','#ff7675','#a29bfe','#fd79a8','#00cec9'];
const FALLBACK: AvailableModel[] = [{ id:'glm-5.1', name:'GLM-5.1', provider:'Zhipu', color:'#6c5ce7', is_default:true }];
const CACHE_MSGS = 'v_msgs', CACHE_CONV = 'v_conv';

function gid(){ return Date.now().toString(36)+Math.random().toString(36).slice(2); }
function load<T>(k:string,fallback:T):T{try{const r=localStorage.getItem(k);return r?JSON.parse(r):fallback}catch{return fallback}}
function save(k:string,v:any){try{localStorage.setItem(k,JSON.stringify(v))}catch{}}

async function fetchModels(): Promise<AvailableModel[]> {
  try{const r=await fetch('/models');if(r.ok){const d=await r.json();return(d.data||[]).map((m:any,i:number)=>({id:m.id,name:m.id,provider:m.owned_by||'',color:MODEL_COLORS[i%8],is_default:!!m.is_default}));}}catch{}
  try{const r=await fetch('/api/v1/models');if(r.ok){const d=await r.json();return(d.models||[]).map((m:any,i:number)=>({id:m.name,name:m.name,provider:'',color:MODEL_COLORS[i%8],is_default:!!m.is_default}));}}catch{}
  return FALLBACK;
}

// ==================== Component ====================
export default function ChatPage() {
  const [messages, setMessages] = useState<Message[]>(()=>load<Message[]>(CACHE_MSGS,[]));
  const [input, setInput] = useState('');
  const [models, setModels] = useState<AvailableModel[]>(FALLBACK);
  const [selModel, setSelModel] = useState<AvailableModel>(FALLBACK[0]);
  const [streaming, setStreaming] = useState(false);
  const [showModels, setShowModels] = useState(false);
  const [convs, setConvs] = useState<Conversation[]>([]);
  const [activeId, setActiveId] = useState<string|null>(()=>load<string|null>(CACHE_CONV,null));
  const [sidebar, setSidebar] = useState(true);
  const { navCollapsed, toggleNav } = useNavSidebarStore();
  const [hoverDel, setHoverDel] = useState<string|null>(null);
  const endRef = useRef<HTMLDivElement>(null);
  const abortRef = useRef<AbortController|null>(null);
  const modelsRef = useRef(models); modelsRef.current = models;
  const creatingRef = useRef(false);
  const token = getStoredToken()||'';

  // Headers as plain Record to satisfy TypeScript
  const authHeaders: Record<string,string> = token ? { Authorization: `Bearer ${token}` } : {};
  const ctHeaders: Record<string,string> = token ? { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' } : { 'Content-Type': 'application/json' };

  useEffect(()=>{fetchModels().then(m=>{setModels(m);setSelModel(m.find(x=>x.is_default)||m[0]);});},[]);
  useEffect(()=>{
    if(!token)return;
    fetch('/api/v1/conversations',{headers:authHeaders}).then(r=>r.json()).then(d=>d.conversations&&setConvs(d.conversations)).catch(()=>{});
  },[token]);
  useEffect(()=>{endRef.current?.scrollIntoView({behavior:'smooth'});save(CACHE_MSGS,messages.slice(-80));},[messages]);

  const newChat = useCallback(async()=>{
    if(streaming||creatingRef.current)return;creatingRef.current=true;
    try{setMessages([]);setActiveId(null);save(CACHE_CONV,null);save(CACHE_MSGS,[]);
      if(token){const r=await fetch('/api/v1/conversations',{method:'POST',headers:ctHeaders,body:JSON.stringify({title:'新对话',model:selModel.id})});
        if(r.ok){const c=await r.json();setActiveId(c.id);save(CACHE_CONV,c.id);
          fetch('/api/v1/conversations',{headers:authHeaders}).then(r2=>r2.json()).then(d=>d.conversations&&setConvs(d.conversations)).catch(()=>{});}}}finally{creatingRef.current=false;}
  },[streaming,token,selModel.id]);

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

    let cid=activeId;
    if(!cid&&token&&!creatingRef.current){const title=uc.length>30?uc.slice(0,30)+'...':uc;
      const r=await fetch('/api/v1/conversations',{method:'POST',headers:ctHeaders,body:JSON.stringify({title,model:selModel.id})});
      if(r.ok){const c=await r.json();cid=c.id;setActiveId(cid);save(CACHE_CONV,cid);
        fetch('/api/v1/conversations',{headers:authHeaders}).then(r2=>r2.json()).then(d=>d.conversations&&setConvs(d.conversations)).catch(()=>{});}}
    if(cid&&token){fetch('/api/v1/conversations/'+cid+'/messages',{method:'POST',headers:ctHeaders,body:JSON.stringify({id:u.id,role:'user',content:uc,model:selModel.id,tokens:0,timestamp:u.timestamp})}).catch(()=>{});}

    const ctrl=new AbortController();abortRef.current=ctrl;
    try{
      const res=await fetch('/v1/chat/completions',{method:'POST',signal:ctrl.signal,headers:ctHeaders,
        body:JSON.stringify({model:selModel.id,messages:messages.concat(u).map(m=>({role:m.role,content:m.content})),stream:true,max_tokens:4096})});
      if(!res.ok)throw new Error(`HTTP ${res.status}`);
      const reader=res.body?.getReader();if(!reader)throw new Error('No reader');
      const dec=new TextDecoder();let fc='',df=false;
      while(true){const{value,done}=await reader.read();if(done)break;
        for(const line of dec.decode(value,{stream:true}).split('\n')){const d=line.replace(/^(data: )+/,'').trim();if(!d)continue;
          if(d==='[DONE]'){df=true;break;}if(d.startsWith('[BUDGET')){fc+='\n⚠️ 预算耗尽';df=true;break;}
          try{fc+=JSON.parse(d).choices?.[0]?.delta?.content||'';}catch{}}if(df)break;setMessages(p=>p.map(m=>m.id===a.id?{...m,content:fc}:m));}
      const tk=Math.ceil(fc.length/4);
      setMessages(p=>p.map(m=>m.id===a.id?{...m,content:fc,tokens:tk,safety:'safe'}:m));
      if(cid&&token){fetch('/api/v1/conversations/'+cid+'/messages',{method:'POST',headers:ctHeaders,body:JSON.stringify({id:a.id,role:'assistant',content:fc,model:selModel.id,tokens:tk,timestamp:Date.now()})}).catch(()=>{});
        const title=uc.length>30?uc.slice(0,30)+'...':uc;fetch('/api/v1/conversations/'+cid,{method:'PUT',headers:ctHeaders,body:JSON.stringify({title})}).catch(()=>{});
        setConvs(p=>p.map(c=>c.id===cid?{...c,title,updated_at:new Date().toISOString()}:c));}
    }catch(e:any){if(e.name!=='AbortError')setMessages(p=>p.map(m=>m.id===a.id?{...m,content:`❌ ${e.message}`}:m));}
    finally{setStreaming(false);abortRef.current=null;}
  },[input,streaming,messages,selModel,activeId,token]);

  const kd=(e:React.KeyboardEvent)=>{if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();send();}};

  return (
    <div className="flex h-full font-sans antialiased" style={{background:'#0B0F19'}}>
      {/* === Sidebar (ChatGPT-style collapsible) === */}
      <div className={`flex-shrink-0 border-r border-white/[0.06] bg-[#0a0e1a] flex flex-col h-full transition-all duration-300 overflow-hidden ${sidebar?'w-[260px]':'w-0 border-r-0'}`}>
        <div className="flex items-center justify-between p-3 pb-2">
          <button onClick={newChat} className="flex-1 flex items-center gap-2.5 py-2.5 px-3 rounded-xl bg-white/[0.06] hover:bg-white/[0.1] border border-white/[0.08] text-white text-[13px] font-medium cursor-pointer transition-all duration-150">
            <Plus size={15}/> 新建对话
          </button>
          <button onClick={()=>setSidebar(false)} className="ml-1.5 p-1.5 rounded-lg hover:bg-white/[0.06] text-[#5a6a8a] hover:text-white transition-colors flex-shrink-0" title="收起侧栏">
            <PanelLeftClose size={14}/>
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
            <button onClick={toggleNav} className="p-1.5 rounded-lg hover:bg-white/[0.06] text-[#5a6a8a] hover:text-white transition-colors" title={navCollapsed?'展开导航':'收起导航'}>
              {navCollapsed?<PanelLeft size={16}/>:<PanelLeftClose size={16}/>}
            </button>
            <Shield size={16} color="#6c5ce7"/>
            <span className="font-semibold text-[13px] text-white tracking-tight">VERIDACTUS <span className="text-[#6c5ce7] font-medium">Chat</span></span>
          </div>
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
              <div key={msg.id} className={`flex gap-3 mb-6 ${msg.role==='user'?'flex-row-reverse':''}`}>
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
                  {msg.model&&<div className="flex items-center gap-2 mt-1.5 px-1"><span className="text-[10px] text-[#4a5568] font-medium">{msg.model}</span>{msg.tokens?<span className="text-[10px] text-[#3a4568]">{msg.tokens} tokens</span>:null}</div>}
                </div>
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
