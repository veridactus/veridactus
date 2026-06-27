// VERIDACTUS Chat — 安全沙箱对话（Tailwind + 响应式）
import { useState, useRef, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Send, Shield, Zap, Activity, ChevronDown, Columns } from 'lucide-react';
import SafetyShield from './SafetyShield';
import ABCompare from './ABCompare';
import { getStoredToken } from '../../auth/useAuth';

interface Message {
  id: string; role: 'user'|'assistant'|'system'; content: string;
  model?: string; tokens?: number; cost?: number; safety?: 'safe'|'flagged'|'blocked'; timestamp: number;
}
interface AvailableModel { id: string; name: string; provider: string; color: string; is_default: boolean; }
const MODEL_COLORS = ['#6c5ce7','#00d4aa','#74b9ff','#fdcb6e','#ff7675','#a29bfe','#fd79a8','#00cec9'];
const FALLBACK_MODELS: AvailableModel[] = [
  { id:'glm-5.1', name:'GLM-5.1', provider:'Zhipu', color:'#6c5ce7', is_default:true },
];
const CHAT_STORAGE_KEY = 'veridactus_chat_messages';

function generateId(){ return Date.now().toString(36)+Math.random().toString(36).slice(2); }
/** 从 localStorage 加载/保存聊天记录 */
function loadMessages(): Message[] {
  try { const raw = localStorage.getItem(CHAT_STORAGE_KEY); return raw ? JSON.parse(raw) : []; } catch { return []; }
}
function saveMessages(msgs: Message[]) {
  try { localStorage.setItem(CHAT_STORAGE_KEY, JSON.stringify(msgs.slice(-80))); } catch {} // 最多保留 80 条
}
async function fetchAvailableModels(): Promise<AvailableModel[]> {
  try { const r = await fetch('/models'); if (!r.ok) throw new Error(); const data = await r.json();
    return (data.data || []).map((m: any, i: number) => ({ id: m.id, name: m.id, provider: m.owned_by || '', color: MODEL_COLORS[i % MODEL_COLORS.length], is_default: !!m.is_default })); } catch {}
  try { const r = await fetch('/api/v1/models'); if (!r.ok) throw new Error(); const data = await r.json();
    return (data.models || []).map((m: any, i: number) => ({ id: m.name, name: m.name, provider: '', color: MODEL_COLORS[i % MODEL_COLORS.length], is_default: !!m.is_default })); } catch { return FALLBACK_MODELS; }
}

export default function ChatPage() {
  const [messages, setMessages] = useState<Message[]>(() => loadMessages());
  const [input, setInput] = useState('');
  const [availableModels, setAvailableModels] = useState<AvailableModel[]>(FALLBACK_MODELS);
  const [selectedModel, setSelectedModel] = useState<AvailableModel>(FALLBACK_MODELS[0]);
  const [compareModel, setCompareModel] = useState<AvailableModel>(FALLBACK_MODELS[0]);
  const [compareMode, setCompareMode] = useState(false);
  const [comparePrompt, setComparePrompt] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [showModelMenu, setShowModelMenu] = useState(false);
  const [showCompareMenu, setShowCompareMenu] = useState(false);
  const [budgetRemaining, setBudgetRemaining] = useState<number|null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const abortRef = useRef<AbortController|null>(null);

  // 每次 message 变化自动滚动 + 持久化到 localStorage
  useEffect(() => { messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' }); saveMessages(messages); }, [messages]);
  useEffect(() => {
    fetchAvailableModels().then(models => {
      setAvailableModels(models);
      if (models.length > 0) {
        const def = models.find(m => m.is_default) || models[0];
        setSelectedModel(def);
        setCompareModel(models.length > 1 ? models[1] : models[0]);
      }
    });
  }, []);

  const handleSend = useCallback(async () => {
    if (!input.trim() || isStreaming) return;
    const token = getStoredToken();
    const userMsg: Message = { id:generateId(), role:'user', content:input.trim(), timestamp:Date.now() };
    const assistantMsg: Message = { id:generateId(), role:'assistant', content:'', model:selectedModel.id, timestamp:Date.now() };
    const newMessages = [...messages, userMsg, assistantMsg];
    setMessages(newMessages); setInput(''); setIsStreaming(true);
    const controller = new AbortController(); abortRef.current = controller;
    try {
      const res = await fetch('/v1/chat/completions', {
        method:'POST', signal:controller.signal,
        headers:{'Content-Type':'application/json',...(token?{Authorization:`Bearer ${token}`}:{})},
        body:JSON.stringify({model:selectedModel.id, messages:messages.concat(userMsg).map(m=>({role:m.role,content:m.content})),stream:true,max_tokens:4096}),
      });
      if(!res.ok) throw new Error(`HTTP ${res.status}`);
      const budgetRem = res.headers.get('VERIDACTUS-Budget-Remaining');
      if(budgetRem) setBudgetRemaining(parseFloat(budgetRem));
      const reader = res.body?.getReader(); if(!reader) throw new Error('No reader');
      const decoder = new TextDecoder(); let fullContent = '';
      let doneFlag = false;
      while(true){
        const {done,value}=await reader.read(); if(done)break;
        const chunk = decoder.decode(value,{stream:true});
        for(const line of chunk.split('\n').filter(l=>l.startsWith('data: '))){
          const data = line.slice(6).trim();
          if(data==='[DONE]'){ doneFlag = true; break; }
          if(data.startsWith('[VERIDACTUS:BUDGET_EXCEEDED]')){ fullContent+='\n\n⚠️ _预算已耗尽_'; doneFlag = true; break; }
          if(!data) continue;
          try { fullContent += JSON.parse(data).choices?.[0]?.delta?.content||''; } catch {}
        }
        if(doneFlag) break;
        // 🔧 直接 setMessages，不用 requestAnimationFrame（React 18 中 RAF 可能不触发渲染）
        const snapshot = fullContent;
        setMessages(prev => prev.map(m => m.id===assistantMsg.id ? {...m, content: snapshot} : m));
      }
      setMessages(prev => prev.map(m => m.id===assistantMsg.id ? {...m, content: fullContent, tokens: Math.ceil(fullContent.length/4), safety:'safe'} : m));
    } catch(err:any){ if(err.name!=='AbortError') setMessages(prev => prev.map(m => m.id===assistantMsg.id ? {...m, content:`❌ ${err.message}`} : m)); }
    finally { setIsStreaming(false); abortRef.current=null; }
  },[input,isStreaming,messages,selectedModel]);

  const handleKeyDown = (e:React.KeyboardEvent) => { if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();handleSend();} };
  const handleStop = () => { abortRef.current?.abort(); setIsStreaming(false); };

  return (
    <div className="flex flex-col h-full font-sans"
      style={{ background: 'linear-gradient(180deg, #0B0F19 0%, #131633 100%)' }}>
      {/* Header */}
      <header className="flex items-center justify-between px-4 sm:px-6 py-3 border-b border-white/[0.06] bg-[rgba(19,22,51,0.8)] backdrop-blur-[12px] flex-shrink-0 gap-2">
        <div className="flex items-center gap-3">
          <Shield size={24} color="#6c5ce7"/>
          <span className="font-bold text-base text-white hidden sm:inline">VERIDACTUS <span className="text-[#6c5ce7]">Chat</span></span>
          <span className="text-[10px] px-2 py-0.5 rounded-[10px] bg-[rgba(0,212,170,0.15)] text-[#00d4aa] font-semibold hidden sm:inline">BETA</span>
        </div>
        <div className="flex items-center gap-2">
          {/* A/B Toggle */}
          <button onClick={()=>{if(!compareMode&&input.trim()){setComparePrompt(input.trim());setInput('');}setCompareMode(!compareMode);}}
            title="⚔️ A/B 对比" className={`flex items-center gap-1.5 py-2 px-3.5 rounded-btn text-xs font-semibold cursor-pointer transition-all
              ${compareMode?'bg-[rgba(108,92,231,0.25)] border-[rgba(108,92,231,0.5)] text-[#6c5ce7]':'bg-white/[0.05] border-white/10 text-[#8892b0]'} border`}>
            <Columns size={15}/>A/B
          </button>
          {/* Model A */}
          <div className="relative">
            <button onClick={()=>setShowModelMenu(!showModelMenu)} className="flex items-center gap-2 py-2 px-4 rounded-btn bg-[rgba(108,92,231,0.12)] border border-[rgba(108,92,231,0.3)] text-white text-sm font-semibold cursor-pointer">
              <span className="w-2 h-2 rounded-full" style={{background:selectedModel.color,boxShadow:`0 0 8px ${selectedModel.color}`}}/>{selectedModel.name}<ChevronDown size={14}/>
            </button>
            <AnimatePresence>{showModelMenu && (
              <motion.div initial={{opacity:0,y:-4}} animate={{opacity:1,y:0}} exit={{opacity:0}}
                className="absolute top-full right-0 mt-1 bg-[rgba(19,22,51,0.98)] border border-[rgba(108,92,231,0.3)] rounded-xl p-2 min-w-[200px] z-[100] shadow-[0_20px_40px_rgba(0,0,0,0.5)]">
                {availableModels.map(m=>(<div key={m.id} onClick={()=>{setSelectedModel(m);setShowModelMenu(false)}}
                  className={`flex items-center gap-2.5 p-2.5 rounded-lg cursor-pointer text-sm text-white transition-all ${m.id===selectedModel.id?'bg-[rgba(108,92,231,0.15)]':'bg-transparent'}`}>
                  <span className="w-2 h-2 rounded-full" style={{background:m.color}}/><span className="flex-1">{m.name}</span><span className="text-[10px] text-[#8892b0]">{m.provider}</span>
                </div>))}
              </motion.div>
            )}</AnimatePresence>
          </div>
          {/* Model B (A/B mode only) */}
          {compareMode && (<div className="relative">
            <button onClick={()=>setShowCompareMenu(!showCompareMenu)} className="flex items-center gap-2 py-2 px-4 rounded-btn bg-[rgba(0,212,170,0.12)] border border-[rgba(0,212,170,0.3)] text-white text-sm font-semibold cursor-pointer">
              <span className="w-2 h-2 rounded-full" style={{background:compareModel.color,boxShadow:`0 0 8px ${compareModel.color}`}}/>{compareModel.name}<ChevronDown size={14}/>
            </button>
            <AnimatePresence>{showCompareMenu && (
              <motion.div initial={{opacity:0,y:-4}} animate={{opacity:1,y:0}} exit={{opacity:0}}
                className="absolute top-full right-0 mt-1 bg-[rgba(19,22,51,0.98)] border border-[rgba(0,212,170,0.3)] rounded-xl p-2 min-w-[200px] z-[100] shadow-[0_20px_40px_rgba(0,0,0,0.5)]">
                {availableModels.filter(m=>m.id!==selectedModel.id).map(m=>(<div key={m.id} onClick={()=>{setCompareModel(m);setShowCompareMenu(false)}}
                  className={`flex items-center gap-2.5 p-2.5 rounded-lg cursor-pointer text-sm text-white transition-all ${m.id===compareModel.id?'bg-[rgba(0,212,170,0.15)]':'bg-transparent'}`}>
                  <span className="w-2 h-2 rounded-full" style={{background:m.color}}/><span className="flex-1">{m.name}</span><span className="text-[10px] text-[#8892b0]">{m.provider}</span>
                </div>))}
              </motion.div>
            )}</AnimatePresence>
          </div>)}
        </div>
      </header>

      {/* A/B Overlay */}
      <AnimatePresence>{compareMode && comparePrompt && (
        <ABCompare prompt={comparePrompt} modelA={selectedModel.id} modelB={compareModel.id} onClose={()=>{setCompareMode(false);setComparePrompt('')}}/>
      )}</AnimatePresence>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto py-6 px-0">
        <div className="max-w-[800px] mx-auto px-4 sm:px-6">
          <AnimatePresence>
            {messages.length===0 && (
              <motion.div initial={{opacity:0,y:20}} animate={{opacity:1,y:0}} className="text-center pt-[15vh]">
                <motion.div animate={{y:[0,-8,0]}} transition={{duration:2,repeat:Infinity}}>
                  <Shield size={64} color="#6c5ce7" className="mx-auto opacity-50"/>
                </motion.div>
                <h2 className="text-[22px] font-bold text-white mt-5">VERIDACTUS 安全沙箱</h2>
                <p className="text-sm text-[#8892b0] mt-2 max-w-[400px] mx-auto">每个对话都经过 L0 密码学签名审计，确保不可篡改</p>
              </motion.div>
            )}
          </AnimatePresence>
          {messages.map(msg=>(
            <motion.div key={msg.id} initial={{opacity:0,y:12}} animate={{opacity:1,y:0}}
              className={`flex gap-3 mb-6 ${msg.role==='user'?'justify-end':'justify-start'}`}>
              {msg.role==='assistant' && <div className="w-8 h-8 rounded-xl bg-[rgba(108,92,231,0.15)] flex items-center justify-center flex-shrink-0 mt-1"><Shield size={14} color="#6c5ce7"/></div>}
              <div className={`max-w-[75%] sm:max-w-[70%] ${msg.role==='user'?'order-first':''}`}>
                <div className={`p-3 sm:p-4 rounded-xl text-sm leading-relaxed whitespace-pre-wrap ${
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
              {msg.role==='user' && <div className="w-8 h-8 rounded-xl bg-[rgba(0,212,170,0.15)] flex items-center justify-center flex-shrink-0 mt-1"><Zap size={14} color="#00d4aa"/></div>}
            </motion.div>
          ))}
          <div ref={messagesEndRef}/>
        </div>
      </div>

      {/* Input — 移动端适配 */}
      <div className="border-t border-white/[0.06] bg-[rgba(19,22,51,0.8)] backdrop-blur-[12px] p-3 sm:p-4 flex-shrink-0">
        <div className="max-w-[800px] mx-auto flex gap-2 sm:gap-3 items-center">
          <SafetyShield text={input}/>
          <div className="flex-1 relative">
            <textarea value={input} onChange={e=>setInput(e.target.value)} onKeyDown={handleKeyDown}
              placeholder="输入消息... (Enter 发送)"
              className="w-full p-2.5 sm:p-3 pr-10 rounded-btn bg-white/[0.04] border border-white/[0.08] text-sm text-[#e2e8f0] outline-none resize-none font-sans"
              style={{minHeight:44,maxHeight:120}} rows={1}/>
            {budgetRemaining!==null && (
              <span className="absolute right-3 bottom-2 text-[10px] text-[#00d4aa] font-mono">${budgetRemaining.toFixed(4)}</span>
            )}
          </div>
          {isStreaming?(
            <button onClick={handleStop} className="p-2.5 rounded-btn bg-[rgba(255,107,107,0.15)] border-none cursor-pointer"><div className="w-3 h-3 bg-[#ff7675] rounded-sm"/></button>
          ):(
            <button onClick={handleSend} disabled={!input.trim()} className="p-2.5 rounded-btn bg-[#6c5ce7] border-none cursor-pointer text-white disabled:opacity-40 flex-shrink-0"><Send size={16}/></button>
          )}
        </div>
      </div>
    </div>
  );
}
