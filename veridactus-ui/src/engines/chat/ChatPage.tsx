// VERIDACTUS Chat — 多会话安全沙箱（Tailwind + 响应式）
import { useState, useRef, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Send, Shield, Zap, ChevronDown, Plus, Trash2, MessageSquare, PanelLeftClose, PanelLeft } from 'lucide-react';
import SafetyShield from './SafetyShield';
import { getStoredToken } from '../../auth/useAuth';

// ==================== Types ====================
interface Message { id: string; role: 'user'|'assistant'; content: string; model?: string; tokens?: number; timestamp: number; }
interface AvailableModel { id: string; name: string; provider: string; color: string; is_default: boolean; }
interface Conversation { id: string; title: string; model: string; created_at: string; updated_at: string; }

// ==================== Constants ====================
const MODEL_COLORS = ['#6c5ce7','#00d4aa','#74b9ff','#fdcb6e','#ff7675','#a29bfe','#fd79a8','#00cec9'];
const FALLBACK_MODELS: AvailableModel[] = [{ id:'glm-5.1', name:'GLM-5.1', provider:'Zhipu', color:'#6c5ce7', is_default:true }];
const MSG_CACHE = 'veridactus_chat_msgs';
const CONV_CACHE = 'veridactus_active_conv';

function genId(){ return Date.now().toString(36)+Math.random().toString(36).slice(2); }
function loadMsgs(): Message[] { try { const r=localStorage.getItem(MSG_CACHE); return r?JSON.parse(r):[]; }catch{return[];} }
function saveMsgs(msgs:Message[]){ try{localStorage.setItem(MSG_CACHE,JSON.stringify(msgs.slice(-80)));}catch{} }
function loadConvId(): string|null { return localStorage.getItem(CONV_CACHE); }
function saveConvId(id:string){ localStorage.setItem(CONV_CACHE,id); }

async function fetchModels(): Promise<AvailableModel[]> {
  try{const r=await fetch('/models');if(!r.ok)throw new Error();const d=await r.json();
    return(d.data||[]).map((m:any,i:number)=>({id:m.id,name:m.id,provider:m.owned_by||'',color:MODEL_COLORS[i%MODEL_COLORS.length],is_default:!!m.is_default}));}catch{}
  try{const r=await fetch('/api/v1/models');if(!r.ok)throw new Error();const d=await r.json();
    return(d.models||[]).map((m:any,i:number)=>({id:m.name,name:m.name,provider:'',color:MODEL_COLORS[i%MODEL_COLORS.length],is_default:!!m.is_default}));}catch{return FALLBACK_MODELS;}
}

async function fetchConversations(token:string): Promise<Conversation[]> {
  try{const r=await fetch('/api/v1/conversations',{headers:{Authorization:`Bearer ${token}`}});if(!r.ok)return[];const d=await r.json();return d.conversations||[];}catch{return[];}
}
async function createConv(token:string,title:string,model:string): Promise<Conversation|null> {
  try{const r=await fetch('/api/v1/conversations',{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`},body:JSON.stringify({title,model})});if(!r.ok)return null;return r.json();}catch{return null;}
}
async function deleteConvAPI(token:string,id:string){ try{await fetch('/api/v1/conversations/'+id,{method:'DELETE',headers:{Authorization:`Bearer ${token}`}});}catch{} }
async function updateConvTitle(token:string,id:string,title:string){ try{await fetch('/api/v1/conversations/'+id,{method:'PUT',headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`},body:JSON.stringify({title})});}catch{} }
async function saveMsg(token:string,convId:string,msg:Message){ try{await fetch('/api/v1/conversations/'+convId+'/messages',{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${token}`},body:JSON.stringify({id:msg.id,role:msg.role,content:msg.content,model:msg.model||'',tokens:msg.tokens||0,timestamp:msg.timestamp})});}catch{} }

// ==================== Component ====================
export default function ChatPage() {
  const [messages, setMessages] = useState<Message[]>(()=>loadMsgs());
  const [input, setInput] = useState('');
  const [models, setModels] = useState<AvailableModel[]>(FALLBACK_MODELS);
  const [selectedModel, setSelectedModel] = useState<AvailableModel>(FALLBACK_MODELS[0]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [showModelMenu, setShowModelMenu] = useState(false);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeConvId, setActiveConvId] = useState<string|null>(()=>loadConvId());
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [hoverDelId, setHoverDelId] = useState<string|null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const abortRef = useRef<AbortController|null>(null);
  const modelsRef = useRef(models); // ref 避免 useCallback 依赖漂移
  modelsRef.current = models;
  const creatingRef = useRef(false); // 防止重复创建会话
  const token = getStoredToken() || '';

  useEffect(()=>{ fetchModels().then(m=>{setModels(m);const d=m.find(x=>x.is_default)||m[0];setSelectedModel(d);}); },[]);
  useEffect(()=>{ if(token) fetchConversations(token).then(setConversations); },[token]);
  useEffect(()=>{ messagesEndRef.current?.scrollIntoView({behavior:'smooth'}); saveMsgs(messages); },[messages]);

  // ============ New Chat ============
  const handleNewChat = useCallback(async () => {
    if (isStreaming || creatingRef.current) return;
    creatingRef.current = true;
    try {
      setMessages([]); setActiveConvId(null); saveConvId(''); saveMsgs([]);
      if (token) {
        const conv = await createConv(token, '新对话', selectedModel.id);
        if (conv) { setActiveConvId(conv.id); saveConvId(conv.id); setConversations(await fetchConversations(token)); }
      }
    } finally { creatingRef.current = false; }
  }, [isStreaming, token, selectedModel.id]);

  // ============ Switch Conversation ============
  const handleSwitchConv = useCallback(async (conv: Conversation) => {
    if (isStreaming) return;
    setActiveConvId(conv.id); saveConvId(conv.id);
    // Load messages from server
    try {
      const r = await fetch('/api/v1/conversations/'+conv.id, {headers:{Authorization:`Bearer ${token}`}});
      if (r.ok) {
        const d = await r.json();
        const msgs: Message[] = (d.messages||[]).map((m:any)=>({id:m.id,role:m.role,content:m.content,model:m.model||'',tokens:m.tokens||0,timestamp:Date.now()}));
        setMessages(msgs); saveMsgs(msgs);
        if (d.conversation?.model) {
          const fm = modelsRef.current.find(x=>x.id===d.conversation.model);
          if (fm) setSelectedModel(fm);
        }
      }
    } catch {}
  }, [isStreaming, token]); // 移除 models 依赖，用 ref

  // ============ Delete ============
  const handleDeleteConv = useCallback(async (id:string, e:React.MouseEvent) => {
    e.stopPropagation(); if(!token) return;
    await deleteConvAPI(token, id);
    setConversations(prev=>prev.filter(c=>c.id!==id));
    if(activeConvId===id){ setMessages([]); setActiveConvId(null); saveConvId(''); saveMsgs([]); }
  }, [token, activeConvId]);

  // ============ Send ============
  const handleSend = useCallback(async () => {
    if (!input.trim() || isStreaming) return;
    const userContent = input.trim();
    const userMsg: Message = { id:genId(), role:'user', content:userContent, timestamp:Date.now() };
    const assistMsg: Message = { id:genId(), role:'assistant', content:'', model:selectedModel.id, timestamp:Date.now() };
    const newMsgs = [...messages, userMsg, assistMsg];
    setMessages(newMsgs); setInput(''); setIsStreaming(true);

    // 仅在无活动会话时创建（handleNewChat 已创建则复用）
    let convId = activeConvId;
    if (!convId && token && !creatingRef.current) {
      const title = userContent.length>30?userContent.slice(0,30)+'...':userContent;
      const conv = await createConv(token, title, selectedModel.id);
      if (conv) { convId = conv.id; setActiveConvId(conv.id); saveConvId(conv.id); setConversations(await fetchConversations(token)); }
    }
    // 保存用户消息到服务端
    if (convId && token) saveMsg(token, convId, userMsg);

    const controller = new AbortController(); abortRef.current = controller;
    try {
      const res = await fetch('/v1/chat/completions', {
        method:'POST', signal:controller.signal,
        headers:{'Content-Type':'application/json',...(token?{Authorization:`Bearer ${token}`}:{})},
        body:JSON.stringify({model:selectedModel.id, messages:messages.concat(userMsg).map(m=>({role:m.role,content:m.content})),stream:true,max_tokens:4096}),
      });
      if(!res.ok) throw new Error(`HTTP ${res.status}`);
      const reader = res.body?.getReader(); if(!reader) throw new Error('No reader');
      const decoder = new TextDecoder(); let fullContent = '', doneFlag = false;
      while(true){
        const {done,value}=await reader.read(); if(done)break;
        const chunk = decoder.decode(value,{stream:true});
        for(const line of chunk.split('\n')){
          const data = line.replace(/^(data: )+/,'').trim();
          if(!data) continue;
          if(data==='[DONE]'){ doneFlag=true; break; }
          if(data.startsWith('[VERIDACTUS:BUDGET_EXCEEDED]')){ fullContent+='\n\n⚠️ _预算已耗尽_'; doneFlag=true; break; }
          try { fullContent += JSON.parse(data).choices?.[0]?.delta?.content||''; } catch {}
        }
        if(doneFlag) break;
        setMessages(prev=>prev.map(m=>m.id===assistMsg.id?{...m,content:fullContent}:m));
      }
      const tokens = Math.ceil(fullContent.length/4);
      setMessages(prev=>prev.map(m=>m.id===assistMsg.id?{...m,content:fullContent,tokens,safety:'safe'}:m));
      // 保存到服务端 + 更新标题
      if (convId && token) {
        saveMsg(token, convId, {...assistMsg, content:fullContent, tokens});
        const title = userContent.length>30?userContent.slice(0,30)+'...':userContent;
        updateConvTitle(token, convId, title);
        setConversations(prev=>prev.map(c=>c.id===convId?{...c,title,updated_at:new Date().toISOString()}:c));
      }
    } catch(err:any){ if(err.name!=='AbortError') setMessages(prev=>prev.map(m=>m.id===assistMsg.id?{...m,content:`❌ ${err.message}`}:m)); }
    finally { setIsStreaming(false); abortRef.current=null; }
  },[input,isStreaming,messages,selectedModel,activeConvId,token]);

  const handleKeyDown = (e:React.KeyboardEvent)=>{ if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();handleSend();} };
  const handleStop = ()=>{ abortRef.current?.abort(); setIsStreaming(false); };

  return (
    <div className="flex h-full font-sans" style={{ background: 'linear-gradient(180deg, #0B0F19 0%, #131633 100%)' }}>
      {/* Sidebar */}
      <AnimatePresence>
        {sidebarOpen && (
          <motion.aside initial={{width:0}} animate={{width:260}} exit={{width:0}}
            className="flex-shrink-0 border-r border-white/[0.06] bg-[rgba(10,14,39,0.6)] backdrop-blur-[12px] flex flex-col h-full overflow-hidden">
            <div className="p-3 border-b border-white/[0.06]">
              <button onClick={handleNewChat}
                className="w-full flex items-center gap-2 py-2.5 px-3 rounded-btn bg-[#6c5ce7] hover:bg-[#5a4bd1] text-white text-sm font-semibold cursor-pointer transition-colors">
                <Plus size={16}/> 新建对话
              </button>
            </div>
            <div className="flex-1 overflow-y-auto p-2">
              {conversations.length===0 && (
                <p className="text-center text-[11px] text-[#5a6a8a] mt-8 px-4">暂无对话<br/>点击「新建对话」开始</p>
              )}
              {conversations.map(conv=>(
                <div key={conv.id} onClick={()=>handleSwitchConv(conv)}
                  onMouseEnter={()=>setHoverDelId(conv.id)} onMouseLeave={()=>setHoverDelId(null)}
                  className={`flex items-center gap-2 py-2.5 px-3 rounded-lg cursor-pointer text-sm transition-all mb-0.5 group ${
                    conv.id===activeConvId?'bg-[rgba(108,92,231,0.15)] text-white':'text-[#8892b0] hover:bg-white/[0.04] hover:text-white'}`}>
                  <MessageSquare size={14} className="flex-shrink-0 opacity-60" style={{color:conv.id===activeConvId?'#6c5ce7':undefined}}/>
                  <span className="flex-1 truncate text-[12px]">{conv.title}</span>
                  {hoverDelId===conv.id && (
                    <button onClick={(e)=>handleDeleteConv(conv.id,e)}
                      className="p-1 rounded hover:bg-[rgba(255,107,107,0.15)] text-[#5a6a8a] hover:text-[#ff7675] transition-colors">
                      <Trash2 size={12}/>
                    </button>
                  )}
                </div>
              ))}
            </div>
          </motion.aside>
        )}
      </AnimatePresence>

      {/* Main */}
      <div className="flex-1 flex flex-col min-w-0">
        <header className="flex items-center justify-between px-3 sm:px-4 py-2.5 border-b border-white/[0.06] bg-[rgba(19,22,51,0.8)] backdrop-blur-[12px] flex-shrink-0 gap-2">
          <div className="flex items-center gap-2">
            <button onClick={()=>setSidebarOpen(!sidebarOpen)} className="p-1 rounded-lg hover:bg-white/[0.06] text-[#8892b0] hover:text-white transition-colors">
              {sidebarOpen?<PanelLeftClose size={18}/>:<PanelLeft size={18}/>}
            </button>
            <Shield size={20} color="#6c5ce7"/>
            <span className="font-bold text-sm text-white hidden sm:inline">VERIDACTUS <span className="text-[#6c5ce7]">Chat</span></span>
          </div>
          <div className="relative">
            <button onClick={()=>setShowModelMenu(!showModelMenu)} className="flex items-center gap-1.5 py-1.5 px-3 rounded-btn bg-[rgba(108,92,231,0.12)] border border-[rgba(108,92,231,0.3)] text-white text-xs font-semibold cursor-pointer">
              <span className="w-2 h-2 rounded-full" style={{background:selectedModel.color}}/>{selectedModel.name}<ChevronDown size={12}/>
            </button>
            <AnimatePresence>{showModelMenu && (
              <motion.div initial={{opacity:0,y:-4}} animate={{opacity:1,y:0}} exit={{opacity:0}}
                className="absolute top-full right-0 mt-1 bg-[rgba(19,22,51,0.98)] border border-[rgba(108,92,231,0.3)] rounded-xl p-1.5 min-w-[180px] z-[100] shadow-[0_20px_40px_rgba(0,0,0,0.5)]">
                {models.map(m=>(<div key={m.id} onClick={()=>{setSelectedModel(m);setShowModelMenu(false)}}
                  className={`flex items-center gap-2 py-2 px-3 rounded-lg cursor-pointer text-xs text-white transition-all ${m.id===selectedModel.id?'bg-[rgba(108,92,231,0.15)]':'bg-transparent'}`}>
                  <span className="w-2 h-2 rounded-full" style={{background:m.color}}/>{m.name}<span className="text-[10px] text-[#8892b0] ml-auto">{m.provider}</span>
                </div>))}
              </motion.div>
            )}</AnimatePresence>
          </div>
        </header>

        <div className="flex-1 overflow-y-auto py-4">
          <div className="max-w-[800px] mx-auto px-4">
            <AnimatePresence>
              {messages.length===0 && (
                <motion.div initial={{opacity:0,y:20}} animate={{opacity:1,y:0}} className="text-center pt-[12vh]">
                  <motion.div animate={{y:[0,-8,0]}} transition={{duration:2,repeat:Infinity}}>
                    <Shield size={56} color="#6c5ce7" className="mx-auto opacity-40"/>
                  </motion.div>
                  <h2 className="text-xl font-bold text-white mt-4">VERIDACTUS 安全沙箱</h2>
                  <p className="text-sm text-[#8892b0] mt-1.5 max-w-[380px] mx-auto">每个对话经过 L0 密码学签名审计，确保不可篡改</p>
                </motion.div>
              )}
            </AnimatePresence>
            {messages.map(msg=>(
              <motion.div key={msg.id} initial={{opacity:0,y:12}} animate={{opacity:1,y:0}}
                className={`flex gap-2.5 mb-5 ${msg.role==='user'?'justify-end':'justify-start'}`}>
                {msg.role==='assistant' && <div className="w-7 h-7 rounded-lg bg-[rgba(108,92,231,0.15)] flex items-center justify-center flex-shrink-0 mt-1"><Shield size={12} color="#6c5ce7"/></div>}
                <div className={`max-w-[78%] sm:max-w-[72%] ${msg.role==='user'?'order-first':''}`}>
                  <div className={`py-2.5 px-3.5 rounded-xl text-sm leading-relaxed whitespace-pre-wrap ${
                    msg.role==='user'?'bg-[rgba(108,92,231,0.2)] border border-[rgba(108,92,231,0.3)] text-[#e0e6f0] rounded-br-md':
                      'bg-[rgba(255,255,255,0.03)] border border-white/[0.06] text-[#e0e6f0] rounded-bl-md'}`}>
                    {msg.content||(msg.role==='assistant'&&isStreaming?(
                      <span className="flex gap-1 text-[#6c5ce7]">
                        <motion.span animate={{opacity:[0.3,1,0.3]}} transition={{duration:1,repeat:Infinity}}>●</motion.span>
                        <motion.span animate={{opacity:[0.3,1,0.3]}} transition={{duration:1,delay:0.2,repeat:Infinity}}>●</motion.span>
                        <motion.span animate={{opacity:[0.3,1,0.3]}} transition={{duration:1,delay:0.4,repeat:Infinity}}>●</motion.span>
                      </span>):null)}
                  </div>
                  {msg.model && <span className="text-[10px] text-[#5a6a8a] mt-1 block">{msg.model}{msg.tokens?` · ${msg.tokens} tokens`:''}</span>}
                </div>
                {msg.role==='user' && <div className="w-7 h-7 rounded-lg bg-[rgba(0,212,170,0.15)] flex items-center justify-center flex-shrink-0 mt-1"><Zap size={12} color="#00d4aa"/></div>}
              </motion.div>
            ))}
            <div ref={messagesEndRef}/>
          </div>
        </div>

        <div className="border-t border-white/[0.06] bg-[rgba(19,22,51,0.8)] backdrop-blur-[12px] p-3 flex-shrink-0">
          <div className="max-w-[800px] mx-auto flex gap-2 items-center">
            <SafetyShield text={input}/>
            <div className="flex-1 relative">
              <textarea value={input} onChange={e=>setInput(e.target.value)} onKeyDown={handleKeyDown}
                placeholder="输入消息... (Enter 发送)"
                className="w-full p-2.5 rounded-btn bg-white/[0.04] border border-white/[0.08] text-sm text-[#e2e8f0] outline-none resize-none font-sans"
                style={{minHeight:42,maxHeight:120}} rows={1}/>
            </div>
            {isStreaming?(
              <button onClick={handleStop} className="p-2.5 rounded-btn bg-[rgba(255,107,107,0.15)] border-none cursor-pointer"><div className="w-3 h-3 bg-[#ff7675] rounded-sm"/></button>
            ):(
              <button onClick={handleSend} disabled={!input.trim()} className="p-2.5 rounded-btn bg-[#6c5ce7] border-none cursor-pointer text-white disabled:opacity-40 flex-shrink-0"><Send size={16}/></button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
